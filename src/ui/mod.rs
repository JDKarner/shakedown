//! UI module for the Shakedown application
//!
//! This module contains all egui-based UI components including:
//! - Test configuration panel

pub mod test_config_panel;

use crate::app::ShakedownApp;
use egui::{CentralPanel, Context, TopBottomPanel};

/// Main render function for the application UI
pub fn render(app: &mut ShakedownApp, ctx: &Context) {
    // Top panel with status bar
    TopBottomPanel::top("top_panel").show(ctx, |ui| {
        render_top_bar(app, ui);
    });

    // Central panel with test configuration
    CentralPanel::default().show(ctx, |ui| {
        test_config_panel::render(app, ui);
    });
}

/// Render the top status bar
fn render_top_bar(app: &mut ShakedownApp, ui: &mut egui::Ui) {
    ui.horizontal(|ui| {
        ui.heading("🔧 Shakedown");
        ui.separator();

        // Launch Tricorder button
        if ui.button("🖖 Launch Tricorder")
            .on_hover_text("Launch or relaunch the Tricorder system monitor (tricorder --monitor)")
            .clicked()
        {
            app.launch_tricorder();
        }
        ui.separator();

        // Status indicator
        let (status_text, status_color) = match app.test_status() {
            crate::stress::TestStatus::Idle => ("⏹ Idle", egui::Color32::GRAY),
            crate::stress::TestStatus::Running { .. } => {
                ("▶ Running", egui::Color32::from_rgb(0, 200, 0))
            }
            crate::stress::TestStatus::Completed { .. } => {
                ("✓ Completed", egui::Color32::from_rgb(0, 150, 255))
            }
            crate::stress::TestStatus::Stopped => ("⏸ Stopped", egui::Color32::YELLOW),
            crate::stress::TestStatus::Failed { .. } => ("✗ Failed", egui::Color32::RED),
        };

        ui.colored_label(status_color, status_text);

        // Show elapsed time if running or completed
        match app.test_status() {
            crate::stress::TestStatus::Running { start_time, .. } => {
                let elapsed = start_time.elapsed();
                ui.separator();
                ui.label(format!("⏱ {}", format_duration(elapsed)));
            }
            crate::stress::TestStatus::Completed { duration, .. } => {
                ui.separator();
                ui.label(format!("⏱ {} (total)", format_duration(*duration)));
            }
            _ => {}
        }

        // Show progress if running
        if let crate::stress::TestStatus::Running {
            tests_completed,
            tests_total,
            ..
        } = app.test_status()
        {
            ui.separator();
            ui.label(format!("Progress: {}/{}", tests_completed, tests_total));
        }
    });
}

/// Format a duration in a human-readable way
fn format_duration(duration: std::time::Duration) -> String {
    let total_secs = duration.as_secs();
    let hours = total_secs / 3600;
    let minutes = (total_secs % 3600) / 60;
    let seconds = total_secs % 60;

    if hours > 0 {
        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    } else {
        format!("{:02}:{:02}", minutes, seconds)
    }
}
