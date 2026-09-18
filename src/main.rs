mod controls;
mod keyboard;
mod network;
mod overlay;
const EXTENDED: bool = false;
fn extra(_: &deathloop_invader_tool::GameProcess, _: bool, _: bool) -> Option<String> {
    None
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    if std::env::args().any(|a| a == "--inspect") {
        let game = deathloop_invader_tool::GameProcess::attach("Deathloop.exe", "Deathloop.exe")?;
        println!(
            "PID {} | base {:X} | {:?}",
            game.pid,
            game.base_address,
            game.opponent()?
        );
        println!("Network: {:?}", network::read(&game));
        return Ok(());
    }
    let app = overlay::OverlayApp::new()?;
    app.run()?;
    Ok(())
}
