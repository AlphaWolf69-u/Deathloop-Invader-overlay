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
            Box::new(move |cc| {
                // Embed and register the bundled font at creation time
                let mut fonts = egui::FontDefinitions::default();
                fonts.font_data.insert(
                    "handelson".to_owned(),
                    egui::FontData::from_static(include_bytes!("../assets/handelson-two.otf")),
                );
                fonts
                    .families
                    .get_mut(&egui::FontFamily::Proportional)
                    .unwrap()
                    .insert(0, "handelson".to_owned());
                cc.egui_ctx.set_fonts(fonts);

                Box::new(TextApp::new(game_process))
            }),
        );
    }
}

struct TextApp {
    game_process: GameProcess,
    styles_applied: bool,
}

impl TextApp {
    fn new(game_process: GameProcess) -> Self {
        Self { game_process, styles_applied: false }
    }

    fn apply_window_styles_once(&mut self) {
        use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowLongPtrW, SetWindowLongPtrW, SetLayeredWindowAttributes, GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TRANSPARENT};
        use std::ptr;

        unsafe {
            let title: Vec<u16> = "Text Overlay".encode_utf16().chain(Some(0)).collect();
            let hwnd = FindWindowW(ptr::null(), title.as_ptr());
            if hwnd != ptr::null_mut() {
                let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | (WS_EX_LAYERED | WS_EX_TRANSPARENT) as isize);
                // Ensure layered window attributes enabled (per-window alpha/composition)
                SetLayeredWindowAttributes(hwnd, 0, 255, 0x00000002);
            }
        }
        self.styles_applied = true;
    }
}

impl eframe::App for TextApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.styles_applied {
            self.apply_window_styles_once();
        }
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

