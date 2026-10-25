//! Window, keyboard and audio device glue. All game logic, rendering and
//! synthesis live in the library.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use minifb::{KeyRepeat, Window, WindowOptions};
use slipstream_cup::app::{App, Input, Key};
use slipstream_cup::audio::Synth;
use slipstream_cup::canvas::{Canvas, H, W};
use slipstream_cup::save::Save;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--selftest") {
        match slipstream_cup::selftest::run() {
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
        match slipstream_cup::selftest::screenshots(std::path::Path::new(&dir)) {
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
        K::Space => out.push(Key::Drift),
        K::LeftShift | K::RightShift | K::X => out.push(Key::Nitro),
        K::Escape | K::Backspace => out.push(Key::Back),
        K::P => out.push(Key::Pause),
        K::M => out.push(Key::Mute),
        K::H | K::F1 => out.push(Key::Help),
        K::R => out.push(Key::Restart),
        K::Enter | K::NumPadEnter => out.push(Key::Enter),
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
/// that letterboxes the frame.
fn open_window(fullscreen: bool) -> Option<Window> {
    let (w, h) = if fullscreen { screen_size() } else { (W, H) };
    let mut window = Window::new(
        "Slipstream Cup",
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
    Some(window)
}

fn play() {
    let Some(mut window) = open_window(false) else {
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

    while window.is_open() && !app.quit {
        let now = Instant::now();
        // the simulation advances by exactly the real frame time, so motion
        // matches what the display shows (no accumulator hitching)
        let dt = (now - last).as_secs_f32().min(0.05);
        last = now;

        if (window.is_key_pressed(minifb::Key::F11, KeyRepeat::No)
            || window.is_key_pressed(minifb::Key::F, KeyRepeat::No))
            && let Some(w) = open_window(!fullscreen)
        {
            window = w;
            fullscreen = !fullscreen;
        }
        let mut input = Input::default();
        for k in window.get_keys() {
            map_key(k, &mut input.down);
        }
        for k in window.get_keys_pressed(KeyRepeat::No) {
            map_key(k, &mut input.pressed);
        }
        app.update(dt, &input);
        let sounds = app.take_sounds();
        let mix = app.mix();
        if let Ok(mut s) = synth.lock() {
            s.set_mix(mix);
            for e in sounds {
                s.play(e);
            }
        }
        app.draw(&mut canvas);
        if window.update_with_buffer(&canvas.px, W, H).is_err() {
            break;
        }
    }
}
