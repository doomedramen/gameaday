//! Window, input and audio device glue. All game logic, rendering and
//! synthesis live in the library.

use std::sync::{Arc, Mutex};
use std::time::Instant;

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use lanternfall::app::{App, Input, Key};
use lanternfall::audio::Synth;
use lanternfall::gfx::canvas::{Canvas, H, W};
use lanternfall::save::Save;
use minifb::{KeyRepeat, MouseButton, MouseMode, Window, WindowOptions};

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--selftest") {
        match lanternfall::selftest::run() {
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
        match lanternfall::selftest::screenshots(std::path::Path::new(&dir)) {
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

fn play() {
    let mut window = match Window::new(
        "Lanternfall",
        W,
        H,
        WindowOptions {
            resize: false,
            ..WindowOptions::default()
        },
    ) {
        Ok(w) => w,
        Err(e) => {
            eprintln!("could not open window: {e}");
            std::process::exit(1);
        }
    };
    window.set_target_fps(60);
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

    while window.is_open() && !app.quit {
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(0.05);
        last = now;

        let mut input = Input::default();
        if let Some((x, y)) = window.get_mouse_pos(MouseMode::Clamp) {
            input.mouse = (x, y);
        }
        let l = window.get_mouse_down(MouseButton::Left);
        let r = window.get_mouse_down(MouseButton::Right);
        input.click = l && !was_down.0;
        input.right_click = r && !was_down.1;
        was_down = (l, r);
        for k in window.get_keys_pressed(KeyRepeat::No) {
            let mapped = match k {
                minifb::Key::Key1 | minifb::Key::NumPad1 => Some(Key::Num1),
                minifb::Key::Key2 | minifb::Key::NumPad2 => Some(Key::Num2),
                minifb::Key::Key3 | minifb::Key::NumPad3 => Some(Key::Num3),
                minifb::Key::Space => Some(Key::Space),
                minifb::Key::Enter | minifb::Key::NumPadEnter => Some(Key::Enter),
                minifb::Key::Escape => Some(Key::Escape),
                minifb::Key::Tab => Some(Key::Tab),
                minifb::Key::U => Some(Key::U),
                minifb::Key::D => Some(Key::D),
                minifb::Key::H => Some(Key::H),
                minifb::Key::M => Some(Key::M),
                _ => None,
            };
            if let Some(m) = mapped {
                input.keys.push(m);
            }
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
        if window.update_with_buffer(&canvas.px, W, H).is_err() {
            break;
        }
    }
}
