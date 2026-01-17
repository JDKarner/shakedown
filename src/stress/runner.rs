//! Stress-ng process runner and manager

use crate::app::{ExecutionMode, GpuBurnConfig, TestCategory};
use log::{debug, error, info, warn};
use nix::sys::signal::{self, Signal};
use nix::unistd::Pid;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process::{self, Child, Command, Stdio};
use std::sync::{Arc, Mutex};
use std::time::Instant;

/// Status of the stress test runner
#[derive(Debug, Clone)]
pub enum TestStatus {
    /// No tests running
    Idle,
    /// Tests are currently running
    Running {
        current_test: Option<TestCategory>,
        tests_completed: usize,
        tests_total: usize,
        start_time: Instant,
        mode: ExecutionMode,
    },
    /// Tests completed successfully
    Completed {
        duration: std::time::Duration,
        tests_run: usize,
    },
    /// Tests were stopped by user
    Stopped,
    /// Tests failed with error
    Failed { error: String },
}

/// Individual test process info
#[derive(Debug)]
struct TestProcess {
    child: Child,
    category: TestCategory,
    start_time: Instant,
}

/// Manages stress-ng test execution
pub struct StressRunner {
    /// Current test status
    status: TestStatus,

    /// Running processes (for parallel mode, multiple; for sequential, one)
    processes: Arc<Mutex<HashMap<TestCategory, TestProcess>>>,

    /// Queue of tests to run (for sequential mode)
    test_queue: Vec<TestCategory>,

    /// Total tests in current run
    total_tests: usize,

    /// Completed tests count
    completed_tests: usize,

    /// Start time of test run
    start_time: Option<Instant>,

    /// Current execution mode
    execution_mode: ExecutionMode,

    /// Path to stress-ng binary
    stress_ng_path: PathBuf,

    /// Path to jobfiles directory
    jobfiles_path: PathBuf,

    /// Path to gpu_burn binary
    gpu_burn_path: PathBuf,

    /// GPU burn configuration
    gpu_burn_config: GpuBurnConfig,
}

impl Default for StressRunner {
    fn default() -> Self {
        Self::new()
    }
}

impl StressRunner {
    pub fn new() -> Self {
        // Get the directory where our binary is located
        let exe_path = std::env::current_exe().unwrap_or_else(|_| PathBuf::from("."));
        let exe_dir = exe_path
            .parent()
            .unwrap_or_else(|| std::path::Path::new("."));

        // stress-ng should be in the same directory as our binary
        let stress_ng_path = exe_dir.join("stress-ng");

        // Jobfiles should be in a jobfiles subdirectory
        let jobfiles_path = exe_dir.join("jobfiles");

        // gpu_burn should be in the same directory as our binary
        let gpu_burn_path = exe_dir.join("gpu_burn");

        info!("StressRunner initialized");
        info!("stress-ng path: {:?}", stress_ng_path);
        info!("Jobfiles path: {:?}", jobfiles_path);
        info!("gpu_burn path: {:?}", gpu_burn_path);

        Self {
            status: TestStatus::Idle,
            processes: Arc::new(Mutex::new(HashMap::new())),
            test_queue: Vec::new(),
            total_tests: 0,
            completed_tests: 0,
            start_time: None,
            execution_mode: ExecutionMode::Sequential,
            stress_ng_path,
            jobfiles_path,
            gpu_burn_path,
            gpu_burn_config: GpuBurnConfig::default(),
        }
    }

    /// Get current status
    pub fn status(&self) -> &TestStatus {
        &self.status
    }

    /// Check if stress-ng is available
    pub fn is_stress_ng_available(&self) -> bool {
        self.stress_ng_path.exists()
    }
    /// Check if gpu-burn is available
    pub fn is_gpu_burn_available(&self) -> bool {
        self.gpu_burn_path.exists()
    }

    /// Get stress-ng version
    pub fn stress_ng_version(&self) -> Option<String> {
        let output = Command::new(&self.stress_ng_path)
            .arg("--version")
            .output()
            .ok()?;

        String::from_utf8(output.stdout)
            .ok()
            .map(|s| s.trim().to_string())
    }

