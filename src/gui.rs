use chrono::{DateTime, Local};
use crossbeam_channel::{Receiver, Sender};
use eframe;
use eframe::egui::*;
use std::collections::VecDeque;

#[derive(Clone, Debug)]
pub enum GuiMessage {
    UpdateSensitivity(f64),
    UpdateMinArea(u32),
    UpdateDevice(u32),
    StartDetection,
    StopDetection,
    SaveSnapshot,
}

#[derive(Clone, Debug)]
pub struct MotionState {
    pub motion_detected: bool,
    pub motion_count: u32,
    pub last_motion_time: Option<DateTime<Local>>,
    pub fps: f32,
    pub resolution: (i32, i32),
}

pub struct MotionDetectorGui {
    pub sender: Sender<GuiMessage>,
    pub state_receiver: Option<Receiver<MotionState>>,

    // Settings
    sensitivity: f64,
    min_area: u32,
    device: u32,

    // Status
    detector_status: DetectorStatus,
    is_detecting: bool,
    motion_state: MotionState,

    // Camera info
    available_cameras: Vec<String>,

    // UI state
    show_about: bool,
    status_log: Vec<String>,
    auto_scroll: bool,

    // Motion graph data
    motion_history: VecDeque<bool>,
    max_history_points: usize,

    // Animation state
    motion_animation_time: f32,
}

#[derive(Clone, Debug)]
pub enum DetectorStatus {
    Stopped,
    #[allow(dead_code)]
    Starting,
    Running,
    #[allow(dead_code)]
    Error(String),
}

impl MotionDetectorGui {
    pub fn new_with_sender(_cc: &eframe::CreationContext<'_>, sender: Sender<GuiMessage>) -> Self {
        Self {
            sender,
            state_receiver: None,
            sensitivity: 0.3,
            min_area: 500,
            device: 0,
            detector_status: DetectorStatus::Stopped,
            is_detecting: false,
            motion_state: MotionState {
                motion_detected: false,
                motion_count: 0,
                last_motion_time: None,
                fps: 0.0,
                resolution: (640, 480), // Will be detected at runtime
            },
            available_cameras: vec!["Camera 0 - Detecting resolution...".to_string()],
            show_about: false,
            status_log: vec!["GUI Control Panel Started".to_string()],
            auto_scroll: true,
            motion_history: VecDeque::new(),
            max_history_points: 100,
            motion_animation_time: 0.0,
        }
    }

    fn update_settings_from_receiver(&mut self) {
        // Update state from detector thread
        if let Some(ref receiver) = self.state_receiver {
            while let Ok(state) = receiver.try_recv() {
                let was_motion_detected = self.motion_state.motion_detected;
                self.motion_state = state.clone();

                // Add to motion history for graph
                self.motion_history.push_back(state.motion_detected);
                if self.motion_history.len() > self.max_history_points {
                    self.motion_history.pop_front();
                }

                // Update detector status based on detection state
                if self.is_detecting {
                    self.detector_status = DetectorStatus::Running;
                }

                // Update camera resolution info on first status update
                if self.available_cameras[0] == "Camera 0 - Detecting..." {
                    self.available_cameras[0] =
                        format!("Camera 0 - {}x{}", state.resolution.0, state.resolution.1);
                }

                // Log motion detection events
                if state.motion_detected && !was_motion_detected {
                    self.status_log.push(format!(
                        "Motion detected! (#{}) FPS: {:.1}",
                        state.motion_count, state.fps
                    ));
                    if self.status_log.len() > 100 {
                        self.status_log.remove(0);
                    }
                }
            }
        }
    }

