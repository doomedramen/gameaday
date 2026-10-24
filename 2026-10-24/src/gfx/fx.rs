//! Cosmetic effects: particles, popups, screen shake, Wren's scarf, trail
//! and ambient weather. Nothing here influences the simulation.

use super::canvas::{Canvas, H, W, hash2};
use crate::level::Biome;
use crate::sim::*;

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max: f32,
    pub size: f32,
    pub col: u32,
    pub grav: f32,
    pub drag: f32,
    pub add: bool,
    /// 0 dot, 1 streak, 2 expanding ring, 3 shard
    pub shape: u8,
}

#[derive(Clone, Debug)]
pub struct Popup {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub life: f32,
    pub col: u32,
    pub scale: f32,
}

pub struct Fx {
    pub parts: Vec<Particle>,
    pub weather: Vec<Particle>,
    pub popups: Vec<Popup>,
    pub shake: f32,
    pub flash: f32,
    pub flash_col: u32,
    pub scarf: [(f32, f32); 9],
    pub trail: Vec<(f32, f32, f32)>,
    rng: u32,
    pub time: f32,
}

impl Default for Fx {
    fn default() -> Fx {
        Fx::new()
    }
}

impl Fx {
    pub fn new() -> Fx {
        Fx {
            parts: Vec::with_capacity(1200),
            weather: Vec::with_capacity(160),
            popups: vec![],
            shake: 0.0,
            flash: 0.0,
            flash_col: 0xffffff,
            scarf: [(0.0, 0.0); 9],
            trail: vec![],
            rng: 0x9e3779b9,
            time: 0.0,
        }
    }

    pub fn clear(&mut self) {
        self.parts.clear();
        self.popups.clear();
        self.weather.clear();
        self.trail.clear();
        self.shake = 0.0;
        self.flash = 0.0;
    }

    fn rand(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        (self.rng >> 8) as f32 / 16_777_216.0
    }

    fn rr(&mut self, a: f32, b: f32) -> f32 {
        a + (b - a) * self.rand()
    }

    #[allow(clippy::too_many_arguments)]
    pub fn spawn(
        &mut self,
        x: f32,
        y: f32,
        vx: f32,
        vy: f32,
        life: f32,
        size: f32,
        col: u32,
        grav: f32,
        add: bool,
        shape: u8,
    ) {
        if self.parts.len() > 1800 {
            return;
        }
        self.parts.push(Particle {
            x,
            y,
            vx,
            vy,
            life,
            max: life,
            size,
            col,
            grav,
            drag: 1.2,
            add,
            shape,
        });
    }

    pub fn burst(
        &mut self,
        x: f32,
        y: f32,
        n: usize,
        speed: f32,
        life: f32,
        size: f32,
        cols: &[u32],
        grav: f32,
        add: bool,
    ) {
        for _ in 0..n {
            let a = self.rr(0.0, std::f32::consts::TAU);
            let s = self.rr(0.25, 1.0) * speed;
            let col = cols[(self.rand() * cols.len() as f32) as usize % cols.len()];
            let l = self.rr(0.6, 1.0) * life;
            let sz = self.rr(0.6, 1.2) * size;
            self.spawn(x, y, a.cos() * s, a.sin() * s, l, sz, col, grav, add, 0);
        }
    }

    pub fn ring(&mut self, x: f32, y: f32, size: f32, col: u32, life: f32) {
        self.spawn(x, y, 0.0, 0.0, life, size, col, 0.0, true, 2);
    }

    pub fn popup(&mut self, x: f32, y: f32, text: String, col: u32, scale: f32) {
        self.popups.push(Popup {
            x,
            y,
            text,
            life: 1.0,
            col,
            scale,
        });
    }

    pub fn add_shake(&mut self, s: f32) {
        self.shake = self.shake.max(s);
    }

    pub fn add_flash(&mut self, a: f32, col: u32) {
        if a > self.flash {
            self.flash = a;
            self.flash_col = col;
        }
    }

