use deathloop_cheat::GameProcess;
use eframe::{egui, NativeOptions};
use std::error::Error;

pub struct OverlayApp {
    game_process: GameProcess,
}

impl OverlayApp {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let game_process = GameProcess::attach("Deathloop.exe", "Deathloop.exe")?;
        Ok(Self { game_process })
    }

    pub fn run(self) {
        let game_process = self.game_process;

        let window_size = egui::vec2(280.0, 20.0);

        let native_options = NativeOptions {
            decorated: false,
            transparent: true,
            always_on_top: true,
            initial_window_size: Some(window_size),
            resizable: false,
            ..Default::default()
        };

        let _ = eframe::run_native(
            "Text Overlay",
            native_options,
            Box::new(move |_cc| Box::new(TextApp::new(game_process))),
        );
    }
}

struct TextApp {
    game_process: GameProcess,
}

impl TextApp {
    fn new(game_process: GameProcess) -> Self {
        Self { game_process }
    }
}

impl eframe::App for TextApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Transparent visuals
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(egui::Color32::WHITE);
        visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
        visuals.panel_fill = egui::Color32::TRANSPARENT;
        visuals.faint_bg_color = egui::Color32::TRANSPARENT;
        visuals.extreme_bg_color = egui::Color32::TRANSPARENT;
        visuals.window_rounding = 0.0.into();
        ctx.set_visuals(visuals);

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.horizontal_centered(|ui| {
                let font_size = 20.0;
                let addr = self.game_process.base_address + 0x3335638;
                let label = match self.game_process.read_string(addr, 256) {
                    Ok(name) => format!("Host: {}", name),
                    Err(e) => format!("Error: {}", e),
                };

                ui.label(egui::RichText::new(label).size(font_size).color(egui::Color32::WHITE));
            });

            ui.add_space(6.0);
        });

        // Keep repainting to update overlay text
        ctx.request_repaint();
    }
}

