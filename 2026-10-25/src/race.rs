//! One race: cars, laps, positions, collisions, slipstream, pickups,
//! hazards, style chains, ghosts and particles. Deterministic for a given
//! seed and dt sequence.

use crate::ai::{self, AiState};
use crate::car::{CAR_RAD, Car, CarEvent, Controls, Stats};
use crate::math::{V2, v2, wrap_angle};
use crate::rng::Rng;
use crate::track::{Biome, HALF_W, PickupKind, Surface, Track, WALL};

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Player,
    Rocket,
    Drifter,
    Bully,
    Blocker,
    Wasp,
    Ace,
    Champion,
}

impl Kind {
    pub fn label(self) -> &'static str {
        match self {
            Kind::Player => "You",
            Kind::Rocket => "Rocket",
            Kind::Drifter => "Drifter",
            Kind::Bully => "Bully",
            Kind::Blocker => "Blocker",
            Kind::Wasp => "Wasp",
            Kind::Ace => "Ace",
            Kind::Champion => "Champion",
        }
    }
    pub fn blurb(self) -> &'static str {
        match self {
            Kind::Player => "",
            Kind::Rocket => "Huge top speed, clumsy in corners. Draft it on the straights.",
            Kind::Drifter => "Drifts every bend and fires mini-turbos out of them.",
            Kind::Bully => "Heavy truck. Flashes red, then rams you into the wall.",
            Kind::Blocker => "Watches the mirror and covers your line.",
            Kind::Wasp => "Quick bike off the line, light enough to bump aside.",
            Kind::Ace => "Clean, consistent and quick everywhere.",
            Kind::Champion => "The reigning champion. Perfect lines, smart nitro, defends.",
        }
    }
    pub fn is_bike(self) -> bool {
        self == Kind::Wasp
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Driver {
    pub name: &'static str,
    pub kind: Kind,
    pub sprite: &'static str,
    pub color: u32,
}

pub const PLAYER: Driver = Driver {
    name: "You",
    kind: Kind::Player,
    sprite: "car_red_1",
    color: 0xF06A1E,
};

pub const ROSTER: [Driver; 6] = [
    Driver {
        name: "Blaze",
        kind: Kind::Rocket,
        sprite: "car_blue_1",
        color: 0x2F8FD8,
    },
    Driver {
        name: "Moss",
        kind: Kind::Drifter,
        sprite: "car_green_3",
        color: 0x37B866,
    },
    Driver {
        name: "Brick",
        kind: Kind::Bully,
        sprite: "car_black_4",
        color: 0x55585E,
    },
    Driver {
        name: "Penny",
        kind: Kind::Blocker,
        sprite: "car_yellow_2",
        color: 0xF2C318,
    },
    Driver {
        name: "Zip",
        kind: Kind::Wasp,
        sprite: "motorcycle_green",
        color: 0x3FCB74,
    },
    Driver {
        name: "Juno",
        kind: Kind::Ace,
        sprite: "car_blue_5",
        color: 0x4AA3E8,
    },
];

pub const CHAMPION: Driver = Driver {
    name: "Sable",
    kind: Kind::Champion,
    sprite: "car_black_5",
    color: 0xE8C34A,
};

/// Base stats for a rival kind at a given skill (0.8 .. 1.1).
pub fn rival_stats(kind: Kind, skill: f32) -> Stats {
    let mut s = Stats::default();
    match kind {
        Kind::Rocket => {
            s.top *= 1.07;
            s.accel *= 1.05;
            s.grip *= 0.86;
            s.steer *= 0.92;
        }
        Kind::Drifter => {
            s.top *= 0.98;
            s.grip *= 1.04;
            s.drift_rate *= 1.3;
        }
        Kind::Bully => {
            s.top *= 0.98;
            s.mass = 1.7;
            s.accel *= 0.92;
            s.fragility = 0.5;
        }
        Kind::Blocker => {
            s.top *= 0.99;
        }
        Kind::Wasp => {
            s.accel *= 1.25;
            s.steer *= 1.12;
            s.mass = 0.6;
            s.fragility = 1.4;
            s.top *= 0.99;
        }
        Kind::Ace => {
            s.grip *= 1.03;
        }
        Kind::Champion => {
            s.top *= 1.05;
            s.accel *= 1.06;
            s.grip *= 1.06;
            s.mass = 1.2;
        }
        Kind::Player => {}
    }
    s.top *= skill;
    s.accel *= 0.9 + 0.1 * skill;
    s
}

#[derive(Clone, Debug)]
pub struct Racer {
    pub driver: Driver,
    pub car: Car,
    pub ai: AiState,
    pub idx: usize,
    pub s: f32,
    pub lat: f32,
    /// Unwrapped distance raced (negative on the grid).
    pub progress: f32,
    pub lap: i32,
    pub lap_start: f32,
    pub lap_times: Vec<f32>,
    pub finish: Option<f32>,
    pub controls: Controls,
    pub place: usize,
    pub wall_cd: f32,
    pub bump_cd: f32,
    /// Who we were drafting most recently and when (for slingshots).
    pub drafted: Option<(usize, f32)>,
    pub stalled: f32,
    pub vmax: Vec<f32>,
    pub last_skid: [Option<V2>; 2],
}

impl Racer {
    pub fn is_player(&self) -> bool {
        self.driver.kind == Kind::Player
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PKind {
    Smoke,
    Dust,
    Spark,
    Flame,
    Confetti,
    Wind,
    Star,
}

#[derive(Clone, Copy, Debug)]
pub struct Particle {
    pub pos: V2,
    pub vel: V2,
    pub life: f32,
    pub max: f32,
    pub size: f32,
    pub color: u32,
    pub kind: PKind,
    pub rot: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Cone {
    pub pos: V2,
    pub vel: V2,
    pub rot: f32,
    pub spin: f32,
    pub down: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct PickupState {
    pub pos: V2,
    pub kind: PickupKind,
    pub respawn: f32,
}

#[derive(Clone, Debug)]
pub struct Popup {
    pub text: String,
    pub pos: V2,
    pub t: f32,
    pub color: u32,
    pub big: bool,
}

/// Sound effects the race asks for.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Sfx {
    CrashHeavy,
    CrashMedium,
    Bump,
    Ram,
    Cone,
    Pickup,
    Repair,
    Turbo(u8),
    TierUp(u8),
    NitroStart,
    Pad,
    Shift,
    Spin,
    CountBeep,
    GoBeep,
    Lap,
    FinalLap,
    Finish,
    Style,
    Slingshot,
    BullyWarn,
    Stall,
    PerfectStart,
    Overtake,
}

#[derive(Clone, Copy, Debug)]
pub struct SoundEvent {
    pub sfx: Sfx,
    /// 0..1 loudness from distance to the camera.
    pub vol: f32,
    pub pan: f32,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Mode {
    Championship,
    TimeTrial,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Countdown,
    Running,
    /// The player has finished; the field is completing the lap.
    Cooldown,
    Done,
}

#[derive(Clone, Debug, Default)]
pub struct Style {
    pub chain: u32,
    pub chain_pts: u32,
    pub timer: f32,
    pub total: u32,
    pub best_chain: u32,
    pub max_mult: u32,
    pub label: String,
    pub label_t: f32,
    pub drifts: u32,
    pub slingshots: u32,
    pub overtakes: u32,
    pub near_misses: u32,
    pub cones: u32,
}

impl Style {
    pub fn mult(&self) -> u32 {
        (1 + self.chain.saturating_sub(1)).min(self.max_mult.max(1))
    }
}

/// Recorded car pose for ghosts.
pub type GhostFrame = (f32, f32, f32);
pub const GHOST_HZ: f32 = 20.0;

#[derive(Clone, Copy, Debug, Default)]
pub struct Assists {
    pub steer: bool,
    pub auto_throttle: bool,
}

pub struct Race {
    pub track: Track,
    pub mode: Mode,
    pub racers: Vec<Racer>,
    pub laps: i32,
    pub phase: Phase,
    pub countdown: f32,
    pub t: f32,
    pub cooldown: f32,
    pub cones: Vec<Cone>,
    pub pickups: Vec<PickupState>,
    pub particles: Vec<Particle>,
    pub skids: Vec<(V2, V2, f32, u32)>,
    pub sounds: Vec<SoundEvent>,
    pub popups: Vec<Popup>,
    pub shake: f32,
    pub style: Style,
    pub rng: Rng,
    pub ghost: Option<Vec<GhostFrame>>,
    pub ghost_rec: Vec<GhostFrame>,
    pub ghost_acc: f32,
    pub best_lap: Option<f32>,
    pub best_ghost: Option<Vec<GhostFrame>>,
    pub assists: Assists,
    pub autopilot: bool,
    pub player_launch: f32,
    pub early_throttle: bool,
    pub last_beep: i32,
    pub hint_flags: u32,
    pub big_hits: u32,
    pub player_wall_hits: u32,
    pub flash: f32,
    pub final_lap_announced: bool,
    pub prev_place: usize,
    pub stats_time_drafting: f32,
    pub stats_turbos: u32,
    pub stats_nitro_secs: f32,
    /// Rivals' per-race pace multiplier (difficulty).
    pub skill: f32,
    /// Smoothed keyboard steering.
    pub steer_s: f32,
}

pub const STEP_DT: f32 = 1.0 / 120.0;

impl Race {
    /// `player_stats` are the garage+parts stats; `rivals` the field.
    pub fn new(
        track: Track,
        mode: Mode,
        laps: i32,
        player_stats: Stats,
        rivals: &[Driver],
        skill: f32,
        seed: u64,
    ) -> Race {
        let mut rng = Rng::new(seed);
        let mut racers = Vec::new();
        let mut grid: Vec<Driver> = rivals.to_vec();
        // Player starts at the back of the grid in a championship (more
        // overtaking), alone in a time trial.
        let n = grid.len();
        rng.shuffle(&mut grid);
        let mut order: Vec<Driver> = grid;
        if mode == Mode::Championship {
            order.push(PLAYER);
        } else {
            order = vec![PLAYER];
        }
        for (slot, d) in order.iter().enumerate() {
            let (pos, ang) = track.grid_pos(slot);
            let stats = if d.kind == Kind::Player {
                player_stats
            } else {
                let jitter = rng.range(-0.012, 0.012);
                rival_stats(d.kind, skill + jitter)
            };
            let mut car = Car::new(pos, ang, stats);
            car.nitro = 0.2;
            let loc = track.locate(pos, None);
            let progress = loc.s - track.len;
            let vmax = ai::speed_profile(&track, &stats, d.kind);
            racers.push(Racer {
                driver: *d,
                car,
                ai: AiState::new(d.kind, &mut rng),
                idx: loc.i,
                s: loc.s,
                lat: loc.lat,
                progress,
                lap: 0,
                lap_start: 0.0,
                lap_times: Vec::new(),
                finish: None,
                controls: Controls::default(),
                place: slot + 1,
                wall_cd: 0.0,
                bump_cd: 0.0,
                drafted: None,
                stalled: 0.0,
                vmax,
                last_skid: [None, None],
            });
        }
        let _ = n;
        let cones = track
            .cones
            .iter()
            .map(|p| Cone {
                pos: *p,
                vel: V2::default(),
                rot: 0.0,
                spin: 0.0,
                down: false,
            })
            .collect();
        let pickups = track
            .pickups
            .iter()
            .map(|p| PickupState {
                pos: p.pos,
                kind: p.kind,
                respawn: 0.0,
            })
            .collect();
        let mut race = Race {
            track,
            mode,
            racers,
            laps,
            phase: Phase::Countdown,
            countdown: 3.4,
            t: 0.0,
            cooldown: 0.0,
            cones,
            pickups,
            particles: Vec::new(),
            skids: Vec::new(),
            sounds: Vec::new(),
            popups: Vec::new(),
            shake: 0.0,
            style: Style {
                max_mult: player_stats.style_cap,
                ..Style::default()
            },
            rng,
            ghost: None,
            ghost_rec: Vec::new(),
            ghost_acc: 0.0,
            best_lap: None,
            best_ghost: None,
            assists: Assists::default(),
            autopilot: false,
            player_launch: 0.0,
            early_throttle: false,
            last_beep: 4,
            hint_flags: 0,
            big_hits: 0,
            player_wall_hits: 0,
            flash: 0.0,
            final_lap_announced: false,
            prev_place: 0,
            stats_time_drafting: 0.0,
            stats_turbos: 0,
            stats_nitro_secs: 0.0,
            skill,
            steer_s: 0.0,
        };
        race.update_places();
        race.prev_place = race.player().place;
        race
    }

    pub fn player_idx(&self) -> usize {
        self.racers.iter().position(|r| r.is_player()).unwrap_or(0)
    }
    pub fn player(&self) -> &Racer {
        &self.racers[self.player_idx()]
    }

    pub fn lap_of(&self, r: &Racer) -> i32 {
        ((r.progress / self.track.len).floor() as i32 + 1).clamp(1, self.laps)
    }

    fn sound(&mut self, sfx: Sfx, at: Option<V2>) {
        let (vol, pan) = if let Some(p) = at {
            let cam = self.player().car.pos;
            let d = p - cam;
            (
                (1.0 - d.len() / 1100.0).clamp(0.0, 1.0),
                (d.x / 700.0).clamp(-1.0, 1.0),
            )
        } else {
            (1.0, 0.0)
        };
        if vol > 0.02 {
            self.sounds.push(SoundEvent { sfx, vol, pan });
        }
    }

    fn popup(&mut self, text: impl Into<String>, pos: V2, color: u32, big: bool) {
        // stack fresh callouts instead of printing them on top of each other
        let busy = self
            .popups
            .iter()
            .filter(|p| p.t < 0.7 && p.pos.dist(pos) < 160.0)
            .map(|p| if p.big { 2.0 } else { 1.0 })
            .sum::<f32>();
        let pos = pos - v2(0.0, busy * 26.0);
        self.popups.push(Popup {
            text: text.into(),
            pos,
            t: 0.0,
            color,
            big,
        });
    }

    /// Advance by a frame of `dt` seconds, split into fixed-size substeps.
    pub fn update(&mut self, dt: f32, player: Controls) {
        let n = (dt / STEP_DT).ceil().max(1.0) as usize;
        let h = dt / n as f32;
        for _ in 0..n {
            self.step(h, player);
        }
        // visuals at frame rate
        for p in self.particles.iter_mut() {
            p.pos += p.vel * dt;
            p.vel = p.vel * (1.0 - 1.8 * dt).max(0.0);
            p.life -= dt;
            p.rot += dt * 3.0;
        }
        self.particles.retain(|p| p.life > 0.0);
        if self.particles.len() > 1800 {
            let k = self.particles.len() - 1800;
            self.particles.drain(0..k);
        }
        for p in self.popups.iter_mut() {
            p.t += dt;
        }
        self.popups.retain(|p| p.t < if p.big { 1.6 } else { 1.1 });
        self.shake = (self.shake - dt * 2.4).max(0.0);
        self.flash = (self.flash - dt * 2.5).max(0.0);
        self.style.label_t = (self.style.label_t - dt).max(0.0);
    }

    fn step(&mut self, dt: f32, player_in: Controls) {
        match self.phase {
            Phase::Countdown => {
                self.countdown -= dt;
                let beep = self.countdown.ceil() as i32;
                if beep < self.last_beep && (1..=3).contains(&beep) {
                    self.last_beep = beep;
                    self.sound(Sfx::CountBeep, None);
                }
                // rocket start: throttle held too early stalls, a press in the
                // last 0.35s before green launches
                let pi = self.player_idx();
                let wants =
                    player_in.throttle > 0.5 || self.assists.auto_throttle || self.autopilot;
                if wants && self.countdown > 1.0 && !self.assists.auto_throttle && !self.autopilot {
                    self.early_throttle = true;
                }
                if !wants {
                    self.early_throttle = false;
                    self.player_launch = 0.0;
                } else if self.countdown
                    <= if self.racers[pi].car.stats.launch {
                        0.7
                    } else {
                        0.35
                    }
                {
                    self.player_launch += dt;
                }
                for r in self.racers.iter_mut() {
                    r.car.rpm = if r.idx == usize::MAX {
                        0.0
                    } else {
                        (r.car.rpm + dt * 2.0).min(0.6)
                    };
                }
                if self.countdown <= 0.0 {
                    self.phase = Phase::Running;
                    self.sound(Sfx::GoBeep, None);
                    let early = self.early_throttle;
                    let launched = self.player_launch > 0.0 && !early;
                    let pos = self.racers[pi].car.pos;
                    if early {
                        self.racers[pi].stalled = 0.7;
                        self.sound(Sfx::Stall, None);
                        self.popup("STALLED!", pos, 0xFF6B5B, true);
                    } else if launched {
                        let r = &mut self.racers[pi];
                        r.car.boost_t = if r.car.stats.launch { 1.9 } else { 1.1 };
                        r.car.boost_k = 1.0;
                        self.sound(Sfx::PerfectStart, None);
                        self.popup("ROCKET START!", pos, 0x7FE3FF, true);
                        self.add_style(150, "ROCKET START", pos);
                    }
                    // rivals get varied launches
                    for i in 0..self.racers.len() {
                        if i == pi {
                            continue;
                        }
                        let roll = self.rng.f();
                        let r = &mut self.racers[i];
                        if roll < 0.25 * self.skill {
                            r.car.boost_t = 0.8;
                            r.car.boost_k = 0.9;
                        } else if roll > 0.92 {
                            r.stalled = 0.4;
                        }
                    }
                }
                return;
            }
            Phase::Done => return,
            _ => {}
        }
        self.t += dt;
        let pi = self.player_idx();
        let n = self.racers.len();

        for i in 0..n {
            if ai::think(self, i, dt) {
                let p = self.racers[i].car.pos;
                self.sound(Sfx::BullyWarn, Some(p));
            }
        }

        // --- controls
        let mut controls = vec![Controls::default(); n];
        for i in 0..n {
            let finished = self.racers[i].finish.is_some();
            controls[i] = if i == pi && !self.autopilot && !finished {
                let mut c = player_in;
                // keyboard steering ramps in over ~0.1s and recentres faster
                let rate = if c.steer == 0.0 || c.steer.signum() != self.steer_s.signum() {
                    14.0
                } else {
                    9.0
                };
                self.steer_s = crate::math::approach(self.steer_s, c.steer, rate * dt);
                c.steer = self.steer_s;
                if self.assists.auto_throttle && c.throttle >= 0.0 {
                    c.throttle = 1.0;
                }
                if self.assists.steer {
                    let a = ai::drive(self, i, dt);
                    c.steer = (c.steer * 0.55 + a.steer * 0.45).clamp(-1.0, 1.0);
                    // brake for the player only when the car is badly over
                    if a.throttle < -0.5 && c.throttle > 0.0 {
                        c.throttle = 0.2;
                    }
                }
                c
            } else {
                ai::drive(self, i, dt)
            };
        }
        // --- physics
        let road_grip = self.track.spec.biome.road_grip();
        let mut car_events: Vec<(usize, CarEvent)> = Vec::new();
        for i in 0..n {
            let r = &mut self.racers[i];
            let mut c = controls[i];
            if r.stalled > 0.0 {
                r.stalled -= dt;
                c.throttle = 0.0;
            }
            r.controls = c;
            let mut ev = Vec::new();
            r.car.step(c, road_grip, dt, &mut ev);
            for e in ev {
                car_events.push((i, e));
            }
            let loc = self.track.locate(r.car.pos, Some(r.idx));
            let mut ds = loc.s - r.s;
            if ds > self.track.len / 2.0 {
                ds -= self.track.len;
            } else if ds < -self.track.len / 2.0 {
                ds += self.track.len;
            }
            r.progress += ds;
            r.idx = loc.i;
            r.s = loc.s;
            r.lat = loc.lat;
            r.car.surface = self.track.surface(r.car.pos, &loc);
            if r.car.stats.kerb_nitro && r.car.surface == Surface::Kerb && r.car.speed() > 300.0 {
                r.car.nitro = (r.car.nitro + 0.12 * dt).min(1.0);
            }
            r.wall_cd = (r.wall_cd - dt).max(0.0);
            r.bump_cd = (r.bump_cd - dt).max(0.0);
        }
        for (i, e) in car_events {
            self.on_car_event(i, e);
        }

        self.collide_walls();
        self.collide_cars();
        self.slipstream(dt);
        self.touch_objects(dt);
        self.laps_and_places();
        self.effects(dt);
        self.style_tick(dt);
        self.record_ghost(dt);

        if self.phase == Phase::Cooldown {
            self.cooldown += dt;
            let all = self.racers.iter().all(|r| r.finish.is_some());
            if all || self.cooldown > 9.0 {
                self.finish_field();
                self.phase = Phase::Done;
            }
        }
    }

    fn on_car_event(&mut self, i: usize, e: CarEvent) {
        let is_p = self.racers[i].is_player();
        let pos = self.racers[i].car.pos;
        match e {
            CarEvent::Turbo(t) => {
                self.sound(Sfx::Turbo(t), Some(pos));
                if is_p {
                    self.stats_turbos += 1;
                    self.style.drifts += 1;
                    let pts = [0, 60, 150, 300][t as usize];
                    let name = ["", "DRIFT", "SUPER DRIFT", "ULTRA DRIFT"][t as usize];
                    self.add_style(pts, name, pos);
                    self.shake = self.shake.max(0.12 * t as f32);
                }
            }
            CarEvent::TierUp(t) => {
                if is_p {
                    self.sound(Sfx::TierUp(t), None);
                }
            }
            CarEvent::Shift => {
                if is_p {
                    self.sound(Sfx::Shift, None);
                }
            }
            CarEvent::OilSpin => {
                self.sound(Sfx::Spin, Some(pos));
                if is_p {
                    self.popup("OIL!", pos, 0x404048, false);
                    self.hint_flags |= 1 << 3;
                }
                for _ in 0..14 {
                    let a = self.rng.range(0.0, std::f32::consts::TAU);
                    self.particles.push(Particle {
                        pos,
                        vel: V2::from_angle(a) * self.rng.range(20.0, 90.0),
                        life: 0.9,
                        max: 0.9,
                        size: 11.0,
                        color: 0x3A3A44,
                        kind: PKind::Smoke,
                        rot: 0.0,
                    });
                }
            }
        }
    }

    fn collide_walls(&mut self) {
        let n = self.racers.len();
        for i in 0..n {
            let (circles, hint) = {
                let r = &self.racers[i];
                (r.car.circles(), r.idx)
            };
            for c in circles {
                let loc = self.track.locate(c, Some(hint));
                let lim = WALL - CAR_RAD + 2.0;
                if loc.lat.abs() > lim {
                    let nrm_out = self.track.nor[loc.i] * loc.lat.signum();
                    let pen = loc.lat.abs() - lim;
                    let r = &mut self.racers[i];
                    r.car.pos -= nrm_out * pen;
                    let vn = r.car.vel.dot(nrm_out);
                    if vn > 0.0 {
                        // reflect softly, scrape along
                        let vt = r.car.vel - nrm_out * vn;
                        r.car.vel = vt * 0.9 - nrm_out * vn * 0.3;
                        let dmg = r.car.impact(vn, 1.0);
                        // knock the nose away from the wall
                        let away = wrap_angle(r.car.vel.angle() - r.car.heading);
                        r.car.heading = wrap_angle(r.car.heading + away * 0.3);
                        if r.wall_cd <= 0.0 && vn > 70.0 {
                            r.wall_cd = 0.25;
                            let heavy = vn > 330.0;
                            let is_p = r.is_player();
                            let pos = c;
                            if is_p {
                                self.player_wall_hits += 1;
                                self.shake = self.shake.max(if heavy { 0.55 } else { 0.22 });
                                if heavy {
                                    self.big_hits += 1;
                                    self.flash = 0.35;
                                }
                                self.break_chain();
                            }
                            self.sound(
                                if heavy {
                                    Sfx::CrashHeavy
                                } else {
                                    Sfx::CrashMedium
                                },
                                Some(pos),
                            );
                            self.sparks(pos, nrm_out * -1.0, if heavy { 18 } else { 8 });
                            let _ = dmg;
                        }
                    }
                }
            }
        }
    }

    fn collide_cars(&mut self) {
        let n = self.racers.len();
        for i in 0..n {
            for j in (i + 1)..n {
                let (ci, cj) = (self.racers[i].car.circles(), self.racers[j].car.circles());
                let mut hit: Option<(V2, f32, V2)> = None;
                for a in ci {
                    for b in cj {
                        let d = b - a;
                        let dl = d.len();
                        if dl < CAR_RAD * 2.0 && dl > 1e-3 {
                            let pen = CAR_RAD * 2.0 - dl;
                            if hit.is_none_or(|h| pen > h.1) {
                                hit = Some((d * (1.0 / dl), pen, (a + b) * 0.5));
                            }
                        }
                    }
                }
                let Some((nrm, pen, at)) = hit else { continue };
                let (mi, mj) = (self.racers[i].car.stats.mass, self.racers[j].car.stats.mass);
                let tot = mi + mj;
                self.racers[i].car.pos -= nrm * (pen * mj / tot);
                self.racers[j].car.pos += nrm * (pen * mi / tot);
                let rel = self.racers[j].car.vel - self.racers[i].car.vel;
                let vn = rel.dot(nrm);
                if vn < 0.0 {
                    let imp = -(1.0 + 0.35) * vn / (1.0 / mi + 1.0 / mj);
                    let ram_i = self.racers[i].ai.ramming();
                    let ram_j = self.racers[j].ai.ramming();
                    let push = if ram_i || ram_j { 1.5 } else { 1.0 };
                    self.racers[i].car.vel -= nrm * (imp / mi * if ram_j { push } else { 1.0 });
                    self.racers[j].car.vel += nrm * (imp / mj * if ram_i { push } else { 1.0 });
                    // contact spins the lighter car a little
                    let side_i = nrm.cross(self.racers[i].car.fwd()).signum();
                    self.racers[i].car.heading += side_i * (-vn / 900.0).min(0.2) * (mj / tot);
                    self.racers[j].car.heading -= side_i * (-vn / 900.0).min(0.2) * (mi / tot);
                    let sev = -vn;
                    self.racers[i].car.impact(sev * (mj / tot) * 1.3, 0.8);
                    self.racers[j].car.impact(sev * (mi / tot) * 1.3, 0.8);
                    if sev > 60.0 && self.racers[i].bump_cd <= 0.0 && self.racers[j].bump_cd <= 0.0
                    {
                        self.racers[i].bump_cd = 0.3;
                        self.racers[j].bump_cd = 0.3;
                        let involves_p = self.racers[i].is_player() || self.racers[j].is_player();
                        let ram = ram_i || ram_j;
                        self.sound(if ram { Sfx::Ram } else { Sfx::Bump }, Some(at));
                        self.sparks(at, nrm, if sev > 200.0 { 10 } else { 4 });
                        if involves_p {
                            self.shake =
                                self.shake
                                    .max(if ram { 0.5 } else { (sev / 600.0).min(0.35) });
                            if ram {
                                self.flash = 0.25;
                                self.hint_flags |= 1 << 4;
                            }
                        }
                        if ram {
                            if ram_i {
                                self.racers[i].ai.rammed();
                            }
                            if ram_j {
                                self.racers[j].ai.rammed();
                            }
                        }
                    }
                }
            }
        }
    }

    fn slipstream(&mut self, dt: f32) {
        let n = self.racers.len();
        let pi = self.player_idx();
        for i in 0..n {
            let me = &self.racers[i];
            let fwd = me.car.fwd();
            let sp = me.car.speed();
            let range = 290.0 * me.car.stats.draft_range;
            let mut found = None;
            if sp > 280.0 {
                for j in 0..n {
                    if i == j {
                        continue;
                    }
                    let o = &self.racers[j];
                    let rel = o.car.pos - me.car.pos;
                    let along = rel.dot(fwd);
                    let side = rel.dot(fwd.perp()).abs();
                    if along > 40.0
                        && along < range
                        && side < 30.0 + along * 0.07
                        && o.car.speed() > 200.0
                    {
                        found = Some(j);
                        break;
                    }
                }
            }
            let me = &mut self.racers[i];
            if let Some(j) = found {
                me.car.draft += dt;
                me.car.nitro = (me.car.nitro + 0.17 * dt * me.car.stats.draft_rate).min(1.0);
                me.drafted = Some((j, self.t));
                if i == pi {
                    self.stats_time_drafting += dt;
                    if self.hint_flags & 1 == 0 && me.car.draft > 0.4 {
                        self.hint_flags |= 1;
                    }
                }
            } else {
                me.car.draft = (me.car.draft - dt * 3.0).max(0.0);
            }
        }
        // wind lines around the player while drafting
        let p = &self.racers[pi];
        if p.car.draft > 0.15 && self.rng.chance(0.5) {
            let f = p.car.fwd();
            let side = f.perp() * self.rng.range(-34.0, 34.0);
            let pos = p.car.pos + f * self.rng.range(20.0, 80.0) + side;
            self.particles.push(Particle {
                pos,
                vel: p.car.vel * 0.6,
                life: 0.3,
                max: 0.3,
                size: 26.0,
                color: 0xE8F6FF,
                kind: PKind::Wind,
                rot: f.angle(),
            });
        }
        if self.racers[pi].car.nitro_on {
            self.stats_nitro_secs += dt;
        }
    }

    fn touch_objects(&mut self, dt: f32) {
        let pi = self.player_idx();
        // cones
        for k in 0..self.cones.len() {
            {
                let c = &mut self.cones[k];
                c.pos += c.vel * dt;
                c.vel = c.vel * (1.0 - 2.5 * dt).max(0.0);
                c.rot += c.spin * dt;
                c.spin *= (1.0 - 2.0 * dt).max(0.0);
            }
            for i in 0..self.racers.len() {
                let car = &self.racers[i].car;
                let hit = car
                    .circles()
                    .iter()
                    .any(|p| p.dist(self.cones[k].pos) < CAR_RAD + 9.0);
                if hit && car.speed() > 60.0 && self.cones[k].vel.len() < car.speed() * 0.6 {
                    let v = car.vel;
                    let jitter = v2(self.rng.range(-80.0, 80.0), self.rng.range(-80.0, 80.0));
                    let spin = self.rng.range(-14.0, 14.0);
                    let c = &mut self.cones[k];
                    c.vel = v * 1.1 + jitter;
                    c.spin = spin;
                    let first = !c.down;
                    c.down = true;
                    let pos = c.pos;
                    self.racers[i].car.vel = self.racers[i].car.vel * 0.97;
                    self.sound(Sfx::Cone, Some(pos));
                    if i == pi && first {
                        self.style.cones += 1;
                        self.add_style(25, "CONE", pos);
                    }
                }
            }
        }
        // pickups
        for k in 0..self.pickups.len() {
            if self.pickups[k].respawn > 0.0 {
                self.pickups[k].respawn -= dt;
                continue;
            }
            let pos = self.pickups[k].pos;
            let kind = self.pickups[k].kind;
            for i in 0..self.racers.len() {
                if self.racers[i].car.pos.dist(pos) < 30.0 {
                    let car = &mut self.racers[i].car;
                    let m = car.stats.pickup_mult;
                    match kind {
                        PickupKind::Nitro => car.nitro = (car.nitro + 0.3 * m).min(1.0),
                        PickupKind::Repair => car.hp = (car.hp + 35.0 * m).min(100.0),
                    }
                    self.pickups[k].respawn = 6.0;
                    if i == pi {
                        self.sound(
                            if kind == PickupKind::Nitro {
                                Sfx::Pickup
                            } else {
                                Sfx::Repair
                            },
                            None,
                        );
                        let (t, c) = if kind == PickupKind::Nitro {
                            ("+NITRO", 0x5BD8FF)
                        } else {
                            ("+REPAIR", 0x6BE38A)
                        };
                        self.popup(t, pos, c, false);
                        self.hint_flags |= 1 << 2;
                    }
                    for _ in 0..10 {
                        let a = self.rng.range(0.0, std::f32::consts::TAU);
                        let col = if kind == PickupKind::Nitro {
                            0x7FE3FF
                        } else {
                            0x8BF0A0
                        };
                        self.particles.push(Particle {
                            pos,
                            vel: V2::from_angle(a) * self.rng.range(60.0, 160.0),
                            life: 0.5,
                            max: 0.5,
                            size: 4.0,
                            color: col,
                            kind: PKind::Star,
                            rot: 0.0,
                        });
                    }
                    break;
                }
            }
        }
        // boost pads
        for k in 0..self.track.pads.len() {
            let pad = self.track.pads[k];
            for i in 0..self.racers.len() {
                let car = &mut self.racers[i].car;
                if car.pos.dist(pad.pos) < 34.0 && car.boost_t < 0.6 {
                    car.boost_t = 1.0;
                    car.boost_k = 1.0;
                    if i == pi {
                        self.sound(Sfx::Pad, None);
                        self.add_style(20, "BOOST", pad.pos);
                        self.shake = self.shake.max(0.15);
                    }
                }
            }
        }
    }

    fn laps_and_places(&mut self) {
        let len = self.track.len;
        let pi = self.player_idx();
        for i in 0..self.racers.len() {
            let r = &mut self.racers[i];
            if r.finish.is_some() {
                continue;
            }
            let lap_now = (r.progress / len).floor() as i32 + 1;
            if lap_now > r.lap {
                if r.lap >= 1 {
                    let lt = self.t - r.lap_start;
                    r.lap_times.push(lt);
                    r.lap_start = self.t;
                    if i == pi {
                        let best = self.best_lap.is_none_or(|b| lt < b);
                        if best {
                            self.best_lap = Some(lt);
                            self.best_ghost = Some(std::mem::take(&mut self.ghost_rec));
                        }
                        self.ghost_rec.clear();
                        self.ghost_acc = 0.0;
                    }
                } else {
                    r.lap_start = self.t;
                    if i == pi {
                        self.ghost_rec.clear();
                    }
                }
                r.lap = lap_now;
                if r.lap > self.laps {
                    r.finish = Some(self.t);
                    if i == pi {
                        self.phase = Phase::Cooldown;
                        let pos = r.car.pos;
                        self.sound(Sfx::Finish, None);
                        self.bank_style();
                        for _ in 0..80 {
                            let a = self.rng.range(0.0, std::f32::consts::TAU);
                            let cols = [0xFF5A5A, 0xFFD34A, 0x5BD8FF, 0x6BE38A, 0xFFFFFF];
                            let col = *self.rng.pick(&cols);
                            self.particles.push(Particle {
                                pos,
                                vel: V2::from_angle(a) * self.rng.range(80.0, 420.0),
                                life: 1.6,
                                max: 1.6,
                                size: 5.0,
                                color: col,
                                kind: PKind::Confetti,
                                rot: a,
                            });
                        }
                    }
                } else if i == pi && r.lap >= 2 {
                    let final_lap = r.lap == self.laps;
                    self.sound(if final_lap { Sfx::FinalLap } else { Sfx::Lap }, None);
                    if final_lap {
                        self.final_lap_announced = true;
                    }
                }
            }
        }
        self.update_places();
        let p = self.player().place;
        // the first seconds off the grid are a shuffle, not overtakes
        if self.phase == Phase::Running && p < self.prev_place && self.t > 3.0 {
            let me = self.player_idx();
            let pos = self.racers[me].car.pos;
            // slingshot if we were drafting the car we just passed
            let passed: Vec<usize> = self
                .racers
                .iter()
                .enumerate()
                .filter(|(j, r)| *j != me && r.place == p + 1)
                .map(|(j, _)| j)
                .collect();
            let sling = self.racers[me]
                .drafted
                .is_some_and(|(j, t)| passed.contains(&j) && self.t - t < 1.6);
            if sling {
                self.style.slingshots += 1;
                self.add_style(300, "SLINGSHOT!", pos);
                self.sound(Sfx::Slingshot, None);
                self.hint_flags |= 1 << 1;
            } else {
                self.style.overtakes += 1;
                self.add_style(100, "OVERTAKE", pos);
                self.sound(Sfx::Overtake, None);
            }
        }
        self.prev_place = p;
    }

    pub fn update_places(&mut self) {
        let mut order: Vec<usize> = (0..self.racers.len()).collect();
        order.sort_by(|&a, &b| {
            let (ra, rb) = (&self.racers[a], &self.racers[b]);
            match (ra.finish, rb.finish) {
                (Some(x), Some(y)) => x.partial_cmp(&y).unwrap(),
                (Some(_), None) => std::cmp::Ordering::Less,
                (None, Some(_)) => std::cmp::Ordering::Greater,
                (None, None) => rb.progress.partial_cmp(&ra.progress).unwrap(),
            }
        });
        for (place, &i) in order.iter().enumerate() {
            self.racers[i].place = place + 1;
        }
    }

    /// Give unfinished cars an estimated finishing time from their pace.
    fn finish_field(&mut self) {
        let total = self.track.len * self.laps as f32;
        for r in self.racers.iter_mut() {
            if r.finish.is_none() {
                let done = r.progress.max(1.0);
                let pace = done / self.t.max(1.0);
                r.finish = Some(self.t + (total - done).max(0.0) / pace.max(50.0));
            }
        }
        self.update_places();
    }

    fn effects(&mut self, dt: f32) {
        let biome = self.track.spec.biome;
        for i in 0..self.racers.len() {
            let r = &self.racers[i];
            let car = &r.car;
            let sp = car.speed();
            let f = car.fwd();
            let rt = car.right();
            let rear = car.pos - f * 22.0;
            // hysteresis: a mark in progress continues at a lower slip
            let slip_th = if r.last_skid[0].is_some() { 0.1 } else { 0.2 };
            let skidding = car.drifting
                || (car.slip > slip_th && sp > 150.0)
                || (r.controls.throttle < -0.5 && car.fwd_speed() > 250.0);
            let on_road = matches!(car.surface, Surface::Road | Surface::Kerb | Surface::Oil);
            let mut marks = [None, None];
            if skidding && on_road {
                for (k, side) in [-11.0f32, 11.0].iter().enumerate() {
                    let p = rear + rt * *side;
                    if let Some(prev) = r.last_skid[k] {
                        self.skids.push((prev, p, 0.3, 0x2C3038));
                    }
                    marks[k] = Some(p);
                }
                if self.rng.chance(dt * 30.0) {
                    let pos = rear + rt * self.rng.range(-12.0, 12.0);
                    let col = if biome == Biome::Timberline {
                        0xD8C0A0
                    } else {
                        0xEDEFF2
                    };
                    self.particles.push(Particle {
                        pos,
                        vel: -car.vel * 0.1
                            + v2(self.rng.range(-20.0, 20.0), self.rng.range(-20.0, 20.0)),
                        life: 0.7,
                        max: 0.7,
                        size: 9.0,
                        color: col,
                        kind: PKind::Smoke,
                        rot: 0.0,
                    });
                }
            }
            if !on_road && sp > 120.0 && self.rng.chance(dt * 40.0) {
                let col = match car.surface {
                    Surface::Mud => 0x6E4A2A,
                    Surface::Sand => 0xE6D3A0,
                    _ => match biome {
                        Biome::DuneSea => 0xE6D3A0,
                        _ => 0x7FBF5A,
                    },
                };
                let pos = rear + rt * self.rng.range(-12.0, 12.0);
                self.particles.push(Particle {
                    pos,
                    vel: -car.vel * 0.25
                        + v2(self.rng.range(-30.0, 30.0), self.rng.range(-30.0, 30.0)),
                    life: 0.6,
                    max: 0.6,
                    size: 7.0,
                    color: col,
                    kind: PKind::Dust,
                    rot: 0.0,
                });
                if matches!(car.surface, Surface::Mud | Surface::Sand) {
                    self.skids.push((rear, rear - f * 6.0, 0.12, 0x5A3B20));
                }
            }
            self.racers[i].last_skid = marks;
            let r = &self.racers[i];
            let car = &r.car;
            // drift sparks show the turbo tier
            if car.drifting {
                let tier = car.drift_tier();
                if tier > 0 && self.rng.chance(dt * 45.0) {
                    let col = [0, 0x5BC8FF, 0xFF9A3C, 0xC86BFF][tier as usize];
                    let side = if self.rng.chance(0.5) { -12.0 } else { 12.0 };
                    let pos = rear + rt * side;
                    let a = (-f).angle() + self.rng.range(-0.8, 0.8);
                    self.particles.push(Particle {
                        pos,
                        vel: V2::from_angle(a) * self.rng.range(80.0, 220.0),
                        life: 0.25,
                        max: 0.25,
                        size: 3.0,
                        color: col,
                        kind: PKind::Spark,
                        rot: a,
                    });
                }
            }
            if car.boosting() && self.rng.chance(dt * 50.0) {
                let pos = car.pos - f * 32.0 + rt * self.rng.range(-6.0, 6.0);
                let col = if car.nitro_on { 0x5BD8FF } else { 0xFFB347 };
                self.particles.push(Particle {
                    pos,
                    vel: -f * self.rng.range(120.0, 220.0) + car.vel * 0.5,
                    life: 0.22,
                    max: 0.22,
                    size: 9.0,
                    color: col,
                    kind: PKind::Flame,
                    rot: 0.0,
                });
            }
            if car.hp < 30.0 && self.rng.chance(dt * 12.0) {
                let pos = car.pos + f * 18.0;
                self.particles.push(Particle {
                    pos,
                    vel: v2(self.rng.range(-15.0, 15.0), self.rng.range(-30.0, -5.0)),
                    life: 1.0,
                    max: 1.0,
                    size: 10.0,
                    color: 0x50545C,
                    kind: PKind::Smoke,
                    rot: 0.0,
                });
            }
        }
        if self.skids.len() > 4000 {
            let k = self.skids.len() - 4000;
            self.skids.drain(0..k);
        }
    }

    fn sparks(&mut self, at: V2, dir: V2, n: usize) {
        for _ in 0..n {
            let a = dir.angle() + self.rng.range(-1.2, 1.2);
            self.particles.push(Particle {
                pos: at,
                vel: V2::from_angle(a) * self.rng.range(120.0, 380.0),
                life: 0.35,
                max: 0.35,
                size: 3.0,
                color: 0xFFD34A,
                kind: PKind::Spark,
                rot: a,
            });
        }
    }

    // ---- style chain -------------------------------------------------------

    fn add_style(&mut self, pts: u32, label: &str, pos: V2) {
        self.style.chain += 1;
        let mult = self.style.mult();
        let gained = pts * mult;
        self.style.chain_pts += gained;
        self.style.timer = 2.6;
        self.style.label = label.to_string();
        self.style.label_t = 1.4;
        self.style.best_chain = self.style.best_chain.max(self.style.chain);
        let pi = self.player_idx();
        let car = &mut self.racers[pi].car;
        car.nitro = (car.nitro + pts as f32 / 1800.0).min(1.0);
        let txt = if mult > 1 {
            format!("{label} +{pts} x{mult}")
        } else {
            format!("{label} +{pts}")
        };
        // the big call-out already says it (rocket start)
        if label != "ROCKET START" {
            self.popup(txt, pos, 0xFFE27A, false);
        }
        self.sound(Sfx::Style, None);
    }

    fn style_tick(&mut self, dt: f32) {
        if self.style.chain > 0 {
            self.style.timer -= dt;
            if self.style.timer <= 0.0 {
                self.bank_style();
            }
        }
        // near misses: passing a rival very closely at speed without contact
        let pi = self.player_idx();
        let (pp, pv) = (self.racers[pi].car.pos, self.racers[pi].car.speed());
        if pv > 420.0 && self.phase == Phase::Running {
            for j in 0..self.racers.len() {
                if j == pi {
                    continue;
                }
                let d = self.racers[j].car.pos.dist(pp);
                let rel = (self.racers[pi].car.vel - self.racers[j].car.vel).len();
                let near = d < 52.0 && d > CAR_RAD * 2.0 + 2.0 && rel > 140.0;
                let flag = self.racers[j].ai.near_flag;
                if near && !flag && self.racers[j].bump_cd <= 0.0 {
                    self.racers[j].ai.near_flag = true;
                    self.style.near_misses += 1;
                    self.add_style(80, "CLOSE CALL", pp);
                } else if d > 140.0 {
                    self.racers[j].ai.near_flag = false;
                }
            }
        }
    }

    fn break_chain(&mut self) {
        if self.style.chain > 0 {
            self.bank_style();
        }
    }

    fn bank_style(&mut self) {
        self.style.total += self.style.chain_pts;
        self.style.chain = 0;
        self.style.chain_pts = 0;
        self.style.timer = 0.0;
    }

    fn record_ghost(&mut self, dt: f32) {
        let pi = self.player_idx();
        let r = &self.racers[pi];
        if r.lap < 1 || r.finish.is_some() {
            return;
        }
        self.ghost_acc += dt;
        let want = ((self.t - r.lap_start) * GHOST_HZ) as usize + 1;
        while self.ghost_rec.len() < want {
            self.ghost_rec
                .push((r.car.pos.x, r.car.pos.y, r.car.heading));
        }
    }

    /// Ghost pose at the current lap time (time trial).
    pub fn ghost_pose(&self) -> Option<(V2, f32)> {
        let g = self.ghost.as_ref()?;
        let r = self.player();
        if g.len() < 2 || r.lap < 1 || r.finish.is_some() {
            return None;
        }
        let t = (self.t - r.lap_start) * GHOST_HZ;
        let i = t.floor() as usize;
        if i + 1 >= g.len() {
            let l = g[g.len() - 1];
            return Some((v2(l.0, l.1), l.2));
        }
        let f = t - i as f32;
        let (a, b) = (g[i], g[i + 1]);
        let h = a.2 + wrap_angle(b.2 - a.2) * f;
        Some((v2(a.0 + (b.0 - a.0) * f, a.1 + (b.1 - a.1) * f), h))
    }

    /// Final classification: racer indices in finishing order.
    pub fn classification(&self) -> Vec<usize> {
        let mut order: Vec<usize> = (0..self.racers.len()).collect();
        order.sort_by_key(|&i| self.racers[i].place);
        order
    }

    /// Lateral distance to the road edge (for hints/assists).
    pub fn edge_margin(&self, i: usize) -> f32 {
        HALF_W - self.racers[i].lat.abs()
    }

    /// Upcoming corner for the player's pace notes: (distance, corner).
    pub fn next_corner(&self) -> Option<(f32, crate::track::Corner)> {
        let r = self.player();
        let len = self.track.len;
        let mut best: Option<(f32, crate::track::Corner)> = None;
        for c in &self.track.corners {
            let mut d = c.start_s - r.s;
            let inside = if c.end_s >= c.start_s {
                r.s >= c.start_s && r.s <= c.end_s
            } else {
                r.s >= c.start_s || r.s <= c.end_s
            };
            if inside {
                d = 0.0;
            } else if d < -40.0 {
                d += len;
            }
            if (-40.0..900.0).contains(&d) && best.is_none_or(|b| d < b.0) {
                best = Some((d, *c));
            }
        }
        best
    }
}