    /// React to a simulation event.
    pub fn on_event(&mut self, e: &Event, biome: Biome, player: &Player) {
        let (x, y) = (e.x, e.y);
        let dust = [0xe8e0d0, 0xc8bca8, 0xffffff];
        match e.kind {
            Ev::Jump => {
                for i in 0..6 {
                    let d = if i % 2 == 0 { -1.0 } else { 1.0 };
                    let vx = self.rr(10.0, 70.0) * d;
                    let vy = self.rr(-40.0, -5.0);
                    self.spawn(
                        x + d * 4.0,
                        y,
                        vx,
                        vy,
                        0.35,
                        3.0,
                        dust[i % 3],
                        30.0,
                        false,
                        0,
                    );
                }
            }
            Ev::WallJump => {
                let side = e.v;
                for _ in 0..7 {
                    let vx = -side * self.rr(30.0, 110.0);
                    let vy = self.rr(-90.0, 20.0);
                    let oy = self.rr(-8.0, 10.0);
                    self.spawn(x, y + oy, vx, vy, 0.35, 2.6, 0xffffff, 60.0, true, 0);
                }
            }
            Ev::AirJump => {
                self.ring(x, y, 20.0, 0xbfe8ff, 0.3);
            }
            Ev::Land => {
                let k = (e.v / 700.0).clamp(0.2, 1.0);
                let n = (4.0 + 8.0 * k) as usize;
                for i in 0..n {
                    let d = if i % 2 == 0 { -1.0 } else { 1.0 };
                    let vx = self.rr(20.0, 90.0 + 80.0 * k) * d;
                    let vy = self.rr(-60.0, -8.0);
                    self.spawn(
                        x + d * 5.0,
                        y,
                        vx,
                        vy,
                        0.4,
                        3.0 + 2.0 * k,
                        dust[i % 3],
                        40.0,
                        false,
                        0,
                    );
                }
                if k > 0.6 {
                    self.add_shake(2.5 * k);
                }
            }
            Ev::HookHit => {
                self.burst(
                    x,
                    y,
                    12,
                    120.0,
                    0.35,
                    2.6,
                    &[0xffe9a8, 0xffffff, 0xffc060],
                    0.0,
                    true,
                );
                self.ring(x, y, 26.0, 0xffe9a8, 0.35);
            }
            Ev::HookMiss => {
                self.burst(x, y, 4, 40.0, 0.2, 2.0, &[0xaaaaaa, 0xffffff], 0.0, true);
            }
            Ev::Release => {
                if e.v > 400.0 {
                    self.burst(x, y, 10, 90.0, 0.4, 2.4, &[0xbfe8ff, 0xffffff], 0.0, true);
                }
            }
            Ev::RopeSnap | Ev::Brittle => {
                self.burst(
                    x,
                    y,
                    16,
                    140.0,
                    0.6,
                    2.6,
                    &[0xd8c8a8, 0xa88858, 0xffffff],
                    90.0,
                    false,
                );
                self.add_shake(2.0);
            }
            Ev::Crystal => {
                self.burst(
                    x,
                    y,
                    9,
                    110.0,
                    0.45,
                    2.4,
                    &[0x7af0ff, 0xffffff, 0xb8f8ff],
                    0.0,
                    true,
                );
                self.popup(x, y - 14.0, format!("+{}", e.v as i64), 0x9df4ff, 1.0);
            }
            Ev::EchoOrb => {
                self.ring(x, y, 40.0, 0x8aa8ff, 0.5);
                self.burst(
                    x,
                    y,
                    18,
                    130.0,
                    0.6,
                    3.0,
                    &[0x8aa8ff, 0xc8b8ff, 0xffffff],
                    0.0,
                    true,
                );
                self.popup(x, y - 18.0, "ECHO +".to_string(), 0xb8c8ff, 1.0);
            }
            Ev::HeartShard => {
                self.burst(
                    x,
                    y,
                    20,
                    130.0,
                    0.7,
                    3.0,
                    &[0xff6f7f, 0xffb0b8, 0xffffff],
                    -30.0,
                    true,
                );
                self.popup(x, y - 18.0, "+1 HEART".to_string(), 0xff8f9f, 1.1);
            }
            Ev::Chest => {
                self.burst(
                    x,
                    y,
                    40,
                    220.0,
                    1.0,
                    3.4,
                    &[0xffe066, 0xffffff, 0xffb347],
                    120.0,
                    true,
                );
                self.popup(x, y - 24.0, "RELIC!".to_string(), 0xffe066, 1.4);
                self.add_flash(0.25, 0xffe9a0);
            }
            Ev::Hurt => {
                self.burst(
                    player.x,
                    player.y,
                    22,
                    220.0,
                    0.5,
                    3.0,
                    &[0xff4f4f, 0xffa0a0, 0xffffff],
                    100.0,
                    true,
                );
                self.add_shake(9.0);
                self.add_flash(0.35, 0xff2020);
            }
            Ev::Shield => {
                self.ring(player.x, player.y, 40.0, 0x9ce8ff, 0.5);
                self.burst(
                    player.x,
                    player.y,
                    16,
                    160.0,
                    0.5,
                    2.6,
                    &[0x9ce8ff, 0xffffff],
                    0.0,
                    true,
                );
                self.add_shake(4.0);
            }
            Ev::Stomp => {
                self.burst(
                    x,
                    y,
                    12,
                    150.0,
                    0.4,
                    3.0,
                    &[0xffe066, 0xffffff, 0xff9a52],
                    120.0,
                    true,
                );
                self.ring(x, y, 28.0, 0xffe066, 0.3);
                self.add_shake(3.5);
            }
            Ev::Ram => {
                self.burst(
                    x,
                    y,
                    30,
                    300.0,
                    0.6,
                    3.4,
                    &[0xff9a52, 0xffe066, 0xffffff, 0xff5a3a],
                    140.0,
                    true,
                );
                self.ring(x, y, 54.0, 0xffb060, 0.4);
                self.add_shake(7.5);
                self.add_flash(0.18, 0xffd890);
            }
            Ev::Smash => {
                for _ in 0..3 {
                    let a = self.rr(0.0, std::f32::consts::TAU);
                    let s = self.rr(80.0, 330.0);
                    self.spawn(
                        x,
                        y,
                        a.cos() * s,
                        a.sin() * s - 60.0,
                        0.9,
                        4.5,
                        0xbff4ff,
                        500.0,
                        false,
                        3,
                    );
                }
                self.burst(x, y, 4, 200.0, 0.4, 2.4, &[0xffffff, 0xaaf0ff], 0.0, true);
                self.add_shake(6.0);
            }
            Ev::Pit => {
                self.popup(player.x, player.y - 30.0, "FELL".to_string(), 0xd0d0e0, 0.9);
            }
            Ev::Checkpoint => {
                for i in 0..20 {
                    let vy = -self.rr(60.0, 200.0);
                    let vx = self.rr(-40.0, 40.0);
                    let c = if i % 2 == 0 { 0xffe9a0 } else { 0xffffff };
                    let ox = self.rr(-8.0, 8.0);
                    self.spawn(x + ox, y - 20.0, vx, vy, 1.0, 2.6, c, 0.0, true, 0);
                }
                self.popup(x, y - 80.0, "CHECKPOINT".to_string(), 0xffe9a0, 1.0);
            }
            Ev::SpitterFire => {
                self.burst(
                    x,
                    y - 6.0,
                    8,
                    120.0,
                    0.25,
                    2.6,
                    &[0xffa050, 0xff5a3a, 0xffffff],
                    0.0,
                    true,
                );
            }
            Ev::GeyserBurst => {
                for _ in 0..24 {
                    let vx = self.rr(-40.0, 40.0);
                    let vy = -self.rr(180.0, 420.0);
                    let c = [0xff6a20, 0xffb040, 0xffe080][(self.rand() * 3.0) as usize % 3];
                    let ox = self.rr(-10.0, 10.0);
                    self.spawn(x + ox, y, vx, vy, 0.8, 3.4, c, 200.0, true, 0);
                }
                self.add_shake(2.0);
            }
            Ev::StrikeHit => {
                self.burst(
                    x,
                    y,
                    30,
                    320.0,
                    0.5,
                    3.0,
                    &[0xffffff, 0xc8b8ff, 0xffe566],
                    80.0,
                    true,
                );
                self.add_shake(8.0);
                self.add_flash(0.5, 0xe8e0ff);
            }
            Ev::Bounce => {
                self.ring(x, y, 24.0, 0xffffff, 0.3);
                self.burst(x, y, 8, 120.0, 0.35, 2.4, &[0x9be07a, 0xffffff], 0.0, true);
            }
            Ev::Crumble => {
                self.burst(
                    x,
                    y,
                    10,
                    90.0,
                    0.7,
                    3.2,
                    &[0xa89878, 0x786850],
                    380.0,
                    false,
                );
            }
            Ev::Cleared => {
                for _ in 0..60 {
                    let a = self.rr(-3.0, -0.14);
                    let s = self.rr(160.0, 480.0);
                    let c = [0xffe066, 0xff7a9a, 0x7af0ff, 0xffffff, 0x9be07a]
                        [(self.rand() * 5.0) as usize % 5];
                    self.spawn(x, y, a.cos() * s, a.sin() * s, 1.4, 3.6, c, 400.0, true, 3);
                }
                self.add_flash(0.3, 0xffffff);
            }
            Ev::BossHit => {
                self.burst(
                    x,
                    y,
                    60,
                    420.0,
                    1.0,
                    4.0,
                    &[0xffffff, 0xc8b8ff, 0xffe566, 0xff7ad0],
                    100.0,
                    true,
                );
                self.ring(x, y, 120.0, 0xffffff, 0.6);
                self.add_shake(16.0);
                self.add_flash(0.7, 0xffffff);
            }
            Ev::BossStun => {
                self.add_shake(12.0);
                for i in 0..16 {
                    let d = if i % 2 == 0 { -1.0 } else { 1.0 };
                    let vx = d * self.rr(60.0, 260.0);
                    let vy = self.rr(-120.0, -20.0);
                    self.spawn(
                        x + d * 30.0,
                        y + 36.0,
                        vx,
                        vy,
                        0.7,
                        5.0,
                        0xc8c0d8,
                        300.0,
                        false,
                        0,
                    );
                }
            }
            Ev::BossRoar => {
                self.add_shake(10.0);
                self.add_flash(0.3, 0xc8b8ff);
            }
            Ev::BossAttack => {
                self.add_shake(3.0);
            }
            Ev::BossDown => {
                for k in 0..6 {
                    let ox = (k as f32 - 2.5) * 30.0;
                    let oy = self.rr(-30.0, 30.0);
                    self.burst(
                        x + ox,
                        y + oy,
                        40,
                        380.0,
                        1.4,
                        4.4,
                        &[0xffffff, 0xffe566, 0xff7ad0, 0xc8b8ff],
                        60.0,
                        true,
                    );
                    self.ring(x + ox, y, 80.0 + k as f32 * 20.0, 0xffffff, 0.8);
                }
                self.add_shake(20.0);
                self.add_flash(1.0, 0xffffff);
            }
            Ev::Dying => {
                self.burst(
                    player.x,
                    player.y,
                    30,
                    260.0,
                    0.9,
                    3.4,
                    &[0xff6f6f, 0xffffff, 0x9ce8ff],
                    160.0,
                    true,
                );
                self.add_shake(10.0);
            }
            Ev::Revive => {
                self.ring(player.x, player.y, 90.0, 0xffe066, 0.8);
                self.burst(
                    player.x,
                    player.y,
                    50,
                    300.0,
                    1.0,
                    3.4,
                    &[0xffe066, 0xffb347, 0xffffff],
                    -40.0,
                    true,
                );
                self.add_flash(0.5, 0xffe9a0);
                self.popup(
                    player.x,
                    player.y - 40.0,
                    "PHOENIX!".to_string(),
                    0xffe066,
                    1.5,
                );
            }
            Ev::ChainBreak => {}
            _ => {}
        }
        let _ = biome;
    }

