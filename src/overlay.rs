/// Overlay rendering module for the Deathloop Invader Tool.
///
/// Creates a transparent, topmost overlay window that reads and displays
/// information from the Deathloop game process memory.
use deathloop_invader_tool::GameProcess;
use std::error::Error;
use std::ffi::OsStr;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use std::time::{Duration, Instant};
use windows_sys::Win32::Foundation::SIZE;
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    AddFontMemResourceEx, BI_RGB, BITMAPINFO, BITMAPINFOHEADER, BLENDFUNCTION, CreateCompatibleDC,
    CreateDIBSection, CreateFontIndirectW, DIB_RGB_COLORS, DT_CENTER, DeleteDC, DeleteObject,
    DrawTextW, GetDC, LOGFONTW, ReleaseDC, RemoveFontMemResourceEx, SelectObject, SetBkMode,
    SetTextColor, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CREATESTRUCTW, CreateWindowExW, DefWindowProcW, DestroyWindow, DispatchMessageW, GWLP_USERDATA,
    GetMessageW, IDC_ARROW, KillTimer, LoadCursorW, MSG, PostQuitMessage, RegisterClassW, SetTimer,
    TranslateMessage, ULW_ALPHA, UpdateLayeredWindow, WM_CREATE, WM_DESTROY, WM_TIMER, WNDCLASSW,
    WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOOLWINDOW, WS_EX_TOPMOST, WS_EX_TRANSPARENT, WS_POPUP,
    WS_VISIBLE,
};

/// Windows class name for the overlay window.
const WINDOW_CLASS_NAME: &str = "DeathloopOverlayWindow";
/// Window title displayed in the overlay.
const WINDOW_TITLE: &str = "Alpha Wolf's Invader Tool Overlay";
/// Width of the overlay window in pixels.
const OVERLAY_WIDTH: i32 = 950;
/// Height of the overlay window in pixels.
const OVERLAY_HEIGHT: i32 = 240;
/// Timer ID used for periodic overlay updates.
const TIMER_ID: usize = 1;
/// Interval between overlay refreshes in milliseconds (100ms = 10 FPS).
const TIMER_INTERVAL_MS: u32 = 50;
/// Embedded font data loaded at compile time from the assets directory.
const FONT_DATA: &[u8] = include_bytes!("../assets/handelson-two.otf");

/// Registers the embedded font with Windows GDI.
///
/// Adds the font data to the system font table so it can be used
/// with `CreateFontIndirectW` for rendering text on the overlay.
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn register_font() -> Result<*mut core::ffi::c_void, Box<dyn std::error::Error>> {
    let mut num_fonts = 0;
    let resource = unsafe {
        AddFontMemResourceEx(
            FONT_DATA.as_ptr() as _,
            FONT_DATA.len() as u32,
            null_mut(),
            &raw mut num_fonts,
        )
    };

    if resource.is_null() {
        Err("Failed to register embedded font".into())
    } else {
        Ok(resource)
    }
}

/// Main application struct that holds the game process handle and font resource.
///
/// Manages the lifecycle of the overlay including process attachment,
/// window creation, and cleanup.
pub struct OverlayApp {
    /// Handle to the attached Deathloop game process.
    game_process: Option<GameProcess>,
    next_attach: Instant,
    last_text: Option<String>,
    controls: crate::controls::Controls,
    keyboard: crate::keyboard::Keyboard,
    network: crate::network::Monitor,
    /// Opaque handle to the registered font resource (must be cleaned up on drop).
    font_mem_resource: *mut core::ffi::c_void,
}

impl OverlayApp {
    /// Creates a new `OverlayApp` by attaching to the Deathloop process
    /// and registering the embedded font.
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let game_process = GameProcess::attach("Deathloop.exe", "Deathloop.exe").ok();
        let font_mem_resource = unsafe { register_font()? };
        Ok(Self {
            game_process,
            next_attach: Instant::now(),
            last_text: None,
            controls: crate::controls::Controls::load(),
            keyboard: crate::keyboard::Keyboard::new(),
            network: crate::network::Monitor::default(),
            font_mem_resource,
        })
    }

    /// Runs the overlay application: registers the window class,
    /// creates a layered transparent window, and enters the message loop.
    pub fn run(self) -> Result<(), Box<dyn Error>> {
        unsafe {
            let hinstance = GetModuleHandleW(null());
            let class_name = to_wstr(WINDOW_CLASS_NAME);
            let title = to_wstr(WINDOW_TITLE);

            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: hinstance,
                lpszClassName: class_name.as_ptr(),
                hCursor: LoadCursorW(null_mut(), IDC_ARROW),
                hIcon: windows_sys::Win32::UI::WindowsAndMessaging::LoadIconW(
                    hinstance,
                    101usize as *const u16,
                ),
                style: 0,
                ..zeroed()
            };

            if RegisterClassW(&wnd_class) == 0 {
                return Err(std::io::Error::last_os_error().into());
            }

            let mut app_box = Box::new(self);
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED
                    | WS_EX_TRANSPARENT
                    | WS_EX_TOPMOST
                    | WS_EX_TOOLWINDOW
                    | WS_EX_NOACTIVATE,
                class_name.as_ptr(),
                title.as_ptr(),
                WS_POPUP | WS_VISIBLE,
                100,
                100,
                OVERLAY_WIDTH,
                OVERLAY_HEIGHT,
                null_mut(),
                null_mut(),
                hinstance,
                (&mut *app_box as *mut OverlayApp).cast(),
            );

            if hwnd.is_null() {
                return Err(std::io::Error::last_os_error().into());
            }

            if SetTimer(hwnd, TIMER_ID, TIMER_INTERVAL_MS, None) == 0 {
                let error = std::io::Error::last_os_error();
                DestroyWindow(hwnd);
                return Err(error.into());
            }
            crate::controls::tray(hwnd, true); // Keyboard controls also work without a tray icon.

            let mut msg = MSG::default();
            loop {
                let result = GetMessageW(&mut msg, null_mut(), 0, 0);
                if result == 0 {
                    break;
                }
                if result == -1 {
                    let error = std::io::Error::last_os_error();
                    DestroyWindow(hwnd);
                    return Err(error.into());
                }
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
            Ok(())
        }
    }
}

