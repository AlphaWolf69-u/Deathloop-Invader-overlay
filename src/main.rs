mod overlay;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let app = overlay::OverlayApp::new()?;
    app.run();
    Ok(())
}