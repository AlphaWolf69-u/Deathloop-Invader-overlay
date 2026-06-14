use deathloop_invader_tool::GameProcess;
use std::error::Error;
use std::ffi::OsStr;
use std::mem::{size_of, zeroed};
use std::os::windows::ffi::OsStrExt;
use std::ptr::{null, null_mut};
use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, RECT, WPARAM};
use windows_sys::Win32::Graphics::Gdi::{
    AddFontMemResourceEx, BLENDFUNCTION, CreateCompatibleDC, CreateDIBSection, CreateFontIndirectW,
    DeleteDC, DeleteObject, DrawTextW, GetDC, LOGFONTW, ReleaseDC, RemoveFontMemResourceEx,
    SelectObject, SetBkMode, SetTextColor, BITMAPINFO, BITMAPINFOHEADER, BI_RGB, DIB_RGB_COLORS,
    DT_CENTER, DT_SINGLELINE, DT_VCENTER, TRANSPARENT,
};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::WindowsAndMessaging::{
    CreateWindowExW, DefWindowProcW, DispatchMessageW, GetMessageW, LoadCursorW,
    PostQuitMessage, RegisterClassW, SetTimer, TranslateMessage,
    UpdateLayeredWindow, GWLP_USERDATA, IDC_ARROW, MSG, ULW_ALPHA, WM_CREATE, WM_DESTROY,
    WM_TIMER, WNDCLASSW, WS_EX_LAYERED, WS_EX_NOACTIVATE, WS_EX_TOPMOST, WS_EX_TRANSPARENT,
    WS_EX_TOOLWINDOW, WS_POPUP, WS_VISIBLE, CREATESTRUCTW,
};
use windows_sys::Win32::Foundation::SIZE;

const WINDOW_CLASS_NAME: &str = "DeathloopOverlayWindow";
const WINDOW_TITLE: &str = "Text Overlay";
const OVERLAY_WIDTH: i32 = 300;
const OVERLAY_HEIGHT: i32 = 40;
const TIMER_ID: usize = 1;
const TIMER_INTERVAL_MS: u32 = 100;
const FONT_DATA: &[u8] = include_bytes!("../assets/handelson-two.otf");

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn register_font() -> Result<*mut core::ffi::c_void, Box<dyn std::error::Error>> {
    let mut num_fonts = 0;
    let resource = unsafe {
        AddFontMemResourceEx(
            FONT_DATA.as_ptr() as _,
            FONT_DATA.len() as u32,
            null_mut(),
            &mut num_fonts,
        )
    };

    if resource == null_mut() {
        Err("Failed to register embedded font".into())
    } else {
        Ok(resource)
    }
}

pub struct OverlayApp {
    game_process: GameProcess,
    font_mem_resource: *mut core::ffi::c_void,
}

impl OverlayApp {
    pub fn new() -> Result<Self, Box<dyn Error>> {
        let game_process = GameProcess::attach("Deathloop.exe", "Deathloop.exe")?;
        let font_mem_resource = unsafe { register_font()? };
        Ok(Self {
            game_process,
            font_mem_resource,
        })
    }

