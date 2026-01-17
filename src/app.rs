//! Main application state and egui integration

use crate::stress::runner::{StressRunner, TestStatus};
use std::sync::{Arc, Mutex};

/// Test categories available for stress testing
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TestCategory {
    Cpu,
    Memory,
    Disk,
    Io,
    GpuBurn,
    TwentyFourHour,
}

impl TestCategory {
    pub fn all() -> &'static [TestCategory] {
        &[
            TestCategory::Cpu,
            TestCategory::Memory,
            TestCategory::Disk,
            TestCategory::Io,
            TestCategory::GpuBurn,
            TestCategory::TwentyFourHour,
        ]
    }

    pub fn name(&self) -> &'static str {
        match self {
            TestCategory::Cpu => "CPU",
            TestCategory::Memory => "Memory",
            TestCategory::Disk => "Disk",
            TestCategory::Io => "I/O",
            TestCategory::GpuBurn => "GPU Burn",
            TestCategory::TwentyFourHour => "24 Hour",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            TestCategory::Cpu => "Stress CPU cores with computation-heavy workloads",
            TestCategory::Memory => "Test RAM integrity and memory subsystem",
            TestCategory::Disk => "Stress storage devices with read/write operations",
            TestCategory::Io => "Stress I/O subsystem with async/sync operations",
            TestCategory::GpuBurn => "Stress test GPU with compute-intensive workloads",
            TestCategory::TwentyFourHour => "Stress test system for 24 hours",
        }
    }

    pub fn jobfile_name(&self) -> &'static str {
        match self {
            TestCategory::Cpu => "cpu.job",
            TestCategory::Memory => "memory.job",
            TestCategory::Disk => "disk.job",
            TestCategory::Io => "io.job",
            TestCategory::GpuBurn => "gpu_burn.job", // Not actually used for GPU burn
            TestCategory::TwentyFourHour => "24-hour.job",
        }
    }
}

/// Execution mode for running tests
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ExecutionMode {
    #[default]
    Sequential,
    Parallel,
}

impl ExecutionMode {
    pub fn name(&self) -> &'static str {
        match self {
            ExecutionMode::Sequential => "Sequential",
            ExecutionMode::Parallel => "Parallel",
        }
    }
}

/// GPU Burn configuration
#[derive(Debug, Clone)]
pub struct GpuBurnConfig {
    pub memory_percent: u8,
    pub use_doubles: bool,
    pub use_tensor_cores: bool,
    pub duration_hours: u8,
}

impl Default for GpuBurnConfig {
    fn default() -> Self {
        Self {
            memory_percent: 90,
            use_doubles: false,
            use_tensor_cores: false,
            duration_hours: 1,
        }
    }
}

/// Test configuration state
#[derive(Debug, Clone)]
pub struct TestConfig {
    pub selected: std::collections::HashMap<TestCategory, bool>,
    pub execution_mode: ExecutionMode,
    pub gpu_burn: GpuBurnConfig,
}

impl Default for TestConfig {
    fn default() -> Self {
        let mut selected = std::collections::HashMap::new();
        for cat in TestCategory::all() {
            selected.insert(*cat, false);
        }
        Self {
            selected,
            execution_mode: ExecutionMode::Sequential,
            gpu_burn: GpuBurnConfig::default(),
        }
    }
}

impl TestConfig {
    pub fn selected_tests(&self) -> Vec<TestCategory> {
        self.selected
            .iter()
            .filter(|(_, &v)| v)
            .map(|(k, _)| *k)
            .collect()
    }

    pub fn any_selected(&self) -> bool {
        self.selected.values().any(|&v| v)
    }
}

/// Main application state
pub struct ShakedownApp {
    /// Test configuration
    pub config: TestConfig,

    /// Stress test runner
    pub runner: StressRunner,

    /// Fan monitor and shared fan list
    pub fans: Arc<Mutex<Vec<crate::monitor::iofan::Fan>>>,
    pub fan_monitor: crate::monitor::iofan::FanMonitor,

    /// Last update time for monitors
    last_fan_update: std::time::Instant,

    /// Update intervals
    fan_update_interval: std::time::Duration,
}

impl ShakedownApp {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let fans = Arc::new(Mutex::new(Vec::new()));

        let mut app = Self {
            config: TestConfig::default(),
            runner: StressRunner::new(),
            fans: fans.clone(),
            fan_monitor: crate::monitor::iofan::FanMonitor::new(fans.clone()),
            last_fan_update: std::time::Instant::now(),
            fan_update_interval: std::time::Duration::from_secs(2),
        };

        // Prime monitors immediately so UI has data on first frame
        app.fan_monitor.update();

        app
    }

    /// Update monitors if their intervals have elapsed
    pub fn update_monitors(&mut self) {
        let now = std::time::Instant::now();

        if now.duration_since(self.last_fan_update) >= self.fan_update_interval {
            self.fan_monitor.update();
            self.last_fan_update = now;
        }

        // Update runner status
        self.runner.update_status();
    }

    /// Start stress tests with current configuration
    pub fn start_tests(&mut self) -> anyhow::Result<()> {
        let tests = self.config.selected_tests();
        if tests.is_empty() {
            return Err(anyhow::anyhow!("No tests selected"));
        }

        // Update GPU burn configuration in runner
        self.runner
            .set_gpu_burn_config(self.config.gpu_burn.clone());

        self.runner.start(tests, self.config.execution_mode)?;
        Ok(())
    }

    /// Stop running stress tests
    pub fn stop_tests(&mut self) {
        self.runner.stop();
    }

    /// Check if tests are currently running
    pub fn is_running(&self) -> bool {
        matches!(self.runner.status(), TestStatus::Running { .. })
    }

    /// Get current test status
    pub fn test_status(&self) -> &TestStatus {
        self.runner.status()
    }
}

impl eframe::App for ShakedownApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update monitors
        self.update_monitors();

        // Request repaint for live updates
        ctx.request_repaint_after(std::time::Duration::from_millis(500));

        // Render UI
        crate::ui::render(self, ctx);
    }
}
