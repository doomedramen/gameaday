//! Nebula Drift - GameADay day 2 (2026-10-21).
//!
//! Top-down arena survival with animated sprites, particles, screen shake,
//! a synth soundtrack of pre-rendered WAVs and a boss every 5th wave.
//!
//! Controls:
//!   WASD / Arrows  - thrust
//!   Mouse          - aim
//!   Left click / Space - fire
//!   Shift / Right click - dash
//!   E / Middle click - energy pulse (needs a full meter)
//!   Enter / Space  - start, restart
//!   Esc            - quit

mod audio;
mod render;

use audio::{Audio, Sound};
use gameaday_2026_10_21::{Game, GameState, ParticleKind, SoundEvent, Vec2, WORLD_H, WORLD_W};
use minifb::{Key, MouseButton, MouseMode, Window, WindowOptions};
use render::{Canvas, Starfield};
use std::time::Instant;

const WIN_W: usize = 1000;
const WIN_H: usize = 690;

fn main() {
    // `--selftest` runs the whole pipeline headlessly (sim + renderer + audio
    // generation) so the shipping binary can be exercised in CI or over ssh
    // without opening a window. It exits 0 on success.
    if std::env::args().any(|a| a == "--selftest") {
        selftest();
        return;
    }
    if let Some(dir) = std::env::args().skip_while(|a| a != "--screenshot").nth(1) {
        screenshots(&dir);
        return;
    }

    let mut window = Window::new(
        "Nebula Drift - GameADay Day 2",
        WIN_W,
        WIN_H,
        WindowOptions {
            resize: true,
            ..WindowOptions::default()
        },
    )
    .expect("failed to open window");
    window.set_target_fps(60);

    let seed: u32 = rand::random();
    let mut game = Game::new(seed);
    let mut canvas = Canvas::new(WIN_W, WIN_H);
    let mut stars = Starfield::new(180);
    let mut audio = Audio::new(true);

    // Aim smoothing so the ship rotates fluidly instead of snapping.
    let mut smooth_aim = Vec2::new(1.0, 0.0);
    let mut prev_start = false;
    let mut last = Instant::now();

    while window.is_open() && !window.is_key_down(Key::Escape) {
        let now = Instant::now();
        let dt = (now - last).as_secs_f32().min(0.05);
        last = now;

        // ---------------- input ----------------
        let mouse_down = window.get_mouse_down(MouseButton::Left);
        let right_down = window.get_mouse_down(MouseButton::Right);
        let mid_down = window.get_mouse_down(MouseButton::Middle);

        let start_down = window.is_key_down(Key::Enter) || window.is_key_down(Key::Space);

        let mut input = gameaday_2026_10_21::Input {
            up: window.is_key_down(Key::W) || window.is_key_down(Key::Up),
            down: window.is_key_down(Key::S) || window.is_key_down(Key::Down),
            left: window.is_key_down(Key::A) || window.is_key_down(Key::Left),
            right: window.is_key_down(Key::D) || window.is_key_down(Key::Right),
            fire: mouse_down || window.is_key_down(Key::Space),
            dash: right_down || window.is_key_down(Key::LeftShift),
            pulse: mid_down || window.is_key_down(Key::E),
            start: start_down && !prev_start,
            ..Default::default()
        };
        prev_start = start_down;

        // Aim from the pointer relative to the ship.
        if let Some((mx, my)) = window.get_mouse_pos(MouseMode::Clamp) {
            let world = screen_to_world(&canvas, mx, my);
            input.pointer = Vec2::new(world.x - game.player_pos.x, world.y - game.player_pos.y);
        }

        // ---------------- update ----------------
        // Feed a smoothed pointer so rotation reads as animation, not teleport.
        let mut sim_input = input.clone();
        if sim_input.pointer.len() > 1.0 {
            let target = sim_input.pointer.normalized();
            let k = (dt * 14.0).min(1.0);
            smooth_aim.x += (target.x - smooth_aim.x) * k;
            smooth_aim.y += (target.y - smooth_aim.y) * k;
            sim_input.pointer = Vec2::new(smooth_aim.x * 100.0, smooth_aim.y * 100.0);
        }
        game.update(dt, &sim_input);

        // ---------------- sound ----------------
        let events: Vec<SoundEvent> = game.events.drain(..).collect();
        for e in events {
            audio.play(match e {
                SoundEvent::Shoot => Sound::Shoot,
                SoundEvent::Hit => Sound::Hit,
                SoundEvent::Explode => Sound::Explode,
                SoundEvent::Orb => Sound::Orb,
                SoundEvent::Hurt => Sound::Hurt,
                SoundEvent::WaveStart => Sound::WaveStart,
                SoundEvent::Pulse => Sound::Pulse,
                SoundEvent::Dash => Sound::Dash,
                SoundEvent::BossSpawn => Sound::BossSpawn,
                SoundEvent::Powerup => Sound::Powerup,
                SoundEvent::Spit => Sound::Spit,
            });
        }

        // ---------------- render ----------------
        if canvas.w != window.get_size().0 || canvas.h != window.get_size().1 {
            let (w, h) = window.get_size();
            if w > 0 && h > 0 {
                canvas = Canvas::new(w, h);
            }
        }
        // NOTE: the buffer is fully cleared every frame. This is the fix for
        // the frame-ghosting the player reported on day 1.
        canvas.clear(0x05070D);
        draw_scene(&mut canvas, &mut stars, &game);

        let _ = window.update_with_buffer(&canvas.buf, canvas.w, canvas.h);
    }
}

