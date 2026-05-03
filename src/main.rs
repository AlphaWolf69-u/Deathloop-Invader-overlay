use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, HINSTANCE, HWND, BOOL},
    System::{
        Diagnostics::Debug::{ReadProcessMemory, WriteProcessMemory},
        ProcessStatus::EnumProcesses,
        Threading::{OpenProcess, PROCESS_VM_READ, PROCESS_VM_WRITE, PROCESS_QUERY_INFORMATION},
    },
    UI::Input::KeyboardAndMouse::{GetAsyncKeyState, VK_SPACE},
};
use std::{thread, time::Duration};

fn main() {
    let process_name = "Deathloop.exe";
    let pid = find_process_by_name(process_name).expect("Process not found");

    let handle = unsafe {
        OpenProcess(
            PROCESS_VM_READ | PROCESS_VM_WRITE | PROCESS_QUERY_INFORMATION,
            0,
            pid,
        )
    };

    if handle == 0 {
        panic!("Failed to open process");
    }

    let base_address = get_module_base(pid, "Deathloop.exe")
        .expect("Could not find Deathloop.exe base"); // main module
    println!("Deathloop base address: {:X}", base_address);

    let host_name_offset = 0x3335638;
    let host_name_address = base_address + host_name_offset;

    // Read the string (assuming it's a null-terminated C string)
    let mut buffer = [0u8; 64];  // adjust size if needed
    let mut bytes_read = 0usize;

    unsafe {
        ReadProcessMemory(
            handle,
            host_name_address as _,
            buffer.as_mut_ptr() as _,
            buffer.len(),
            &mut bytes_read,
        );
    }

    // Convert to Rust string
    let name = std::str::from_utf8(&buffer[..bytes_read])
        .map(|s| s.trim_end_matches('\0'))
        .expect("Failed to parse host name");

    loop {
        println!("Host name: {}", name);
        thread::sleep(Duration::from_millis(1)); // don't hog CPU
    }

    unsafe { CloseHandle(handle); }
}