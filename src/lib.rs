/// Memory reading utilities for interacting with the Deathloop process.
///
/// This module provides the `GameProcess` struct which wraps Windows API calls
/// to attach to a game process, read its memory, and retrieve strings.

use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE},
    System::{
        Diagnostics::Debug::ReadProcessMemory,
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Module32First, Module32Next, TH32CS_SNAPMODULE, MODULEENTRY32,
        },
        ProcessStatus::{EnumProcesses},
        Threading::{OpenProcess, PROCESS_VM_READ, PROCESS_QUERY_INFORMATION},
    },
};

/// Represents an attached game process for memory reading.
///
/// Wraps a Windows process handle along with the base address of the target module
/// and the process ID. Provides methods for reading arbitrary memory values and strings.
pub struct GameProcess {
    /// Windows handle to the target process.
    pub handle: HANDLE,
    /// Base memory address of the loaded module.
    pub base_address: u64,
    /// Process ID of the target process.
    pub pid: u32,
}

impl GameProcess {
    /// Attaches to a running process by name and finds its module base address.
    ///
    /// # Arguments
    /// * `process_name` - Name of the executable process to attach to (e.g., "Deathloop.exe")
    /// * `module_name` - Name of the module to find the base address of
    ///
    /// # Returns
    /// A `GameProcess` instance on success, or an error string on failure.
    pub fn attach(process_name: &str, module_name: &str) -> Result<Self, String> {
        let pid = find_process_by_name(process_name)
            .ok_or_else(|| format!("Process {} not found", process_name))?;

        let handle = unsafe { OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, 0, pid) };
        if handle.is_null() {
            return Err("Failed to open process. Run as Administrator!".to_string());
        }

        let base_address = get_module_base(pid, module_name)
            .ok_or_else(|| "Could not find module base".to_string())?;

        println!("Attached to {} | Base: {:X}", process_name, base_address);

        Ok(GameProcess { handle, base_address, pid })
    }

    /// Reads a value of type `T` from the specified memory address.
    ///
    /// # Arguments
    /// * `address` - Absolute memory address to read from
    ///
    /// # Returns
    /// The decoded value of type `T` on success, or an error string.
    pub fn read_memory<T: Copy>(&self, address: u64) -> Result<T, String> {
        let mut value = unsafe { std::mem::zeroed() };
        let size = std::mem::size_of::<T>();
        let mut bytes_read = 0usize;

        let success = unsafe {
            ReadProcessMemory(
                self.handle,
                address as _,
                &mut value as *mut _ as _,
                size,
                &mut bytes_read,
            )
        };

        if success == 0 || bytes_read != size {
            return Err(format!("Failed to read memory at {:X}", address));
        }

        Ok(value)
    }

    /// Reads a null-terminated UTF-8 string from the specified memory address.
    ///
    /// # Arguments
    /// * `address` - Absolute memory address to read from
    /// * `max_len` - Maximum number of bytes to read
    ///
    /// # Returns
    /// A `String` on success, or an error string.
    pub fn read_string(&self, address: u64, max_len: usize) -> Result<String, String> {
        let mut buf = vec![0u8; max_len];
        let mut bytes_read = 0usize;

        let success = unsafe {
            ReadProcessMemory(
                self.handle,
                address as _,
                buf.as_mut_ptr() as _,
                max_len,
                &mut bytes_read,
            )
        };

        if success == 0 {
            return Err(format!("Failed to read string at {:X}", address));
        }

        let null_pos = buf.iter().position(|&b| b == 0).unwrap_or(bytes_read);
        Ok(String::from_utf8_lossy(&buf[..null_pos]).to_string())
    }
}

/// Automatically closes the process handle when `GameProcess` is dropped.
impl Drop for GameProcess {
    fn drop(&mut self) {
        unsafe { CloseHandle(self.handle); }
    }
}


/// Finds the process ID of a running process by its executable name.
///
/// Enumerates all processes using `EnumProcesses` and matches the name
/// using `K32GetModuleBaseNameA` for reliable name retrieval.
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
        // Use K32GetModuleBaseNameA from kernel32 (more reliable)
        unsafe { windows_sys::Win32::System::ProcessStatus::K32GetModuleBaseNameA(
            h, std::ptr::null_mut(), module_buf.as_mut_ptr(), module_buf.len() as u32
        ); }
        unsafe { CloseHandle(h); }

        if let Ok(m) = std::str::from_utf8(&module_buf) {
            if m.trim_end_matches('\0').eq_ignore_ascii_case(name) {
                return Some(pid);
            }
        }
    }
    None
}

/// Finds the base address of a loaded module within a process.
///
/// Uses the Module32 API to enumerate modules in the target process
/// and returns the base address of the matching module.
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