fn screen_to_world(c: &Canvas, sx: f32, sy: f32) -> Vec2 {
    Vec2::new((sx - c.off_x) / c.scale, (sy - c.off_y) / c.scale)
}

/// Headless end-to-end check: exercises the same update/render/sound paths the
/// real loop uses, across title -> play -> boss -> death -> restart.
fn selftest() {
    eprintln!("nebula-drift selftest: starting");
    let mut game = Game::new(20261021);
    let mut canvas = Canvas::new(WIN_W, WIN_H);
    let mut stars = Starfield::new(180);
    let mut rng = gameaday_2026_10_21::Rng32::new(99);

    // Sound clips must generate without touching the speakers.
    let clips = audio::encode_wav(&audio::test_clip(), 22_050);
    assert!(clips.len() > 44, "wav encoder produced no audio payload");
    eprintln!(
        "nebula-drift selftest: wav encoder ok ({} bytes)",
        clips.len()
    );

    let mut frames = 0usize;
    let mut max_wave = 0u32;
    let mut saw_game_over = false;
    let mut restarted = false;

    for i in 0..60 * 400 {
        let mut input = gameaday_2026_10_21::Input {
            fire: true,
            dash: i % 90 == 0,
            pulse: i % 300 == 0,
            start: i % 7 == 0,
            ..Default::default()
        };
        let ang = rng.range(0.0, std::f32::consts::TAU);
        input.up = ang < 1.5;
        input.down = ang >= 3.0;
        input.left = (1.5..2.3).contains(&ang);
        input.right = (2.3..3.0).contains(&ang);
        // Aim (and therefore shoot) at the nearest enemy so the bot survives
        // long enough to reach the deeper waves.
        if let Some(e) = game
            .enemies
            .iter()
            .min_by(|a, b| {
                let da = (a.pos.x - game.player_pos.x).hypot(a.pos.y - game.player_pos.y);
                let db = (b.pos.x - game.player_pos.x).hypot(b.pos.y - game.player_pos.y);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
        {
            input.pointer = Vec2::new(e.pos.x - game.player_pos.x, e.pos.y - game.player_pos.y);
        } else {
            input.pointer = Vec2::new(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0));
        }

        game.update(1.0 / 60.0, &input);
        game.events.clear();
        max_wave = max_wave.max(game.wave);

        // Render every 3rd frame to keep the selftest quick.
        if i % 3 == 0 {
            canvas.clear(0x05070D);
            draw_scene(&mut canvas, &mut stars, &game);
            frames += 1;
            // Every pixel must be written (no stale rows / ghost bands).
            let mut zeros = 0usize;
            for px in canvas.buf.iter() {
                if *px == 0 {
                    zeros += 1;
                }
            }
            let total = canvas.buf.len();
            assert!(
                zeros * 200 < total,
                "canvas left {zeros}/{total} pixels untouched (ghosting)"
            );
        }

        if game.state == GameState::GameOver {
            saw_game_over = true;
        }
        if saw_game_over && game.state == GameState::Playing && game.wave == 1 {
            restarted = true;
            break;
        }
    }

    assert!(frames > 0, "rendered no frames");
    assert!(max_wave >= 4, "bot only reached wave {max_wave}");
    assert!(restarted, "restart after game over did not work");

    // Force a death and confirm the game-over screen renders and restarts.
    game.health = 0.0;
    game.invuln = 0.0;
    game.update(1.0 / 60.0, &gameaday_2026_10_21::Input::default());
    game.events.clear();
    assert_eq!(game.state, GameState::GameOver);
    for _ in 0..60 {
        canvas.clear(0x05070D);
        draw_scene(&mut canvas, &mut stars, &game);
        game.update(1.0 / 60.0, &gameaday_2026_10_21::Input::default());
        game.events.clear();
    }
    let restart = gameaday_2026_10_21::Input {
        start: true,
        ..Default::default()
    };
    game.update(1.0 / 60.0, &restart);
    assert_eq!(game.state, GameState::Playing);
    assert_eq!(game.wave, 1, "restart must begin a fresh run");
    let _ = saw_game_over;

    // Force a boss wave and render it, so boss visuals are covered too.
    let mut boss = Game::new(5);
    boss.start_run();
    boss.wave = 4;
    boss.spawn_queue = 0;
    boss.enemies.clear();
    boss.wave_banner = 0.0;
    boss.update_waves(1.0 / 60.0);
    assert!(boss.enemies.iter().any(|e| e.kind.is_boss()));
    let mut boss_frames = 0;
    for _ in 0..120 {
        boss.update(1.0 / 60.0, &gameaday_2026_10_21::Input::default());
        boss.events.clear();
        canvas.clear(0x05070D);
        draw_scene(&mut canvas, &mut stars, &boss);
        boss_frames += 1;
    }
    assert!(boss_frames > 0);
    assert!(
        render::boss_bar(&boss).is_some(),
        "boss bar should be visible"
    );

    eprintln!(
        "nebula-drift selftest: OK - {frames} frames rendered, max wave {max_wave}, \
         boss wave + boss bar + game over + restart verified"
    );
}

/// Renders representative frames to BMP files so the visuals can be reviewed
/// without a display. Not part of normal play.
fn screenshots(dir: &str) {
    let _ = std::fs::create_dir_all(dir);
    let mut canvas = Canvas::new(WIN_W, WIN_H);
    let mut stars = Starfield::new(180);

    let save = |canvas: &Canvas, name: &str| {
        let path = format!("{dir}/{name}.bmp");
        let _ = std::fs::write(&path, to_bmp(&canvas.buf, canvas.w, canvas.h));
        eprintln!("wrote {path}");
    };

    // Title screen.
    let mut title = Game::new(20261021);
    for _ in 0..30 {
        title.update(1.0 / 60.0, &gameaday_2026_10_21::Input::default());
        title.events.clear();
    }
    canvas.set_viewport(0.0, 0.0);
    canvas.clear(0x05070D);
    draw_scene(&mut canvas, &mut stars, &title);
    save(&canvas, "title");

    // Mid-combat.
    let mut g = Game::new(20261021);
    g.start_run();
    let mut rng = gameaday_2026_10_21::Rng32::new(4242);
    for i in 0..60 * 26 {
        let mut input = gameaday_2026_10_21::Input {
            fire: true,
            dash: i % 80 == 0,
            ..Default::default()
        };
        let ang = rng.range(0.0, std::f32::consts::TAU);
        input.up = ang < 1.5;
        input.down = ang >= 3.0;
        input.left = (1.5..2.3).contains(&ang);
        input.right = (2.3..3.0).contains(&ang);
        if let Some(e) = g
            .enemies
            .iter()
            .min_by(|a, b| {
                let da = (a.pos.x - g.player_pos.x).hypot(a.pos.y - g.player_pos.y);
                let db = (b.pos.x - g.player_pos.x).hypot(b.pos.y - g.player_pos.y);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
        {
            input.pointer = Vec2::new(e.pos.x - g.player_pos.x, e.pos.y - g.player_pos.y);
        }
        g.update(1.0 / 60.0, &input);
        g.events.clear();
        if g.state == GameState::GameOver {
            g.start_run();
        }
    }
    canvas.set_viewport(0.0, 0.0);
    canvas.clear(0x05070D);
    draw_scene(&mut canvas, &mut stars, &g);
    save(&canvas, "combat");

    // Boss wave.
    let mut boss = Game::new(7);
    boss.start_run();
    boss.wave = 4;
    boss.spawn_queue = 0;
    boss.enemies.clear();
    boss.wave_banner = 0.0;
    boss.update_waves(1.0 / 60.0);
    for i in 0..120 {
        let mut input = gameaday_2026_10_21::Input {
            fire: true,
            ..Default::default()
        };
        input.pointer = Vec2::new(
            boss.player_pos.x - boss.enemies[0].pos.x,
            boss.player_pos.y - boss.enemies[0].pos.y,
        );
        let _ = i;
        boss.update(1.0 / 60.0, &input);
        boss.events.clear();
    }
    canvas.set_viewport(0.0, 0.0);
    canvas.clear(0x05070D);
    draw_scene(&mut canvas, &mut stars, &boss);
    save(&canvas, "boss");

    // Game over.
    boss.health = 0.0;
    boss.invuln = 0.0;
    boss.update(1.0 / 60.0, &gameaday_2026_10_21::Input::default());
    boss.events.clear();
    canvas.clear(0x05070D);
    draw_scene(&mut canvas, &mut stars, &boss);
    save(&canvas, "gameover");
}

/// 24-bit BMP writer (bottom-up, BGR, rows padded to 4 bytes).
fn to_bmp(buf: &[u32], w: usize, h: usize) -> Vec<u8> {
    let row_bytes = w * 3;
    let pad = (4 - (row_bytes % 4)) % 4;
    let stride = row_bytes + pad;
    let data_size = stride * h;
    let file_size = 54 + data_size;
    let mut out = Vec::with_capacity(file_size);
    out.extend_from_slice(b"BM");
    out.extend_from_slice(&(file_size as u32).to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&54u32.to_le_bytes());
    out.extend_from_slice(&40u32.to_le_bytes());
    out.extend_from_slice(&(w as i32).to_le_bytes());
    out.extend_from_slice(&(h as i32).to_le_bytes());
    out.extend_from_slice(&1u16.to_le_bytes());
    out.extend_from_slice(&24u16.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&(data_size as u32).to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&2835u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    out.extend_from_slice(&0u32.to_le_bytes());
    for y in (0..h).rev() {
        for x in 0..w {
            let px = buf[y * w + x];
            out.push((px & 0xFF) as u8);
            out.push(((px >> 8) & 0xFF) as u8);
            out.push(((px >> 16) & 0xFF) as u8);
        }
        // Row padding to a 4-byte boundary.
        out.resize(out.len() + pad, 0);
    }
    out
}

fn draw_scene(c: &mut Canvas, stars: &mut Starfield, g: &Game) {
    // Camera shake, scaled by the current shake magnitude.
    let t = g.time;
    let shake_x = (t * 61.0).sin() * g.shake;
    let shake_y = (t * 47.0).cos() * g.shake;
    c.set_viewport(shake_x, shake_y);

    // Arena backdrop.
    c.world_rect(0.0, 0.0, WORLD_W, WORLD_H, 0x0A0E18, 1.0);
    stars.draw(c, t);
    // Arena border glow.
    let pulse = 0.35 + (t * 1.5).sin() * 0.1;
    draw_arena_border(c, pulse);

    // Particles behind actors.
    for p in &g.particles {
        let life_frac = (p.life / p.max_life).clamp(0.0, 1.0);
        match p.kind {
            ParticleKind::Spark | ParticleKind::Thruster => {
                c.world_px_blend(p.pos.x, p.pos.y, p.color, life_frac);
                if p.size > 2.0 {
                    c.world_px_blend(p.pos.x + 1.0, p.pos.y, p.color, life_frac * 0.6);
                    c.world_px_blend(p.pos.x, p.pos.y + 1.0, p.color, life_frac * 0.6);
                }
            }
            ParticleKind::Ring => {
                let r = p.size * (1.0 - life_frac);
                c.ring(p.pos.x, p.pos.y, r, 6.0 * life_frac, p.color, life_frac);
            }
        }
    }

    // Orbs and powerups.
    for o in &g.orbs {
        render::draw_orb(c, o.pos, o.anim, o.life);
    }
    for p in &g.powerups {
        render::draw_powerup(c, p.pos, p.kind, p.anim, p.life);
    }

    // Spits (boss projectiles): pulsing violet orbs with a trail.
    for s in &g.spits {
        c.ring(s.pos.x, s.pos.y, 9.0, 2.0, 0xC77BFF, 0.7);
        c.circle(s.pos.x, s.pos.y, 6.0, 0xB05CFF);
        c.circle(s.pos.x, s.pos.y, 2.6, 0xF0E0FF);
    }

    // Bullets: bright streaks.
    for b in &g.bullets {
        let d = b.vel.normalized();
        for k in 0..4 {
            let f = k as f32 * 2.2;
            c.world_px_blend(
                b.pos.x - d.x * f,
                b.pos.y - d.y * f,
                0x8FE9FF,
                1.0 - k as f32 * 0.22,
            );
        }
        c.circle(b.pos.x, b.pos.y, 2.6, 0xEAF6FF);
    }

    // Enemies.
    for e in &g.enemies {
        render::draw_enemy(c, e.kind, e.pos, e.flash, e.spawn_t, t, e.seed);
    }

    // Player.
    if g.state == GameState::Playing {
        render::draw_player(c, g);
    }

    // Full-screen damage / pulse flash.
    if g.flash_amount > 0.0 {
        overlay_flash(c, g.flash_color, g.flash_amount);
    }

    // Ambient vignette so the arena edges feel dark and moody.
    vignette(c);

    match g.state {
        GameState::Title => draw_title(c, g),
        GameState::Playing => render::draw_hud(c, g),
        GameState::GameOver => {
            render::draw_hud(c, g);
            draw_game_over(c, g);
        }
    }
}

fn draw_arena_border(c: &mut Canvas, pulse: f32) {
    let steps = 120;
    for i in 0..steps {
        let fx = i as f32 / steps as f32;
        let x = fx * WORLD_W;
        let y = 0.0;
        c.world_px_blend(x, y, 0x5AC8FF, pulse);
        c.world_px_blend(x, WORLD_H, 0x5AC8FF, pulse);
    }
    for i in 0..steps {
        let fy = i as f32 / steps as f32;
        let y = fy * WORLD_H;
        c.world_px_blend(0.0, y, 0x5AC8FF, pulse);
        c.world_px_blend(WORLD_W, y, 0x5AC8FF, pulse);
    }
}

fn overlay_flash(c: &mut Canvas, color: u32, amount: f32) {
    let a = amount.clamp(0.0, 0.85);
    for y in 0..c.h as i32 {
        for x in 0..c.w as i32 {
            c.px_glow(x, y, color, a * 0.35);
        }
    }
}

fn vignette(c: &mut Canvas) {
    let cx = c.w as f32 * 0.5;
    let cy = c.h as f32 * 0.5;
    let maxd = (cx * cx + cy * cy).sqrt();
    // Darken a sparse 2x2 grid; the corner alpha comes from a cheap distance
    // approximation so this stays fast on every frame.
    for y in (0..c.h as i32).step_by(2) {
        let dy = y as f32 - cy;
        for x in (0..c.w as i32).step_by(2) {
            let dx = x as f32 - cx;
            let d = (dx * dx + dy * dy).sqrt() / maxd;
            if d > 0.62 {
                let a = ((d - 0.62) / 0.38 * 0.55).min(0.55);
                c.px_blend(x, y, 0x000000, a);
                c.px_blend(x + 1, y, 0x000000, a);
                c.px_blend(x, y + 1, 0x000000, a);
                c.px_blend(x + 1, y + 1, 0x000000, a);
            }
        }
    }
}

fn draw_title(c: &mut Canvas, g: &Game) {
    let w = c.w as f32;
    let h = c.h as f32;

    // Dim the arena behind the menu.
    for y in (0..c.h as i32).step_by(2) {
        for x in (0..c.w as i32).step_by(2) {
            c.px_blend(x, y, 0x02040A, 0.62);
            c.px_blend(x + 1, y, 0x02040A, 0.62);
            c.px_blend(x, y + 1, 0x02040A, 0.62);
            c.px_blend(x + 1, y + 1, 0x02040A, 0.62);
        }
    }

    render::draw_text_centered(c, w * 0.5, h * 0.16, "NEBULA", 6.0, 0x9FE8FF);
    render::draw_text_centered(c, w * 0.5, h * 0.16 + 48.0, "DRIFT", 6.0, 0xEAF6FF);

    let bob = (g.time * 2.0).sin() * 4.0;
    render::draw_text_centered(
        c,
        w * 0.5,
        h * 0.42 + bob,
        "PRESS ENTER TO LAUNCH",
        2.0,
        0xFFE066,
    );

    let lines = [
        "WASD / ARROWS - THRUST",
        "MOUSE - AIM   CLICK / SPACE - FIRE",
        "SHIFT / RMB - DASH   E / MMB - PULSE",
        "COLLECT ORBS TO CHARGE THE PULSE",
        "SURVIVE. BOSS EVERY 5 WAVES.",
    ];
    let mut y = h * 0.56;
    for l in lines {
        render::draw_text_centered(c, w * 0.5, y, l, 1.3, 0x8A93A6);
        y += 22.0;
    }

    // Little animated ship preview, floating in the middle of the arena.
    render::draw_ship_at(
        c,
        Vec2::new(WORLD_W * 0.5, WORLD_H * 0.5),
        Vec2::new((g.time).cos(), (g.time).sin()),
        g.time,
        false,
    );
}

fn draw_game_over(c: &mut Canvas, g: &Game) {
    let w = c.w as f32;
    let h = c.h as f32;
    for y in (0..c.h as i32).step_by(2) {
        for x in (0..c.w as i32).step_by(2) {
            c.px_blend(x, y, 0x04060C, 0.66);
            c.px_blend(x + 1, y, 0x04060C, 0.66);
            c.px_blend(x, y + 1, 0x04060C, 0.66);
            c.px_blend(x + 1, y + 1, 0x04060C, 0.66);
        }
    }
    render::draw_text_centered(c, w * 0.5, h * 0.22, "SHIP LOST", 4.0, 0xFF5C6C);
    render::draw_text_centered(
        c,
        w * 0.5,
        h * 0.36,
        &format!("FINAL SCORE {}", g.score),
        2.4,
        0xEAF6FF,
    );
    render::draw_text_centered(
        c,
        w * 0.5,
        h * 0.36 + 34.0,
        &format!("WAVE REACHED {}", g.wave),
        1.6,
        0x9FE8FF,
    );
    let best = g.best_score.max(g.score);
    render::draw_text_centered(
        c,
        w * 0.5,
        h * 0.36 + 58.0,
        &format!("BEST {}", best),
        1.4,
        0xFFE066,
    );
    let bob = (g.time * 3.0).sin() * 3.0;
    render::draw_text_centered(
        c,
        w * 0.5,
        h * 0.58 + bob,
        "PRESS ENTER TO FLY AGAIN",
        1.6,
        0xFFE066,
    );
}
