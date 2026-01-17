//! Test configuration panel UI
//!
//! Provides the interface for selecting and configuring stress tests.

use crate::app::{ExecutionMode, ShakedownApp, TestCategory};
use crate::stress::runner::TestStatus;
use egui::{Color32, RichText, Ui};

/// Render the test configuration panel
pub fn render(app: &mut ShakedownApp, ui: &mut Ui) {
    ui.heading("🔧 Test Configuration");
    ui.add_space(8.0);

    // Check if stress-ng is available
    let stress_ng_available = app.runner.is_stress_ng_available();

    if !stress_ng_available {
        ui.horizontal(|ui| {
            ui.label(RichText::new("⚠").color(Color32::YELLOW).size(16.0));
            ui.label(RichText::new("stress-ng not found!").color(Color32::YELLOW));
        });
        ui.add_space(8.0);
    }
    // Check if stress-ng is available
    let gpu_burn_available = app.runner.is_gpu_burn_available();

    if !gpu_burn_available {
        ui.horizontal(|ui| {
            ui.label(RichText::new("⚠").color(Color32::YELLOW).size(16.0));
            ui.label(RichText::new("gpu-burn not found!").color(Color32::YELLOW));
        });
        ui.add_space(8.0);
    }

    // Test category selection
    ui.group(|ui| {
        ui.label(RichText::new("Select Tests").strong());
        ui.add_space(4.0);

        let is_running = app.is_running();

        for category in TestCategory::all() {
            let selected = app.config.selected.get(category).copied().unwrap_or(false);

            ui.horizontal(|ui| {
                let mut sel = selected;
                let checkbox = egui::Checkbox::new(&mut sel, "");

                if ui.add_enabled(!is_running, checkbox).changed() {
                    app.config.selected.insert(*category, sel);
                }

                // Category icon and name
                let icon = category_icon(*category);
                ui.label(RichText::new(icon).size(16.0));
                ui.label(RichText::new(category.name()).strong());
            });

            // Description
            ui.indent(category.name(), |ui| {
                ui.label(
                    RichText::new(category.description())
                        .small()
                        .color(Color32::GRAY),
                );
            });

            // GPU Burn configuration options
            if *category == TestCategory::GpuBurn && selected {
                ui.indent("gpu_burn_config", |ui| {
                    ui.add_space(4.0);

                    // Memory percentage slider
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Memory:").small());
                        let mut mem_pct = app.config.gpu_burn.memory_percent as f32;
                        if ui
                            .add_enabled(
                                !is_running,
                                egui::Slider::new(&mut mem_pct, 10.0..=100.0)
                                    .suffix("%")
                                    .step_by(5.0),
                            )
                            .changed()
                        {
                            app.config.gpu_burn.memory_percent = mem_pct as u8;
                        }
                    });

                    // Duration slider
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Duration:").small());
                        let mut duration = app.config.gpu_burn.duration_hours as f32;
                        if ui
                            .add_enabled(
                                !is_running,
                                egui::Slider::new(&mut duration, 1.0..=48.0)
                                    .suffix(" hr")
                                    .step_by(1.0),
                            )
                            .changed()
                        {
                            app.config.gpu_burn.duration_hours = duration as u8;
                        }
                    });

                    // Use doubles checkbox
                    ui.horizontal(|ui| {
                        let mut use_doubles = app.config.gpu_burn.use_doubles;
                        if ui
                            .add_enabled(
                                !is_running,
                                egui::Checkbox::new(&mut use_doubles, "Use Doubles (-d)"),
                            )
                            .changed()
                        {
                            app.config.gpu_burn.use_doubles = use_doubles;
                        }
                    });

                    // Use tensor cores checkbox
                    ui.horizontal(|ui| {
                        let mut use_tc = app.config.gpu_burn.use_tensor_cores;
                        if ui
                            .add_enabled(
                                !is_running,
                                egui::Checkbox::new(&mut use_tc, "Use Tensor Cores (-tc)"),
                            )
                            .changed()
                        {
                            app.config.gpu_burn.use_tensor_cores = use_tc;
                        }
                    });

                    ui.add_space(2.0);
                });
            }

            ui.add_space(2.0);
        }

        ui.add_space(8.0);

        // Select all / Deselect all buttons
        ui.horizontal(|ui| {
            if ui
                .add_enabled(!is_running, egui::Button::new("Select All"))
                .clicked()
            {
                for category in TestCategory::all() {
                    app.config.selected.insert(*category, true);
                }
            }

            if ui
                .add_enabled(!is_running, egui::Button::new("Deselect All"))
                .clicked()
            {
                for category in TestCategory::all() {
                    app.config.selected.insert(*category, false);
                }
            }
        });
    });

    ui.add_space(16.0);

    // Execution mode selection
    ui.group(|ui| {
        ui.label(RichText::new("Execution Mode").strong());
        ui.add_space(4.0);

        let is_running = app.is_running();

        ui.horizontal(|ui| {
            let sequential = ui.add_enabled(
                !is_running,
                egui::RadioButton::new(
                    app.config.execution_mode == ExecutionMode::Sequential,
                    "Sequential",
                ),
            );
            if sequential.clicked() {
                app.config.execution_mode = ExecutionMode::Sequential;
            }

            let parallel = ui.add_enabled(
                !is_running,
                egui::RadioButton::new(
                    app.config.execution_mode == ExecutionMode::Parallel,
                    "Parallel",
                ),
            );
            if parallel.clicked() {
                app.config.execution_mode = ExecutionMode::Parallel;
            }
        });

        ui.add_space(4.0);

        let mode_desc = match app.config.execution_mode {
            ExecutionMode::Sequential => {
                "Tests will run one after another. Easier to identify which test causes issues."
            }
            ExecutionMode::Parallel => {
                "All tests run simultaneously. Maximum stress on the system."
            }
        };

        ui.label(RichText::new(mode_desc).small().color(Color32::GRAY));
    });

    ui.add_space(16.0);

    // Control buttons and status
    render_controls(app, ui);
}

