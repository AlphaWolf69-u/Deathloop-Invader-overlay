use deathloop_cheat::GameProcess;
use eframe::egui;
use windows_sys::Win32::Foundation::HWND;
use windows_sys::Win32::UI::WindowsAndMessaging::{FindWindowW, GetWindowLongPtrW, SetWindowLongPtrW, SetLayeredWindowAttributes, GWL_EXSTYLE, WS_EX_TRANSPARENT, WS_EX_LAYERED, WS_EX_NOACTIVATE, LWA_COLORKEY};

fn get_hwnd() -> Option<HWND> {
    let title = "Deathloop Overlay";
    let title_wide: Vec<u16> = title.encode_utf16().chain(std::iter::once(0)).collect();
    let hwnd = unsafe { FindWindowW(std::ptr::null(), title_wide.as_ptr()) };
    if hwnd.is_null() { None } else { Some(hwnd) }
}

pub struct OverlayApp {
    game: Option<GameProcess>,
    host_name: String,
    last_update: std::time::Instant,
    initialized: bool,
}

impl OverlayApp {
    pub fn new() -> Self {
        Self {
            game: GameProcess::attach("Deathloop.exe", "Deathloop.exe").ok(),  // Fixed: two arguments
            host_name: "Waiting for Deathloop...".to_string(),
            last_update: std::time::Instant::now(),
            initialized: false,
        }
    }
}

impl eframe::App for OverlayApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if !self.initialized {
            if let Some(hwnd) = get_hwnd() {
                unsafe {
                    let current_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
                    let new_style = current_style | WS_EX_TRANSPARENT as isize | WS_EX_LAYERED as isize | WS_EX_NOACTIVATE as isize;
                    SetWindowLongPtrW(hwnd, GWL_EXSTYLE, new_style);
                    // Magenta color key for transparency
                    SetLayeredWindowAttributes(hwnd, 0x00FF00FF, 0, LWA_COLORKEY);
                }
            }
            self.initialized = true;
        }

        if self.last_update.elapsed().as_millis() > 300 {
            if let Some(game) = &self.game {
                let addr = game.base_address + 0x3335638;   // Fixed: base_address
                self.host_name = game.read_string(addr, 128);
            }
            self.last_update = std::time::Instant::now();
        }

        egui::CentralPanel::default()
            .frame(egui::Frame {
                fill: egui::Color32::TRANSPARENT,
                ..Default::default()
            })
            .show(ctx, |ui| {
                ui.style_mut().visuals.panel_fill = egui::Color32::from_rgb(255, 0, 255);
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