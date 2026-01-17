//! Jobfile management utilities for stress-ng configuration

use crate::app::{ExecutionMode, TestCategory};
use anyhow::{Context, Result};
use std::fs;
use std::path::{Path, PathBuf};

/// Manages stress-ng jobfiles
pub struct JobfileManager {
    /// Base directory containing jobfiles
    jobfiles_dir: PathBuf,
    /// Temporary directory for modified jobfiles
    temp_dir: PathBuf,
}

impl JobfileManager {
    /// Create a new jobfile manager, locating the jobfiles directory
    pub fn new() -> Result<Self> {
        let jobfiles_dir = Self::find_jobfiles_dir()?;
        let temp_dir = std::env::temp_dir().join("shakedown_jobs");

        // Ensure temp directory exists
        if !temp_dir.exists() {
            fs::create_dir_all(&temp_dir)
                .context("Failed to create temporary jobfiles directory")?;
        }

        Ok(Self {
            jobfiles_dir,
            temp_dir,
        })
    }

    /// Find the jobfiles directory relative to the executable
    fn find_jobfiles_dir() -> Result<PathBuf> {
        // Try multiple locations
        let candidates = vec![
            // Next to the executable
            std::env::current_exe()
                .ok()
                .and_then(|p| p.parent().map(|p| p.join("jobfiles"))),
            // Current working directory
            std::env::current_dir().ok().map(|p| p.join("jobfiles")),
            // Development location
            Some(PathBuf::from("jobfiles")),
        ];

        for candidate in candidates.into_iter().flatten() {
            if candidate.exists() && candidate.is_dir() {
                return Ok(candidate);
            }
        }

        Err(anyhow::anyhow!(
            "Could not find jobfiles directory. Ensure 'jobfiles/' exists next to the executable."
        ))
    }

    /// Get the path to a jobfile for a test category
    pub fn get_jobfile_path(&self, category: TestCategory) -> PathBuf {
        self.jobfiles_dir.join(category.jobfile_name())
    }

    /// Check if a jobfile exists for the given category
    pub fn jobfile_exists(&self, category: TestCategory) -> bool {
        self.get_jobfile_path(category).exists()
    }

    /// Read the content of a jobfile
    pub fn read_jobfile(&self, category: TestCategory) -> Result<String> {
        let path = self.get_jobfile_path(category);
        fs::read_to_string(&path)
            .with_context(|| format!("Failed to read jobfile: {}", path.display()))
    }

    /// Create a modified jobfile with the specified execution mode
    /// Returns the path to the (possibly temporary) jobfile
    pub fn prepare_jobfile(
        &self,
        category: TestCategory,
        execution_mode: ExecutionMode,
    ) -> Result<PathBuf> {
        let original_path = self.get_jobfile_path(category);
        let content = fs::read_to_string(&original_path)
            .with_context(|| format!("Failed to read jobfile: {}", original_path.display()))?;

        // Modify the run mode in the jobfile
        let modified_content = Self::set_execution_mode(&content, execution_mode);

        // Write to temp file
        let temp_path = self.temp_dir.join(format!(
            "{}_{}",
            execution_mode_str(execution_mode),
            category.jobfile_name()
        ));

        fs::write(&temp_path, modified_content).with_context(|| {
            format!("Failed to write temporary jobfile: {}", temp_path.display())
        })?;

        Ok(temp_path)
    }

    /// Modify the execution mode in jobfile content
    fn set_execution_mode(content: &str, mode: ExecutionMode) -> String {
        let mode_str = execution_mode_str(mode);
        let mut lines: Vec<String> = Vec::new();
        let mut found_run_directive = false;

        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.starts_with("run ") {
                // Replace existing run directive
                lines.push(format!("run {}", mode_str));
                found_run_directive = true;
            } else {
                lines.push(line.to_string());
            }
        }

        // If no run directive was found, add one at the beginning (after comments)
        if !found_run_directive {
            let mut result: Vec<String> = Vec::new();
            let mut inserted = false;

            for line in lines {
                if !inserted && !line.trim().starts_with('#') && !line.trim().is_empty() {
                    result.push(format!("run {}", mode_str));
                    inserted = true;
                }
                result.push(line);
            }

            if !inserted {
                result.push(format!("run {}", mode_str));
            }

            return result.join("\n");
        }

