/*
 * shakedown -- a stress testing tool for Linux systems.
 * Copyright (C) 2026  Joshua Karner
 *
 * This program is free software; you can redistribute it and/or modify
 * it under the terms of the GNU General Public License as published by
 * the Free Software Foundation; either version 2 of the License, or
 * (at your option) any later version.
 *
 * This program is distributed in the hope that it will be useful,
 * but WITHOUT ANY WARRANTY; without even the implied warranty of
 * MERCHANTABILITY or FITNESS FOR A PARTICULAR PURPOSE.  See the
 * GNU General Public License for more details.
 *
 * You should have received a copy of the GNU General Public License
 * along with this program; if not, write to the Free Software
 * Foundation, Inc., 51 Franklin Street, Fifth Floor, Boston, MA 02110-1301 USA.
 */
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
use tokio::process::Command;

fn main() -> Result<()> {
    // Initialize logging
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info")).init();

    info!("Starting Shakedown - Hardware Stress Testing Application");

    let native_options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_title("Shakedown")
            .with_inner_size([500.0, 650.0])
            .with_min_inner_size([350.0, 650.0])
            .with_position(egui::Pos2::new(0.0, 0.0)),
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
