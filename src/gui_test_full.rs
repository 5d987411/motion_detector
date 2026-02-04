// Comprehensive GUI test with real functionality
use eframe;
use eframe::egui;
use std::time::Instant;

struct TestMotionDetector {
    sensitivity: f64,
    min_area: u32,
    device: u32,
    is_detecting: bool,
    motion_count: u32,
    motion_detected: bool,
    fps: f32,
    resolution: (i32, i32),
    last_motion_time: Option<Instant>,
    status_log: Vec<String>,
    auto_scroll: bool,
    show_about: bool,
    test_animation: f32,
}

impl Default for TestMotionDetector {
    fn default() -> Self {
        Self {
            sensitivity: 0.3,
            min_area: 500,
            device: 0,
            is_detecting: false,
            motion_count: 0,
            motion_detected: false,
            fps: 30.0,
            resolution: (1920, 1080),
            last_motion_time: None,
            status_log: vec!["GUI Test Started".to_string()],
            auto_scroll: true,
            show_about: false,
            test_animation: 0.0,
        }
    }
}

impl eframe::App for TestMotionDetector {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Simulate motion detection updates
        if self.is_detecting {
            self.test_animation += 0.05;
            if self.test_animation > 1.0 {
                self.test_animation = 0.0;
                // Simulate random motion detection
                if (self.motion_count % 3) == 0 {
                    self.motion_detected = true;
                    self.motion_count += 1;
                    self.last_motion_time = Some(Instant::now());
                    self.status_log.push(format!(
                        "Motion detected! (#{}). FPS: {:.1}",
                        self.motion_count, self.fps
                    ));
                } else {
                    self.motion_detected = false;
                }
                if self.status_log.len() > 100 {
                    self.status_log.remove(0);
                }
            }
        }