    /// Start stress tests
    pub fn start(&mut self, tests: Vec<TestCategory>, mode: ExecutionMode) -> anyhow::Result<()> {
        if tests.is_empty() {
            return Err(anyhow::anyhow!("No tests selected"));
        }

        // Verify stress-ng exists
        if !self.is_stress_ng_available() {
            return Err(anyhow::anyhow!(
                "stress-ng not found at {:?}. Please run the build script first.",
                self.stress_ng_path
            ));
        }
        // Verify gpu_burn exists
        if !self.is_gpu_burn_available() {
            return Err(anyhow::anyhow!(
                "gpu_burn not found at {:?}. Please run the build script first.",
                self.gpu_burn_path
            ));
        }

        // Verify jobfiles exist for non-GPU burn tests
        for test in &tests {
            if *test != TestCategory::GpuBurn {
                let jobfile = self.jobfiles_path.join(test.jobfile_name());
                if !jobfile.exists() {
                    return Err(anyhow::anyhow!("Jobfile not found: {:?}", jobfile));
                }
            }
        }

        // Verify gpu_burn exists if GPU burn test is selected
        if tests.contains(&TestCategory::GpuBurn) && !self.gpu_burn_path.exists() {
            return Err(anyhow::anyhow!(
                "gpu_burn not found at {:?}. Please ensure gpu_burn binary is available.",
                self.gpu_burn_path
            ));
        }

        info!("Starting {} tests in {:?} mode", tests.len(), mode);

        self.total_tests = tests.len();
        self.completed_tests = 0;
        self.start_time = Some(Instant::now());
        self.execution_mode = mode;

        match mode {
            ExecutionMode::Sequential => {
                self.test_queue = tests;
                self.start_next_sequential_test()?;
            }
            ExecutionMode::Parallel => {
                self.test_queue.clear();
                self.start_parallel_tests(tests)?;
            }
        }

        self.update_running_status();
        Ok(())
    }

    /// Start the next test in sequential mode
    fn start_next_sequential_test(&mut self) -> anyhow::Result<()> {
        if let Some(test) = self.test_queue.first().cloned() {
            // If this test is already running (in processes map), don't start it again.
            if let Ok(processes) = self.processes.lock() {
                if processes.contains_key(&test) {
                    debug!("{} test already running, skipping start", test.name());
                    return Ok(());
                }
            }
            self.start_single_test(test)?;
        }
        Ok(())
    }

    /// Start all tests in parallel
    fn start_parallel_tests(&mut self, tests: Vec<TestCategory>) -> anyhow::Result<()> {
        for test in tests {
            self.start_single_test(test)?;
        }
        Ok(())
    }

    /// Start a single stress-ng or gpu_burn process
    fn start_single_test(&mut self, category: TestCategory) -> anyhow::Result<()> {
        // Ensure we don't spawn the same category twice
        if let Ok(processes) = self.processes.lock() {
            if processes.contains_key(&category) {
                debug!("{} already running, skipping spawn", category.name());
                return Ok(());
            }
        }

        let child = if category == TestCategory::GpuBurn {
            // Start GPU burn process
            info!("Starting {} test", category.name());

            let mut cmd = Command::new(&self.gpu_burn_path);

            // Add memory percentage
            cmd.arg("-m");
            cmd.arg(format!("{}%", self.gpu_burn_config.memory_percent));

            // Add doubles flag if enabled
            if self.gpu_burn_config.use_doubles {
                cmd.arg("-d");
            }

            // Add tensor cores flag if enabled
            if self.gpu_burn_config.use_tensor_cores {
                cmd.arg("-tc");
            }

            // Add duration in seconds (convert hours to seconds)
            let duration_seconds = (self.gpu_burn_config.duration_hours as u32) * 3600;
            cmd.arg(duration_seconds.to_string());

            cmd.stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| anyhow::anyhow!("Failed to spawn gpu_burn: {}", e))?
        } else {
            // Start stress-ng process
            let jobfile = self.jobfiles_path.join(category.jobfile_name());

            info!(
                "Starting {} test with jobfile {:?}",
                category.name(),
                jobfile
            );

            Command::new(&self.stress_ng_path)
                .arg("--job")
                .arg(&jobfile)
                .arg("--yaml")
                .arg("results.yaml")
                .arg("--log-file")
                .arg("stress-ng.log")
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .spawn()
                .map_err(|e| anyhow::anyhow!("Failed to spawn stress-ng: {}", e))?
        };

        let process = TestProcess {
            child,
            category,
            start_time: Instant::now(),
        };

        if let Ok(mut processes) = self.processes.lock() {
            processes.insert(category, process);
        }