/// Render control buttons and current status
fn render_controls(app: &mut ShakedownApp, ui: &mut Ui) {
    ui.group(|ui| {
        ui.label(RichText::new("Controls").strong());
        ui.add_space(8.0);

        let is_running = app.is_running();
        let any_selected = app.config.any_selected();
        let stress_ng_available = app.runner.is_stress_ng_available();

        ui.horizontal(|ui| {
            // Start button
            let can_start = !is_running && any_selected && stress_ng_available;
            let start_btn = egui::Button::new(RichText::new("▶ Start Tests").color(if can_start {
                Color32::WHITE
            } else {
                Color32::GRAY
            }))
            .fill(if can_start {
                Color32::from_rgb(0, 120, 0)
            } else {
                Color32::from_rgb(60, 60, 60)
            });

            if ui.add_enabled(can_start, start_btn).clicked() {
                if let Err(e) = app.start_tests() {
                    log::error!("Failed to start tests: {}", e);
                }
            }

            // Stop button
            let stop_btn = egui::Button::new(RichText::new("⏹ Stop Tests").color(if is_running {
                Color32::WHITE
            } else {
                Color32::GRAY
            }))
            .fill(if is_running {
                Color32::from_rgb(180, 0, 0)
            } else {
                Color32::from_rgb(60, 60, 60)
            });

            if ui.add_enabled(is_running, stop_btn).clicked() {
                app.stop_tests();
            }

            // Reset button
            if ui
                .add_enabled(
                    !is_running && !matches!(app.test_status(), TestStatus::Idle),
                    egui::Button::new("↺ Reset"),
                )
                .clicked()
            {
                app.runner.reset();
            }
        });

        ui.add_space(12.0);

        // Status display
        render_status(app, ui);
    });
}

