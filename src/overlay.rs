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

        let viewport = egui::ViewportBuilder::default()
            .with_title("Text Overlay")
            .with_inner_size([280.0, 20.0])
            .with_decorations(false)
            .with_transparent(true)
            .with_window_level(egui::WindowLevel::AlwaysOnTop)
            .with_resizable(false);

        let native_options = NativeOptions {
            viewport,
            ..Default::default()
        };

        let _ = eframe::run_native(
            "Text Overlay",
            native_options,
            Box::new(move |cc| {
                let mut fonts = egui::FontDefinitions::default();

                fonts.font_data.insert(
                    "handelson".to_owned(),
                    egui::FontData::from_static(include_bytes!(
                        "../assets/handelson-two.otf"
                    ))
                    .into(),
                );

                fonts
                    .families
                    .get_mut(&egui::FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "handelson".to_owned());

                cc.egui_ctx.set_fonts(fonts);

                Ok(Box::new(TextApp::new(game_process)))
            }),
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
    fn ui(&mut self, ui: &mut egui::Ui, _frame: &mut eframe::Frame) {
        let mut visuals = egui::Visuals::dark();

        visuals.override_text_color = Some(egui::Color32::WHITE);
        visuals.widgets.inactive.bg_fill = egui::Color32::TRANSPARENT;
        visuals.panel_fill = egui::Color32::TRANSPARENT;
        visuals.faint_bg_color = egui::Color32::TRANSPARENT;
        visuals.extreme_bg_color = egui::Color32::TRANSPARENT;

        ui.ctx().set_visuals(visuals);

        egui::CentralPanel::default()
            .frame(egui::Frame::NONE)
            .show_inside(ui, |ui| {
                ui.horizontal_centered(|ui| {
                    let addr = self.game_process.base_address + 0x3335638;

                    let label = match self.game_process.read_string(addr, 256) {
                        Ok(name) => format!("Host: {}", name),
                        Err(e) => format!("Error: {}", e),
                    };

                    ui.label(
                        egui::RichText::new(label)
                            .size(20.0)
                            .color(egui::Color32::WHITE),
                    );
                });

                ui.add_space(6.0);
            });

        ui.ctx().request_repaint();
    }
}