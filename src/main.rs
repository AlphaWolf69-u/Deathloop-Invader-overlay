use deathloop_cheat::GameProcess;   // your library name

fn main() {
    let game = GameProcess::attach("Deathloop.exe", "Deathloop.exe")
        .expect("Failed to attach to Deathloop");

    let host_name_address = game.base_address + 0x3335638;

    loop {
        let name = game.read_string(host_name_address, 128);
        println!("✅ Host name: '{}'", name);
        std::thread::sleep(std::time::Duration::from_secs(5));
    }

    // game.close(); // will run when dropped, but you can call manually
}