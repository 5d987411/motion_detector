// Working GUI that connects to actual motion detection
use chrono::Local;
use eframe;
use eframe::egui;
use opencv::{
    core::{self, Mat},
    imgproc,
    prelude::*,
    videoio::{VideoCapture, CAP_ANY},
};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Clone, Debug)]
pub enum GuiCommand {
    StartDetection,
    StopDetection,
    UpdateSensitivity(f64),
    UpdateMinArea(u32),
    SaveSnapshot,
}

#[derive(Clone, Debug)]
pub struct DetectorStatus {
    pub is_running: bool,
    pub motion_detected: bool,
    pub motion_count: u32,
    pub fps: f32,
    pub resolution: (i32, i32),
    pub last_motion_time: Option<Instant>,
    pub error_message: Option<String>,
}

struct MotionDetectorGui {
    // Communication with detector thread
    command_sender: crossbeam_channel::Sender<GuiCommand>,
    status_receiver: crossbeam_channel::Receiver<DetectorStatus>,

    // Current settings
    sensitivity: f64,
    min_area: u32,

    // Current status
    status: DetectorStatus,

    // UI state
    status_log: Vec<String>,
    auto_scroll: bool,
    show_about: bool,
}

impl MotionDetectorGui {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let (cmd_tx, cmd_rx) = crossbeam_channel::bounded(100);
        let (status_tx, status_rx) = crossbeam_channel::bounded(100);

        // Start motion detector thread
        thread::spawn(move || {
            run_motion_detector(cmd_rx, status_tx);
        });

        Self {
            command_sender: cmd_tx,
            status_receiver: status_rx,
            sensitivity: 0.3,
            min_area: 500,
            status: DetectorStatus {
                is_running: false,
                motion_detected: false,
                motion_count: 0,
                fps: 0.0,
                resolution: (1920, 1080),
                last_motion_time: None,
                error_message: None,
            },
            status_log: vec!["GUI Control Panel Started".to_string()],
            auto_scroll: true,
            show_about: false,
        }
    }

    fn update_status(&mut self) {
        while let Ok(status) = self.status_receiver.try_recv() {
            self.status = status;

            if self.status.motion_detected {
                self.status_log.push(format!(
                    "Motion detected! (#{}). FPS: {:.1}",
                    self.status.motion_count, self.status.fps
                ));
                if self.status_log.len() > 100 {
                    self.status_log.remove(0);
                }
            }

            if let Some(ref error) = self.status.error_message {
                self.status_log.push(format!("Error: {}", error));
            }
        }
    }
}

