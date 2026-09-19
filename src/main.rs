#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

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
        // CLI diagnostics may reuse the launching terminal, never create one.
        #[cfg(target_os = "windows")]
        unsafe {
            attach_parent_console();
        }
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

#[cfg(target_os = "windows")]
unsafe fn attach_parent_console() {
    #[link(name = "kernel32")]
    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
    }
    unsafe {
        AttachConsole(u32::MAX);
    }
}
