//! Read-only access for the researched Deathloop executable build.
use std::mem::{MaybeUninit, size_of};
use windows_sys::Win32::{
    Foundation::{CloseHandle, HANDLE, INVALID_HANDLE_VALUE},
    System::{
        Diagnostics::Debug::ReadProcessMemory,
        Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, MODULEENTRY32W, Module32FirstW, Module32NextW,
            PROCESSENTRY32W, Process32FirstW, Process32NextW, TH32CS_SNAPMODULE,
            TH32CS_SNAPPROCESS,
        },
        Threading::{GetExitCodeProcess, OpenProcess, PROCESS_QUERY_INFORMATION, PROCESS_VM_READ},
    },
};
struct OwnedHandle(HANDLE);
impl Drop for OwnedHandle {
    fn drop(&mut self) {
        unsafe {
            CloseHandle(self.0);
        }
    }
}
mod sealed {
    pub trait Sealed {}
}
/// Only primitive types with no invalid bit patterns can be read safely.
pub trait MemoryValue: sealed::Sealed + Copy {}
macro_rules! values {($($t:ty),*)=>{$(impl sealed::Sealed for $t{} impl MemoryValue for $t{})*};}
values!(u8, u16, u32, u64, i8, i16, i32, i64, f32, f64);
pub struct GameProcess {
    handle: OwnedHandle,
    pub base_address: u64,
    pub pid: u32,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    Host,
    Invader,
}
impl Role {
    pub fn opponent_label(self) -> &'static str {
        match self {
            Self::Host => "Invader",
            Self::Invader => "Host",
        }
    }
    pub fn opponent_name_rva(self) -> u64 {
        match self {
            Self::Host => 0x3334F68,
            Self::Invader => 0x3335638,
        }
    }
    fn from_flag(v: u8) -> Result<Self, String> {
        match v {
            0 => Ok(Self::Invader),
            1 => Ok(Self::Host),
            _ => Err("Invalid host/client flag".into()),
        }
    }
}
#[derive(Debug)]
pub struct OpponentSnapshot {
    pub role: Role,
    pub name: Option<String>,
    pub player_count: usize,
    pub host_day: Option<i32>,
}
impl OpponentSnapshot {
    pub fn display(&self) -> String {
        match &self.name {
            Some(n) => match (self.role, self.host_day) {
                (Role::Invader, Some(day)) => format!("Host: {n} | Day {day}"),
                _ => format!("{}: {}", self.role.opponent_label(), n),
            },
            None => format!("{}: waiting for opponent", self.role.opponent_label()),
        }
    }
}
impl GameProcess {
    pub fn attach(process_name: &str, module_name: &str) -> Result<Self, String> {
        let pid =
            find_process(process_name).ok_or_else(|| format!("{process_name} is not running"))?;
        let raw = unsafe { OpenProcess(PROCESS_VM_READ | PROCESS_QUERY_INFORMATION, 0, pid) };
        if raw.is_null() {
            return Err(format!("OpenProcess: {}", std::io::Error::last_os_error()));
        }
        let handle = OwnedHandle(raw); // also closes on module lookup failure
        let base_address = module_base(pid, module_name).ok_or("Module unavailable")?;
        Ok(Self {
            handle,
            base_address,
            pid,
        })
    }
    pub fn is_running(&self) -> bool {
        let mut code = 0;
        unsafe { GetExitCodeProcess(self.handle.0, &mut code) != 0 && code == 259 }
    }
    pub fn read_memory<T: MemoryValue>(&self, address: u64) -> Result<T, String> {
        let mut value = MaybeUninit::<T>::uninit();
        let mut got = 0;
        let ok = unsafe {
            ReadProcessMemory(
                self.handle.0,
                address as _,
                value.as_mut_ptr() as _,
                size_of::<T>(),
                &mut got,
            )
        };
        if ok == 0 || got != size_of::<T>() {
            return Err(format!("Unreadable memory at {address:X}"));
        }
        Ok(unsafe { value.assume_init() })
    }
    pub fn read_string(&self, address: u64, max_len: usize) -> Result<String, String> {
        let mut bytes = Vec::new();
        for i in 0..max_len {
            let b = self.read_memory::<u8>(address + i as u64)?;
            if b == 0 {
                return Ok(String::from_utf8_lossy(&bytes).into_owned());
            }
            bytes.push(b);
        }
        Err("Unterminated name buffer".into())
    }
    pub fn opponent(&self) -> Result<OpponentSnapshot, String> {
        let game = self.read_memory::<u64>(self.base_address + 0x5BD1010)?;
        if game < 0x10000 || self.read_memory::<u64>(game)? != self.base_address + 0x2613250 {
            return Err("Waiting for a supported game session".into());
        }
        let flag = self.read_memory::<u8>(game + 0x6A730)?;
        let role = Role::from_flag(flag)?;
        let list = self.read_memory::<u64>(game + 0x210)?;
        let count = self.read_memory::<u32>(game + 0x21C)?;
        if count > 4 || (count > 0 && list < 0x10000) {
            return Err("Player list unavailable".into());
        }
        let mut ids = Vec::new();
        for i in 0..count {
            let id = self.read_memory::<u64>(list + u64::from(i) * 8)?;
            if id != u64::MAX && id != 0 && !ids.contains(&id) {
                ids.push(id);
            }
        }
        // Buffers can retain names from the last match: require a second loaded player.
        let name = if ids.len() >= 2 {
            let text = self.read_string(self.base_address + role.opponent_name_rva(), 32)?;
            let text = text.trim();
            if text.is_empty() || text.chars().any(char::is_control) {
                None
            } else {
                Some(text.to_owned())
            }
        } else {
            None
        };
        // On a client this manager holds the replicated host campaign. Never
        // attach the local host's day to the invading opponent's name.
        let host_day = if role == Role::Invader && name.is_some() {
            self.host_campaign_day(game).ok()
        } else {
            None
        };
        if self.read_memory::<u64>(self.base_address + 0x5BD1010)? != game
            || self.read_memory::<u8>(game + 0x6A730)? != flag
            || self.read_memory::<u64>(game + 0x210)? != list
            || self.read_memory::<u32>(game + 0x21C)? != count
        {
            return Err("Session changing".into());
        }
        Ok(OpponentSnapshot {
            role,
            name,
            player_count: ids.len(),
            host_day,
        })
    }
    fn host_campaign_day(&self, game: u64) -> Result<i32, String> {
        let manager = self.read_memory::<u64>(game + 0x34D8)?;
        if manager < 0x10000 || self.read_memory::<u64>(manager)? != self.base_address + 0x2693B50 {
            return Err("Campaign unavailable".into());
        }
        let day = self.read_memory::<i32>(manager + 0x80)?;
        if day < 0 || self.read_memory::<u64>(game + 0x34D8)? != manager {
            return Err("Campaign changing".into());
        }
        // Display the transmitted counter unchanged; no inferred +1 adjustment.
        Ok(day)
    }
}
fn wide(buf: &[u16]) -> String {
    String::from_utf16_lossy(&buf[..buf.iter().position(|v| *v == 0).unwrap_or(buf.len())])
}
fn snapshot(flags: u32, pid: u32) -> Option<OwnedHandle> {
    let h = unsafe { CreateToolhelp32Snapshot(flags, pid) };
    if h == INVALID_HANDLE_VALUE || h.is_null() {
        None
    } else {
        Some(OwnedHandle(h))
    }
}
fn find_process(name: &str) -> Option<u32> {
    let h = snapshot(TH32CS_SNAPPROCESS, 0)?;
    let mut e: PROCESSENTRY32W = unsafe { std::mem::zeroed() };
    e.dwSize = size_of::<PROCESSENTRY32W>() as u32;
    let mut ok = unsafe { Process32FirstW(h.0, &mut e) };
    while ok != 0 {
        if wide(&e.szExeFile).eq_ignore_ascii_case(name) {
            return Some(e.th32ProcessID);
        }
        ok = unsafe { Process32NextW(h.0, &mut e) };
    }
    None
}
fn module_base(pid: u32, name: &str) -> Option<u64> {
    let h = snapshot(TH32CS_SNAPMODULE, pid)?;
    let mut e: MODULEENTRY32W = unsafe { std::mem::zeroed() };
    e.dwSize = size_of::<MODULEENTRY32W>() as u32;
    let mut ok = unsafe { Module32FirstW(h.0, &mut e) };
    while ok != 0 {
        if wide(&e.szModule).eq_ignore_ascii_case(name) {
            return Some(e.modBaseAddr as u64);
        }
        ok = unsafe { Module32NextW(h.0, &mut e) };
    }
    None
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn other_player() {
        assert_eq!(Role::from_flag(1).unwrap().opponent_name_rva(), 0x3334F68);
        assert_eq!(Role::from_flag(0).unwrap().opponent_name_rva(), 0x3335638);
        assert!(Role::from_flag(2).is_err());
    }
    #[test]
    fn labels() {
        assert_eq!(
            OpponentSnapshot {
                role: Role::Host,
                name: Some("Juli".into()),
                player_count: 2,
                host_day: Some(219),
            }
            .display(),
            "Invader: Juli"
        );
        assert_eq!(
            OpponentSnapshot {
                role: Role::Invader,
                name: Some("Colt".into()),
                player_count: 2,
                host_day: Some(219),
            }
            .display(),
            "Host: Colt | Day 219"
        );
        assert_eq!(
            OpponentSnapshot {
                role: Role::Host,
                name: None,
                player_count: 1,
                host_day: None,
            }
            .display(),
            "Invader: waiting for opponent"
        );
    }
    #[test]
    fn missing_day_keeps_name_and_missing_opponent_hides_day() {
        let mut snapshot = OpponentSnapshot {
            role: Role::Invader,
            name: Some("Colt".into()),
            player_count: 2,
            host_day: None,
        };
        assert_eq!(snapshot.display(), "Host: Colt");
        snapshot.host_day = Some(0);
        assert_eq!(snapshot.display(), "Host: Colt | Day 0");
        snapshot.name = None;
        assert_eq!(snapshot.display(), "Host: waiting for opponent");
    }
}
