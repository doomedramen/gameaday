//! Window, input and audio device glue. All game logic, rendering and
//! synthesis live in the library.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cragswing::app::{App, Input, Key};
use cragswing::audio::Synth;
use cragswing::gfx::canvas::{Canvas, H, W};
use cragswing::save::Save;
use minifb::{KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--selftest") {
        match cragswing::selftest::run() {
            Ok(report) => {
                println!("{report}");
                println!("SELFTEST OK");
                return;
            }
            Err(e) => {
                eprintln!("SELFTEST FAILED: {e}");
                std::process::exit(1);
            }
        }
    }
    if let Some(i) = args.iter().position(|a| a == "--screenshot") {
        let dir = args
            .get(i + 1)
            .cloned()
            .unwrap_or_else(|| "screenshots".to_string());
        match cragswing::selftest::screenshots(std::path::Path::new(&dir)) {
            Ok(n) => {
                println!("wrote {n} screenshots to {dir}");
                return;
            }
            Err(e) => {
                eprintln!("screenshot failed: {e}");
                std::process::exit(1);
            }
        }
    }
    play();
}

/// Start the audio device; the game runs silently if none is available.
fn start_audio(synth: Arc<Mutex<Synth>>) -> Option<cpal::Stream> {
    let host = cpal::default_host();
    let device = host.default_output_device()?;
    let config = device.default_output_config().ok()?;
    let channels = config.channels() as usize;
    let rate = config.sample_rate() as f32;
    if let Ok(mut s) = synth.lock() {
        *s = Synth::new(rate);
    }
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device
            .build_output_stream(
                config.into(),
                move |data: &mut [f32], _| {
                    if let Ok(mut s) = synth.lock() {
                        s.render(data, channels);
                    } else {
                        data.fill(0.0);
                    }
                },
                |e| eprintln!("audio error: {e}"),
                None,
            )
            .ok()?,
        _ => return None,
    };
    stream.play().ok()?;
    Some(stream)
}

fn map_key(k: minifb::Key, out: &mut Vec<Key>) {
    use minifb::Key as K;
    match k {
        K::Left | K::A => out.push(Key::Left),
        K::Right | K::D => out.push(Key::Right),
        K::Up | K::W => out.push(Key::Up),
        K::Down | K::S => out.push(Key::Down),
        K::Space | K::K | K::Z | K::C => out.push(Key::Jump),
        K::LeftShift | K::RightShift | K::X | K::J => out.push(Key::Hook),
        K::R => out.push(Key::Rewind),
        K::Escape => {
            out.push(Key::Pause);
            out.push(Key::Back);
        }
        K::P => out.push(Key::Pause),
        K::M => out.push(Key::Mute),
        K::H | K::F1 => out.push(Key::Help),
        K::Enter | K::NumPadEnter => out.push(Key::Enter),
        K::Tab => out.push(Key::Tab),
        K::Key1 | K::NumPad1 => out.push(Key::Num1),
        K::Key2 | K::NumPad2 => out.push(Key::Num2),
        K::Key3 | K::NumPad3 => out.push(Key::Num3),
        K::Backspace => out.push(Key::Back),
        _ => {}
    }
}

/// Logical size of the main display, for full screen.
#[cfg(target_os = "macos")]
fn screen_size() -> (usize, usize) {
    #[link(name = "CoreGraphics", kind = "framework")]
    unsafe extern "C" {
        fn CGMainDisplayID() -> u32;
        fn CGDisplayPixelsWide(d: u32) -> usize;
        fn CGDisplayPixelsHigh(d: u32) -> usize;
    }
    // SAFETY: plain CoreGraphics queries with no pointers involved.
    let (w, h) = unsafe {
        let d = CGMainDisplayID();
        (CGDisplayPixelsWide(d), CGDisplayPixelsHigh(d))
    };
    if w >= W && h >= 300 { (w, h) } else { (W, H) }
}

#[cfg(not(target_os = "macos"))]
fn screen_size() -> (usize, usize) {
    (1920, 1080)
}

/// Open the game window: fixed 1280x720, or a borderless screen-sized window
/// that letterboxes the frame. The size is never changed by the user, so the
/// mouse mapping below always knows it exactly.
fn open_window(fullscreen: bool) -> Option<(Window, (usize, usize))> {
    let (w, h) = if fullscreen { screen_size() } else { (W, H) };
    let mut window = Window::new(
        "Cragswing",
        w,
        h,
        WindowOptions {
            resize: false,
            borderless: fullscreen,
            scale_mode: minifb::ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    )
    .ok()?;
    window.set_target_fps(60);
    if fullscreen {
        window.set_position(0, 0);
    }
    Some((window, (w, h)))
}

fn play() {
    let Some((mut window, mut wsize)) = open_window(false) else {
        eprintln!("could not open window");
        std::process::exit(1);
    };
    let mut fullscreen = false;
    let synth = Arc::new(Mutex::new(Synth::new(48000.0)));
    let _stream = start_audio(synth.clone());

    let seed = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(7);
    let mut app = App::new(Some(Save::default_path()), seed);
    let mut canvas = Canvas::new(W, H);
    let mut last = Instant::now();
    let mut was_down = (false, false);
    let mut last_mouse = (0.0f32, 0.0f32);

    while window.is_open() && !app.quit {
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(0.05);
        last = now;

        let toggle = window.is_key_pressed(minifb::Key::F11, KeyRepeat::No)
            || window.is_key_pressed(minifb::Key::F, KeyRepeat::No);
        if let (true, Some((w, sz))) = (toggle, toggle.then(|| open_window(!fullscreen)).flatten())
        {
            window = w;
            wsize = sz;
            fullscreen = !fullscreen;
        }

        let mut input = Input::default();
        // window coordinates -> the 1280x720 frame, allowing for letterbox bars
        if let Some((mx, my)) = window.get_unscaled_mouse_pos(MouseMode::Pass) {
            let (ww, wh) = (wsize.0 as f32, wsize.1 as f32);
            let k = (ww / W as f32).min(wh / H as f32);
            let (ox, oy) = ((ww - W as f32 * k) / 2.0, (wh - H as f32 * k) / 2.0);
            last_mouse = (
                ((mx - ox) / k).clamp(0.0, W as f32 - 1.0),
                ((my - oy) / k).clamp(0.0, H as f32 - 1.0),
            );
        }
        input.mouse = last_mouse;
        let l = window.get_mouse_down(MouseButton::Left);
        let r = window.get_mouse_down(MouseButton::Right);
        input.mouse_down = l;
        input.rmouse_down = r;
        input.mouse_pressed = l && !was_down.0;
        input.rmouse_pressed = r && !was_down.1;
        was_down = (l, r);
        for k in window.get_keys() {
            map_key(k, &mut input.down);
        }
        for k in window.get_keys_pressed(KeyRepeat::No) {
            map_key(k, &mut input.pressed);
        }

        app.update(dt, &input);
        let sfx = app.take_sfx();
        if let Ok(mut s) = synth.lock() {
            let vol = if app.muted { 0.0 } else { 1.0 };
            s.sfx_volume = vol;
            s.music_volume = 0.8 * vol;
            s.set_music(app.music());
            for e in sfx {
                s.play(e);
            }
        }
        app.draw(&mut canvas);
        window.set_cursor_visibility(!app.cursor_hidden());
        if window.update_with_buffer(&canvas.px, W, H).is_err() {
            break;
        }
    }
}