    /// Advance everything by `dt`.
    pub fn update(&mut self, dt: f32) {
        self.time += dt;
        for p in &mut self.parts {
            p.life -= dt;
            p.vy += p.grav * dt;
            let d = (1.0 - p.drag * dt).max(0.0);
            p.vx *= d;
            if p.shape != 3 {
                p.vy *= d.max(0.9);
            }
            p.x += p.vx * dt;
            p.y += p.vy * dt;
        }
        self.parts.retain(|p| p.life > 0.0);
        for p in &mut self.popups {
            p.life -= dt * 0.9;
            p.y -= 34.0 * dt;
        }
        self.popups.retain(|p| p.life > 0.0);
        self.shake = (self.shake - dt * 22.0).max(0.0);
        self.flash = (self.flash - dt * 2.4).max(0.0);
        for t in &mut self.trail {
            t.2 -= dt;
        }
        self.trail.retain(|t| t.2 > 0.0);
    }

    /// Update the scarf chain toward the neck position, trailing the motion.
    pub fn update_scarf(&mut self, neck: (f32, f32), vx: f32, vy: f32, facing: f32, dt: f32) {
        if self.scarf[0] == (0.0, 0.0) || (self.scarf[0].0 - neck.0).abs() > 400.0 {
            for (i, s) in self.scarf.iter_mut().enumerate() {
                *s = (neck.0 - facing * i as f32 * 4.0, neck.1);
            }
        }
        self.scarf[0] = neck;
        let seg = 5.2;
        for i in 1..self.scarf.len() {
            let (px, py) = self.scarf[i - 1];
            let (mut x, mut y) = self.scarf[i];
            // flutter: lag behind the motion plus a bit of wave
            x -= vx * dt * 0.8;
            y -= vy * dt * 0.5;
            y += 26.0 * dt;
            x += (self.time * 7.0 + i as f32 * 0.9).sin() * 9.0 * dt * i as f32 * 0.3;
            x -= facing * 22.0 * dt;
            let dx = x - px;
            let dy = y - py;
            let d = (dx * dx + dy * dy).sqrt().max(0.01);
            self.scarf[i] = (px + dx / d * seg, py + dy / d * seg);
        }
    }

