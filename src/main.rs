#[cfg(test)]
mod tests;

mod gui;

use anyhow::Result;
use chrono::Local;
use clap::Parser;
use opencv::{
    core::{self, Mat, Vector},
    imgcodecs, imgproc,
    prelude::*,
    videoio::{VideoCapture, CAP_ANY},
};
use std::thread;
use std::time::{Duration, Instant};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    /// Camera device index (default: 0)
    #[arg(short, long, default_value = "0")]
    device: u32,

    /// Motion detection sensitivity (0.0-1.0, default: 0.3)
    #[arg(short, long, default_value = "0.3")]
    sensitivity: f64,

    /// Minimum area for motion detection (default: 500)
    #[arg(short, long, default_value = "500")]
    min_area: u32,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Enable GUI control panel
    #[arg(short, long)]
    gui: bool,
}

struct MotionDetector {
    camera: VideoCapture,
    #[allow(dead_code)]
    sensitivity: f64,
    min_area: u32,
    previous_frame: Mat,
    frame_count: u32,
    motion_count: u32,
    last_motion_time: Option<Instant>,
    last_fps_update: Instant,
    fps_frames: u32,
    current_fps: f32,
}

impl MotionDetector {
    fn new(device: u32, sensitivity: f64, min_area: u32) -> Result<Self> {
        let mut camera = VideoCapture::new(device as i32, CAP_ANY)?;

        if !camera.is_opened()? {
            return Err(anyhow::anyhow!(
                "Failed to open camera device {} - check if device exists and user has permissions",
                device
            ));
        }

        // Set camera properties for better performance
        camera.set(opencv::videoio::CAP_PROP_FRAME_WIDTH, 640.0)?;
        camera.set(opencv::videoio::CAP_PROP_FRAME_HEIGHT, 480.0)?;
        camera.set(opencv::videoio::CAP_PROP_FPS, 30.0)?;

        let mut frame = Mat::default();
        camera.read(&mut frame)?;

        if frame.empty() {
            return Err(anyhow::anyhow!("Failed to capture initial frame"));
        }

        // Convert to grayscale and blur for initial frame to match detection format
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

        Ok(Self {
            camera,
            sensitivity,
            min_area,
            previous_frame: blurred,
            frame_count: 0,
            motion_count: 0,
            last_motion_time: None,
            last_fps_update: Instant::now(),
            fps_frames: 0,
            current_fps: 0.0,
        })
    }

    fn detect_motion(&mut self) -> Result<bool> {
        let mut current_frame = Mat::default();

        if !self.camera.read(&mut current_frame)? {
            return Err(anyhow::anyhow!("Failed to capture frame"));
        }

        if current_frame.empty() {
            return Ok(false);
        }

        // Convert to grayscale
        let mut gray = Mat::default();
        imgproc::cvt_color(&current_frame, &mut gray, imgproc::COLOR_BGR2GRAY, 0)?;

        // Apply Gaussian blur to reduce noise
        let mut blurred = Mat::default();
        imgproc::gaussian_blur(
            &gray,
            &mut blurred,
            opencv::core::Size::new(21, 21),
            0.0,
            0.0,
            opencv::core::BORDER_DEFAULT,
        )?;

        // Compute difference between current frame and previous frame
        let mut diff = Mat::default();
        core::absdiff(&blurred, &self.previous_frame, &mut diff)?;

        // Apply threshold to get binary image
        let mut thresh = Mat::default();
        imgproc::threshold(&diff, &mut thresh, 25.0, 255.0, imgproc::THRESH_BINARY)?;

        // Dilate to fill in holes
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
        let mut contours = Vector::<Vector<opencv::core::Point>>::new();
        imgproc::find_contours(
            &dilated,
            &mut contours,
            imgproc::RETR_EXTERNAL,
            imgproc::CHAIN_APPROX_SIMPLE,
            opencv::core::Point::new(-1, -1),
        )?;

        // Check if any contour meets the minimum area requirement
        let mut motion_detected = false;
        for contour in &contours {
            let area = imgproc::contour_area(&contour, false)?;
            if area > self.min_area as f64 {
                motion_detected = true;
                break;
            }
        }

        // Update previous frame
        self.previous_frame = blurred;
        self.frame_count += 1;

        // Update FPS calculation
        self.fps_frames += 1;
        let now = Instant::now();
        if now.duration_since(self.last_fps_update) >= Duration::from_secs(1) {
            self.current_fps = self.fps_frames as f32;
            self.fps_frames = 0;
            self.last_fps_update = now;
        }

        // Update motion count and time
        if motion_detected {
            self.motion_count += 1;
            self.last_motion_time = Some(now);
        }

        Ok(motion_detected)
    }

