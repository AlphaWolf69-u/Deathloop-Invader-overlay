use deathloop_cheat::GameProcess;
use pixels::{Pixels, SurfaceTexture};
use winit::{
    event::{Event, WindowEvent},
    event_loop::EventLoop,
    window::{WindowBuilder, WindowLevel},
};
use winit::raw_window_handle::{RawWindowHandle, HasWindowHandle};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    SetWindowLongPtrW, GetWindowLongPtrW, SetLayeredWindowAttributes, GWL_EXSTYLE, WS_EX_LAYERED, WS_EX_TRANSPARENT,
};
use rusttype::{Font, Scale, point};

const WIDTH: u32 = 400;
const HEIGHT: u32 = 200;

pub struct OverlayApp {
    game_process: GameProcess,
    event_loop: EventLoop<()>,
    window: winit::window::Window,
    pixels: Pixels,
    font: Font<'static>,
}

impl OverlayApp {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        let game_process = GameProcess::attach("Deathloop.exe", "Deathloop.exe")?;

        let event_loop = EventLoop::new().expect("Failed to create event loop");
        let window = WindowBuilder::new()
            .with_title("Overlay")
            .with_inner_size(winit::dpi::PhysicalSize::new(WIDTH, HEIGHT))
            .with_transparent(true)
            .with_window_level(WindowLevel::AlwaysOnTop)
            .with_decorations(false)
            .build(&event_loop)?;

        // Make mouse passthrough and transparent
        let hwnd = match window.window_handle().unwrap().as_raw() {
            RawWindowHandle::Win32(handle) => handle.hwnd.get() as *mut std::ffi::c_void,
            _ => panic!("Not a Win32 window"),
        };
        unsafe {
            let ex_style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, ex_style | (WS_EX_LAYERED | WS_EX_TRANSPARENT) as isize);
            // SetLayeredWindowAttributes for alpha
            SetLayeredWindowAttributes(hwnd, 0, 255, 0x00000002); // LWA_ALPHA
        }

        let surface_texture = SurfaceTexture::new(WIDTH, HEIGHT, &window);
        let pixels = Pixels::new(WIDTH, HEIGHT, surface_texture)?;

        // Load font - using a simple font, assuming we have it
        // For now, use a default or panic
        let font_data = include_bytes!("../assets/arial.ttf"); // Need to download this
        let font = Font::try_from_bytes(font_data).expect("Failed to load font");

        Ok(Self {
            game_process,
            event_loop,
            window,
            pixels,
            font,
        })
    }

    pub fn run(self) {
        let OverlayApp { event_loop, window, mut pixels, font, game_process } = self;

        let _ = event_loop.run(move |event, event_loop| {
            match event {
                Event::WindowEvent { event: WindowEvent::CloseRequested, .. } => {
                    event_loop.exit();
                }
                Event::AboutToWait => {
                    // Update and redraw
                    let host_name = std::env::var("COMPUTERNAME").unwrap_or_else(|_| "Unknown".to_string());
                    let addr = game_process.base_address + 0x3335638;
                    let game_string = game_process.read_string(addr, 256);

                    // Render
                    let frame = pixels.frame_mut();
                    // Clear to transparent
                    for pixel in frame.chunks_exact_mut(4) {
                        pixel[0] = 0; // r
                        pixel[1] = 0; // g
                        pixel[2] = 0; // b
                        pixel[3] = 0; // a
                    }

                    // Draw text
                    draw_text(frame, &font, &format!("Host: {}", host_name), 10.0, 10.0, [255, 255, 255, 255], WIDTH as f32, HEIGHT as f32);
                    draw_text(frame, &font, &format!("Game: {}", game_string), 10.0, 40.0, [255, 255, 255, 255], WIDTH as f32, HEIGHT as f32);

                    // Render
                    if let Err(e) = pixels.render() {
                        eprintln!("Render error: {}", e);
                    }

                    window.request_redraw();
                }
                _ => {}
            }
        });
    }
}

fn draw_text(frame: &mut [u8], font: &Font, text: &str, x: f32, y: f32, color: [u8; 4], width: f32, height: f32) {
    let scale = Scale::uniform(20.0);
    let v_metrics = font.v_metrics(scale);
    let glyphs: Vec<_> = font.layout(text, scale, point(x, y + v_metrics.ascent)).collect();

    for glyph in glyphs {
        if let Some(bb) = glyph.pixel_bounding_box() {
            glyph.draw(|gx, gy, gv| {
                let px = (bb.min.x + gx as i32) as usize;
                let py = (bb.min.y + gy as i32) as usize;
                if px < width as usize && py < height as usize {
                    let idx = (py * width as usize + px) * 4;
                    if idx + 3 < frame.len() {
                        frame[idx] = color[0];
                        frame[idx + 1] = color[1];
                        frame[idx + 2] = color[2];
                        frame[idx + 3] = (gv * 255.0) as u8;
                    }
                }
            });
        }
    }
}

