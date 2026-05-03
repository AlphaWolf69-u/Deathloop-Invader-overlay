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

pub struct GameProcess {
    pub handle: HANDLE,
    pub base_address: u64,
    pub pid: u32,
}

impl GameProcess {
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

    pub fn read_memory<T: Copy>(&self, address: u64) -> T {
        let mut value = unsafe { std::mem::zeroed() };
        let size = std::mem::size_of::<T>();

        unsafe {
            ReadProcessMemory(
                self.handle,
                address as _,
                &mut value as *mut _ as _,
                size,
                std::ptr::null_mut(),
            );
        }
        value
    }

    pub fn read_string(&self, address: u64, max_len: usize) -> String {
        let mut buffer = vec![0u8; max_len];
        let mut bytes_read = 0usize;

        unsafe {
            ReadProcessMemory(
                self.handle,
                address as _,
                buffer.as_mut_ptr() as _,
                max_len,
                &mut bytes_read,
            );
        }

        let real_len = buffer.iter().position(|&b| b == 0).unwrap_or(bytes_read);
        String::from_utf8_lossy(&buffer[0..real_len]).to_string()
    }

    pub fn close(self) {
        unsafe { CloseHandle(self.handle); }
    }
}


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