        // Menu bar
        egui::TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            egui::menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_about, "About");
                });

                ui.menu_button("Camera", |ui| {
                    if ui.button("Start Detection").clicked() {
                        self.is_detecting = true;
                        self.status_log.push("Detection started".to_string());
                    }
                    if ui.button("Stop Detection").clicked() {
                        self.is_detecting = false;
                        self.motion_detected = false;
                        self.status_log.push("Detection stopped".to_string());
                    }
                });
            });
        });

        // About window
        if self.show_about {
            egui::Window::new("About Motion Detector")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("Motion Detector v0.1.0");
                    ui.label("A Rust-based motion detection application");
                    ui.label("with Logitech camera support");
                    ui.separator();
                    ui.label("Features:");
                    ui.label("• Real-time motion detection");
                    ui.label("• Configurable sensitivity");
                    ui.label("• Snapshot capture");
                    ui.label("• GUI control panel");
                    ui.separator();
                    if ui.button("Close").clicked() {
                        self.show_about = false;
                    }
                });
        }

        // Main layout
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                // Control Panel
                ui.vertical(|ui| {
                    ui.set_min_width(300.0);
                    ui.heading("⚙️ Motion Detector Controls");
                    ui.separator();

                    // Camera selection
                    ui.horizontal(|ui| {
                        ui.label("Camera:");
                        egui::ComboBox::from_label("")
                            .selected_text("Camera 0 - 1920x1080")
                            .show_ui(ui, |ui| {
                                ui.selectable_value(&mut self.device, 0, "Camera 0 - 1920x1080");
                                ui.selectable_value(&mut self.device, 1, "Camera 1 - Unknown");
                            });
                    });

                    ui.add_space(10.0);

                    // Sensitivity slider
                    ui.horizontal(|ui| {
                        ui.label("Sensitivity:");
                        if ui
                            .add(egui::Slider::new(&mut self.sensitivity, 0.0..=1.0).text(""))
                            .changed()
                        {
                            self.status_log
                                .push(format!("Sensitivity changed to {:.2}", self.sensitivity));
                        }
                        ui.label(format!("{:.2}", self.sensitivity));
                    });

                    // Min area slider
                    ui.horizontal(|ui| {
                        ui.label("Min Area:");
                        if ui
                            .add(egui::Slider::new(&mut self.min_area, 50..=5000).text(""))
                            .changed()
                        {
                            self.status_log
                                .push(format!("Min area changed to {}", self.min_area));
                        }
                        ui.label(format!("{} px", self.min_area));
                    });

                    ui.add_space(10.0);

                    // Detection toggle
                    ui.horizontal(|ui| {
                        if self.is_detecting {
                            if ui
                                .add(
                                    egui::Button::new("⏹️ Stop Detection").fill(egui::Color32::RED),
                                )
                                .clicked()
                            {
                                self.is_detecting = false;
                                self.motion_detected = false;
                                self.status_log
                                    .push("Detection stopped by user".to_string());
                            }
                        } else {
                            if ui
                                .add(
                                    egui::Button::new("▶️ Start Detection")
                                        .fill(egui::Color32::GREEN),
                                )
                                .clicked()
                            {
                                self.is_detecting = true;
                                self.status_log
                                    .push("Detection started by user".to_string());
                            }
                        }

                        if ui.add(egui::Button::new("📸 Save Snapshot")).clicked() {
                            self.status_log.push("Manual snapshot saved".to_string());
                        }
                    });

                    // FPS control
                    ui.add_space(10.0);
                    ui.horizontal(|ui| {
                        ui.label("Simulated FPS:");
                        if ui
                            .add(egui::Slider::new(&mut self.fps, 1.0..=60.0).text(""))
                            .changed()
                        {
                            self.status_log
                                .push(format!("FPS changed to {:.1}", self.fps));
                        }
                    });
                });

                ui.separator();

                // Status Panel
                ui.vertical(|ui| {
                    ui.set_min_width(250.0);
                    ui.heading("📊 Status");
                    ui.separator();

                    // Detector status
                    ui.horizontal(|ui| {
                        ui.label("Status:");
                        let status_text = if self.is_detecting {
                            "Running"
                        } else {
                            "Stopped"
                        };
                        let status_color = if self.is_detecting {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::GRAY
                        };
                        ui.colored_label(status_color, status_text);
                    });

                    // Motion status
                    ui.horizontal(|ui| {
                        ui.label("Motion:");
                        let motion_color = if self.motion_detected {
                            egui::Color32::RED
                        } else {
                            egui::Color32::GREEN
                        };
                        let motion_text = if self.motion_detected {
                            "DETECTED"
                        } else {
                            "None"
                        };
                        ui.colored_label(motion_color, motion_text);

                        ui.label(format!("Count: {}", self.motion_count));
                    });

                    // FPS and resolution
                    ui.horizontal(|ui| {
                        ui.label(format!("FPS: {:.1}", self.fps));
                        ui.label(format!(
                            "Resolution: {}x{}",
                            self.resolution.0, self.resolution.1
                        ));
                    });

                    // Last motion time
                    if let Some(last_time) = self.last_motion_time {
                        let duration = last_time.elapsed();
                        ui.label(format!("Last Motion: {:.1}s ago", duration.as_secs_f32()));
                    }

                    // Visual indicator
                    ui.add_space(10.0);
                    ui.heading("Motion Indicator:");
                    let indicator_color = if self.motion_detected {
                        egui::Color32::RED
                    } else {
                        egui::Color32::from_rgb(50, 200, 50)
                    };
                    ui.painter().rect_filled(
                        egui::Rect::from_min_size(ui.cursor().min, egui::vec2(200.0, 50.0)),
                        5.0,
                        indicator_color,
                    );
                    ui.advance_cursor_after_rect(egui::Rect::from_min_size(
                        ui.cursor().min,
                        egui::vec2(200.0, 50.0),
                    ));
                });

                ui.separator();

                // Log Panel
                ui.vertical(|ui| {
                    ui.set_min_width(300.0);
                    ui.horizontal(|ui| {
                        ui.heading("📝 Activity Log");
                        ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
                        if ui.button("Clear").clicked() {
                            self.status_log.clear();
                        }
                    });
                    ui.separator();

                    egui::ScrollArea::vertical()
                        .auto_shrink([false, false])
                        .stick_to_bottom(self.auto_scroll)
                        .show(ui, |ui| {
                            for (i, entry) in self.status_log.iter().enumerate() {
                                ui.label(entry);
                                if i < self.status_log.len() - 1 {
                                    ui.separator();
                                }
                            }
                        });
                });
            });
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Motion Detector GUI Test",
        options,
        Box::new(|_cc| Box::new(TestMotionDetector::default())),
    )
}