/// Render current test status
fn render_status(app: &ShakedownApp, ui: &mut Ui) {
    let status = app.test_status();

    match status {
        TestStatus::Idle => {
            ui.horizontal(|ui| {
                ui.label(RichText::new("●").color(Color32::GRAY));
                ui.label("Ready");
            });

            if !app.config.any_selected() {
                ui.label(
                    RichText::new("Select at least one test category to begin.")
                        .small()
                        .color(Color32::GRAY),
                );
            }
        }

        TestStatus::Running {
            current_test,
            tests_completed,
            tests_total,
            start_time,
            mode,
        } => {
            ui.horizontal(|ui| {
                // Animated spinner effect using frame time
                let spinner_chars = ["◐", "◓", "◑", "◒"];
                let idx = (start_time.elapsed().as_millis() / 200) as usize % spinner_chars.len();
                ui.label(
                    RichText::new(spinner_chars[idx])
                        .color(Color32::from_rgb(100, 200, 100))
                        .size(16.0),
                );
                ui.label(RichText::new("Running").color(Color32::from_rgb(100, 200, 100)));
            });

            // Progress
            let progress = *tests_completed as f32 / *tests_total as f32;
            let progress_bar = egui::ProgressBar::new(progress)
                .text(format!("{}/{} tests", tests_completed, tests_total))
                .animate(true);
            ui.add(progress_bar);

            // Current test info
            ui.horizontal(|ui| {
                ui.label(RichText::new("Mode:").small());
                ui.label(RichText::new(mode.name()).small().strong());
            });

            if *mode == ExecutionMode::Sequential {
                if let Some(current) = current_test {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Current:").small());
                        ui.label(
                            RichText::new(format!(
                                "{} {}",
                                category_icon(*current),
                                current.name()
                            ))
                            .small()
                            .strong(),
                        );
                    });
                }
            } else {
                // Show all running tests for parallel mode
                let running = app.runner.running_tests();
                if !running.is_empty() {
                    ui.horizontal(|ui| {
                        ui.label(RichText::new("Running:").small());
                        for test in &running {
                            ui.label(RichText::new(format!("{}", category_icon(*test))).small());
                        }
                    });
                }
            }

            // Elapsed time
            let elapsed = start_time.elapsed();
            let mins = elapsed.as_secs() / 60;
            let secs = elapsed.as_secs() % 60;
            ui.label(
                RichText::new(format!("Elapsed: {:02}:{:02}", mins, secs))
                    .small()
                    .color(Color32::GRAY),
            );
        }

        TestStatus::Completed {
            duration,
            tests_run,
        } => {
            ui.horizontal(|ui| {
                ui.label(RichText::new("✓").color(Color32::GREEN).size(16.0));
                ui.label(RichText::new("Completed").color(Color32::GREEN));
            });

            let mins = duration.as_secs() / 60;
            let secs = duration.as_secs() % 60;
            ui.label(
                RichText::new(format!(
                    "{} tests completed in {:02}:{:02}",
                    tests_run, mins, secs
                ))
                .small(),
            );
        }

        TestStatus::Stopped => {
            ui.horizontal(|ui| {
                ui.label(RichText::new("⏹").color(Color32::YELLOW).size(16.0));
                ui.label(RichText::new("Stopped").color(Color32::YELLOW));
            });
            ui.label(RichText::new("Tests were stopped by user.").small());
        }

        TestStatus::Failed { error } => {
            ui.horizontal(|ui| {
                ui.label(RichText::new("✗").color(Color32::RED).size(16.0));
                ui.label(RichText::new("Failed").color(Color32::RED));
            });
            ui.label(
                RichText::new(error)
                    .small()
                    .color(Color32::from_rgb(255, 150, 150)),
            );
        }
    }
}

/// Get an icon for a test category
fn category_icon(category: TestCategory) -> &'static str {
    match category {
        TestCategory::Cpu => "🔥",
        TestCategory::Memory => "🧠",
        TestCategory::Disk => "💾",
        TestCategory::Io => "⚡",
        TestCategory::GpuBurn => "🎮",
        TestCategory::TwentyFourHour => "🌐",
    }
}

/// Render a compact status indicator for use in other panels
pub fn render_status_indicator(app: &ShakedownApp, ui: &mut Ui) {
    let status = app.test_status();

    match status {
        TestStatus::Idle => {
            ui.label(RichText::new("●").color(Color32::GRAY));
        }
        TestStatus::Running { start_time, .. } => {
            let spinner_chars = ["◐", "◓", "◑", "◒"];
            let idx = (start_time.elapsed().as_millis() / 200) as usize % spinner_chars.len();
            ui.label(RichText::new(spinner_chars[idx]).color(Color32::from_rgb(100, 200, 100)));
        }
        TestStatus::Completed { .. } => {
            ui.label(RichText::new("✓").color(Color32::GREEN));
        }
        TestStatus::Stopped => {
            ui.label(RichText::new("⏹").color(Color32::YELLOW));
        }
        TestStatus::Failed { .. } => {
            ui.label(RichText::new("✗").color(Color32::RED));
        }
    }
}