    fn render_control_panel(&mut self, ui: &mut Ui) {
        ui.heading("⚙️ Motion Detector Controls");
        ui.separator();

        // Camera selection
        ui.horizontal(|ui| {
            ui.label("Camera:");
            let mut selected_index = self.device as usize;
            let camera_names: Vec<&str> =
                self.available_cameras.iter().map(|s| s.as_str()).collect();

            ComboBox::from_label("")
                .selected_text(
                    self.available_cameras
                        .get(selected_index)
                        .cloned()
                        .unwrap_or_else(|| "Unknown".to_string()),
                )
                .show_ui(ui, |ui| {
                    for (i, camera_name) in camera_names.iter().enumerate() {
                        if ui
                            .selectable_label(selected_index == i, *camera_name)
                            .clicked()
                        {
                            selected_index = i;
                            self.device = i as u32;
                            let _ = self.sender.send(GuiMessage::UpdateDevice(i as u32));
                        }
                    }
                });
        });

        ui.add_space(10.0);

        // Sensitivity slider
        ui.horizontal(|ui| {
            ui.label("Sensitivity:");
            let mut sensitivity = self.sensitivity;
            if ui
                .add(Slider::new(&mut sensitivity, 0.0..=1.0).text(""))
                .changed()
            {
                self.sensitivity = sensitivity;
                let _ = self.sender.send(GuiMessage::UpdateSensitivity(sensitivity));
            }
            ui.label(format!("{:.2}", self.sensitivity));
        });

        // Min area slider
        ui.horizontal(|ui| {
            ui.label("Min Area:");
            let mut min_area = self.min_area;
            if ui
                .add(Slider::new(&mut min_area, 50..=5000).text(""))
                .changed()
            {
                self.min_area = min_area;
                let _ = self.sender.send(GuiMessage::UpdateMinArea(min_area));
            }
            ui.label(format!("{} px", self.min_area));
        });

        ui.add_space(10.0);

        // Detection toggle
        ui.horizontal(|ui| {
            if self.is_detecting {
                if ui
                    .add(Button::new("⏹️ Stop Detection").fill(Color32::RED))
                    .clicked()
                {
                    self.is_detecting = false;
                    self.detector_status = DetectorStatus::Stopped;
                    self.status_log.push("Motion detection stopped".to_string());
                    if self.status_log.len() > 100 {
                        self.status_log.remove(0);
                    }
                    let _ = self.sender.send(GuiMessage::StopDetection);
                }
            } else {
                if ui
                    .add(Button::new("▶️ Start Detection").fill(Color32::GREEN))
                    .clicked()
                {
                    self.is_detecting = true;
                    self.detector_status = DetectorStatus::Running;
                    self.status_log.push("Motion detection started".to_string());
                    if self.status_log.len() > 100 {
                        self.status_log.remove(0);
                    }
                    let _ = self.sender.send(GuiMessage::StartDetection);
                }
            }

            if ui.add(Button::new("📸 Save Snapshot")).clicked() {
                self.status_log.push("Manual snapshot saved".to_string());
                if self.status_log.len() > 100 {
                    self.status_log.remove(0);
                }
                let _ = self.sender.send(GuiMessage::SaveSnapshot);
            }
        });
    }