    pub fn push_trail(&mut self, x: f32, y: f32) {
        self.trail.push((x, y, 0.22));
        if self.trail.len() > 12 {
            self.trail.remove(0);
        }
    }

    /// Keep a layer of ambient particles for the biome (screen space).
    pub fn update_weather(&mut self, biome: Biome, cam_x: f32, cam_y: f32, gust: f32, dt: f32) {
        let want = match biome {
            Biome::Mossbluff => 36,
            Biome::Glimmerdeep => 50,
            Biome::Frostspire => 110,
            Biome::Cinderworks => 70,
            Biome::Stormcrown => 120,
            Biome::Beacon => 60,
        };
        let _ = (cam_x, cam_y);
        while self.weather.len() < want {
            let x = self.rr(-40.0, W as f32 + 40.0);
            let y = if matches!(biome, Biome::Cinderworks) {
                self.rr(0.0, H as f32 + 20.0)
            } else {
                self.rr(-20.0, H as f32)
            };
            let (vx, vy, life, size, col, add) = match biome {
                Biome::Mossbluff => (
                    self.rr(8.0, 22.0),
                    self.rr(-6.0, 6.0),
                    self.rr(6.0, 12.0),
                    self.rr(1.4, 2.6),
                    0xfff4c0,
                    true,
                ),
                Biome::Glimmerdeep => (
                    self.rr(-6.0, 6.0),
                    self.rr(-14.0, -4.0),
                    self.rr(6.0, 12.0),
                    self.rr(1.6, 3.0),
                    [0x62f2ff, 0xc77dff, 0x9bb0ff][(self.rand() * 3.0) as usize % 3],
                    true,
                ),
                Biome::Frostspire => (
                    self.rr(-40.0, -10.0),
                    self.rr(30.0, 70.0),
                    self.rr(5.0, 9.0),
                    self.rr(1.4, 3.2),
                    0xffffff,
                    false,
                ),
                Biome::Cinderworks => (
                    self.rr(-12.0, 12.0),
                    -self.rr(30.0, 90.0),
                    self.rr(4.0, 9.0),
                    self.rr(1.4, 3.0),
                    [0xff9a3a, 0xffd24a, 0xff6a20][(self.rand() * 3.0) as usize % 3],
                    true,
                ),
                Biome::Stormcrown => (
                    self.rr(-160.0, -110.0),
                    self.rr(520.0, 700.0),
                    self.rr(0.7, 1.4),
                    1.0,
                    0xb8c0ff,
                    true,
                ),
                Biome::Beacon => (
                    self.rr(-8.0, 8.0),
                    self.rr(-20.0, -6.0),
                    self.rr(5.0, 10.0),
                    self.rr(1.4, 2.6),
                    0xffe9a0,
                    true,
                ),
            };
            self.weather.push(Particle {
                x,
                y,
                vx,
                vy,
                life,
                max: life,
                size,
                col,
                grav: 0.0,
                drag: 0.0,
                add,
                shape: if biome == Biome::Stormcrown { 1 } else { 0 },
            });
        }
        for p in &mut self.weather {
            p.life -= dt;
            p.x += (p.vx + gust * 160.0) * dt;
            p.y += p.vy * dt;
            if biome == Biome::Mossbluff || biome == Biome::Glimmerdeep || biome == Biome::Beacon {
                p.y += (self.time * 1.3 + p.x * 0.01).sin() * 8.0 * dt;
            }
        }
        self.weather.retain(|p| {
            p.life > 0.0
                && p.x > -60.0
                && p.x < W as f32 + 60.0
                && p.y > -40.0
                && p.y < H as f32 + 40.0
        });
    }

