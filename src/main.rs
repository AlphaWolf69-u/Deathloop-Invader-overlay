use eframe::egui;

mod overlay;

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default()
            .with_decorations(false)
            .with_always_on_top(),
        ..Default::default()
    };

    eframe::run_native(
        "Deathloop Overlay",
        options,
        Box::new(|_cc| Ok(Box::new(overlay::OverlayApp::new()))),
    )
}