    fn render_status_panel(&mut self, ui: &mut Ui) {
        ui.heading("📊 Real-time Status");
        ui.separator();

        // Always visible motion indicator light
        ui.horizontal(|ui| {
            if self.motion_state.motion_detected {
                // Animated pulsing green when motion detected
                let pulse = (self.motion_animation_time * 4.0).sin() * 0.3 + 0.7;
                let green_color = Color32::from_rgb(
                    (pulse * 255.0) as u8,
                    (pulse * 255.0) as u8,
                    (pulse * 100.0) as u8,
                );

                ui.add_sized(
                    [100.0, 100.0],
                    Button::new("")
                        .fill(green_color)
                        .stroke(Stroke::new(8.0, Color32::DARK_GREEN)),
                );

                ui.vertical_centered(|ui| {
                    ui.colored_label(Color32::RED, RichText::new("🔴 MOTION").size(24.0));
                    ui.colored_label(Color32::RED, RichText::new("DETECTED!").size(18.0));
                    ui.colored_label(
                        Color32::from_rgb(255, 200, 200),
                        format!("Count: {}", self.motion_state.motion_count),
                    );

                    // Time since last motion
                    if let Some(last_time) = self.motion_state.last_motion_time {
                        let duration = Local::now().signed_duration_since(last_time);
                        if duration.num_seconds() < 60 {
                            ui.label(format!("{} seconds ago", duration.num_seconds()));
                        } else {
                            ui.label(format!("{} minutes ago", duration.num_minutes()));
                        }
                    }
                });
            } else {
                // Steady green when no motion
                ui.add_sized(
                    [100.0, 100.0],
                    Button::new("")
                        .fill(Color32::DARK_GREEN)
                        .stroke(Stroke::new(4.0, Color32::BLACK)),
                );

                ui.vertical_centered(|ui| {
                    ui.colored_label(Color32::GREEN, RichText::new("🟢 CLEAR").size(24.0));
                    ui.colored_label(Color32::GREEN, RichText::new("NO MOTION").size(18.0));
                    ui.colored_label(
                        Color32::from_rgb(200, 255, 200),
                        format!("Count: {}", self.motion_state.motion_count),
                    );
                    ui.label("Monitoring...");
                });
            }
        });
        ui.separator();

        // Enhanced real-time status grid
        ui.columns(2, |columns| {
            // Left column - System status
            columns[0].heading("🔧 System");
            columns[0].separator();

            // Detector status with icon
            columns[0].horizontal(|ui| {
                let (icon, status_text, color) = match &self.detector_status {
                    DetectorStatus::Stopped => ("⏹️", "Stopped", Color32::GRAY),
                    DetectorStatus::Running => ("▶️", "Running", Color32::GREEN),
                    DetectorStatus::Starting => ("⏳", "Starting...", Color32::YELLOW),
                    DetectorStatus::Error(e) => ("❌", "Error", Color32::RED),
                };
                ui.label(icon);
                ui.colored_label(color, status_text);
            });

            // FPS with color coding
            let fps_color = if self.motion_state.fps >= 25.0 {
                Color32::GREEN
            } else if self.motion_state.fps >= 15.0 {
                Color32::YELLOW
            } else {
                Color32::RED
            };
            columns[0].horizontal(|ui| {
                ui.label("📹 FPS:");
                ui.colored_label(fps_color, format!("{:.1}", self.motion_state.fps));
            });

            // Resolution
            columns[0].horizontal(|ui| {
                ui.label("📐 Resolution:");
                ui.label(format!(
                    "{}x{}",
                    self.motion_state.resolution.0, self.motion_state.resolution.1
                ));
            });

            // Right column - Motion status
            columns[1].heading("🎯 Motion");
            columns[1].separator();

            // Motion detection status
            columns[1].horizontal(|ui| {
                let (icon, text, color) = if self.motion_state.motion_detected {
                    ("🔴", "ACTIVE", Color32::RED)
                } else {
                    ("🟢", "CLEAR", Color32::GREEN)
                };
                ui.label(icon);
                ui.colored_label(color, text);
            });

            // Motion count with emphasis
            columns[1].horizontal(|ui| {
                ui.label("📊 Count:");
                if self.motion_state.motion_count > 0 {
                    ui.colored_label(
                        Color32::YELLOW,
                        format!("{}", self.motion_state.motion_count),
                    );
                } else {
                    ui.label("0");
                }
            });

            // Time since last motion
            if let Some(last_time) = self.motion_state.last_motion_time {
                let duration = Local::now().signed_duration_since(last_time);
                let time_text = if duration.num_seconds() < 60 {
                    format!("{}s ago", duration.num_seconds())
                } else if duration.num_minutes() < 60 {
                    format!("{}m ago", duration.num_minutes())
                } else {
                    format!("{}h ago", duration.num_hours())
                };
                columns[1].horizontal(|ui| {
                    ui.label("⏰ Last:");
                    ui.colored_label(Color32::from_rgb(200, 200, 255), time_text);
                });
            } else {
                columns[1].horizontal(|ui| {
                    ui.label("⏰ Last:");
                    ui.label("Never");
                });
            }
        });
    }