impl eframe::App for MotionDetectorGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        self.update_status();

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
            });
        });

        // About window
        if self.show_about {
            egui::Window::new("About Motion Detector")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.heading("Motion Detector v0.1.0");
                    ui.label("Real-time motion detection with GUI control");
                    ui.separator();
                    ui.label("Features:");
                    ui.label("• Live camera feed processing");
                    ui.label("• Configurable sensitivity");
                    ui.label("• Motion event logging");
                    ui.label("• Snapshot capture");
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

                    // Sensitivity slider
                    ui.horizontal(|ui| {
                        ui.label("Sensitivity:");
                        if ui
                            .add(egui::Slider::new(&mut self.sensitivity, 0.0..=1.0))
                            .changed()
                        {
                            let _ = self
                                .command_sender
                                .send(GuiCommand::UpdateSensitivity(self.sensitivity));
                            self.status_log
                                .push(format!("Sensitivity: {:.2}", self.sensitivity));
                        }
                        ui.label(format!("{:.2}", self.sensitivity));
                    });

                    // Min area slider
                    ui.horizontal(|ui| {
                        ui.label("Min Area:");
                        if ui
                            .add(egui::Slider::new(&mut self.min_area, 50..=5000))
                            .changed()
                        {
                            let _ = self
                                .command_sender
                                .send(GuiCommand::UpdateMinArea(self.min_area));
                            self.status_log
                                .push(format!("Min Area: {} px", self.min_area));
                        }
                        ui.label(format!("{} px", self.min_area));
                    });

                    ui.add_space(10.0);

                    // Detection toggle
                    ui.horizontal(|ui| {
                        if self.status.is_running {
                            if ui
                                .add(
                                    egui::Button::new("⏹️ Stop Detection").fill(egui::Color32::RED),
                                )
                                .clicked()
                            {
                                let _ = self.command_sender.send(GuiCommand::StopDetection);
                                self.status_log.push("Detection stopped".to_string());
                            }
                        } else {
                            if ui
                                .add(
                                    egui::Button::new("▶️ Start Detection")
                                        .fill(egui::Color32::GREEN),
                                )
                                .clicked()
                            {
                                let _ = self.command_sender.send(GuiCommand::StartDetection);
                                self.status_log.push("Detection started".to_string());
                            }
                        }

                        if ui.add(egui::Button::new("📸 Save Snapshot")).clicked() {
                            let _ = self.command_sender.send(GuiCommand::SaveSnapshot);
                            self.status_log.push("Manual snapshot saved".to_string());
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
                        let status_text = if self.status.is_running {
                            "Running"
                        } else {
                            "Stopped"
                        };
                        let status_color = if self.status.is_running {
                            egui::Color32::GREEN
                        } else {
                            egui::Color32::GRAY
                        };
                        ui.colored_label(status_color, status_text);
                    });

                    // Motion status
                    ui.horizontal(|ui| {
                        ui.label("Motion:");
                        let motion_color = if self.status.motion_detected {
                            egui::Color32::RED
                        } else {
                            egui::Color32::GREEN
                        };
                        let motion_text = if self.status.motion_detected {
                            "DETECTED"
                        } else {
                            "None"
                        };
                        ui.colored_label(motion_color, motion_text);

                        ui.label(format!("Count: {}", self.status.motion_count));
                    });

                    // FPS and resolution
                    ui.horizontal(|ui| {
                        ui.label(format!("FPS: {:.1}", self.status.fps));
                        ui.label(format!(
                            "Resolution: {}x{}",
                            self.status.resolution.0, self.status.resolution.1
                        ));
                    });

                    // Last motion time
                    if let Some(last_time) = self.status.last_motion_time {
                        let duration = last_time.elapsed();
                        ui.label(format!("Last Motion: {:.1}s ago", duration.as_secs_f32()));
                    }

                    // Visual indicator
                    ui.add_space(10.0);
                    ui.heading("Motion Indicator:");
                    let indicator_color = if self.status.motion_detected {
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

fn run_motion_detector(
    command_receiver: crossbeam_channel::Receiver<GuiCommand>,
    status_sender: crossbeam_channel::Sender<DetectorStatus>,
) {
    let mut detector = match create_detector() {
        Ok(det) => det,
        Err(e) => {
            let _ = status_sender.send(DetectorStatus {
                is_running: false,
                motion_detected: false,
                motion_count: 0,
                fps: 0.0,
                resolution: (0, 0),
                last_motion_time: None,
                error_message: Some(format!("Failed to initialize camera: {}", e)),
            });
            return;
        }
    };

    let mut is_detecting = false;
    let mut _last_motion_time = Instant::now();
    let mut frame_count = 0;
    let mut last_fps_update = Instant::now();
    let mut current_fps = 0.0;

    loop {
        // Process commands
        while let Ok(cmd) = command_receiver.try_recv() {
            match cmd {
                GuiCommand::StartDetection => is_detecting = true,
                GuiCommand::StopDetection => is_detecting = false,
                GuiCommand::UpdateSensitivity(s) => detector.sensitivity = s,
                GuiCommand::UpdateMinArea(a) => detector.min_area = a,
                GuiCommand::SaveSnapshot => {
                    let timestamp = Local::now().format("%Y%m%d_%H%M%S");
                    let filename = format!("motion_{}.jpg", timestamp);
                    if let Err(e) = opencv::imgcodecs::imwrite(
                        &filename,
                        &detector.previous_frame,
                        &opencv::core::Vector::new(),
                    ) {
                        eprintln!("Failed to save snapshot: {}", e);
                    }
                }
            }
        }

        if is_detecting {
            match detect_motion_frame(&mut detector) {
                Ok(motion_detected) => {
                    frame_count += 1;

                    // Update FPS
                    let now = Instant::now();
                    if now.duration_since(last_fps_update) >= Duration::from_secs(1) {
                        current_fps = frame_count as f32;
                        frame_count = 0;
                        last_fps_update = now;
                    }

                    if motion_detected {
                        detector.motion_count += 1;
                        _last_motion_time = now;

                        // Save snapshot
                        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
                        let filename = format!("motion_{}.jpg", timestamp);
                        let _ = opencv::imgcodecs::imwrite(
                            &filename,
                            &detector.previous_frame,
                            &opencv::core::Vector::new(),
                        );
                    }

                    // Send status update
                    let _ = status_sender.send(DetectorStatus {
                        is_running: true,
                        motion_detected,
                        motion_count: detector.motion_count,
                        fps: current_fps,
                        resolution: (
                            detector.previous_frame.cols(),
                            detector.previous_frame.rows(),
                        ),
                        last_motion_time: if motion_detected {
                            Some(now)
                        } else {
                            detector.last_motion_time
                        },
                        error_message: None,
                    });
                }
                Err(e) => {
                    let _ = status_sender.send(DetectorStatus {
                        is_running: false,
                        motion_detected: false,
                        motion_count: detector.motion_count,
                        fps: current_fps,
                        resolution: (0, 0),
                        last_motion_time: detector.last_motion_time,
                        error_message: Some(format!("Detection error: {}", e)),
                    });
                    thread::sleep(Duration::from_secs(1));
                }
            }
        } else {
            // Send status when not detecting
            let _ = status_sender.send(DetectorStatus {
                is_running: false,
                motion_detected: false,
                motion_count: detector.motion_count,
                fps: current_fps,
                resolution: (
                    detector.previous_frame.cols(),
                    detector.previous_frame.rows(),
                ),
                last_motion_time: detector.last_motion_time,
                error_message: None,
            });
            thread::sleep(Duration::from_millis(100));
        }

        thread::sleep(Duration::from_millis(33)); // ~30 FPS
    }
}

struct SimpleDetector {
    camera: VideoCapture,
    sensitivity: f64,
    min_area: u32,
    previous_frame: Mat,
    motion_count: u32,
    last_motion_time: Option<Instant>,
}

fn create_detector() -> anyhow::Result<SimpleDetector> {
    let mut camera = VideoCapture::new(0, CAP_ANY)?;

    if !camera.is_opened()? {
        return Err(anyhow::anyhow!("Failed to open camera"));
    }

    camera.set(opencv::videoio::CAP_PROP_FRAME_WIDTH, 640.0)?;
    camera.set(opencv::videoio::CAP_PROP_FRAME_HEIGHT, 480.0)?;

    let mut frame = Mat::default();
    camera.read(&mut frame)?;

    if frame.empty() {
        return Err(anyhow::anyhow!("Failed to capture initial frame"));
    }

    // Convert to grayscale and blur
    let mut gray = Mat::default();
    imgproc::cvt_color(&frame, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;
    let mut blurred = Mat::default();
    imgproc::gaussian_blur(
        &gray,
        &mut blurred,
        opencv::core::Size::new(21, 21),
        0.0,
        0.0,
        opencv::core::BORDER_DEFAULT,
    )?;

    Ok(SimpleDetector {
        camera,
        sensitivity: 0.3,
        min_area: 500,
        previous_frame: blurred,
        motion_count: 0,
        last_motion_time: None,
    })
}

fn detect_motion_frame(detector: &mut SimpleDetector) -> anyhow::Result<bool> {
    let mut current_frame = Mat::default();

    if !detector.camera.read(&mut current_frame)? {
        return Err(anyhow::anyhow!("Failed to capture frame"));
    }

    if current_frame.empty() {
        return Ok(false);
    }

    // Convert to grayscale and blur
    let mut gray = Mat::default();
    imgproc::cvt_color(&current_frame, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;
    let mut blurred = Mat::default();
    imgproc::gaussian_blur(
        &gray,
        &mut blurred,
        opencv::core::Size::new(21, 21),
        0.0,
        0.0,
        opencv::core::BORDER_DEFAULT,
    )?;

    // Compute difference
    let mut diff = Mat::default();
    core::absdiff(&blurred, &detector.previous_frame, &mut diff)?;

    // Apply threshold
    let mut thresh = Mat::default();
    imgproc::threshold(&diff, &mut thresh, 25.0, 255.0, imgproc::THRESH_BINARY)?;

    // Dilate
    let mut dilated = Mat::default();
    let kernel = Mat::ones(3, 3, opencv::core::CV_8UC1)?;
    imgproc::dilate(
        &thresh,
        &mut dilated,
        &kernel,
        opencv::core::Point::new(-1, -1),
        2,
        opencv::core::BORDER_DEFAULT,
        opencv::core::Scalar::all(0.0),
    )?;

    // Find contours
    let mut contours = opencv::core::Vector::<opencv::core::Vector<opencv::core::Point>>::new();
    imgproc::find_contours(
        &dilated,
        &mut contours,
        imgproc::RETR_EXTERNAL,
        imgproc::CHAIN_APPROX_SIMPLE,
        opencv::core::Point::new(-1, -1),
    )?;

    // Check for motion
    let mut motion_detected = false;
    for contour in &contours {
        let area = imgproc::contour_area(&contour, false)?;
        if area > detector.min_area as f64 {
            motion_detected = true;
            break;
        }
    }

    // Update previous frame
    detector.previous_frame = blurred;

    Ok(motion_detected)
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([900.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Motion Detector Control Panel",
        options,
        Box::new(|cc| Box::new(MotionDetectorGui::new(cc))),
    )
}
