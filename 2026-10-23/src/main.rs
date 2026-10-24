use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
use night_signal::audio::{MusicState, Sfx, Synth};
use night_signal::{Game, H, Screen, W, gfx};
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
fn png(path: PathBuf, p: &[u32]) {
    let file = std::fs::File::create(path).unwrap();
    let mut e = png::Encoder::new(file, W as u32, H as u32);
    e.set_color(png::ColorType::Rgba);
    e.set_depth(png::BitDepth::Eight);
    let mut w = e.write_header().unwrap();
    let mut bytes = Vec::with_capacity(p.len() * 4);
    for v in p {
        bytes.extend_from_slice(&[(v >> 16) as u8, (v >> 8) as u8, *v as u8, 255]);
    }
    w.write_image_data(&bytes).unwrap();
}
fn start_audio(synth: Arc<Mutex<Synth>>) -> Option<cpal::Stream> {
    let device = cpal::default_host().default_output_device()?;
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
fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mut game = Game::new(20261023, 0);
    if args.iter().any(|x| x == "--selftest") {
        assert!(Game::selftest());
        println!(
            "SELFTEST OK: campaign ended, score {}, nights {}",
            game.score, game.night
        );
        return;
    }
    if let Some(i) = args.iter().position(|x| x == "--screenshot") {
        let dir = PathBuf::from(args.get(i + 1).expect("--screenshot <dir>"));
        std::fs::create_dir_all(&dir).unwrap();
        png(dir.join("01_title.png"), &gfx::draw(&game));
        game.start();
        png(dir.join("02_early.png"), &gfx::draw(&game));
        for _ in 0..7 {
            game.broadcast()
        }
        png(dir.join("03_midcampaign.png"), &gfx::draw(&game));
        for _ in 0..8 {
            game.choose(2);
            game.broadcast()
        }
        png(dir.join("04_ending.png"), &gfx::draw(&game));
        return;
    }
    let mut window = Window::new("Night Signal", W, H, WindowOptions::default()).unwrap();
    window.set_target_fps(60);
    let synth = Arc::new(Mutex::new(Synth::new(48_000.0)));
    let _stream = start_audio(synth.clone());
    let mut prev = false;
    while window.is_open() && !window.is_key_down(Key::Escape) {
        if game.screen == Screen::Title
            && (window.is_key_down(Key::Enter) || window.get_mouse_down(MouseButton::Left))
        {
            game.start()
        }
        if game.screen == Screen::End && window.is_key_down(Key::R) {
            game = Game::new(20261023, game.best);
            game.start()
        }
        if game.screen == Screen::Playing {
            for (i, k) in [Key::Key1, Key::Key2, Key::Key3, Key::Key4]
                .iter()
                .enumerate()
            {
                if window.is_key_pressed(*k, minifb::KeyRepeat::No) {
                    game.choose(i as u8)
                }
            }
            if window.is_key_pressed(Key::Space, minifb::KeyRepeat::No) {
                game.broadcast();
                if let Ok(mut s) = synth.lock() {
                    s.play(Sfx::Banner);
                    s.set_music(MusicState {
                        intensity: 0.45,
                        boss: game.night >= 14,
                    });
                }
            }
            if window.is_key_pressed(Key::U, minifb::KeyRepeat::No) {
                game.upgrade()
            }
            if window.is_key_pressed(Key::M, minifb::KeyRepeat::No) {
                game.muted = !game.muted;
                if let Ok(mut s) = synth.lock() {
                    s.music_volume = if game.muted { 0.0 } else { 0.8 };
                    s.sfx_volume = if game.muted { 0.0 } else { 1.0 };
                }
            }
            let down = window.get_mouse_down(MouseButton::Left);
            if down
                && !prev
                && let Some((mx, my)) = window.get_mouse_pos(MouseMode::Clamp)
            {
                if mx > 862. && (212. ..472.).contains(&my) {
                    game.choose(((my - 212.) / 65.) as u8)
                } else if mx > 888. && (598. ..645.).contains(&my) {
                    game.broadcast();
                    if let Ok(mut s) = synth.lock() {
                        s.play(Sfx::Banner);
                        s.set_music(MusicState {
                            intensity: 0.45,
                            boss: game.night >= 14,
                        });
                    }
                } else if mx > 890. && (520. ..584.).contains(&my) {
                    let col = ((mx - 892.) / 108.) as usize;
                    let row = ((my - 520.) / 38.) as usize;
                    game.toggle_power(row * 3 + col);
                } else {
                    for i in 0..game.districts.len() {
                        let d = &game.districts[i];
                        let dx = mx - (d.x + 15) as f32;
                        let dy = my - (d.y + 10) as f32;
                        if dx * dx + dy * dy < 42. * 42. {
                            game.toggle_relay(i);
                            break;
                        }
                    }
                }
            }
            prev = down;
        }
        let frame = gfx::draw(&game);
        window.update_with_buffer(&frame, W, H).unwrap();
    }
}
