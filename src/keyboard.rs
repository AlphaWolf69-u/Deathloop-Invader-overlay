use std::time::{Duration, Instant};
use windows_sys::Win32::UI::Input::KeyboardAndMouse::GetAsyncKeyState;
pub struct Keyboard {
    previous: u16,
    pub hidden: bool,
    pub help: bool,
    hint_until: Instant,
    pub next_refresh: Instant,
}
impl Keyboard {
    pub fn new() -> Self {
        Self {
            previous: 0,
            hidden: false,
            help: false,
            hint_until: Instant::now() + Duration::from_secs(10),
            next_refresh: Instant::now(),
        }
    }
    pub fn poll(&mut self) -> Vec<u32> {
        let mut down = 0u16;
        unsafe {
            if GetAsyncKeyState(0x11) < 0 && GetAsyncKeyState(0x12) < 0 {
                for i in 0..12 {
                    if GetAsyncKeyState(0x70 + i) < 0 {
                        down |= 1 << i;
                    }
                }
            }
        }
        self.edges(down)
    }
    fn edges(&mut self, down: u16) -> Vec<u32> {
        let pressed = down & !self.previous;
        self.previous = down;
        (0..12)
            .filter(|i| pressed & (1 << i) != 0)
            .map(|i| i + 1)
            .collect()
    }
    pub fn hint(&self) -> Option<&'static str> {
        if self.help {
            Some(if crate::EXTENDED {
                "Ctrl+Alt: F1 Name | F2 Day | F3 Network\nF4 Health | F5 Distance | F6 Hide/show\nF7 Help | F12 Close overlay"
            } else {
                "Ctrl+Alt: F1 Name | F2 Day | F3 Network\nF6 Hide/show | F7 Help | F12 Close overlay"
            })
        } else if Instant::now() < self.hint_until {
            Some("Ctrl+Alt+F7: shortcuts | Ctrl+Alt+F12: close")
        } else {
            None
        }
    }
}
#[cfg(test)]
mod tests {
    #[test]
    fn held_key_toggles_once() {
        let mut k = super::Keyboard::new();
        assert_eq!(k.edges(1), vec![1]);
        assert!(k.edges(1).is_empty());
        assert!(k.edges(0).is_empty());
        assert_eq!(k.edges(1 << 11), vec![12]);
    }
}