/// Windows procedure (callback) for the overlay window.
///
/// Handles window creation, timer events for periodic rendering,
/// and window destruction for cleanup.
#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        crate::controls::TRAY_MESSAGE => {
            if lparam as u32 == windows_sys::Win32::UI::WindowsAndMessaging::WM_RBUTTONUP
                && let Some(app) = get_app_ptr(hwnd)
            {
                // TrackPopupMenu pumps messages, including timers. Keep a copy
                // rather than borrowing application state across that loop.
                let mut controls = (*app).controls.clone();
                if controls.menu(hwnd) {
                    (*app).controls = controls;
                    (*app).network.reset();
                    render_overlay(hwnd, &mut *app);
                }
            }
            0
        }
        WM_CREATE => {
            let create_struct = unsafe { &*(lparam as *const CREATESTRUCTW) };
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                    hwnd,
                    GWLP_USERDATA,
                    create_struct.lpCreateParams as isize,
                );
            }
            if let Some(app) = get_app_ptr(hwnd) {
                render_overlay(hwnd, &mut *app);
            }
            0
        }
        WM_TIMER => {
            if wparam == TIMER_ID
                && let Some(app) = get_app_ptr(hwnd)
            {
                let keys = (*app).keyboard.poll();
                let changed = !keys.is_empty();
                for key in keys {
                    match key {
                        12 => {
                            DestroyWindow(hwnd);
                            return 0;
                        }
                        6 => (*app).keyboard.hidden = !(*app).keyboard.hidden,
                        7 => (*app).keyboard.help = !(*app).keyboard.help,
                        _ => (*app).controls.key_toggle(key),
                    }
                }
                if changed || Instant::now() >= (*app).keyboard.next_refresh {
                    (*app).keyboard.next_refresh = Instant::now() + Duration::from_millis(250);
                    render_overlay(hwnd, &mut *app);
                }
            }
            0
        }
        WM_DESTROY => {
            crate::controls::tray(hwnd, false);
            KillTimer(hwnd, TIMER_ID);
            windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

/// Retrieves the `OverlayApp` pointer stored in the window's user data.
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn get_app_ptr(hwnd: HWND) -> Option<*mut OverlayApp> {
    let ptr = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWLP_USERDATA)
        as *mut OverlayApp;
    if ptr.is_null() { None } else { Some(ptr) }
}

impl Drop for OverlayApp {
    fn drop(&mut self) {
        if !self.font_mem_resource.is_null() {
            unsafe {
                RemoveFontMemResourceEx(self.font_mem_resource);
            }
        }
    }
}