    fn save_snapshot(&self, frame: &Mat) -> Result<String> {
        // Create pics directory if it doesn't exist
        std::fs::create_dir_all("pics")?;

        let timestamp = Local::now().format("%Y%m%d_%H%M%S");
        let filename = format!("pics/motion_{}.jpg", timestamp);
        imgcodecs::imwrite(&filename, frame, &Vector::new())?;
        Ok(filename)
    }

    #[allow(dead_code)]
    fn release(&mut self) {
        let _ = self.camera.release();
    }

    fn list_cameras() -> Result<Vec<String>> {
        let mut cameras = Vec::new();

        // Try to detect available cameras (typically 0-3)
        for i in 0..4 {
            let mut cam = VideoCapture::new(i, CAP_ANY)?;
            if cam.is_opened()? {
                let mut frame = Mat::default();
                if cam.read(&mut frame)? && !frame.empty() {
                    cameras.push(format!("Camera {} - {}x{}", i, frame.cols(), frame.rows()));
                }
                cam.release()?;
            }
        }

        Ok(cameras)
    }
}

fn run_cli_mode(args: Args) -> Result<()> {
    let mut detector = MotionDetector::new(args.device, args.sensitivity, args.min_area)?;

    if args.verbose {
        println!("Motion detector active. Press Ctrl+C to stop.");
    }

    let mut motion_count = 0;
    let mut last_motion_time = std::time::Instant::now();

    loop {
        match detector.detect_motion() {
            Ok(true) => {
                let now = std::time::Instant::now();
                if now.duration_since(last_motion_time) > Duration::from_secs(2) {
                    motion_count += 1;
                    last_motion_time = now;

                    let timestamp = Local::now().format("%Y-%m-%d %H:%M:%S");
                    println!("[{}] MOTION DETECTED! (#{})", timestamp, motion_count);

                    // Save snapshot when motion is detected
                    if let Ok(filename) = detector.save_snapshot(&detector.previous_frame) {
                        println!("  Snapshot saved: {}", filename);
                    }
                }
            }
            Ok(false) => {
                // No motion detected, continue
            }
            Err(e) => {
                eprintln!("Error detecting motion: {}", e);
                std::thread::sleep(Duration::from_secs(1));
            }
        }

        // Small delay to prevent excessive CPU usage
        std::thread::sleep(Duration::from_millis(33)); // ~30 FPS
    }
}

fn run_gui_mode() -> Result<()> {
    use crossbeam_channel::bounded;
    use gui::{GuiMessage, MotionDetectorGui, MotionState};

    let (gui_sender, detector_receiver) = bounded::<GuiMessage>(100);
    let (detector_sender, gui_state_receiver) = bounded::<MotionState>(100);

    // Start detector thread
    let detector_handle =
        thread::spawn(move || run_detector_thread(detector_receiver, detector_sender));

    // Start the GUI in the main thread
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_inner_size([800.0, 600.0])
            .with_title("Motion Detector"),
        ..Default::default()
    };

    eframe::run_native(
        "Motion Detector",
        options,
        Box::new(move |cc| {
            let mut gui = MotionDetectorGui::new_with_sender(cc, gui_sender.clone());
            gui.state_receiver = Some(gui_state_receiver.clone());
            Box::new(gui)
        }),
    )
    .map_err(|e| anyhow::anyhow!("GUI error: {}", e))?;

    // Wait for detector thread to finish
    let _ = detector_handle.join();

    Ok(())
}

