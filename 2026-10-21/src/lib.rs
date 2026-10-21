//! Nebula Drift - core game logic.
//!
//! Everything here is pure logic with no windowing or audio so it can be
//! unit tested headlessly. The binary (`main.rs`) owns rendering, input and
//! sound.

pub const WORLD_W: f32 = 900.0;
pub const WORLD_H: f32 = 620.0;

/// Deterministic pseudo-random stream so replays / tests are stable.
pub struct Rng32 {
    state: u32,
}

impl Rng32 {
    pub fn new(seed: u32) -> Self {
        Rng32 {
            state: if seed == 0 { 0x9E37_79B9 } else { seed },
        }
    }

    pub fn next_u32(&mut self) -> u32 {
        // xorshift32
        let mut x = self.state;
        x ^= x << 13;
        x ^= x >> 17;
        x ^= x << 5;
        self.state = x;
        x
    }

    /// Uniform float in [0, 1).
    pub fn next_f32(&mut self) -> f32 {
        (self.next_u32() >> 8) as f32 / (1u32 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.next_f32() * (hi - lo)
    }
}

#[derive(Clone, Copy, Debug)]
pub struct BossBar {
    pub hp: i32,
    pub max_hp: i32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum GameState {
    Title,
    Playing,
    GameOver,
}

#[derive(Clone, Copy, Debug)]
pub struct Vec2 {
    pub x: f32,
    pub y: f32,
}

impl Vec2 {
    pub fn new(x: f32, y: f32) -> Self {
        Vec2 { x, y }
    }
    pub fn len(&self) -> f32 {
        (self.x * self.x + self.y * self.y).sqrt()
    }
    pub fn normalized(&self) -> Vec2 {
        let l = self.len();
        if l < 1e-5 {
            Vec2::new(0.0, 0.0)
        } else {
            Vec2::new(self.x / l, self.y / l)
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnemyKind {
    /// Slow chaser, 1 hp.
    Drifter,
    /// Fast zig-zag dasher, 1 hp.
    Darter,
    /// Big slow tank, 3 hp, splits into two Drifters when killed.
    Bloater,
    /// Boss: huge hp pool, fires slow homing spit, spawns adds.
    Warden,
}

impl EnemyKind {
    pub fn radius(self) -> f32 {
        match self {
            EnemyKind::Drifter => 12.0,
            EnemyKind::Darter => 9.0,
            EnemyKind::Bloater => 22.0,
            EnemyKind::Warden => 46.0,
        }
    }
    pub fn speed(self) -> f32 {
        match self {
            EnemyKind::Drifter => 60.0,
            EnemyKind::Darter => 135.0,
            EnemyKind::Bloater => 38.0,
            EnemyKind::Warden => 30.0,
        }
    }
    pub fn hp(self) -> i32 {
        match self {
            EnemyKind::Drifter => 1,
            EnemyKind::Darter => 1,
            EnemyKind::Bloater => 3,
            EnemyKind::Warden => 40,
        }
    }
    pub fn score(self) -> u32 {
        match self {
            EnemyKind::Drifter => 10,
            EnemyKind::Darter => 20,
            EnemyKind::Bloater => 45,
            EnemyKind::Warden => 300,
        }
    }
    pub fn is_boss(self) -> bool {
        self == EnemyKind::Warden
    }
}

/// Hostile projectile fired by the boss.
#[derive(Clone, Copy, Debug)]
pub struct Spit {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PowerKind {
    /// Repairs 35 hull.
    Repair,
    /// Doubles fire rate for 8s.
    RapidFire,
    /// Refills energy to full.
    Charge,
}

impl PowerKind {
    pub fn color(self) -> u32 {
        match self {
            PowerKind::Repair => 0x7CFF9B,
            PowerKind::RapidFire => 0xFF9B7C,
            PowerKind::Charge => 0x8FD0FF,
        }
    }
    pub fn label(self) -> &'static str {
        match self {
            PowerKind::Repair => "REPAIR",
            PowerKind::RapidFire => "RAPID",
            PowerKind::Charge => "CHARGE",
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Powerup {
    pub pos: Vec2,
    pub vel: Vec2,
    pub kind: PowerKind,
    pub life: f32,
    pub anim: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Enemy {
    pub pos: Vec2,
    pub vel: Vec2,
    pub kind: EnemyKind,
    pub hp: i32,
    pub flash: f32,
    pub spawn_t: f32,
    pub seed: u32,
    pub atk_cd: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Bullet {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Orb {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub anim: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParticleKind {
    Spark,
    Ring,
    Thruster,
}

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub pos: Vec2,
    pub vel: Vec2,
    pub life: f32,
    pub max_life: f32,
    pub color: u32,
    pub size: f32,
    pub kind: ParticleKind,
}

pub struct Game {
    pub state: GameState,
    pub player_pos: Vec2,
    pub player_vel: Vec2,
    pub aim: Vec2,
    pub health: f32,
    pub max_health: f32,
    pub energy: f32,
    pub max_energy: f32,
    pub score: u32,
    pub best_score: u32,
    pub wave: u32,
    pub combo: u32,
    pub combo_timer: f32,
    pub enemies: Vec<Enemy>,
    pub bullets: Vec<Bullet>,
    pub spits: Vec<Spit>,
    pub orbs: Vec<Orb>,
    pub powerups: Vec<Powerup>,
    pub particles: Vec<Particle>,
    pub fire_cooldown: f32,
    pub dash_cooldown: f32,
    pub dash_time: f32,
    pub invuln: f32,
    pub rapid_fire: f32,
    pub pickup_banner: f32,
    pub pickup_label: &'static str,
    pub shake: f32,
    pub time: f32,
    pub wave_banner: f32,
    /// Enemies remaining to spawn this wave.
    pub spawn_queue: u32,
    pub spawn_timer: f32,
    pub rng: Rng32,
    /// Events produced this frame, drained by the audio/effect layer.
    pub events: Vec<SoundEvent>,
    pub flash_color: u32,
    pub flash_amount: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SoundEvent {
    Shoot,
    Hit,
    Explode,
    Orb,
    Hurt,
    WaveStart,
    Pulse,
    Dash,
    BossSpawn,
    Powerup,
    Spit,
}

// Tuning constants.
pub const PLAYER_RADIUS: f32 = 13.0;
pub const PLAYER_ACCEL: f32 = 1500.0;
pub const PLAYER_MAX_SPEED: f32 = 300.0;
pub const PLAYER_FRICTION: f32 = 3.2;
pub const BULLET_SPEED: f32 = 620.0;
pub const BULLET_LIFE: f32 = 1.1;
pub const FIRE_RATE: f32 = 0.135;
pub const DASH_SPEED: f32 = 900.0;
pub const DASH_DURATION: f32 = 0.16;
pub const DASH_COOLDOWN: f32 = 1.4;
pub const PULSE_COST: f32 = 100.0;
pub const PULSE_RADIUS: f32 = 190.0;

impl Game {
    pub fn new(seed: u32) -> Self {
        let mut g = Game {
            state: GameState::Title,
            player_pos: Vec2::new(WORLD_W * 0.5, WORLD_H * 0.5),
            player_vel: Vec2::new(0.0, 0.0),
            aim: Vec2::new(1.0, 0.0),
            health: 100.0,
            max_health: 100.0,
            energy: 0.0,
            max_energy: PULSE_COST,
            score: 0,
            best_score: 0,
            wave: 0,
            combo: 0,
            combo_timer: 0.0,
            enemies: Vec::new(),
            bullets: Vec::new(),
            spits: Vec::new(),
            orbs: Vec::new(),
            powerups: Vec::new(),
            particles: Vec::new(),
            fire_cooldown: 0.0,
            dash_cooldown: 0.0,
            dash_time: 0.0,
            invuln: 0.0,
            rapid_fire: 0.0,
            pickup_banner: 0.0,
            pickup_label: "",
            shake: 0.0,
            time: 0.0,
            wave_banner: 0.0,
            spawn_queue: 0,
            spawn_timer: 0.0,
            rng: Rng32::new(seed),
            events: Vec::new(),
            flash_color: 0,
            flash_amount: 0.0,
        };
        g.start_run();
        // A fresh game sits on the title screen; the player launches it.
        g.state = GameState::Title;
        g
    }

    pub fn start_run(&mut self) {
        self.state = GameState::Playing;
        self.player_pos = Vec2::new(WORLD_W * 0.5, WORLD_H * 0.5);
        self.player_vel = Vec2::new(0.0, 0.0);
        self.health = self.max_health;
        self.energy = 0.0;
        self.score = 0;
        self.wave = 0;
        self.combo = 0;
        self.combo_timer = 0.0;
        self.enemies.clear();
        self.bullets.clear();
        self.spits.clear();
        self.orbs.clear();
        self.powerups.clear();
        self.particles.clear();
        self.fire_cooldown = 0.0;
        self.dash_cooldown = 0.0;
        self.dash_time = 0.0;
        self.invuln = 1.2;
        self.rapid_fire = 0.0;
        self.pickup_banner = 0.0;
        self.shake = 0.0;
        self.time = 0.0;
        self.begin_wave();
    }

    pub fn is_boss_wave(&self) -> bool {
        self.wave.is_multiple_of(5)
    }

    fn begin_wave(&mut self) {
        self.wave += 1;
        self.spawn_timer = 0.0;
        self.wave_banner = 2.0;
        if self.is_boss_wave() {
            // Boss plus a light escort.
            self.spawn_queue = 2 + self.wave / 3;
            self.events.push(SoundEvent::BossSpawn);
            self.shake = 12.0;
            self.spawn_boss();
        } else {
            // Wave scaling: more enemies, tougher mix, but always survivable.
            let base = 3 + self.wave;
            self.spawn_queue = base.min(26);
            self.events.push(SoundEvent::WaveStart);
        }
        // Every 5th wave drops a small heal so runs can go long.
        if self.wave.is_multiple_of(5) {
            self.heal(25.0);
        }
    }

    fn spawn_boss(&mut self) {
        self.enemies.push(Enemy {
            pos: Vec2::new(WORLD_W * 0.5, -60.0),
            vel: Vec2::new(0.0, 0.0),
            kind: EnemyKind::Warden,
            // Bosses scale a little with depth.
            hp: 40 + (self.wave as i32 / 5) * 14,
            flash: 0.0,
            spawn_t: 1.0,
            seed: self.rng.next_u32(),
            atk_cd: 1.5,
        });
    }

    pub fn heal(&mut self, amount: f32) {
        self.health = (self.health + amount).min(self.max_health);
    }

    fn pick_kind(&mut self) -> EnemyKind {
        let r = self.rng.next_f32();
        if self.wave >= 6 && r < 0.18 {
            EnemyKind::Bloater
        } else if self.wave >= 3 && r < 0.55 {
            EnemyKind::Darter
        } else {
            EnemyKind::Drifter
        }
    }
    fn spawn_enemy(&mut self) {
        let kind = self.pick_kind();
        // Spawn just outside a random edge.
        let edge = (self.rng.next_f32() * 4.0) as u32;
        let (x, y) = match edge {
            0 => (self.rng.range(0.0, WORLD_W), -30.0),
            1 => (WORLD_W + 30.0, self.rng.range(0.0, WORLD_H)),
            2 => (self.rng.range(0.0, WORLD_W), WORLD_H + 30.0),
            _ => (-30.0, self.rng.range(0.0, WORLD_H)),
        };
        self.enemies.push(Enemy {
            pos: Vec2::new(x, y),
            vel: Vec2::new(0.0, 0.0),
            kind,
            hp: kind.hp(),
            flash: 0.0,
            spawn_t: 0.5,
            seed: self.rng.next_u32(),
            atk_cd: 1.5,
        });
    }

    pub fn fire(&mut self) {
        if self.state != GameState::Playing || self.fire_cooldown > 0.0 {
            return;
        }
        self.fire_cooldown = if self.rapid_fire > 0.0 {
            FIRE_RATE * 0.5
        } else {
            FIRE_RATE
        };
        let dir = self.aim.normalized(); // Slight spread so it feels alive but stays accurate.
        let ang = self.rng.range(-0.03, 0.03);
        let (s, c) = ang.sin_cos();
        let dx = dir.x * c - dir.y * s;
        let dy = dir.x * s + dir.y * c;
        self.bullets.push(Bullet {
            pos: Vec2::new(
                self.player_pos.x + dx * PLAYER_RADIUS,
                self.player_pos.y + dy * PLAYER_RADIUS,
            ),
            vel: Vec2::new(dx * BULLET_SPEED, dy * BULLET_SPEED),
            life: BULLET_LIFE,
        });
        // Muzzle particles.
        for _ in 0..3 {
            let a = ang + self.rng.range(-0.4, 0.4);
            self.spawn_particle(
                Vec2::new(
                    self.player_pos.x + dx * PLAYER_RADIUS,
                    self.player_pos.y + dy * PLAYER_RADIUS,
                ),
                Vec2::new(a.cos() * 140.0, a.sin() * 140.0),
                0x8FE9FF,
                0.18,
                2.0,
                ParticleKind::Spark,
            );
        }
        self.events.push(SoundEvent::Shoot);
    }

    pub fn dash(&mut self) {
        if self.state != GameState::Playing || self.dash_cooldown > 0.0 || self.dash_time > 0.0 {
            return;
        }
        self.dash_time = DASH_DURATION;
        self.dash_cooldown = DASH_COOLDOWN;
        let dir = if self.player_vel.len() > 20.0 {
            self.player_vel.normalized()
        } else {
            self.aim.normalized()
        };
        self.player_vel = Vec2::new(dir.x * DASH_SPEED, dir.y * DASH_SPEED);
        self.events.push(SoundEvent::Dash);
        for _ in 0..14 {
            let a = self.rng.range(0.0, std::f32::consts::TAU);
            self.spawn_particle(
                self.player_pos,
                Vec2::new(a.cos() * 180.0, a.sin() * 180.0),
                0x7BE0FF,
                0.4,
                2.5,
                ParticleKind::Spark,
            );
        }
    }

    /// Screen-clearing energy pulse. Requires full energy.
    pub fn pulse(&mut self) -> bool {
        if self.state != GameState::Playing || self.energy < self.max_energy {
            return false;
        }
        self.energy = 0.0;
        self.shake = 14.0;
        self.flash_color = 0x66D9FF;
        self.flash_amount = 0.55;
        self.events.push(SoundEvent::Pulse);
        self.particles.push(Particle {
            pos: self.player_pos,
            vel: Vec2::new(0.0, 0.0),
            life: 0.5,
            max_life: 0.5,
            color: 0x66D9FF,
            size: PULSE_RADIUS,
            kind: ParticleKind::Ring,
        });
        // Damage everything in radius, with falloff.
        let mut total_score = 0;
        let mut combo = 0;
        for e in self.enemies.iter_mut() {
            let d = (e.pos.x - self.player_pos.x).hypot(e.pos.y - self.player_pos.y);
            if d <= PULSE_RADIUS {
                let falloff = 1.0 - (d / PULSE_RADIUS) * 0.5;
                e.hp -= (4.0 * falloff).ceil() as i32;
                e.flash = 0.25;
                combo += 1;
                if e.hp <= 0 {
                    total_score += e.kind.score();
                }
            }
        }
        self.register_kills(total_score, combo);
        self.enemies.retain(|e| e.hp > 0);
        true
    }

    fn register_kills(&mut self, points: u32, combo_add: u32) {
        if points == 0 {
            return;
        }
        self.combo += combo_add;
        self.combo_timer = 2.5;
        let mult = self.combo_multiplier();
        self.score += points * mult;
    }

    pub fn combo_multiplier(&self) -> u32 {
        1 + (self.combo / 5)
    }

    fn spawn_particle(
        &mut self,
        pos: Vec2,
        vel: Vec2,
        color: u32,
        life: f32,
        size: f32,
        kind: ParticleKind,
    ) {
        self.particles.push(Particle {
            pos,
            vel,
            life,
            max_life: life,
            color,
            size,
            kind,
        });
    }

    fn burst(&mut self, pos: Vec2, color: u32, count: u32, speed: f32) {
        for _ in 0..count {
            let a = self.rng.range(0.0, std::f32::consts::TAU);
            let s = self.rng.range(speed * 0.4, speed);
            let life = self.rng.range(0.25, 0.6);
            let size = self.rng.range(1.5, 3.5);
            self.spawn_particle(
                pos,
                Vec2::new(a.cos() * s, a.sin() * s),
                color,
                life,
                size,
                ParticleKind::Spark,
            );
        }
    }

    pub fn update(&mut self, dt: f32, input: &Input) {
        if self.state == GameState::Title {
            self.time += dt;
            if input.any_start() {
                self.start_run();
            }
            return;
        }
        if self.state == GameState::GameOver {
            self.time += dt;
            self.shake = (self.shake - dt * 30.0).max(0.0);
            self.flash_amount = (self.flash_amount - dt * 2.0).max(0.0);
            self.update_particles(dt);
            if input.any_start() {
                self.best_score = self.best_score.max(self.score);
                self.start_run();
            }
            return;
        }

        self.time += dt;
        self.flash_amount = (self.flash_amount - dt * 2.5).max(0.0);
        self.wave_banner = (self.wave_banner - dt).max(0.0);
        self.pickup_banner = (self.pickup_banner - dt).max(0.0);
        self.rapid_fire = (self.rapid_fire - dt).max(0.0);
        self.fire_cooldown = (self.fire_cooldown - dt).max(0.0);
        self.dash_cooldown = (self.dash_cooldown - dt).max(0.0);
        self.dash_time = (self.dash_time - dt).max(0.0);
        self.invuln = (self.invuln - dt).max(0.0);
        self.shake = (self.shake - dt * 30.0).max(0.0);

        if self.combo_timer > 0.0 {
            self.combo_timer -= dt;
            if self.combo_timer <= 0.0 {
                self.combo = 0;
            }
        }

        // Aim.
        if input.aim_override.len() > 0.1 {
            self.aim = input.aim_override.normalized();
        } else if input.pointer.len() > 1.0 {
            self.aim = input.pointer.normalized();
        }

        // Movement (thrusty with friction).
        let mut accel = Vec2::new(0.0, 0.0);
        if input.left {
            accel.x -= 1.0;
        }
        if input.right {
            accel.x += 1.0;
        }
        if input.up {
            accel.y -= 1.0;
        }
        if input.down {
            accel.y += 1.0;
        }
        let moving = accel.len() > 0.0;
        if moving {
            let a = accel.normalized();
            let factor = if self.dash_time > 0.0 { 0.25 } else { 1.0 };
            self.player_vel.x += a.x * PLAYER_ACCEL * factor * dt;
            self.player_vel.y += a.y * PLAYER_ACCEL * factor * dt;
            // Thruster trail.
            if self.rng.next_f32() < 0.7 {
                let back = Vec2::new(-a.x, -a.y);
                self.spawn_particle(
                    Vec2::new(
                        self.player_pos.x + back.x * 12.0,
                        self.player_pos.y + back.y * 12.0,
                    ),
                    Vec2::new(back.x * 90.0, back.y * 90.0),
                    0xFFB24D,
                    0.3,
                    2.2,
                    ParticleKind::Thruster,
                );
            }
        }

        // Friction only when not dashing.
        if self.dash_time <= 0.0 {
            let f = (1.0 - PLAYER_FRICTION * dt).max(0.0);
            self.player_vel.x *= f;
            self.player_vel.y *= f;
        }

        // Clamp speed (dash exceeds the normal cap on purpose).
        let max_speed = if self.dash_time > 0.0 {
            DASH_SPEED
        } else {
            PLAYER_MAX_SPEED
        };
        let sp = self.player_vel.len();
        if sp > max_speed {
            let s = max_speed / sp;
            self.player_vel.x *= s;
            self.player_vel.y *= s;
        }

        self.player_pos.x += self.player_vel.x * dt;
        self.player_pos.y += self.player_vel.y * dt;

        // Bounce off the arena walls with damping.
        if self.player_pos.x < PLAYER_RADIUS {
            self.player_pos.x = PLAYER_RADIUS;
            self.player_vel.x = self.player_vel.x.abs() * 0.5;
        }
        if self.player_pos.x > WORLD_W - PLAYER_RADIUS {
            self.player_pos.x = WORLD_W - PLAYER_RADIUS;
            self.player_vel.x = -self.player_vel.x.abs() * 0.5;
        }
        if self.player_pos.y < PLAYER_RADIUS {
            self.player_pos.y = PLAYER_RADIUS;
            self.player_vel.y = self.player_vel.y.abs() * 0.5;
        }
        if self.player_pos.y > WORLD_H - PLAYER_RADIUS {
            self.player_pos.y = WORLD_H - PLAYER_RADIUS;
            self.player_vel.y = -self.player_vel.y.abs() * 0.5;
        }

        if input.fire {
            self.fire();
        }
        if input.pulse {
            self.pulse();
        }
        if input.dash {
            self.dash();
        }

        self.update_bullets(dt);
        self.update_enemies(dt);
        self.update_spits(dt);
        self.update_orbs(dt);
        self.update_powerups(dt);
        self.update_particles(dt);
        self.update_waves(dt);

        if self.health <= 0.0 {
            self.health = 0.0;
            self.state = GameState::GameOver;
            self.best_score = self.best_score.max(self.score);
            self.shake = 20.0;
            self.flash_color = 0xFF3B3B;
            self.flash_amount = 0.8;
            self.burst(self.player_pos, 0xFF6B6B, 40, 320.0);
        }
    }

    fn update_bullets(&mut self, dt: f32) {
        for b in self.bullets.iter_mut() {
            b.pos.x += b.vel.x * dt;
            b.pos.y += b.vel.y * dt;
            b.life -= dt;
        }
        // Bullet vs enemy.
        let mut hits: Vec<(usize, usize)> = Vec::new();
        for (bi, b) in self.bullets.iter().enumerate() {
            for (ei, e) in self.enemies.iter().enumerate() {
                let r = e.kind.radius();
                let dx = b.pos.x - e.pos.x;
                let dy = b.pos.y - e.pos.y;
                if dx * dx + dy * dy <= r * r {
                    hits.push((bi, ei));
                    break;
                }
            }
        }

        let mut dead_bullets: Vec<usize> = Vec::new();
        let mut score_gain = 0u32;
        let mut combo_add = 0u32;
        let mut spawn_children: Vec<(EnemyKind, Vec2)> = Vec::new();
        let mut explode_positions: Vec<(Vec2, u32, u32)> = Vec::new();

        for (bi, ei) in hits {
            if dead_bullets.contains(&bi) {
                continue;
            }
            dead_bullets.push(bi);
            if let Some(e) = self.enemies.get_mut(ei) {
                e.hp -= 1;
                e.flash = 0.12;
                // Knock the enemy back a touch.
                let b = &self.bullets[bi];
                let d = b.vel.normalized();
                e.vel.x += d.x * 60.0;
                e.vel.y += d.y * 60.0;
                let pos = e.pos;
                let kind = e.kind;
                if e.hp <= 0 {
                    score_gain += kind.score();
                    combo_add += 1;
                    explode_positions.push((pos, kind_color(kind), 10 + kind.radius() as u32));
                    if kind == EnemyKind::Bloater {
                        for _ in 0..2 {
                            let a = self.rng.range(0.0, std::f32::consts::TAU);
                            spawn_children.push((
                                EnemyKind::Drifter,
                                Vec2::new(pos.x + a.cos() * 22.0, pos.y + a.sin() * 22.0),
                            ));
                        }
                    }
                    if kind.is_boss() {
                        // Boss death: triple burst, guaranteed powerups.
                        for _ in 0..3 {
                            let a = self.rng.range(0.0, std::f32::consts::TAU);
                            explode_positions.push((
                                Vec2::new(pos.x + a.cos() * 26.0, pos.y + a.sin() * 26.0),
                                0xD7B3FF,
                                26,
                            ));
                        }
                        self.orbs.push(Orb {
                            pos,
                            vel: Vec2::new(0.0, -40.0),
                            life: 14.0,
                            anim: 0.0,
                        });
                        for k in [PowerKind::Repair, PowerKind::RapidFire, PowerKind::Charge] {
                            let a = self.rng.range(0.0, std::f32::consts::TAU);
                            self.powerups.push(Powerup {
                                pos,
                                vel: Vec2::new(a.cos() * 90.0, a.sin() * 90.0),
                                kind: k,
                                life: 18.0,
                                anim: 0.0,
                            });
                        }
                    }
                    // Drop an orb sometimes.
                    if self.rng.next_f32() < 0.55 {
                        let a = self.rng.range(0.0, std::f32::consts::TAU);
                        self.orbs.push(Orb {
                            pos,
                            vel: Vec2::new(a.cos() * 60.0, a.sin() * 60.0),
                            life: 12.0,
                            anim: self.rng.range(0.0, std::f32::consts::TAU),
                        });
                    }
                } else {
                    self.events.push(SoundEvent::Hit);
                }
            }
        }

        dead_bullets.sort_unstable();
        for &bi in dead_bullets.iter().rev() {
            if bi < self.bullets.len() {
                self.bullets.remove(bi);
            }
        }
        self.enemies.retain(|e| e.hp > 0);

        for (pos, color, count) in explode_positions {
            self.burst(pos, color, count, 260.0);
            self.shake = (self.shake + 3.0).min(16.0);
            self.events.push(SoundEvent::Explode);
        }
        for (kind, pos) in spawn_children {
            self.enemies.push(Enemy {
                pos,
                vel: Vec2::new(0.0, 0.0),
                kind,
                hp: kind.hp(),
                flash: 0.0,
                spawn_t: 0.3,
                seed: self.rng.next_u32(),
                atk_cd: 1.5,
            });
        }
        self.register_kills(score_gain, combo_add);

        // Expire bullets (off-world or out of life).
        self.bullets.retain(|b| {
            b.life > 0.0
                && b.pos.x > -20.0
                && b.pos.x < WORLD_W + 20.0
                && b.pos.y > -20.0
                && b.pos.y < WORLD_H + 20.0
        });
    }

    fn update_enemies(&mut self, dt: f32) {
        let player = self.player_pos;
        let mut damage = 0.0;
        let mut boss_shots: Vec<(Vec2, Vec2)> = Vec::new();
        for e in self.enemies.iter_mut() {
            e.flash = (e.flash - dt).max(0.0);
            e.atk_cd = (e.atk_cd - dt).max(0.0);
            if e.spawn_t > 0.0 {
                e.spawn_t = (e.spawn_t - dt).max(0.0);
            }
            // Homing with per-kind flavor.
            let dx = player.x - e.pos.x;
            let dy = player.y - e.pos.y;
            let d = (dx * dx + dy * dy).sqrt().max(1e-3);
            let dir = Vec2::new(dx / d, dy / d);
            let speed = match e.kind {
                EnemyKind::Drifter => e.kind.speed(),
                EnemyKind::Darter => {
                    // Weave: sidestep sinusoidally.
                    let t = self.time * 6.0 + e.seed as f32 * 0.01;
                    let perp = Vec2::new(-dir.y, dir.x);
                    let weave = t.sin() * 0.9;
                    // Fold weave into velocity.
                    e.vel.x += perp.x * weave * 240.0 * dt;
                    e.vel.y += perp.y * weave * 240.0 * dt;
                    e.kind.speed()
                }
                EnemyKind::Bloater => e.kind.speed(),
                EnemyKind::Warden => {
                    // Hover at range, strafe, and lob spit.
                    let desired = 210.0;
                    let approach: f32 = if d > desired + 40.0 {
                        1.0
                    } else if d < desired - 60.0 {
                        -1.0
                    } else {
                        0.0
                    };
                    if e.atk_cd <= 0.0 && e.spawn_t <= 0.0 {
                        e.atk_cd = 1.8;
                        let spread: f32 = 0.22;
                        boss_shots.push((e.pos, dir));
                        let (s1, c1) = spread.sin_cos();
                        boss_shots.push((
                            e.pos,
                            Vec2::new(dir.x * c1 - dir.y * s1, dir.x * s1 + dir.y * c1),
                        ));
                        let (s2, c2) = (-spread).sin_cos();
                        boss_shots.push((
                            e.pos,
                            Vec2::new(dir.x * c2 - dir.y * s2, dir.x * s2 + dir.y * c2),
                        ));
                    }
                    if approach >= 0.0 {
                        approach * 55.0
                    } else {
                        -approach.abs() * 55.0
                    }
                }
            };
            e.vel.x += dir.x * speed * 5.0 * dt;
            e.vel.y += dir.y * speed * 5.0 * dt;
            // Clamp to target speed.
            let sp = e.vel.len();
            if sp > speed {
                let s = speed / sp;
                e.vel.x *= s;
                e.vel.y *= s;
            }
            e.pos.x += e.vel.x * dt;
            e.pos.y += e.vel.y * dt;

            // Contact damage.
            let r = e.kind.radius() + PLAYER_RADIUS;
            if d <= r && e.spawn_t <= 0.0 {
                damage += match e.kind {
                    EnemyKind::Bloater => 26.0,
                    EnemyKind::Darter => 12.0,
                    EnemyKind::Drifter => 14.0,
                    EnemyKind::Warden => 34.0,
                };
                // Shove enemy back so it doesn't tick every frame.
                e.pos.x -= dir.x * (r - d + 6.0);
                e.pos.y -= dir.y * (r - d + 6.0);
                e.vel.x = -dir.x * 120.0;
                e.vel.y = -dir.y * 120.0;
            }
        }

        if damage > 0.0 && self.invuln <= 0.0 {
            self.health -= damage;
            self.invuln = 0.6;
            self.shake = (self.shake + 8.0).min(20.0);
            self.combo = 0;
            self.flash_color = 0xFF3B3B;
            self.flash_amount = 0.4;
            self.events.push(SoundEvent::Hurt);
            let p = self.player_pos;
            self.burst(p, 0xFF8080, 12, 180.0);
        }

        for (pos, dir) in boss_shots {
            let d = dir.normalized();
            self.spits.push(Spit {
                pos: Vec2::new(pos.x + d.x * 44.0, pos.y + d.y * 44.0),
                vel: Vec2::new(d.x * 210.0, d.y * 210.0),
                life: 4.0,
            });
            self.events.push(SoundEvent::Spit);
        }
    }

    fn update_spits(&mut self, dt: f32) {
        let player = self.player_pos;
        let mut total = 0.0;
        for s in self.spits.iter_mut() {
            s.life -= dt;
            s.pos.x += s.vel.x * dt;
            s.pos.y += s.vel.y * dt;
            // Gentle homing so they threaten but can be outrun.
            let dx = player.x - s.pos.x;
            let dy = player.y - s.pos.y;
            let d = (dx * dx + dy * dy).sqrt().max(1e-3);
            let sp = s.vel.len().max(1e-3);
            s.vel.x += (dx / d) * 55.0 * dt * sp / 210.0;
            s.vel.y += (dy / d) * 55.0 * dt * sp / 210.0;
            if d < PLAYER_RADIUS + 7.0 {
                total += 9.0;
                s.life = -1.0;
            }
        }
        self.spits.retain(|s| {
            s.life > 0.0
                && s.pos.x > -40.0
                && s.pos.x < WORLD_W + 40.0
                && s.pos.y > -40.0
                && s.pos.y < WORLD_H + 40.0
        });

        if total > 0.0 && self.invuln <= 0.0 {
            self.health -= total;
            self.invuln = 0.5;
            self.shake = (self.shake + 6.0).min(20.0);
            self.combo = 0;
            self.flash_color = 0xC77BFF;
            self.flash_amount = 0.35;
            self.events.push(SoundEvent::Hurt);
        }
    }

    fn update_powerups(&mut self, dt: f32) {
        let player = self.player_pos;
        let mut taken: Vec<PowerKind> = Vec::new();
        for p in self.powerups.iter_mut() {
            p.life -= dt;
            p.anim += dt * 5.0;
            p.pos.x += p.vel.x * dt;
            p.pos.y += p.vel.y * dt;
            let f = (1.0 - 2.0 * dt).max(0.0);
            p.vel.x *= f;
            p.vel.y *= f;
            let dx = player.x - p.pos.x;
            let dy = player.y - p.pos.y;
            let d = (dx * dx + dy * dy).sqrt().max(1e-3);
            if d < PLAYER_RADIUS + 15.0 {
                taken.push(p.kind);
                p.life = -1.0;
            }
        }
        for kind in taken {
            self.pickup_banner = 1.8;
            self.pickup_label = kind.label();
            match kind {
                PowerKind::Repair => {
                    self.heal(35.0);
                    self.burst(self.player_pos, 0x7CFF9B, 20, 200.0);
                }
                PowerKind::RapidFire => {
                    self.rapid_fire = 8.0;
                    self.burst(self.player_pos, 0xFF9B7C, 20, 200.0);
                }
                PowerKind::Charge => {
                    self.energy = self.max_energy;
                    self.burst(self.player_pos, 0x8FD0FF, 20, 200.0);
                }
            }
            self.events.push(SoundEvent::Powerup);
        }
        self.powerups.retain(|p| p.life > 0.0);
    }

    fn update_orbs(&mut self, dt: f32) {
        let player = self.player_pos;
        let mut collected = 0;
        for o in self.orbs.iter_mut() {
            o.life -= dt;
            o.anim += dt * 4.0;
            o.pos.x += o.vel.x * dt;
            o.pos.y += o.vel.y * dt;
            let f = (1.0 - 2.5 * dt).max(0.0);
            o.vel.x *= f;
            o.vel.y *= f;
            // Magnet toward player when close.
            let dx = player.x - o.pos.x;
            let dy = player.y - o.pos.y;
            let d = (dx * dx + dy * dy).sqrt().max(1e-3);
            if d < 130.0 {
                o.vel.x += (dx / d) * 620.0 * dt;
                o.vel.y += (dy / d) * 620.0 * dt;
            }
            if d < PLAYER_RADIUS + 12.0 {
                collected += 1;
                o.life = -1.0;
            }
        }
        if collected > 0 {
            self.energy = (self.energy + 20.0 * collected as f32).min(self.max_energy);
            self.score += 5 * collected * self.combo_multiplier();
            self.events.push(SoundEvent::Orb);
            for _ in 0..collected {
                let p = self.player_pos;
                self.burst(p, 0xFFE066, 8, 160.0);
            }
        }
        self.orbs.retain(|o| o.life > 0.0);
    }

    fn update_particles(&mut self, dt: f32) {
        for p in self.particles.iter_mut() {
            p.life -= dt;
            p.pos.x += p.vel.x * dt;
            p.pos.y += p.vel.y * dt;
            let f = (1.0 - 3.0 * dt).max(0.0);
            p.vel.x *= f;
            p.vel.y *= f;
        }
        self.particles.retain(|p| p.life > 0.0);
    }

    pub fn update_waves(&mut self, dt: f32) {
        if self.spawn_queue > 0 {
            self.spawn_timer -= dt;
            if self.spawn_timer <= 0.0 {
                self.spawn_enemy();
                self.spawn_queue -= 1;
                // Spawn faster in later waves.
                let gap = (0.55 - self.wave as f32 * 0.02).max(0.12);
                self.spawn_timer = gap;
            }
            return;
        }
        if self.wave_banner > 0.0 {
            return;
        }
        let boss_alive = self.enemies.iter().any(|e| e.kind.is_boss());
        if self.is_boss_wave() {
            // Boss waves only end when the boss dies.
            if !boss_alive {
                self.begin_wave();
            }
        } else if self.enemies.is_empty() {
            // Small breather then next wave.
            self.begin_wave();
        }
    }

    /// Enemies that are still spawning/animating in.
    pub fn enemies_alive(&self) -> usize {
        self.enemies.len() + self.spawn_queue as usize
    }
}

pub fn kind_color(kind: EnemyKind) -> u32 {
    match kind {
        EnemyKind::Drifter => 0xFF6FA8,
        EnemyKind::Darter => 0xFFC24D,
        EnemyKind::Bloater => 0x9B7BFF,
        EnemyKind::Warden => 0xD7B3FF,
    }
}

/// Input snapshot for one frame, produced by the binary from the window.
#[derive(Clone, Debug, Default)]
pub struct Input {
    pub up: bool,
    pub down: bool,
    pub left: bool,
    pub right: bool,
    pub fire: bool,
    pub pulse: bool,
    pub dash: bool,
    /// Pointer position relative to the player, in world units.
    pub pointer: Vec2,
    /// Non-zero when an arrow-key/gamepad style aim overrides the pointer.
    pub aim_override: Vec2,
    /// A key that starts / restarts the game is pressed.
    pub start: bool,
}

impl Input {
    pub fn any_start(&self) -> bool {
        self.start
    }
}

impl Default for Vec2 {
    fn default() -> Self {
        Vec2::new(0.0, 0.0)
    }
}