/// Renders the overlay: reads game memory, draws text onto a bitmap,
/// and updates the layered window with the new frame.
#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn render_overlay(hwnd: HWND, app: &mut OverlayApp) {
    if app.game_process.as_ref().is_some_and(|g| !g.is_running()) {
        app.game_process = None;
        app.next_attach = Instant::now();
    }
    if app.game_process.is_none() && Instant::now() >= app.next_attach {
        app.game_process = GameProcess::attach("Deathloop.exe", "Deathloop.exe").ok();
        app.next_attach = Instant::now() + Duration::from_secs(2);
    }
    let c = &app.controls;
    let visible = c.name || c.day || c.network || (crate::EXTENDED && (c.health || c.distance));
    let mut text = if !visible || app.keyboard.hidden {
        String::new()
    } else {
        match &app.game_process {
            Some(game) => {
                let mut lines = Vec::new();
                match game.opponent() {
                    Ok(s) => {
                        let mut identity = Vec::new();
                        if c.name {
                            identity.push(format!(
                                "{}: {}",
                                s.role.opponent_label(),
                                s.name.as_deref().unwrap_or("waiting for opponent")
                            ));
                        }
                        if c.day
                            && s.name.is_some()
                            && let Some(day) = s.host_day
                        {
                            identity.push(format!("Day {day}"));
                        }
                        if !identity.is_empty() {
                            lines.push(identity.join(" | "));
                        }
                    }
                    Err(_) => {
                        if c.name || c.day {
                            lines.push("Waiting for game session".into());
                        }
                    }
                }
                if c.network {
                    lines.push(app.network.display(game));
                }
                if crate::EXTENDED
                    && (c.health || c.distance)
                    && let Some(extra) = crate::extra(game, c.health, c.distance)
                {
                    lines.push(extra);
                }
                lines.join("\n")
            }
            None => {
                app.network.reset();
                "Waiting for Deathloop (check process access)".into()
            }
        }
    };
    if !app.keyboard.hidden
        && let Some(hint) = app.keyboard.hint()
    {
        if !text.is_empty() {
            text.push('\n');
        }
        text.push_str(hint);
    }
    if app.last_text.as_deref() == Some(text.as_str()) {
        return;
    }

    let hdc_screen = GetDC(null_mut());
    if hdc_screen.is_null() {
        return;
    }

    let bmi = BITMAPINFO {
        bmiHeader: BITMAPINFOHEADER {
            biSize: size_of::<BITMAPINFOHEADER>() as u32,
            biWidth: OVERLAY_WIDTH,
            biHeight: -OVERLAY_HEIGHT,
            biPlanes: 1,
            biBitCount: 32,
            biCompression: BI_RGB,
            ..zeroed()
        },
        ..zeroed()
    };

    let mut bits: *mut core::ffi::c_void = null_mut();
    let hbitmap = CreateDIBSection(hdc_screen, &bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
    if hbitmap.is_null() || bits.is_null() {
        if !hbitmap.is_null() {
            DeleteObject(hbitmap);
        }
        ReleaseDC(null_mut(), hdc_screen);
        return;
    }

    let hdc_mem = CreateCompatibleDC(hdc_screen);
    if hdc_mem.is_null() {
        DeleteObject(hbitmap);
        ReleaseDC(null_mut(), hdc_screen);
        return;
    }

    let old_bitmap = SelectObject(hdc_mem, hbitmap as _);
    let buffer_size = (OVERLAY_WIDTH * OVERLAY_HEIGHT * 4) as usize;
    std::ptr::write_bytes(bits, 0, buffer_size);

    let text_wide = to_wstr(&text);
    let mut lf: LOGFONTW = zeroed();
    let font_name = to_wstr("Handelson Two");
    lf.lfHeight = -24;
    lf.lfWeight = 400;
    lf.lfCharSet = 1;
    lf.lfQuality = 4; // Grayscale antialiasing for transparent text.
    for (dst, src) in lf.lfFaceName.iter_mut().zip(font_name.iter()) {
        *dst = *src;
    }
    let hfont = CreateFontIndirectW(&lf);
    if hfont.is_null() {
        SelectObject(hdc_mem, old_bitmap);
        DeleteDC(hdc_mem);
        DeleteObject(hbitmap);
        ReleaseDC(null_mut(), hdc_screen);
        return;
    }
    let old_font = SelectObject(hdc_mem, hfont as _);
    SetBkMode(hdc_mem, TRANSPARENT as i32);
    SetTextColor(hdc_mem, 0x00FFFFFF);
    DrawTextW(
        hdc_mem,
        text_wide.as_ptr(),
        -1,
        &mut RECT {
            left: 0,
            top: 0,
            right: OVERLAY_WIDTH,
            bottom: OVERLAY_HEIGHT,
        },
        DT_CENTER | windows_sys::Win32::Graphics::Gdi::DT_NOPREFIX,
    );

    let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, buffer_size / 4);
    for pixel in pixels.iter_mut() {
        // White-on-black RGB is already premultiplied coverage.
        let coverage = (*pixel & 255)
            .max((*pixel >> 8) & 255)
            .max((*pixel >> 16) & 255);
        *pixel = (*pixel & 0x00FF_FFFF) | (coverage << 24);
    }

    let blend = BLENDFUNCTION {
        BlendOp: 0,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: 1,
    };

    let updated = UpdateLayeredWindow(
        hwnd,
        hdc_screen,
        null_mut(),
        &SIZE {
            cx: OVERLAY_WIDTH,
            cy: OVERLAY_HEIGHT,
        },
        hdc_mem,
        &POINT { x: 0, y: 0 },
        0,
        &blend,
        ULW_ALPHA,
    );

    SelectObject(hdc_mem, old_font);
    DeleteObject(hfont as _);
    SelectObject(hdc_mem, old_bitmap);
    DeleteDC(hdc_mem);
    DeleteObject(hbitmap);
    ReleaseDC(null_mut(), hdc_screen);
    if updated != 0 {
        app.last_text = Some(text);
    }
}

/// Converts a Rust `&str` to a null-terminated UTF-16 wide string (`Vec<u16>`).
///
/// Used for passing string data to Windows API functions that expect wide strings.
fn to_wstr(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}