fn run_detector_thread(
    receiver: crossbeam_channel::Receiver<gui::GuiMessage>,
    sender: crossbeam_channel::Sender<gui::MotionState>,
) -> Result<()> {
    use gui::{GuiMessage, MotionState};

    let mut detector = match MotionDetector::new(0, 0.3, 500) {
        Ok(det) => det,
        Err(e) => {
            eprintln!("ERROR: Failed to initialize detector: {}", e);
            return Err(e);
        }
    };
    let mut is_running = false;
    let mut last_snapshot_time = std::time::Instant::now();

    loop {
        // Process GUI messages
        while let Ok(msg) = receiver.try_recv() {
            match msg {
                GuiMessage::StartDetection => {
                    println!("DEBUG: Received StartDetection message");
                    is_running = true;
                }
                GuiMessage::StopDetection => {
                    println!("DEBUG: Received StopDetection message");
                    is_running = false;
                }
                GuiMessage::UpdateSensitivity(s) => {
                    detector.sensitivity = s;
                }
                GuiMessage::UpdateMinArea(area) => {
                    detector.min_area = area;
                }
                GuiMessage::UpdateDevice(device) => {
                    // Stop detection first
                    is_running = false;

                    // Release current camera
                    let _ = detector.camera.release();

                    // Small delay to ensure camera is fully released
                    std::thread::sleep(Duration::from_millis(100));

                    // Try to create new detector with new device
                    match MotionDetector::new(device, detector.sensitivity, detector.min_area) {
                        Ok(new_detector) => {
                            detector = new_detector;
                            println!("Successfully switched to device {}", device);
                        }
                        Err(e) => {
                            eprintln!("Failed to switch to device {}: {}", device, e);
                            // Try to recreate with original device (0) as fallback
                            match MotionDetector::new(0, detector.sensitivity, detector.min_area) {
                                Ok(fallback_detector) => {
                                    detector = fallback_detector;
                                    println!("Fallback to device 0 successful");
                                }
                                Err(fallback_err) => {
                                    eprintln!("Fallback to device 0 also failed: {}", fallback_err);
                                }
                            }
                        }
                    }
                }
                GuiMessage::SaveSnapshot => {
                    // Save current frame as snapshot
                    if let Err(e) = detector.save_snapshot(&detector.previous_frame) {
                        eprintln!("Failed to save snapshot: {}", e);
                    }
                }
            }
        }

        // Run detection if active
        if is_running {
            match detector.detect_motion() {
                Ok(motion_detected) => {
                    let motion_state = MotionState {
                        motion_detected,
                        motion_count: detector.motion_count,
                        last_motion_time: detector.last_motion_time.map(|_| Local::now()),
                        fps: detector.current_fps,
                        resolution: (640, 480), // TODO: Get actual resolution
                    };

                    // Send state to GUI (non-blocking)
                    let _ = sender.try_send(motion_state.clone());

                    // Save snapshot when motion is detected (same logic as CLI mode)
                    if motion_detected {
                        let now = std::time::Instant::now();
                        if now.duration_since(last_snapshot_time) > Duration::from_secs(2) {
                            if let Ok(filename) = detector.save_snapshot(&detector.previous_frame) {
                                println!("  Motion snapshot saved: {}", filename);
                                last_snapshot_time = now;
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Detection error: {}", e);
                    thread::sleep(Duration::from_secs(1));
                }
            }
        } else {
            thread::sleep(Duration::from_millis(100));
        }
    }
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.gui {
        run_gui_mode()
    } else {
        if args.verbose {
            println!("Motion Detector Starting...");
            println!("Device: {}", args.device);
            println!("Sensitivity: {}", args.sensitivity);
            println!("Min Area: {}", args.min_area);

            // List available cameras
            match MotionDetector::list_cameras() {
                Ok(cameras) => {
                    println!("Available cameras:");
                    for camera in cameras {
                        println!("  {}", camera);
                    }
                }
                Err(e) => println!("Warning: Could not list cameras: {}", e),
            }
        }

        run_cli_mode(args)
    }
}