        lines.join("\n")
    }

    /// Get available test categories (those with existing jobfiles)
    pub fn available_categories(&self) -> Vec<TestCategory> {
        TestCategory::all()
            .iter()
            .filter(|cat| self.jobfile_exists(**cat))
            .copied()
            .collect()
    }

    /// Validate all jobfiles exist
    pub fn validate_jobfiles(&self) -> Vec<(TestCategory, bool)> {
        TestCategory::all()
            .iter()
            .map(|cat| (*cat, self.jobfile_exists(*cat)))
            .collect()
    }

    /// Get the jobfiles directory path
    pub fn jobfiles_dir(&self) -> &Path {
        &self.jobfiles_dir
    }

    /// Clean up temporary jobfiles
    pub fn cleanup(&self) -> Result<()> {
        if self.temp_dir.exists() {
            for entry in fs::read_dir(&self.temp_dir)? {
                let entry = entry?;
                if entry.path().extension().map_or(false, |e| e == "job") {
                    fs::remove_file(entry.path()).ok();
                }
            }
        }
        Ok(())
    }
}

impl Drop for JobfileManager {
    fn drop(&mut self) {
        // Best effort cleanup
        let _ = self.cleanup();
    }
}

/// Convert execution mode to stress-ng run directive string
fn execution_mode_str(mode: ExecutionMode) -> &'static str {
    match mode {
        ExecutionMode::Sequential => "sequential",
        ExecutionMode::Parallel => "parallel",
    }
}

/// Information about a jobfile
#[derive(Debug, Clone)]
pub struct JobfileInfo {
    pub category: TestCategory,
    pub path: PathBuf,
    pub exists: bool,
    pub line_count: usize,
    pub stressor_count: usize,
}

impl JobfileInfo {
    pub fn from_path(category: TestCategory, path: PathBuf) -> Self {
        let exists = path.exists();
        let (line_count, stressor_count) = if exists {
            if let Ok(content) = fs::read_to_string(&path) {
                let lines = content.lines().count();
                let stressors = count_stressors(&content);
                (lines, stressors)
            } else {
                (0, 0)
            }
        } else {
            (0, 0)
        };

        Self {
            category,
            path,
            exists,
            line_count,
            stressor_count,
        }
    }
}

/// Count the number of stressor definitions in a jobfile
fn count_stressors(content: &str) -> usize {
    content
        .lines()
        .filter(|line| {
            let trimmed = line.trim();
            // A stressor line typically starts with the stressor name followed by a number or 0
            // and doesn't start with # (comment), run, timeout, verbose, etc.
            !trimmed.is_empty()
                && !trimmed.starts_with('#')
                && !trimmed.starts_with("run ")
                && !trimmed.starts_with("timeout")
                && !trimmed.starts_with("verbose")
                && !trimmed.starts_with("metrics")
                && !trimmed.starts_with("verify")
                && !trimmed.starts_with("aggressive")
                && !trimmed.starts_with("keep-name")
                && !trimmed.starts_with("log-")
                && !trimmed.starts_with("oom-")
                && !trimmed.starts_with("thermal")
                && !trimmed.contains('-')  // Options like --timeout contain dashes
                && trimmed.split_whitespace().count() >= 2
                && trimmed
                    .split_whitespace()
                    .nth(1)
                    .map_or(false, |s| s.parse::<u32>().is_ok() || s.ends_with('%'))
        })
        .count()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_execution_mode_str() {
        assert_eq!(execution_mode_str(ExecutionMode::Sequential), "sequential");
        assert_eq!(execution_mode_str(ExecutionMode::Parallel), "parallel");
    }

    #[test]
    fn test_set_execution_mode_replaces_existing() {
        let content = "# Comment\nrun sequential\ncpu 0\n";
        let modified = JobfileManager::set_execution_mode(content, ExecutionMode::Parallel);
        assert!(modified.contains("run parallel"));
        assert!(!modified.contains("run sequential"));
    }

    #[test]
    fn test_set_execution_mode_adds_if_missing() {
        let content = "# Comment\ncpu 0\nmemory 0\n";
        let modified = JobfileManager::set_execution_mode(content, ExecutionMode::Sequential);
        assert!(modified.contains("run sequential"));
    }
}
