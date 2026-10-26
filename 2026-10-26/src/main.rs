//! Native window and audio adapters. All rules, input actions and rendering live in the library.
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use minifb::{Key, KeyRepeat, MouseButton, MouseMode, ScaleMode, Window, WindowOptions};
use std::{
    sync::{Arc, Mutex},
    time::{Instant, SystemTime, UNIX_EPOCH},
};
use the_last_glasshouse::{
    audio::{AudioState, Synth},
    view::{H, View, W},
    *,
};
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.iter().any(|s| s == "--selftest") {
        match selftest() {
            Ok(s) => println!("{s}"),
            Err(e) => {
                eprintln!("SELFTEST FAILED: {e}");
                std::process::exit(1)
            }
        }
        return;
    }
    if let Some(i) = args.iter().position(|s| s == "--screenshot") {
        let dir = args.get(i + 1).map_or("screenshots", String::as_str);
        if let Err(e) = view::screenshots(std::path::Path::new(dir)) {
            eprintln!("Screenshot error: {e}");
            std::process::exit(1)
        }
        println!("Screenshots written to {dir}");
        return;
    }
    if let Err(e) = play() {
        eprintln!("{e}");
        std::process::exit(1)
    }
}
fn start_audio(shared: Arc<Mutex<AudioState>>) -> Option<cpal::Stream> {
    let device = cpal::default_host().default_output_device()?;
    let config = device.default_output_config().ok()?;
    if config.sample_format() != cpal::SampleFormat::F32 {
        return None;
    }
    let channels = config.channels() as usize;
    let mut synth = Synth::new(config.sample_rate());
    let stream = device
        .build_output_stream(
            config.into(),
            move |data: &mut [f32], _| {
                let s = shared.lock().map(|s| *s).unwrap_or(AudioState {
                    mute: true,
                    season: 0,
                    battle: false,
                    boss: false,
                    event: 0,
                    impact: 0,
                });
                for frame in data.chunks_mut(channels) {
                    let v = synth.next_sample(s);
                    frame.fill(v);
                }
            },
            |e| eprintln!("Audio: {e}"),
            None,
        )
        .ok()?;
    stream.play().ok()?;
    Some(stream)
}
#[cfg(target_os = "macos")]
fn toggle_fullscreen(w: &Window) {
    use std::ffi::{c_char, c_void};
    #[link(name = "objc")]
    unsafe extern "C" {
        fn sel_registerName(name: *const c_char) -> *mut c_void;
        fn objc_msgSend();
    } // SAFETY: minifb returns its live NSWindow; toggleFullScreen: takes a nullable sender and returns void. Invoked on the UI thread.
    unsafe {
        let selector = sel_registerName(c"toggleFullScreen:".as_ptr());
        let send: unsafe extern "C" fn(*mut c_void, *mut c_void, *mut c_void) =
            std::mem::transmute(objc_msgSend as *const ());
        send(w.get_window_handle(), selector, std::ptr::null_mut());
    }
}
#[cfg(not(target_os = "macos"))]
fn toggle_fullscreen(_w: &Window) {}
fn play() -> Result<(), Box<dyn std::error::Error>> {
    let mut window = Window::new(
        "The Last Glasshouse",
        W,
        H,
        WindowOptions {
            resize: true,
            scale_mode: ScaleMode::AspectRatioStretch,
            ..WindowOptions::default()
        },
    )?;
    window.set_target_fps(60);
    let seed = SystemTime::now().duration_since(UNIX_EPOCH)?.as_nanos() as u64;
    let mut app = App::new(seed);
    let mut view = View::new();
    let audio = Arc::new(Mutex::new(AudioState {
        mute: false,
        season: 0,
        battle: false,
        boss: false,
        event: 0,
        impact: 0,
    }));
    let _stream = start_audio(audio.clone());
    let mut last = Instant::now();
    let mut left_down = false;
    let mut right_down = false;
    view.render(&app);
    while window.is_open() && !app.quit {
        let now = Instant::now();
        app.tick((now - last).as_secs_f32());
        last = now;
        let (ww, wh) = window.get_size();
        let scale = (ww as f32 / W as f32).min(wh as f32 / H as f32);
        let offset = (
            (ww as f32 - W as f32 * scale) / 2.,
            (wh as f32 - H as f32 * scale) / 2.,
        );
        if let Some((x, y)) = window.get_mouse_pos(MouseMode::Pass) {
            app.cursor = ((x - offset.0) / scale, (y - offset.1) / scale);
        } else {
            app.cursor = (-1., -1.);
        }
        app.hover = view.action_at(app.cursor);
        let left = window.get_mouse_down(MouseButton::Left);
        let right = window.get_mouse_down(MouseButton::Right);
        if left && !left_down {
            app.act(app.hover);
        }
        if right && !right_down {
            if let Action::Card(i) = app.hover {
                if i < 9 {
                    app.act(Action::Compost(i));
                }
            } else {
                app.game.battle.selected = None;
            }
        }
        left_down = left;
        right_down = right;
        for k in window.get_keys_pressed(KeyRepeat::No) {
            let action = match k {
                Key::Escape => {
                    if app.overlay == Overlay::None {
                        if app.game.battle.selected.take().is_some() {
                            Action::None
                        } else {
                            Action::Pause
                        }
                    } else {
                        Action::Close
                    }
                }
                Key::Space => Action::EndTurn,
                Key::U => Action::Undo,
                Key::H | Key::F1 => Action::Help,
                Key::P => Action::Pause,
                Key::M => Action::Mute,
                Key::F | Key::F11 => Action::Fullscreen,
                Key::D => Action::Codex,
                Key::Key1 => Action::Card(0),
                Key::Key2 => Action::Card(1),
                Key::Key3 => Action::Card(2),
                Key::Key4 => Action::Card(3),
                Key::Key5 => Action::Card(4),
                Key::Key6 => Action::Card(5),
                Key::Key7 => Action::Card(6),
                Key::Key8 => Action::Card(7),
                Key::Key9 => Action::Card(8),
                Key::Enter => {
                    if app.game.screen == Screen::Title {
                        Action::Start
                    } else {
                        Action::None
                    }
                }
                _ => Action::None,
            };
            app.act(action);
        }
        if app.fullscreen_requested {
            toggle_fullscreen(&window);
            app.fullscreen_requested = false;
        }
        if let Ok(mut state) = audio.lock() {
            *state = AudioState {
                mute: app.muted,
                season: app.game.season(),
                battle: app.game.screen == Screen::Battle,
                boss: matches!(app.game.stage, 5 | 11 | 17),
                event: app.sound_event,
                impact: app.game.impact,
            };
        }
        view.render(&app);
        window.update_with_buffer(&view.pixels, W, H)?;
    }
    Ok(())
}