    /// Draw world-space particles (camera offset applied by caller's canvas).
    pub fn draw(&self, c: &mut Canvas) {
        for p in &self.parts {
            draw_particle(c, p, true);
        }
    }

    pub fn draw_weather(&self, c: &mut Canvas) {
        for p in &self.weather {
            let fade = (p.life / p.max)
                .min(1.0)
                .min((p.max - p.life) * 2.0 + 0.05)
                .clamp(0.0, 1.0);
            let a = fade * if p.shape == 1 { 0.35 } else { 0.8 };
            if p.shape == 1 {
                c.line(
                    p.x,
                    p.y,
                    p.x - p.vx * 0.03,
                    p.y - p.vy * 0.03,
                    1.2,
                    p.col,
                    a,
                );
            } else if p.add {
                c.glow(p.x, p.y, p.size * 3.0, p.col, a * 0.7);
                c.circle(p.x, p.y, p.size * 0.6, 0xffffff, a * 0.6);
            } else {
                c.circle(p.x, p.y, p.size, p.col, a);
            }
        }
    }

    pub fn draw_popups(&self, c: &mut Canvas) {
        for p in &self.popups {
            let a = (p.life * 1.6).min(1.0);
            let s = 2.0 * p.scale;
            c.text_center(p.x, p.y, &p.text, s, p.col, a);
        }
    }
}

