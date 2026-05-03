use windows_sys::Win32::{
    Foundation::CloseHandle,
    System::{
        Diagnostics::Debug::ReadProcessMemory,
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Module32First, Module32Next, TH32CS_SNAPMODULE, MODULEENTRY32,
        },
        ProcessStatus::EnumProcesses,
        Threading::{OpenProcess, PROCESS_VM_READ, PROCESS_QUERY_INFORMATION},
        WindowsProgramming::GetModuleBaseNameA,
    },
};

use std::{thread, time::Duration};

fn main() {
    let process_name = "Deathloop.exe";
    let pid = match find_process_by_name(process_name) {
        Some(p) => p,
        None => panic!("Deathloop.exe not found. Start the game first!"),
    };

    let handle = unsafe { OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, 0, pid) };

    if handle.is_null() {
        panic!("Failed to open process. Run as Administrator!");
    }

    let base_address = match get_module_base(pid, "Deathloop.exe") {
        Some(b) => b,
        None => {
            unsafe { CloseHandle(handle) };
            panic!("Could not find base address");
        }
    };

    println!("Deathloop base address: {:X}", base_address);

    let host_name_offset = 0x3335638u64;
    let host_name_address = base_address + host_name_offset;

    let mut buffer = [0u8; 128];
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

    let real_len = buffer.iter().position(|&b| b == 0).unwrap_or(bytes_read);
    let name = std::str::from_utf8(&buffer[0..real_len]).unwrap_or("<invalid>");

    println!("Host name: '{}'", name);
    println!("Length: {}", name.len());

    println!("Reading every 5 seconds...");
    loop {
        thread::sleep(Duration::from_secs(5));
    }
}

// ====================== HELPERS ======================

fn find_process_by_name(name: &str) -> Option<u32> {
    let mut pids = [0u32; 1024];
    let mut bytes_returned = 0u32;

    unsafe {
        EnumProcesses(pids.as_mut_ptr(), (pids.len() * 4) as u32, &mut bytes_returned);
    }

    let count = bytes_returned as usize / 4;

    for &pid in &pids[0..count] {
        if pid == 0 { continue; }

        let h = unsafe { OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, 0, pid) };
        if h.is_null() { continue; }

        let mut module_buf = [0u8; 260];
        unsafe { GetModuleBaseNameA(h, 0, module_buf.as_mut_ptr(), module_buf.len() as u32); }
        unsafe { CloseHandle(h); }

        if let Ok(m) = std::str::from_utf8(&module_buf) {
            if m.trim_end_matches('\0').eq_ignore_ascii_case(name) {
                return Some(pid);
            }
        }
    }
    None
}

fn get_module_base(pid: u32, module_name: &str) -> Option<u64> {
    let snapshot = unsafe { CreateToolhelp32Snapshot(TH32CS_SNAPMODULE, pid) };
    if snapshot.is_null() {
        return None;
    }

    let mut me = MODULEENTRY32 {
        dwSize: std::mem::size_of::<MODULEENTRY32>() as u32,
        ..unsafe { std::mem::zeroed() }
    };

    if unsafe { Module32First(snapshot, &mut me) } != 0 {
        loop {
            // Fixed: convert [i8; 256] to &[u8]
            let mod_name = unsafe {
                let slice = std::slice::from_raw_parts(me.szModule.as_ptr() as *const u8, me.szModule.len());
                std::str::from_utf8(slice)
                    .unwrap_or("")
                    .trim_end_matches('\0')
            };

            if mod_name.eq_ignore_ascii_case(module_name) {
                unsafe { CloseHandle(snapshot) };
                return Some(me.modBaseAddr as u64);
            }

            if unsafe { Module32Next(snapshot, &mut me) } == 0 {
                break;
            }
        }
    }

    unsafe { CloseHandle(snapshot) };
    None
}