    pub fn run(self) {
        unsafe {
            let hinstance = GetModuleHandleW(null());
            let class_name = to_wstr(WINDOW_CLASS_NAME);
            let title = to_wstr(WINDOW_TITLE);

            let wnd_class = WNDCLASSW {
                lpfnWndProc: Some(window_proc),
                hInstance: hinstance,
                lpszClassName: class_name.as_ptr(),
                hCursor: LoadCursorW(null_mut(), IDC_ARROW),
                style: 0,
                ..zeroed()
            };

            RegisterClassW(&wnd_class);

            let app_box = Box::into_raw(Box::new(self));
            let hwnd = CreateWindowExW(
                WS_EX_LAYERED | WS_EX_TRANSPARENT | WS_EX_TOPMOST | WS_EX_TOOLWINDOW | WS_EX_NOACTIVATE,
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
                app_box as _,
            );

            if hwnd == null_mut() {
                drop(Box::from_raw(app_box));
                return;
            }

            SetTimer(hwnd, TIMER_ID, TIMER_INTERVAL_MS, None);

            let mut msg = MSG::default();
            while GetMessageW(&mut msg, null_mut(), 0, 0) != 0 {
                TranslateMessage(&msg);
                DispatchMessageW(&msg);
            }
        }
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe extern "system" fn window_proc(
    hwnd: HWND,
    msg: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    match msg {
        WM_CREATE => {
            let create_struct = unsafe { &*(lparam as *const CREATESTRUCTW) };
            unsafe {
                windows_sys::Win32::UI::WindowsAndMessaging::SetWindowLongPtrW(
                    hwnd,
                    GWLP_USERDATA,
                    create_struct.lpCreateParams as isize,
                );
            }
            if let Some(app) = get_app(hwnd) {
                render_overlay(hwnd, app);
            }
            0
        }
        WM_TIMER => {
            if wparam as usize == TIMER_ID {
                if let Some(app) = get_app(hwnd) {
                    render_overlay(hwnd, app);
                }
            }
            0
        }
        WM_DESTROY => {
            if let Some(app_ptr) = get_app_ptr(hwnd) {
                let app = Box::from_raw(app_ptr);
                if app.font_mem_resource != null_mut() {
                    RemoveFontMemResourceEx(app.font_mem_resource);
                }
            }
            PostQuitMessage(0);
            0
        }
        _ => DefWindowProcW(hwnd, msg, wparam, lparam),
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn get_app_ptr(hwnd: HWND) -> Option<*mut OverlayApp> {
    let ptr = windows_sys::Win32::UI::WindowsAndMessaging::GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *mut OverlayApp;
    if ptr.is_null() {
        None
    } else {
        Some(ptr)
    }
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn get_app(hwnd: HWND) -> Option<&'static mut OverlayApp> {
    get_app_ptr(hwnd).map(|ptr| &mut *ptr)
}

#[allow(unsafe_op_in_unsafe_fn)]
unsafe fn render_overlay(hwnd: HWND, app: &mut OverlayApp) {
    let text = match app.game_process.read_string(app.game_process.base_address + 0x3335638, 256) {
        Ok(name) => format!("Host: {}", name),
        Err(e) => format!("Error: {}", e),
    };

    let hdc_screen = GetDC(null_mut());
    if hdc_screen == null_mut() {
        return;
    }

    let mut bmi = BITMAPINFO {
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
    let hbitmap = CreateDIBSection(hdc_screen, &mut bmi, DIB_RGB_COLORS, &mut bits, null_mut(), 0);
    if hbitmap == null_mut() || bits.is_null() {
        ReleaseDC(null_mut(), hdc_screen);
        return;
    }

    let hdc_mem = CreateCompatibleDC(hdc_screen);
    if hdc_mem == null_mut() {
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
    for (dst, src) in lf.lfFaceName.iter_mut().zip(font_name.iter()) {
        *dst = *src;
    }
    let hfont = CreateFontIndirectW(&lf);
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
        DT_CENTER | DT_VCENTER | DT_SINGLELINE,
    );

    let pixels = std::slice::from_raw_parts_mut(bits as *mut u32, (buffer_size / 4) as usize);
    for pixel in pixels.iter_mut() {
        if (*pixel & 0x00FF_FFFF) != 0 {
            *pixel |= 0xFF00_0000;
        }
    }

    let blend = BLENDFUNCTION {
        BlendOp: 0,
        BlendFlags: 0,
        SourceConstantAlpha: 255,
        AlphaFormat: 1,
    };

    UpdateLayeredWindow(
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
}

fn to_wstr(value: &str) -> Vec<u16> {
    OsStr::new(value)
        .encode_wide()
        .chain(std::iter::once(0))
        .collect()
}