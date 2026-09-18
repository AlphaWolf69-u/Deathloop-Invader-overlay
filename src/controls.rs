use std::{fs, path::PathBuf};
use windows_sys::Win32::{
    Foundation::{HWND, POINT},
    UI::{Shell::*, WindowsAndMessaging::*},
};
pub const TRAY_MESSAGE: u32 = WM_APP + 17;

#[derive(Clone)]
pub struct Controls {
    pub name: bool,
    pub day: bool,
    pub network: bool,
    pub health: bool,
    pub distance: bool,
    path: PathBuf,
}
impl Controls {
    pub fn key_toggle(&mut self, key: u32) {
        match key {
            1 => self.name = !self.name,
            2 => self.day = !self.day,
            3 => self.network = !self.network,
            4 if crate::EXTENDED => self.health = !self.health,
            5 if crate::EXTENDED => self.distance = !self.distance,
            _ => return,
        }
        if let Err(error) = self.save() {
            eprintln!("Could not save overlay settings: {error}");
        }
    }
    pub fn load() -> Self {
        let path = std::env::current_exe()
            .unwrap_or_else(|_| PathBuf::from("overlay.exe"))
            .with_extension("ini");
        let mut s = Self {
            name: true,
            day: true,
            network: true,
            health: true,
            distance: true,
            path,
        };
        if let Ok(text) = fs::read_to_string(&s.path) {
            s.parse(&text);
        }
        s
    }
    fn parse(&mut self, text: &str) {
        for line in text.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let flag = match value.trim() {
                    "1" => true,
                    "0" => false,
                    _ => continue,
                };
                match key.trim() {
                    "name" => self.name = flag,
                    "day" => self.day = flag,
                    "network" => self.network = flag,
                    "health" => self.health = flag,
                    "distance" => self.distance = flag,
                    _ => {}
                }
            }
        }
    }
    fn save(&self) -> std::io::Result<()> {
        fs::write(
            &self.path,
            format!(
                "name={}\nday={}\nnetwork={}\nhealth={}\ndistance={}\n",
                self.name as u8,
                self.day as u8,
                self.network as u8,
                self.health as u8,
                self.distance as u8
            ),
        )
    }
    pub fn menu(&mut self, hwnd: HWND) -> bool {
        unsafe {
            let menu = CreatePopupMenu();
            if menu.is_null() {
                return false;
            }
            let mut entries = vec![
                (1, "Opponent name", self.name),
                (2, "Host day", self.day),
                (3, "Network statistics", self.network),
            ];
            if crate::EXTENDED {
                entries.extend([
                    (4, "Opponent health", self.health),
                    (5, "Distance", self.distance),
                ]);
            }
            for (id, label, checked) in entries {
                AppendMenuW(
                    menu,
                    MF_STRING | if checked { MF_CHECKED } else { MF_UNCHECKED },
                    id,
                    wide(label).as_ptr(),
                );
            }
            AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
            AppendMenuW(menu, MF_STRING, 6, wide("Hide all").as_ptr());
            AppendMenuW(menu, MF_STRING, 7, wide("Show all").as_ptr());
            AppendMenuW(menu, MF_STRING, 8, wide("Exit overlay").as_ptr());
            let mut point = POINT::default();
            GetCursorPos(&mut point);
            SetForegroundWindow(hwnd);
            let command = TrackPopupMenu(
                menu,
                TPM_RETURNCMD | TPM_RIGHTBUTTON,
                point.x,
                point.y,
                0,
                hwnd,
                std::ptr::null(),
            );
            DestroyMenu(menu);
            PostMessageW(hwnd, WM_NULL, 0, 0);
            match command {
                1 => self.name = !self.name,
                2 => self.day = !self.day,
                3 => self.network = !self.network,
                4 => self.health = !self.health,
                5 => self.distance = !self.distance,
                6 | 7 => {
                    let v = command == 7;
                    self.name = v;
                    self.day = v;
                    self.network = v;
                    self.health = v;
                    self.distance = v;
                }
                8 => {
                    DestroyWindow(hwnd);
                    return false;
                }
                _ => return false,
            }
            if let Err(e) = self.save() {
                MessageBoxW(
                    hwnd,
                    wide(&format!(
                        "Display changed, but settings could not be saved: {e}"
                    ))
                    .as_ptr(),
                    wide("Overlay settings").as_ptr(),
                    MB_OK | MB_ICONWARNING,
                );
            }
            true
        }
    }
}
fn wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(Some(0)).collect()
}
pub fn tray(hwnd: HWND, add: bool) -> bool {
    unsafe {
        let mut data: NOTIFYICONDATAW = std::mem::zeroed();
        data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
        data.hWnd = hwnd;
        data.uID = 1;
        data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
        data.uCallbackMessage = TRAY_MESSAGE;
        data.hIcon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
        let label = if crate::EXTENDED {
            "Deathloop detailed overlay - right-click controls"
        } else {
            "Deathloop name/network overlay - right-click controls"
        };
        for (dst, src) in data.szTip.iter_mut().zip(wide(label)) {
            *dst = src;
        }
        Shell_NotifyIconW(if add { NIM_ADD } else { NIM_DELETE }, &data) != 0
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn settings_parse() {
        let mut c = super::Controls {
            name: true,
            day: true,
            network: true,
            health: true,
            distance: true,
            path: std::path::PathBuf::new(),
        };
        c.parse("name=0\nday=1\nnetwork=0\nhealth=oops\ndistance=0\n");
        assert!(!c.name && c.day && !c.network && c.health && !c.distance);
    }
}
