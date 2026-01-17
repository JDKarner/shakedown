//! Shakedown - Linux Hardware Stress Testing Application
//!
//! A GUI application for running stress-ng tests to detect hardware faults.

mod app;
mod monitor;
mod stress;
mod ui;

use anyhow::Result;
use eframe::egui;
use log::info;

fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting Shakedown - Hardware Stress Testing Application");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Shakedown")
            .with_inner_size([400.0, 600.0])
            .with_min_inner_size([350.0, 500.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Shakedown",
        native_options,
        Box::new(|cc| Ok(Box::new(app::ShakedownApp::new(cc)))),
    )
    .map_err(|e| anyhow::anyhow!("Failed to run application: {}", e))?;

    Ok(())
}