    fn render_motion_graph(&mut self, ui: &mut Ui) {
        ui.heading("📈 Motion Graph");
        ui.separator();

        // Current motion status
        ui.horizontal(|ui| {
            ui.label("Current:");
            if self.motion_state.motion_detected {
                ui.colored_label(Color32::GREEN, RichText::new("🟢 MOTION"));
            } else {
                ui.colored_label(Color32::RED, RichText::new("🔴 NO MOTION"));
            }

            ui.label(format!("History: {} points", self.motion_history.len()));
            ui.label(format!(
                "Events detected: {}",
                self.motion_history.iter().filter(|&x| *x).count()
            ));
        });

        ui.add_space(5.0);

        // Real-time motion graph visualization
        if self.motion_history.len() > 1 {
            let graph_height = 100.0;
            let graph_rect = ui.available_rect_before_wrap();
            let painter = ui.painter();

            // Graph background
            painter.rect_filled(graph_rect, 0.0, Color32::from_rgb(20, 20, 25));

            // Draw grid lines
            let grid_color = Color32::from_rgb(40, 40, 45);
            for i in 0..=5 {
                let y = graph_rect.min.y + (i as f32 / 5.0) * graph_height;
                painter.line_segment(
                    [pos2(graph_rect.min.x, y), pos2(graph_rect.max.x, y)],
                    Stroke::new(1.0, grid_color),
                );
            }

            // Draw motion line
            let mut last_point = None;
            for (i, motion) in self
                .motion_history
                .iter()
                .rev()
                .take(self.max_history_points)
                .enumerate()
            {
                let x = graph_rect.min.x
                    + (i as f32 / self.max_history_points as f32) * graph_rect.width();
                let y = if *motion {
                    graph_rect.min.y + graph_height * 0.2
                } else {
                    graph_rect.min.y + graph_height * 0.8
                };

                let current_point = pos2(x, y);

                // Connect points
                if let Some(last) = last_point {
                    let line_color = if *motion {
                        Color32::GREEN
                    } else {
                        Color32::RED
                    };
                    painter.line_segment([last, current_point], Stroke::new(2.0, line_color));
                }

                // Draw point
                let point_color = if *motion {
                    Color32::GREEN
                } else {
                    Color32::RED
                };
                let point_size = if *motion { 5.0 } else { 3.0 };
                painter.circle_filled(current_point, point_size, point_color);

                // Add glow effect for motion
                if *motion {
                    painter.circle_filled(
                        current_point,
                        point_size + 2.0,
                        Color32::from_rgba_premultiplied(0, 255, 0, 30),
                    );
                }

                last_point = Some(current_point);
            }

            // Draw threshold line
            let threshold_y = graph_rect.min.y + graph_height * 0.5;
            painter.line_segment(
                [
                    pos2(graph_rect.min.x, threshold_y),
                    pos2(graph_rect.max.x, threshold_y),
                ],
                Stroke::new(1.0, Color32::YELLOW),
            );
        } else {
            ui.label("Waiting for motion data...");
        }
    }

