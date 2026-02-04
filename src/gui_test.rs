// Simple GUI test without OpenCV
use eframe;
use eframe::egui;

struct SimpleGui {
    value: f32,
    text: String,
}

impl Default for SimpleGui {
    fn default() -> Self {
        Self {
            value: 0.3,
            text: "Hello Motion Detector GUI!".to_string(),
        }
    }
}

impl eframe::App for SimpleGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("🎥 Motion Detector GUI Test");
            ui.separator();

            ui.horizontal(|ui| {
                ui.label("Sensitivity:");
                ui.add(egui::Slider::new(&mut self.value, 0.0..=1.0));
                ui.label(format!("{:.2}", self.value));
            });

            ui.separator();
            ui.label(&self.text);

            if ui.button("Test Button").clicked() {
                self.text = format!("Button clicked! Value: {:.2}", self.value);
            }
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([400.0, 300.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Simple GUI Test",
        options,
        Box::new(|_cc| Box::new(SimpleGui::default())),
    )
}
