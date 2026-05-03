use deathloop_cheat::GameProcess;
use eframe::egui;

pub struct OverlayApp {
    game: Option<GameProcess>,
    host_name: String,
    last_update: std::time::Instant,
}

impl OverlayApp {
    pub fn new() -> Self {
        Self {
            game: GameProcess::attach("Deathloop.exe", "Deathloop.exe").ok(),  // Fixed: two arguments
            host_name: "Waiting for Deathloop...".to_string(),
            last_update: std::time::Instant::now(),
        }
    }
}

impl eframe::App for OverlayApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if self.last_update.elapsed().as_millis() > 300 {
            if let Some(game) = &self.game {
                let addr = game.base_address + 0x3335638;   // Fixed: base_address
                self.host_name = game.read_string(addr, 128);
            }
            self.last_update = std::time::Instant::now();
        }

        egui::CentralPanel::default()
            .frame(egui::Frame::none())
            .show(ctx, |ui| {
                ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                    ui.add_space(80.0);
                    ui.label(
                        egui::RichText::new(format!("Host: {}", self.host_name))
                            .size(32.0)
                            .color(egui::Color32::from_rgb(0, 255, 120))
                            .strong(),
                    );

                    ui.label(
                        egui::RichText::new("Deathloop Overlay - Press F12 to toggle")
                            .size(14.0)
                            .color(egui::Color32::GRAY),
                    );
                });
            });

        ctx.request_repaint_after(std::time::Duration::from_millis(100));
    }
}