        Ok(())
    }

    /// Update GPU burn configuration
    pub fn set_gpu_burn_config(&mut self, config: GpuBurnConfig) {
        self.gpu_burn_config = config;
    }

    /// Update the running status
    fn update_running_status(&mut self) {
        let current_test = if self.execution_mode == ExecutionMode::Sequential {
            self.test_queue.first().copied()
        } else {
            None
        };

        self.status = TestStatus::Running {
            current_test,
            tests_completed: self.completed_tests,
            tests_total: self.total_tests,
            start_time: self.start_time.unwrap_or_else(Instant::now),
            mode: self.execution_mode,
        };
    }

    /// Stop all running tests
    pub fn stop(&mut self) {
        info!("Stopping all stress tests");

        if let Ok(mut processes) = self.processes.lock() {
            for (category, mut process) in processes.drain() {
                info!(
                    "Terminating {} test (PID: {})",
                    category.name(),
                    process.child.id()
                );

                // First try SIGTERM
                let pid = Pid::from_raw(process.child.id() as i32);
                if let Err(e) = signal::kill(pid, Signal::SIGTERM) {
                    warn!("Failed to send SIGTERM to {}: {}", category.name(), e);
                }

                // Give it a moment to terminate gracefully
                std::thread::sleep(std::time::Duration::from_millis(500));

                // Check if still running and force kill
                match process.child.try_wait() {
                    Ok(None) => {
                        warn!("Process {} still running, sending SIGKILL", category.name());
                        if let Err(e) = signal::kill(pid, Signal::SIGKILL) {
                            error!("Failed to send SIGKILL to {}: {}", category.name(), e);
                        }
                        let _ = process.child.wait();
                    }
                    Ok(Some(_)) => {
                        debug!("{} terminated gracefully", category.name());
                    }
                    Err(e) => {
                        error!(
                            "Error checking process status for {}: {}",
                            category.name(),
                            e
                        );
                    }
                }
            }
        }

        self.test_queue.clear();
        self.status = TestStatus::Stopped;
    }

    /// Update status by checking running processes
    pub fn update_status(&mut self) {
        if !matches!(self.status, TestStatus::Running { .. }) {
            return;
        }

        let mut completed = Vec::new();
        let mut failed = None;

        // Check all running processes
        if let Ok(mut processes) = self.processes.lock() {
            for (category, process) in processes.iter_mut() {
                match process.child.try_wait() {
                    Ok(Some(exit_status)) => {
                        if exit_status.success() {
                            info!("{} test completed successfully", category.name());
                            completed.push(*category);
                        } else {
                            let code = exit_status.code().unwrap_or(-1);
                            error!("{} test failed with exit code {}", category.name(), code);
                            failed = Some(format!(
                                "{} test failed with exit code {}",
                                category.name(),
                                code
                            ));
                        }
                    }
                    Ok(None) => {
                        // Still running
                    }
                    Err(e) => {
                        error!("Error checking {} test status: {}", category.name(), e);
                        failed = Some(format!("Error checking {} status: {}", category.name(), e));
                    }
                }
            }

            // Remove completed processes
            for cat in &completed {
                processes.remove(cat);
            }
        }

        // Update completed count
        self.completed_tests += completed.len();

        // In sequential mode, remove completed from queue and start next
        if self.execution_mode == ExecutionMode::Sequential {
            for cat in &completed {
                self.test_queue.retain(|c| c != cat);
            }

            // Start next test if queue is not empty and no failure
            if failed.is_none() && !self.test_queue.is_empty() {
                if let Err(e) = self.start_next_sequential_test() {
                    failed = Some(format!("Failed to start next test: {}", e));
                }
            }
        }

        // Update status based on results
        if let Some(error) = failed {
            self.stop();
            self.status = TestStatus::Failed { error };
        } else if self.completed_tests >= self.total_tests {
            let duration = self.start_time.map(|t| t.elapsed()).unwrap_or_default();
            self.status = TestStatus::Completed {
                duration,
                tests_run: self.total_tests,
            };
            info!("All tests completed in {:?}", duration);
        } else {
            self.update_running_status();
        }
    }

    /// Get list of currently running test categories
    pub fn running_tests(&self) -> Vec<TestCategory> {
        if let Ok(processes) = self.processes.lock() {
            processes.keys().copied().collect()
        } else {
            Vec::new()
        }
    }

    /// Get elapsed time since tests started
    pub fn elapsed_time(&self) -> Option<std::time::Duration> {
        self.start_time.map(|t| t.elapsed())
    }

    /// Reset runner to idle state
    pub fn reset(&mut self) {
        self.stop();
        self.status = TestStatus::Idle;
        self.test_queue.clear();
        self.total_tests = 0;
        self.completed_tests = 0;
        self.start_time = None;
    }
}

impl Drop for StressRunner {
    fn drop(&mut self) {
        // Ensure all processes are stopped when runner is dropped
        self.stop();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_status_default() {
        let runner = StressRunner::new();
        assert!(matches!(runner.status(), TestStatus::Idle));
    }

    #[test]
    fn test_no_tests_selected() {
        let mut runner = StressRunner::new();
        let result = runner.start(vec![], ExecutionMode::Sequential);
        assert!(result.is_err());
    }

    #[test]
    fn test_sequential_does_not_double_start() {
        use std::process::Command as PCommand;

        let mut runner = StressRunner::new();

        // Insert a long-running dummy process into the processes map to simulate a running test
        let child = PCommand::new("sleep")
            .arg("10")
            .spawn()
            .expect("spawn sleep");
        let tp = TestProcess {
            child,
            category: TestCategory::Cpu,
            start_time: Instant::now(),
        };
        {
            let mut processes = runner.processes.lock().unwrap();
            processes.insert(TestCategory::Cpu, tp);
        }

        // Put the same test at the front of the queue and attempt to start next sequential test
        runner.test_queue = vec![TestCategory::Cpu];

        let before = { runner.processes.lock().unwrap().len() };
        runner.start_next_sequential_test().expect("start next");
        let after = { runner.processes.lock().unwrap().len() };

        assert_eq!(before, after);

        // Cleanup the dummy child: remove it while holding the lock, but drop the lock before waiting
        let child_opt = {
            let mut processes = runner.processes.lock().unwrap();
            processes.remove(&TestCategory::Cpu).map(|tp| tp.child)
        };
        if let Some(mut child) = child_opt {
            let _ = child.kill();
            let _ = child.wait();
        }
    }
}