pub fn draw_particle(c: &mut Canvas, p: &Particle, world: bool) {
    let _ = world;
    let t = (p.life / p.max).clamp(0.0, 1.0);
    match p.shape {
        2 => {
            let r = p.size * (1.0 - t) + 4.0;
            c.ring(p.x, p.y, r, r, 2.2, p.col, t * 0.8);
        }
        1 => {
            c.line(
                p.x,
                p.y,
                p.x - p.vx * 0.04,
                p.y - p.vy * 0.04,
                p.size * 0.5,
                p.col,
                t,
            );
        }
        3 => {
            let a = (hash2((p.max * 1000.0) as i32, 3) & 255) as f32 / 40.0 + p.life * 6.0;
            let s = p.size;
            let (ca, sa) = (a.cos(), a.sin());
            let pts = [
                (p.x + ca * s * 1.4, p.y + sa * s * 1.4),
                (p.x - sa * s * 0.7, p.y + ca * s * 0.7),
                (p.x - ca * s, p.y - sa * s),
            ];
            c.poly(&pts, p.col, t.sqrt());
        }
        _ => {
            if p.add {
                c.glow(p.x, p.y, p.size * 2.6, p.col, t * 0.8);
                c.circle(p.x, p.y, p.size * 0.5, 0xffffff, t * 0.7);
            } else {
                c.circle(p.x, p.y, p.size * (0.4 + 0.6 * t), p.col, t.sqrt());
            }
        }
    }
}