    fn render_log_panel(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.heading("📝 Activity Log");
            ui.checkbox(&mut self.auto_scroll, "Auto-scroll");
            if ui.button("Clear").clicked() {
                self.status_log.clear();
            }
        });
        ui.separator();

        ScrollArea::vertical()
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
    }

    fn render_menu_bar(&mut self, ctx: &egui::Context) {
        TopBottomPanel::top("menu_bar").show(ctx, |ui| {
            menu::bar(ui, |ui| {
                ui.menu_button("File", |ui| {
                    if ui.button("Quit").clicked() {
                        ctx.send_viewport_cmd(ViewportCommand::Close);
                    }
                });

                ui.menu_button("View", |ui| {
                    ui.checkbox(&mut self.show_about, "About");
                });

                ui.menu_button("Camera", |ui| {
                    if ui.button("Toggle Detection").clicked() {
                        if self.is_detecting {
                            self.is_detecting = false;
                            self.detector_status = DetectorStatus::Stopped;
                            self.status_log.push("Motion detection stopped".to_string());
                            if self.status_log.len() > 100 {
                                self.status_log.remove(0);
                            }
                            let _ = self.sender.send(GuiMessage::StopDetection);
                        } else {
                            self.is_detecting = true;
                            self.detector_status = DetectorStatus::Running;
                            self.status_log.push("Motion detection started".to_string());
                            if self.status_log.len() > 100 {
                                self.status_log.remove(0);
                            }
                            let _ = self.sender.send(GuiMessage::StartDetection);
                        }
                    }
                });
            });
        });
    }

    fn render_green_light_panel(&mut self, ctx: &egui::Context) {
        TopBottomPanel::top("motion_indicator").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.add_space(10.0);
                if self.motion_state.motion_detected {
                    // Animated green light
                    let pulse = (self.motion_animation_time * 3.0).sin() * 0.3 + 0.7;
                    let glow_color = Color32::from_rgb(
                        (pulse * 150.0) as u8,
                        (50.0 + pulse * 205.0) as u8,
                        (pulse * 150.0) as u8,
                    );

                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [100.0, 50.0],
                            Button::new("")
                                .fill(glow_color)
                                .stroke(Stroke::new(6.0, Color32::DARK_GREEN)),
                        );
                        ui.vertical(|ui| {
                            ui.colored_label(
                                Color32::GREEN,
                                RichText::new("🟢 MOTION DETECTED").size(24.0),
                            );
                        });
                    });
                } else {
                    ui.horizontal(|ui| {
                        ui.add_sized(
                            [100.0, 50.0],
                            Button::new("")
                                .fill(Color32::from_rgb(40, 40, 40))
                                .stroke(Stroke::new(2.0, Color32::GRAY)),
                        );
                        ui.vertical(|ui| {
                            ui.colored_label(
                                Color32::GRAY,
                                RichText::new("🔴 NO MOTION").size(24.0),
                            );
                        });
                    });
                }
            });
        });
    }
}

impl eframe::App for MotionDetectorGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update animation time
        self.motion_animation_time += ctx.input(|i| i.stable_dt);

        // Update motion detection state
        self.update_settings_from_receiver();

        // Render menu bar
        self.render_menu_bar(ctx);

        // Render prominent green light indicator at top
        self.render_green_light_panel(ctx);

        // About window
        if self.show_about {
            Window::new("About Motion Detector")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("Motion Detector v0.1.0");
                    ui.label("A Rust-based motion detection application");
                    ui.label("with enhanced GUI and real-time visualization");
                    ui.separator();
                    ui.label("Features:");
                    ui.label("• Real-time motion detection");
                    ui.label("• Animated green light indicators");
                    ui.label("• Live motion graph visualization");
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
        CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_top(|ui| {
                // Left panel - Controls
                ui.vertical(|ui| {
                    ui.set_min_width(300.0);
                    self.render_control_panel(ui);
                });

                ui.separator();

                // Middle panel - Status and Graph
                ui.vertical(|ui| {
                    ui.set_min_width(300.0);
                    self.render_status_panel(ui);
                    ui.add_space(10.0);
                    self.render_motion_graph(ui);
                });

                ui.separator();

                // Right panel - Activity Log
                ui.vertical(|ui| {
                    ui.set_min_width(350.0);
                    self.render_log_panel(ui);
                });
            });
        });
    }
}
