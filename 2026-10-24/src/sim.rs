//! The deterministic simulation: Wren's movement and rope, enemies, hazards,
//! pickups and scoring. Everything advances in fixed 1/60 s steps from
//! `World::step`, so a `World` can be cloned and simulated ahead (the
//! autopilot does exactly that) and snapshots give us time rewind.

use std::sync::Arc;

use crate::boss::Boss;
use crate::level::*;
use crate::rng::Rng;

pub const DT: f32 = 1.0 / 60.0;
/// Player half extents.
pub const HW: f32 = 10.0;
pub const HH: f32 = 15.0;

pub const RUN: f32 = 235.0;
const GROUND_ACC: f32 = 2600.0;
const GROUND_FRIC: f32 = 3000.0;
const ICE_ACC: f32 = 520.0;
const ICE_FRIC: f32 = 260.0;
const AIR_ACC: f32 = 1600.0;
const AIR_FRIC: f32 = 520.0;
const GRAV: f32 = 1900.0;
const FALL_MULT: f32 = 1.3;
const MAX_FALL: f32 = 800.0;
const MAX_SPEED: f32 = 980.0;
const COYOTE: f32 = 0.11;
const JUMP_BUF: f32 = 0.13;
const WALL_JUMP_VX: f32 = 275.0;
const WALL_JUMP_VY: f32 = -610.0;
const MIN_ROPE: f32 = 56.0;
/// Seconds a brittle anchor holds.
pub const BRITTLE_HOLD: f32 = 1.45;
/// Seconds of rewindable history kept.
pub const SAFE_STILL: f32 = 0.22;

#[derive(Clone, Copy, Debug, Default)]
pub struct Controls {
    /// -1, 0, 1.
    pub dir: i8,
    pub up: bool,
    pub down: bool,
    pub jump_pressed: bool,
    pub jump_held: bool,
    pub hook_pressed: bool,
    pub hook_held: bool,
    /// World-space target of the hook; `None` auto-aims up and forward.
    pub aim: Option<(f32, f32)>,
}

/// Modifiers from boons, workshop perks and assists.
#[derive(Clone, Debug)]
pub struct Mods {
    pub max_hp: i32,
    pub hook_range: f32,
    pub reel: f32,
    pub pump: f32,
    pub smash_v: f32,
    pub magnet: f32,
    pub crystal_value: f32,
    pub air_jumps: i32,
    pub jump_v: f32,
    pub wall_slide: f32,
    pub shield: bool,
    pub lamp: f32,
    pub echo_max: f32,
    pub assist_aim: bool,
    pub invincible: bool,
    pub revive: bool,
    pub chain_time: f32,
}

impl Default for Mods {
    fn default() -> Mods {
        Mods {
            max_hp: 4,
            hook_range: 400.0,
            reel: 150.0,
            pump: 520.0,
            smash_v: 440.0,
            magnet: 46.0,
            crystal_value: 1.0,
            air_jumps: 0,
            jump_v: 640.0,
            wall_slide: 110.0,
            shield: false,
            lamp: 1.0,
            echo_max: 100.0,
            assist_aim: true,
            invincible: false,
            revive: false,
            chain_time: 3.2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rope {
    pub anchor: usize,
    pub ax: f32,
    pub ay: f32,
    pub len: f32,
    pub t: f32,
}

#[derive(Clone, Debug)]
pub struct Player {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub facing: i8,
    pub on_ground: bool,
    pub ice: bool,
    pub coyote: f32,
    pub jump_buf: f32,
    pub wall: i8,
    pub wall_side: i8,
    pub wall_coyote: f32,
    pub wall_lock: f32,
    pub jumping: bool,
    pub air_jumps: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub invuln: f32,
    pub rope: Option<Rope>,
    pub hook_cd: f32,
    pub charged: bool,
    pub speed: f32,
    pub still: f32,
    pub safe: (f32, f32),
    pub shield: bool,
    pub run_t: f32,
    pub air_t: f32,
    pub dying: f32,
    pub dead: bool,
    pub hurt_t: f32,
    pub since_ground: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct AnchorState {
    pub x: f32,
    pub y: f32,
    pub alive: bool,
    pub stress: f32,
    pub respawn: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Block {
    pub solid: bool,
    pub kind: Tile,
    /// Crumble countdown (>0 while crumbling).
    pub timer: f32,
    /// Seconds until a fallen crumble tile returns.
    pub gone: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct Ent {
    pub kind: EntKind,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub hw: f32,
    pub hh: f32,
    pub t: f32,
    pub hp: i32,
    pub state: u8,
    pub dir: f32,
    pub a: f32,
    pub b: f32,
    pub c: f32,
    pub home_x: f32,
    pub home_y: f32,
    pub alive: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Ev {
    Jump,
    WallJump,
    AirJump,
    Land,
    HookHit,
    HookMiss,
    Release,
    RopeSnap,
    Brittle,
    Crystal,
    EchoOrb,
    HeartShard,
    Chest,
    Hurt,
    Stomp,
    Ram,
    Smash,
    Pit,
    Checkpoint,
    SpitterCharge,
    SpitterFire,
    GeyserWarn,
    GeyserBurst,
    StrikeWarn,
    StrikeHit,
    Bounce,
    Crumble,
    Cleared,
    Shield,
    BossHit,
    BossStun,
    BossRoar,
    BossAttack,
    BossDown,
    Dying,
    ChainBreak,
    Revive,
}

#[derive(Clone, Copy, Debug)]
pub struct Event {
    pub kind: Ev,
    pub x: f32,
    pub y: f32,
    pub v: f32,
}

#[derive(Clone, Debug, Default)]
pub struct Stats {
    pub crystals: u32,
    pub score: i64,
    pub chain: u32,
    pub chain_t: f32,
    pub best_chain: u32,
    pub damage: u32,
    pub falls: u32,
    pub kills: u32,
    pub smashes: u32,
    pub hooks: u32,
    pub chests: u32,
    pub rams: u32,
    /// Hits taken by cause (for balance tests).
    pub causes: Vec<(&'static str, u32)>,
}

impl Stats {
    pub fn mult(&self) -> u32 {
        (1 + self.chain / 3).min(6)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Playing,
    Dead,
    Cleared,
}

#[derive(Clone)]
pub struct World {
    pub level: Arc<Level>,
    pub mods: Mods,
    pub p: Player,
    pub ents: Vec<Ent>,
    pub anchors: Vec<AnchorState>,
    pub blocks: Vec<Block>,
    pub time: f32,
    pub frame: u64,
    pub rng: Rng,
    pub stats: Stats,
    pub events: Vec<Event>,
    pub cp_next: usize,
    pub outcome: Outcome,
    pub boss: Option<Boss>,
    /// Last missed hook: target and remaining seconds (for drawing).
    pub hook_miss: Option<(f32, f32, f32)>,
    /// Gust factor in -1..1 for gfx.
    pub gust: f32,
    pub gate_open: bool,
    /// What last hurt Wren (for the game-over screen).
    pub last_cause: &'static str,
}

pub fn dist(ax: f32, ay: f32, bx: f32, by: f32) -> f32 {
    ((ax - bx) * (ax - bx) + (ay - by) * (ay - by)).sqrt()
}

impl World {
    pub fn new(level: Arc<Level>, mods: Mods, start_hp: i32) -> World {
        let (sx, sy) = level.spawn;
        let hp = start_hp.clamp(1, mods.max_hp);
        let p = Player {
            x: sx,
            y: sy,
            vx: 0.0,
            vy: 0.0,
            facing: 1,
            on_ground: false,
            ice: false,
            coyote: 0.0,
            jump_buf: 0.0,
            wall: 0,
            wall_side: 0,
            wall_coyote: 0.0,
            wall_lock: 0.0,
            jumping: false,
            air_jumps: mods.air_jumps,
            hp,
            max_hp: mods.max_hp,
            invuln: 0.0,
            rope: None,
            hook_cd: 0.0,
            charged: false,
            speed: 0.0,
            still: 0.0,
            safe: (sx, sy),
            shield: mods.shield,
            run_t: 0.0,
            air_t: 0.0,
            dying: 0.0,
            dead: false,
            hurt_t: 0.0,
            since_ground: 0.0,
        };
        let anchors = level
            .anchors
            .iter()
            .map(|a| AnchorState {
                x: a.x,
                y: a.y,
                alive: true,
                stress: 0.0,
                respawn: 0.0,
            })
            .collect();
        let blocks = level
            .block_tiles
            .iter()
            .map(|&(x, y)| {
                let k = level.tile(x, y);
                Block {
                    solid: true,
                    kind: k,
                    timer: 0.0,
                    gone: 0.0,
                }
            })
            .collect();
        let mut ents = Vec::with_capacity(level.ents.len() + 16);
        for s in &level.ents {
            ents.push(Ent::from_spawn(s));
        }
        let boss = if level.boss {
            Some(Boss::new(&level))
        } else {
            None
        };
        World {
            rng: Rng::new(level.seed ^ 0xC4A6),
            level,
            mods,
            p,
            ents,
            anchors,
            blocks,
            time: 0.0,
            frame: 0,
            stats: Stats::default(),
            events: Vec::new(),
            cp_next: 0,
            outcome: Outcome::Playing,
            boss,
            hook_miss: None,
            gust: 0.0,
            gate_open: false,
            last_cause: "",
        }
    }

    pub fn emit(&mut self, kind: Ev, x: f32, y: f32, v: f32) {
        self.events.push(Event { kind, x, y, v });
    }

    /// Clone without anything the autopilot's lookahead does not need.
    pub fn fork(&self) -> World {
        let mut w = self.clone();
        w.events = Vec::new();
        w
    }

    // ------------------------------------------------------------ collision

    #[inline]
    pub fn solid_tile(&self, tx: i32, ty: i32) -> bool {
        match self.level.tile(tx, ty) {
            ROCK | ICE => true,
            GLASS | CRUMBLE => match self.level.block_at(tx, ty) {
                Some(b) => self.blocks[b].solid,
                None => true,
            },
            _ => false,
        }
    }

    pub fn box_hits(&self, x: f32, y: f32, hw: f32, hh: f32) -> bool {
        let x0 = ((x - hw) / T).floor() as i32;
        let x1 = ((x + hw - 0.001) / T).floor() as i32;
        let y0 = ((y - hh) / T).floor() as i32;
        let y1 = ((y + hh - 0.001) / T).floor() as i32;
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                if self.solid_tile(tx, ty) {
                    return true;
                }
            }
        }
        false
    }

    /// Is the line between two points free of solid tiles?
    pub fn ray_clear(&self, x0: f32, y0: f32, x1: f32, y1: f32) -> bool {
        let d = dist(x0, y0, x1, y1);
        let n = (d / 8.0).ceil().max(1.0) as i32;
        for i in 1..n {
            let t = i as f32 / n as f32;
            let x = x0 + (x1 - x0) * t;
            let y = y0 + (y1 - y0) * t;
            if self.solid_tile((x / T).floor() as i32, (y / T).floor() as i32) {
                return false;
            }
        }
        true
    }

    /// Break glass tiles overlapping the box if we are going fast enough.
    /// Returns true when every blocking tile was glass and has shattered.
    fn try_smash(&mut self, x: f32, y: f32) -> bool {
        if self.p.speed < self.mods.smash_v {
            return false;
        }
        let x0 = ((x - HW) / T).floor() as i32;
        let x1 = ((x + HW - 0.001) / T).floor() as i32;
        let y0 = ((y - HH) / T).floor() as i32;
        let y1 = ((y + HH - 0.001) / T).floor() as i32;
        let mut hit = vec![];
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                if self.solid_tile(tx, ty) {
                    if self.level.tile(tx, ty) == GLASS {
                        hit.push((tx, ty));
                    } else {
                        return false;
                    }
                }
            }
        }
        if hit.is_empty() {
            return false;
        }
        let mut broke = 0;
        for (tx, ty) in hit {
            for oy in -1..=1 {
                for ox in -1..=1 {
                    if let Some(b) = self.level.block_at(tx + ox, ty + oy)
                        && self.level.tile(tx + ox, ty + oy) == GLASS
                        && self.blocks[b].solid
                    {
                        self.blocks[b].solid = false;
                        broke += 1;
                        self.emit(
                            Ev::Smash,
                            (tx + ox) as f32 * T + T / 2.0,
                            (ty + oy) as f32 * T + T / 2.0,
                            self.p.speed,
                        );
                    }
                }
            }
        }
        if broke > 0 {
            self.stats.smashes += 1;
            self.stats.score += 25 * broke as i64;
            self.p.vx *= 0.88;
            self.p.vy *= 0.88;
        }
        true
    }

    /// Move horizontally with collision; returns true if blocked.
    fn move_x(&mut self, dx: f32) -> bool {
        let mut rem = dx;
        while rem.abs() > 0.0001 {
            let mut step = rem.clamp(-3.0, 3.0);
            let mut moved = false;
            // shrink the step until it fits, so we end flush against the wall
            for _ in 0..7 {
                let nx = self.p.x + step;
                if !self.box_hits(nx, self.p.y, HW, HH) {
                    self.p.x = nx;
                    rem -= step;
                    moved = true;
                    break;
                }
                if step.abs() < 0.05 {
                    break;
                }
                step *= 0.5;
            }
            if moved {
                continue;
            }
            let step = rem.clamp(-0.05, 0.05);
            let nx = self.p.x + step;
            if self.try_smash(nx, self.p.y) {
                self.p.x = nx;
                rem -= step;
                continue;
            }
            // ledge pop: step up small lips when moving along the ground.
            if self.p.vy >= -50.0 {
                let mut popped = false;
                for up in 1..=7 {
                    if !self.box_hits(nx, self.p.y - up as f32, HW, HH) {
                        self.p.x = nx;
                        self.p.y -= up as f32;
                        rem -= step;
                        popped = true;
                        break;
                    }
                }
                if popped {
                    continue;
                }
            }
            self.p.vx = 0.0;
            return true;
        }
        false
    }

    /// Move vertically with collision; returns +1 if landed, -1 if hit head.
    fn move_y(&mut self, dy: f32) -> i8 {
        let mut rem = dy;
        while rem.abs() > 0.0001 {
            let mut step = rem.clamp(-3.0, 3.0);
            let mut moved = false;
            for _ in 0..7 {
                let ny = self.p.y + step;
                if !self.box_hits(self.p.x, ny, HW, HH) {
                    self.p.y = ny;
                    rem -= step;
                    moved = true;
                    break;
                }
                if step.abs() < 0.05 {
                    break;
                }
                step *= 0.5;
            }
            if moved {
                continue;
            }
            let step = rem.clamp(-0.05, 0.05);
            let ny = self.p.y + step;
            if self.try_smash(self.p.x, ny) {
                self.p.y = ny;
                rem -= step;
                continue;
            }
            if step < 0.0 {
                // corner correction: slide round the edge of a ceiling.
                let mut done = false;
                for nudge in 1..=7 {
                    for s in [-1.0f32, 1.0] {
                        let nx = self.p.x + s * nudge as f32;
                        if !self.box_hits(nx, ny, HW, HH) {
                            self.p.x = nx;
                            self.p.y = ny;
                            rem -= step;
                            done = true;
                            break;
                        }
                    }
                    if done {
                        break;
                    }
                }
                if done {
                    continue;
                }
                self.p.vy = self.p.vy.max(0.0);
                return -1;
            }
            self.p.vy = 0.0;
            return 1;
        }
        0
    }

    fn probe_ground(&self) -> (bool, bool) {
        let y = self.p.y + 1.0;
        if !self.box_hits(self.p.x, y, HW - 0.5, HH) {
            return (false, false);
        }
        let fy = ((self.p.y + HH + 0.5) / T).floor() as i32;
        let lx = ((self.p.x - HW + 1.0) / T).floor() as i32;
        let rx = ((self.p.x + HW - 1.0) / T).floor() as i32;
        let ice = self.level.tile(lx, fy) == ICE || self.level.tile(rx, fy) == ICE;
        (true, ice)
    }

    fn probe_wall(&self) -> i8 {
        if self.box_hits(self.p.x - 1.5, self.p.y - 4.0, HW, HH - 6.0) {
            -1
        } else if self.box_hits(self.p.x + 1.5, self.p.y - 4.0, HW, HH - 6.0) {
            1
        } else {
            0
        }
    }

    // ------------------------------------------------------------- stepping

    pub fn step(&mut self, c: &Controls) {
        if self.outcome != Outcome::Playing {
            return;
        }
        self.frame += 1;
        self.time += DT;
        self.update_anchors();
        self.update_blocks();
        self.update_player(c);
        self.update_ents();
        self.update_boss();
        self.update_pickups_and_flags();
        if self.stats.chain > 0 {
            self.stats.chain_t -= DT;
            if self.stats.chain_t <= 0.0 {
                self.stats.chain = 0;
                self.emit(Ev::ChainBreak, self.p.x, self.p.y, 0.0);
            }
        }
        if let Some((_, _, t)) = &mut self.hook_miss {
            *t -= DT;
            if *t <= 0.0 {
                self.hook_miss = None;
            }
        }
        if self.p.dead {
            self.outcome = Outcome::Dead;
        }
    }

    fn update_anchors(&mut self) {
        for i in 0..self.anchors.len() {
            let def = self.level.anchors[i];
            let st = &mut self.anchors[i];
            if !st.alive {
                st.respawn -= DT;
                if st.respawn <= 0.0 {
                    st.alive = true;
                    st.stress = 0.0;
                }
            }
            if def.kind == AnchorKind::Rail {
                let s = 0.5 + 0.5 * (self.time * 1.9 + def.phase).sin();
                st.x = def.x + (def.x2 - def.x) * s;
                st.y = def.y + (def.y2 - def.y) * s;
            }
            if def.kind == AnchorKind::Brittle && st.alive {
                let held = self.p.rope.is_some_and(|r| r.anchor == i);
                if held {
                    st.stress += DT;
                } else {
                    st.stress = (st.stress - DT * 0.6).max(0.0);
                }
            }
        }
        // brittle break
        let mut snap = None;
        for i in 0..self.anchors.len() {
            if self.level.anchors[i].kind == AnchorKind::Brittle
                && self.anchors[i].alive
                && self.anchors[i].stress >= BRITTLE_HOLD
            {
                self.anchors[i].alive = false;
                self.anchors[i].respawn = 4.0;
                let (x, y) = (self.anchors[i].x, self.anchors[i].y);
                self.emit(Ev::Brittle, x, y, 0.0);
                if self.p.rope.is_some_and(|r| r.anchor == i) {
                    snap = Some(i);
                }
            }
        }
        if snap.is_some() {
            self.p.rope = None;
        }
        if let Some(r) = &mut self.p.rope {
            let a = self.anchors[r.anchor];
            r.ax = a.x;
            r.ay = a.y;
        }
    }

    fn update_blocks(&mut self) {
        let px = self.p.x;
        let py = self.p.y;
        let mut events = vec![];
        for (i, b) in self.blocks.iter_mut().enumerate() {
            if b.kind != CRUMBLE {
                continue;
            }
            let (tx, ty) = self.level.block_tiles[i];
            if b.solid {
                if b.timer > 0.0 {
                    b.timer -= DT;
                    if b.timer <= 0.0 {
                        b.solid = false;
                        b.gone = 2.8;
                        events.push((tx, ty));
                    }
                } else {
                    // standing on it?
                    let top = ty as f32 * T;
                    if (py + HH - top).abs() < 3.0
                        && px + HW > tx as f32 * T
                        && px - HW < (tx + 1) as f32 * T
                    {
                        b.timer = 0.38;
                    }
                }
            } else {
                b.gone -= DT;
                if b.gone <= 0.0 {
                    // only return when nobody is inside it
                    let cx = tx as f32 * T + T / 2.0;
                    let cy = ty as f32 * T + T / 2.0;
                    if (px - cx).abs() > HW + T / 2.0 || (py - cy).abs() > HH + T / 2.0 {
                        b.solid = true;
                        b.timer = 0.0;
                    } else {
                        b.gone = 0.2;
                    }
                }
            }
        }
        for (tx, ty) in events {
            self.emit(
                Ev::Crumble,
                tx as f32 * T + T / 2.0,
                ty as f32 * T + T / 2.0,
                0.0,
            );
        }
    }

    // --------------------------------------------------------------- player

    /// Which anchor a hook press with these controls would catch.
    pub fn find_anchor(&self, c: &Controls) -> Option<usize> {
        let px = self.p.x;
        let py = self.p.y - 4.0;
        let range = self.mods.hook_range;
        let (dirx, diry, cone, assisted) = match c.aim {
            Some((ax, ay)) => {
                let (dx, dy) = (ax - px, ay - py);
                let l = (dx * dx + dy * dy).sqrt().max(1.0);
                let cone = if self.mods.assist_aim { 0.48 } else { 0.2 };
                (dx / l, dy / l, cone, true)
            }
            None => {
                let (dx, dy) = (self.p.facing as f32 * 0.6, -0.8);
                let l = (dx * dx + dy * dy).sqrt();
                (dx / l, dy / l, 1.25, false)
            }
        };
        let mut best = None;
        let mut best_score = f32::INFINITY;
        for (i, a) in self.anchors.iter().enumerate() {
            if !a.alive {
                continue;
            }
            let (dx, dy) = (a.x - px, a.y - py);
            let d = (dx * dx + dy * dy).sqrt();
            if !(24.0..=range).contains(&d) {
                continue;
            }
            let cosang = ((dx * dirx + dy * diry) / d).clamp(-1.0, 1.0);
            let ang = cosang.acos();
            if ang > cone {
                continue;
            }
            // without assist, require the cursor to be near the anchor itself.
            if assisted && !self.mods.assist_aim {
                let perp = (dx * diry - dy * dirx).abs();
                if perp > 34.0 {
                    continue;
                }
            }
            if !self.ray_clear(px, py, a.x, a.y) {
                continue;
            }
            let score = ang * 240.0 + d * 0.12;
            if score < best_score {
                best_score = score;
                best = Some(i);
            }
        }
        best
    }

    fn fire_hook(&mut self, c: &Controls) {
        if let Some(i) = self.find_anchor(c) {
            let a = self.anchors[i];
            let d = dist(self.p.x, self.p.y, a.x, a.y);
            self.p.rope = Some(Rope {
                anchor: i,
                ax: a.x,
                ay: a.y,
                len: d.max(MIN_ROPE),
                t: 0.0,
            });
            self.p.hook_cd = 0.1;
            self.stats.hooks += 1;
            self.bump_chain(1, a.x, a.y);
            self.emit(Ev::HookHit, a.x, a.y, i as f32);
        } else {
            let (tx, ty) = match c.aim {
                Some((ax, ay)) => {
                    let (dx, dy) = (ax - self.p.x, ay - self.p.y);
                    let l = (dx * dx + dy * dy).sqrt().max(1.0);
                    let r = self.mods.hook_range.min(l);
                    (self.p.x + dx / l * r, self.p.y + dy / l * r)
                }
                None => (self.p.x + self.p.facing as f32 * 120.0, self.p.y - 180.0),
            };
            self.hook_miss = Some((tx, ty, 0.2));
            self.p.hook_cd = 0.22;
            self.emit(Ev::HookMiss, tx, ty, 0.0);
        }
    }

    fn release_rope(&mut self) {
        if self.p.rope.take().is_some() {
            self.p.hook_cd = self.p.hook_cd.max(0.08);
            self.emit(Ev::Release, self.p.x, self.p.y, self.p.speed);
        }
    }

    pub fn bump_chain(&mut self, n: u32, x: f32, y: f32) {
        let _ = (x, y);
        self.stats.chain += n;
        self.stats.chain_t = self.mods.chain_time;
        self.stats.best_chain = self.stats.best_chain.max(self.stats.chain);
    }

    fn update_player(&mut self, c: &Controls) {
        if self.p.dying > 0.0 {
            // dramatic fall; no control.
            self.p.dying -= DT;
            self.p.vy = (self.p.vy + GRAV * DT).min(MAX_FALL);
            self.p.vx *= 0.97;
            let (vx, vy) = (self.p.vx, self.p.vy);
            self.move_x(vx * DT);
            self.move_y(vy * DT);
            if self.p.dying <= 0.0 {
                self.p.dead = true;
            }
            return;
        }
        let m = self.mods.clone();
        self.p.invuln = (self.p.invuln - DT).max(0.0);
        self.p.hurt_t = (self.p.hurt_t - DT).max(0.0);
        self.p.hook_cd = (self.p.hook_cd - DT).max(0.0);
        self.p.wall_lock = (self.p.wall_lock - DT).max(0.0);

        // ground / wall state
        let (g, ice) = if self.p.vy >= 0.0 {
            self.probe_ground()
        } else {
            (false, false)
        };
        let was_ground = self.p.on_ground;
        self.p.on_ground = g;
        self.p.ice = ice;
        if g {
            self.p.coyote = COYOTE;
            self.p.air_jumps = m.air_jumps;
            self.p.since_ground = 0.0;
            self.p.air_t = 0.0;
        } else {
            self.p.coyote = (self.p.coyote - DT).max(0.0);
            self.p.since_ground += DT;
            self.p.air_t += DT;
        }
        self.p.wall = if g { 0 } else { self.probe_wall() };
        if self.p.wall != 0 {
            self.p.wall_side = self.p.wall;
            self.p.wall_coyote = 0.12;
        } else {
            self.p.wall_coyote = (self.p.wall_coyote - DT).max(0.0);
        }
        if c.jump_pressed {
            self.p.jump_buf = JUMP_BUF;
        } else {
            self.p.jump_buf = (self.p.jump_buf - DT).max(0.0);
        }
        if c.dir != 0 {
            self.p.facing = c.dir;
        }

        // hook
        if c.hook_pressed && self.p.hook_cd <= 0.0 {
            if self.p.rope.is_some() {
                self.release_rope();
            }
            self.fire_hook(c);
        }
        if self.p.rope.is_some() && !c.hook_held {
            self.release_rope();
        }

        // jump
        if self.p.jump_buf > 0.0 {
            if self.p.rope.is_some() {
                self.release_rope();
                self.p.vy -= 260.0;
                self.p.jump_buf = 0.0;
                self.p.jumping = true;
                self.emit(Ev::Jump, self.p.x, self.p.y + HH, 0.0);
            } else if self.p.coyote > 0.0 {
                self.p.vy = -m.jump_v;
                self.p.coyote = 0.0;
                self.p.jump_buf = 0.0;
                self.p.jumping = true;
                self.p.on_ground = false;
                self.emit(Ev::Jump, self.p.x, self.p.y + HH, 0.0);
            } else if self.p.wall_coyote > 0.0 && self.p.wall_side != 0 {
                let side = self.p.wall_side;
                self.p.vy = WALL_JUMP_VY;
                self.p.vx = -side as f32 * WALL_JUMP_VX;
                self.p.facing = -side;
                self.p.wall_lock = 0.14;
                self.p.wall_coyote = 0.0;
                self.p.jump_buf = 0.0;
                self.p.jumping = true;
                self.emit(
                    Ev::WallJump,
                    self.p.x + side as f32 * HW,
                    self.p.y,
                    side as f32,
                );
            } else if self.p.air_jumps > 0 {
                self.p.air_jumps -= 1;
                self.p.vy = -m.jump_v * 0.92;
                self.p.jump_buf = 0.0;
                self.p.jumping = true;
                self.emit(Ev::AirJump, self.p.x, self.p.y + HH, 0.0);
            }
        }
        if self.p.jumping {
            if self.p.vy >= 0.0 {
                self.p.jumping = false;
            } else if !(c.jump_held || c.jump_pressed) {
                self.p.vy *= 0.5;
                self.p.jumping = false;
            }
        }

        // horizontal control
        let roped = self.p.rope.is_some();
        let dir = if self.p.wall_lock > 0.0 { 0 } else { c.dir };
        let dirf = dir as f32;
        if roped && !self.p.on_ground {
            if dir != 0 {
                self.p.vx += dirf * m.pump * DT;
            }
        } else if self.p.on_ground {
            let (acc, fric) = if self.p.ice {
                (ICE_ACC, ICE_FRIC)
            } else {
                (GROUND_ACC, GROUND_FRIC)
            };
            if dir != 0 {
                let target = dirf * RUN;
                if self.p.vx * dirf < RUN {
                    self.p.vx = approach(self.p.vx, target, acc * DT);
                } else {
                    self.p.vx = approach(self.p.vx, target, fric * DT);
                }
            } else {
                self.p.vx = approach(self.p.vx, 0.0, fric * DT);
            }
        } else if dir != 0 {
            if self.p.vx * dirf < RUN {
                self.p.vx = approach(self.p.vx, dirf * RUN, AIR_ACC * DT);
            } else {
                // keep swing momentum; only a little drag
                self.p.vx = approach(self.p.vx, dirf * RUN, 90.0 * DT);
            }
        } else {
            let fr = if self.p.vx.abs() > RUN {
                90.0
            } else {
                AIR_FRIC
            };
            self.p.vx = approach(self.p.vx, 0.0, fr * DT);
        }

        // gravity
        let mut grav = GRAV;
        if !roped {
            if self.p.vy > 0.0 {
                grav *= FALL_MULT;
            } else if c.jump_held && self.p.vy.abs() < 90.0 {
                grav *= 0.62;
            }
        }
        if !self.p.on_ground {
            self.p.vy += grav * DT;
        } else if self.p.vy > 0.0 {
            self.p.vy = 0.0;
        }
        let cap = if roped { 1100.0 } else { MAX_FALL };
        self.p.vy = self.p.vy.min(cap);
        // wall slide
        if self.p.wall != 0 && self.p.vy > 0.0 && dir == self.p.wall && !roped {
            self.p.vy = self.p.vy.min(m.wall_slide);
        }
        // wind
        self.apply_wind();

        // rope reel
        if let Some(r) = &mut self.p.rope {
            if c.up {
                r.len = (r.len - m.reel * DT).max(MIN_ROPE);
            }
            if c.down {
                r.len = (r.len + m.reel * DT).min(m.hook_range * 1.1);
            }
            r.t += DT;
        }

        // a little rope drag: swings keep their height only if you pump them
        if roped && !self.p.on_ground {
            let k = 1.0 - 0.16 * DT;
            self.p.vx *= k;
            self.p.vy *= k;
        }
        // speed clamp
        let sp = (self.p.vx * self.p.vx + self.p.vy * self.p.vy).sqrt();
        if sp > MAX_SPEED {
            self.p.vx *= MAX_SPEED / sp;
            self.p.vy *= MAX_SPEED / sp;
        }
        self.p.speed = sp.min(MAX_SPEED);
        self.p.charged = self.p.speed >= m.smash_v;

        // move
        let (vx, vy) = (self.p.vx, self.p.vy);
        let blocked_x = self.move_x(vx * DT);
        let vy = self.p.vy.max(vy.min(self.p.vy + 0.0001)); // vy may have changed
        let _ = vy;
        let landed = self.move_y(self.p.vy * DT);
        let _ = blocked_x;

        // rope constraint
        if let Some(r) = self.p.rope {
            let dx = self.p.x - r.ax;
            let dy = self.p.y - r.ay;
            let d = (dx * dx + dy * dy).sqrt().max(0.001);
            if d > r.len {
                let nx = dx / d;
                let ny = dy / d;
                let over = d - r.len;
                let (cx, cy) = (-nx * over, -ny * over);
                self.move_x(cx);
                self.move_y(cy);
                let vr = self.p.vx * nx + self.p.vy * ny;
                if vr > 0.0 {
                    self.p.vx -= vr * nx;
                    self.p.vy -= vr * ny;
                }
            } else if let Some(rr) = &mut self.p.rope {
                // take up slack so the rope stays taut
                rr.len = d.max(MIN_ROPE);
            }
            if !self.ray_clear(self.p.x, self.p.y - 4.0, r.ax, r.ay) {
                self.p.rope = None;
                self.emit(Ev::RopeSnap, self.p.x, self.p.y, 0.0);
            }
        }

        // landing detection
        let (g2, _) = self.probe_ground();
        if landed > 0 || (g2 && !was_ground && self.p.vy >= 0.0) {
            if !was_ground {
                let impact = vy.abs().max(self.p.vy.abs());
                if impact > 200.0 || self.p.since_ground > 0.2 {
                    self.emit(Ev::Land, self.p.x, self.p.y + HH, impact);
                }
            }
            self.p.on_ground = true;
            self.p.vy = 0.0;
        }

        // stillness and safe spot
        if self.p.on_ground && !self.p.rope.is_some() {
            self.p.still += DT;
            if self.p.still > SAFE_STILL {
                let tx_l = ((self.p.x - HW) / T).floor() as i32;
                let tx_r = ((self.p.x + HW - 0.01) / T).floor() as i32;
                let ty = ((self.p.y + HH + 1.0) / T).floor() as i32;
                let solid_both = [tx_l, tx_r]
                    .iter()
                    .all(|&tx| matches!(self.level.tile(tx, ty), ROCK | ICE));
                if solid_both && !self.hazard_near(self.p.x, self.p.y, 70.0) {
                    self.p.safe = (self.p.x, self.p.y);
                }
            }
        } else {
            self.p.still = 0.0;
        }
        if self.p.on_ground {
            self.p.run_t += self.p.vx.abs() * DT * 0.06;
        }

        // hazards: spikes and pits
        self.check_spikes();
        if self.p.y > self.level.pit_y() {
            self.fall_into_pit();
        }
    }

    fn hazard_near(&self, x: f32, y: f32, r: f32) -> bool {
        let x0 = ((x - r) / T).floor() as i32;
        let x1 = ((x + r) / T).floor() as i32;
        let y0 = ((y - r) / T).floor() as i32;
        let y1 = ((y + r) / T).floor() as i32;
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                if is_spike(self.level.tile(tx, ty)) {
                    return true;
                }
            }
        }
        for e in &self.ents {
            if e.alive
                && matches!(
                    e.kind,
                    EntKind::Skitter
                        | EntKind::Bulwark
                        | EntKind::Saw
                        | EntKind::Wisp
                        | EntKind::Bolt
                        | EntKind::Geyser
                )
                && (e.x - x).abs() < r + 40.0
                && (e.y - y).abs() < r + 60.0
            {
                return true;
            }
        }
        false
    }

    fn apply_wind(&mut self) {
        let mut ax = 0.0;
        let mut ay = 0.0;
        let mut gust = 0.0;
        for e in &self.ents {
            if e.kind != EntKind::Wind || !e.alive {
                continue;
            }
            if (self.p.x - e.x).abs() < e.hw && (self.p.y - e.y).abs() < e.hh {
                let f = wind_factor(self.time, e.c);
                ax += e.a * f;
                ay += e.b * f;
                gust = f;
            }
        }
        self.gust = gust;
        self.p.vx += ax * DT;
        self.p.vy += ay * DT;
    }

    fn check_spikes(&mut self) {
        if self.p.invuln > 0.0 && self.p.hurt_t > 0.0 {
            return;
        }
        let x0 = ((self.p.x - HW + 2.0) / T).floor() as i32;
        let x1 = ((self.p.x + HW - 2.0) / T).floor() as i32;
        let y0 = ((self.p.y - HH + 2.0) / T).floor() as i32;
        let y1 = ((self.p.y + HH - 1.0) / T).floor() as i32;
        for ty in y0..=y1 {
            for tx in x0..=x1 {
                let t = self.level.tile(tx, ty);
                if !is_spike(t) {
                    continue;
                }
                // hitbox: the half of the tile the spikes occupy
                let (bx, by, bw, bh) = match t {
                    SPIKE_UP => (tx as f32 * T + 3.0, ty as f32 * T + 14.0, T - 6.0, 18.0),
                    SPIKE_DOWN => (tx as f32 * T + 3.0, ty as f32 * T, T - 6.0, 18.0),
                    SPIKE_LEFT => (tx as f32 * T, ty as f32 * T + 3.0, 18.0, T - 6.0),
                    _ => (tx as f32 * T + 14.0, ty as f32 * T + 3.0, 18.0, T - 6.0),
                };
                if self.p.x + HW > bx
                    && self.p.x - HW < bx + bw
                    && self.p.y + HH > by
                    && self.p.y - HH < by + bh
                {
                    self.hurt_by("spikes", self.p.x, self.p.y + 10.0, true);
                    return;
                }
            }
        }
    }

    pub fn hurt_by(&mut self, cause: &'static str, from_x: f32, from_y: f32, reset: bool) {
        if self.p.invuln <= 0.0 && self.p.dying <= 0.0 && !self.mods.invincible {
            self.last_cause = cause;
        }
        self.hurt(from_x, from_y, reset);
    }

    /// Lose a heart. `reset` moves Wren back to the last safe footing.
    pub fn hurt(&mut self, from_x: f32, from_y: f32, reset: bool) {
        if self.p.invuln > 0.0 || self.p.dying > 0.0 || self.mods.invincible {
            return;
        }
        let _ = from_y;
        self.p.rope = None;
        if self.p.shield {
            self.p.shield = false;
            self.p.invuln = 1.0;
            self.emit(Ev::Shield, self.p.x, self.p.y, 0.0);
            return;
        }
        self.p.hp -= 1;
        self.stats.damage += 1;
        let cause = self.last_cause;
        match self.stats.causes.iter_mut().find(|c| c.0 == cause) {
            Some(c) => c.1 += 1,
            None => self.stats.causes.push((cause, 1)),
        }
        self.stats.chain = 0;
        self.p.invuln = 1.5;
        self.p.hurt_t = 0.35;
        self.emit(Ev::Hurt, self.p.x, self.p.y, self.p.hp as f32);
        if self.p.hp <= 0 && self.mods.revive {
            self.mods.revive = false;
            self.p.hp = 1;
            self.p.invuln = 2.2;
            self.emit(Ev::Revive, self.p.x, self.p.y, 0.0);
            return;
        }
        if self.p.hp <= 0 {
            self.p.dying = 1.7;
            self.p.vy = -420.0;
            self.p.vx = if from_x > self.p.x { -120.0 } else { 120.0 };
            self.emit(Ev::Dying, self.p.x, self.p.y, 0.0);
            return;
        }
        if reset {
            let (sx, sy) = self.p.safe;
            self.p.x = sx;
            self.p.y = sy;
            self.p.vx = 0.0;
            self.p.vy = 0.0;
            self.p.invuln = 1.6;
        } else {
            let away = if self.p.x >= from_x { 1.0 } else { -1.0 };
            self.p.vx = away * 260.0;
            self.p.vy = -380.0;
        }
    }

    fn fall_into_pit(&mut self) {
        self.stats.falls += 1;
        self.stats.chain = 0;
        self.p.rope = None;
        self.emit(Ev::Pit, self.p.x, self.p.y, 0.0);
        let (sx, sy) = self.p.safe;
        self.p.x = sx;
        self.p.y = sy;
        self.p.vx = 0.0;
        self.p.vy = 0.0;
        self.p.invuln = self.p.invuln.max(0.8);
        self.stats.score = (self.stats.score - 40).max(0);
    }

    // ------------------------------------------------------------- entities

    fn update_ents(&mut self) {
        let (px, py) = (self.p.x, self.p.y);
        let mut spawn: Vec<Ent> = vec![];
        for i in 0..self.ents.len() {
            let mut e = self.ents[i];
            if !e.alive {
                continue;
            }
            let near = (e.x - px).abs() < 1100.0 && (e.y - py).abs() < 800.0;
            if !near && e.kind != EntKind::Saw {
                continue;
            }
            e.t += DT;
            match e.kind {
                EntKind::Skitter | EntKind::Bulwark => self.walker(&mut e),
                EntKind::Spitter => self.spitter(&mut e, &mut spawn),
                EntKind::Bolt => self.bolt(&mut e),
                EntKind::Saw => {
                    let s = 0.5 + 0.5 * (e.t * e.c + e.dir).sin();
                    e.x = e.home_x + (e.a - e.home_x) * s;
                    e.y = e.home_y + (e.b - e.home_y) * s;
                }
                EntKind::Geyser => self.geyser(&mut e),
                EntKind::Wisp => self.wisp(&mut e),
                EntKind::Strike => self.strike_zone(&mut e),
                EntKind::Wind => {
                    if e.dir < 0.0 {
                        e.vx -= DT;
                        if e.vx <= 0.0 {
                            e.alive = false;
                        }
                    }
                }
                EntKind::Crystal => {
                    let d = dist(e.x, e.y, px, py);
                    if d < self.mods.magnet * 3.2 && self.p.dying <= 0.0 {
                        let k = (1.0 - d / (self.mods.magnet * 3.2)).clamp(0.0, 1.0);
                        let s = 120.0 + 480.0 * k;
                        e.x += (px - e.x) / d.max(1.0) * s * DT;
                        e.y += (py - e.y) / d.max(1.0) * s * DT;
                    }
                }
                _ => {}
            }
            self.ents[i] = e;
        }
        self.ents.extend(spawn);
        self.ents.retain(|e| e.alive);
        self.player_vs_ents();
    }

    fn floor_ahead(&self, x: f32, y: f32, hh: f32, dir: f32) -> bool {
        // solid below the leading foot, and no wall ahead
        let fx = x + dir * 14.0;
        let below = self.solid_tile((fx / T).floor() as i32, ((y + hh + 4.0) / T).floor() as i32);
        let wall = self.solid_tile(
            ((x + dir * 16.0) / T).floor() as i32,
            ((y - 2.0) / T).floor() as i32,
        );
        below && !wall
    }

    fn walker(&mut self, e: &mut Ent) {
        if e.state == 9 {
            // dazed Bulwark
            e.vx = 0.0;
            if e.t > 0.5 {
                e.state = 0;
            }
            return;
        }
        let speed = if e.kind == EntKind::Skitter {
            58.0
        } else {
            40.0
        };
        if !self.floor_ahead(e.x, e.y, e.hh, e.dir) {
            e.dir = -e.dir;
        }
        e.x += e.dir * speed * DT;
    }

    fn spitter(&mut self, e: &mut Ent, spawn: &mut Vec<Ent>) {
        let (px, py) = (self.p.x, self.p.y);
        let d = dist(e.x, e.y, px, py);
        match e.state {
            0 => {
                // idle: wait for the player in range with a clear line
                e.b = (e.b - DT).max(0.0);
                if d < 460.0 && e.b <= 0.0 && self.ray_clear(e.x, e.y - 6.0, px, py) {
                    e.state = 1;
                    e.t = 0.0;
                    e.a = (py - e.y + 6.0).atan2(px - e.x);
                    self.emit(Ev::SpitterCharge, e.x, e.y, 0.0);
                }
            }
            1 => {
                // charge: aim line follows the player until the last moment
                let lead = (d / 260.0).min(0.9);
                let tx = px + self.p.vx * lead * 0.8;
                let ty = py + self.p.vy.clamp(-200.0, 200.0) * lead * 0.4;
                let target = (ty - e.y + 6.0).atan2(tx - e.x);
                if e.t < 0.55 {
                    // turn toward the target
                    let mut da = target - e.a;
                    while da > std::f32::consts::PI {
                        da -= std::f32::consts::TAU;
                    }
                    while da < -std::f32::consts::PI {
                        da += std::f32::consts::TAU;
                    }
                    e.a += da.clamp(-4.0 * DT, 4.0 * DT);
                }
                if e.t >= 0.8 {
                    e.state = 0;
                    e.b = 2.1;
                    self.emit(Ev::SpitterFire, e.x, e.y, 0.0);
                    let mut bolt = Ent::basic(EntKind::Bolt, e.x, e.y - 6.0, 5.0, 5.0);
                    bolt.vx = e.a.cos() * 250.0;
                    bolt.vy = e.a.sin() * 250.0;
                    bolt.hp = 1;
                    spawn.push(bolt);
                }
            }
            _ => {}
        }
    }

    fn bolt(&mut self, e: &mut Ent) {
        e.x += e.vx * DT;
        e.y += e.vy * DT;
        if e.t > 4.5 || self.solid_tile((e.x / T).floor() as i32, (e.y / T).floor() as i32) {
            e.alive = false;
        }
    }

    fn geyser(&mut self, e: &mut Ent) {
        let period = e.a.max(2.0);
        let ph = (self.time + e.b) % period;
        let old = e.state;
        e.state = if ph < period - 1.6 {
            0
        } else if ph < period - 0.9 {
            1
        } else {
            2
        };
        if e.state != old {
            if e.state == 1 {
                self.emit(Ev::GeyserWarn, e.x, e.y, 0.0);
            } else if e.state == 2 {
                self.emit(Ev::GeyserBurst, e.x, e.y, 0.0);
            }
        }
    }

    fn wisp(&mut self, e: &mut Ent) {
        let (px, py) = (self.p.x, self.p.y);
        let d = dist(e.x, e.y, px, py);
        let bob = (e.t * 2.3 + e.dir).sin() * 22.0;
        if d < 340.0 && d > 1.0 {
            e.x += (px - e.x) / d * 62.0 * DT;
            e.y += (py - e.y) / d * 62.0 * DT;
        } else {
            // drift back home
            let hd = dist(e.x, e.y, e.home_x, e.home_y);
            if hd > 4.0 {
                e.x += (e.home_x - e.x) / hd * 40.0 * DT;
                e.y += (e.home_y - e.y) / hd * 40.0 * DT;
            }
        }
        e.vy = bob;
    }

    fn strike_zone(&mut self, e: &mut Ent) {
        // state 0: waiting; 1: warning (a = x of strike); 2: striking
        let (px, py) = (self.p.x, self.p.y);
        match e.state {
            0 => {
                e.b = (e.b - DT).max(0.0);
                if e.b <= 0.0 && (px - e.x).abs() < e.hw && (py - e.y).abs() < e.hh {
                    e.state = 1;
                    e.t = 0.0;
                    e.a = (px + self.p.vx * 0.45).clamp(e.x - e.hw, e.x + e.hw);
                    e.c = py + 40.0;
                    self.emit(Ev::StrikeWarn, e.a, e.c, 0.0);
                }
            }
            1 => {
                if e.t >= 1.05 {
                    e.state = 2;
                    e.t = 0.0;
                    self.emit(Ev::StrikeHit, e.a, e.c, 0.0);
                }
            }
            _ => {
                if e.t >= 0.28 {
                    e.state = 0;
                    e.b = 2.4;
                    if e.hw == 0.0 {
                        e.alive = false;
                    }
                }
            }
        }
    }

    fn player_vs_ents(&mut self) {
        if self.p.dying > 0.0 {
            return;
        }
        let (px, py) = (self.p.x, self.p.y);
        for i in 0..self.ents.len() {
            let mut e = self.ents[i];
            if !e.alive {
                continue;
            }
            match e.kind {
                EntKind::Skitter | EntKind::Bulwark | EntKind::Wisp | EntKind::Spitter => {
                    let ox = (px - e.x).abs() < HW + e.hw - 2.0;
                    let oy = (py - e.y).abs() < HH + e.hh - 2.0;
                    if !(ox && oy) {
                        continue;
                    }
                    let stomp = self.p.vy > 40.0 && (py + HH) - (e.y - e.hh) < 16.0;
                    if stomp {
                        if e.kind == EntKind::Bulwark {
                            if self.p.charged {
                                self.kill_ent(&mut e, true);
                            } else {
                                // bounce off its shield
                                self.p.vy = -520.0;
                                e.state = 9;
                                e.t = 0.0;
                                self.emit(Ev::Bounce, e.x, e.y - e.hh, 0.0);
                            }
                        } else {
                            self.kill_ent(&mut e, false);
                            self.p.vy = if self.p.jumping { -640.0 } else { -520.0 };
                            self.p.air_jumps = self.mods.air_jumps;
                        }
                    } else if self.p.charged {
                        self.kill_ent(&mut e, true);
                    } else if e.state != 9 && e.kind != EntKind::Spitter {
                        // turrets only hurt through their bolts
                        self.hurt_by(e.kind.name(), e.x, e.y, false);
                    }
                }
                EntKind::Saw => {
                    if dist(px, py, e.x, e.y) < 17.0 + 12.0 {
                        self.hurt_by(e.kind.name(), e.x, e.y, false);
                    }
                }
                EntKind::Bolt => {
                    if (px - e.x).abs() < HW + 4.0 && (py - e.y).abs() < HH + 4.0 {
                        e.alive = false;
                        self.hurt_by(e.kind.name(), e.x, e.y, false);
                    }
                }
                EntKind::Geyser => {
                    if e.state == 2 && (px - e.x).abs() < HW + 15.0 {
                        let top = e.y - 170.0;
                        if py + HH > top && py - HH < e.y {
                            self.hurt_by(e.kind.name(), e.x, e.y, false);
                        }
                    }
                }
                EntKind::Strike => {
                    if e.state == 2 && (px - e.a).abs() < HW + 22.0 {
                        self.hurt_by("lightning", e.a, e.c, false);
                    }
                }
                EntKind::Bloom => {
                    if self.p.vy >= 0.0
                        && (px - e.x).abs() < HW + e.hw
                        && (py + HH - (e.y - e.hh)).abs() < 14.0
                    {
                        self.p.vy = -930.0;
                        self.p.jumping = false;
                        self.p.rope = None;
                        e.t = 0.0;
                        e.state = 1;
                        self.emit(Ev::Bounce, e.x, e.y - e.hh, 1.0);
                    } else if e.t > 0.4 {
                        e.state = 0;
                    }
                }
                _ => {}
            }
            self.ents[i] = e;
        }
    }

    fn kill_ent(&mut self, e: &mut Ent, ram: bool) {
        e.alive = false;
        self.stats.kills += 1;
        self.stats.score += if ram { 60 } else { 40 };
        self.bump_chain(2, e.x, e.y);
        if ram {
            self.stats.rams += 1;
            self.emit(Ev::Ram, e.x, e.y, self.p.speed);
        } else {
            self.emit(Ev::Stomp, e.x, e.y - e.hh, 0.0);
        }
    }

    fn update_pickups_and_flags(&mut self) {
        let (px, py) = (self.p.x, self.p.y);
        let mut got = vec![];
        for (i, e) in self.ents.iter().enumerate() {
            if !e.alive {
                continue;
            }
            match e.kind {
                EntKind::Crystal | EntKind::EchoOrb | EntKind::HeartShard | EntKind::Chest => {
                    let r = match e.kind {
                        EntKind::Chest => 26.0,
                        EntKind::Crystal => 20.0,
                        _ => 24.0,
                    };
                    if (px - e.x).abs() < HW + r - 4.0 && (py - e.y).abs() < HH + r - 4.0 {
                        got.push(i);
                    }
                }
                _ => {}
            }
        }
        for i in got {
            let e = self.ents[i];
            self.ents[i].alive = false;
            match e.kind {
                EntKind::Crystal => {
                    self.bump_chain(1, e.x, e.y);
                    let mult = self.stats.mult();
                    let v = (10.0 * self.mods.crystal_value * mult as f32) as i64;
                    self.stats.score += v;
                    self.stats.crystals += 1;
                    self.emit(Ev::Crystal, e.x, e.y, v as f32);
                }
                EntKind::EchoOrb => {
                    self.stats.score += 50;
                    self.emit(Ev::EchoOrb, e.x, e.y, 0.0);
                }
                EntKind::HeartShard => {
                    self.stats.score += 50;
                    if self.p.hp < self.p.max_hp {
                        self.p.hp += 1;
                    }
                    self.emit(Ev::HeartShard, e.x, e.y, 0.0);
                }
                EntKind::Chest => {
                    self.stats.chests += 1;
                    self.stats.score += 150;
                    self.emit(Ev::Chest, e.x, e.y, 0.0);
                }
                _ => {}
            }
        }
        // checkpoints
        while self.cp_next < self.level.checkpoints.len() {
            let (cx, cy) = self.level.checkpoints[self.cp_next];
            if self.p.x >= cx {
                self.cp_next += 1;
                self.p.safe = (cx, cy - HH - 1.0);
                self.emit(Ev::Checkpoint, cx, cy, self.cp_next as f32);
            } else {
                break;
            }
        }
        // exit gate
        if !self.level.boss || self.gate_open {
            let (ex, ey) = self.level.exit;
            if (self.p.x - ex).abs() < 34.0 && (self.p.y - ey).abs() < 64.0 && self.p.dying <= 0.0 {
                self.outcome = Outcome::Cleared;
                self.emit(Ev::Cleared, ex, ey, 0.0);
            }
        }
    }

    /// Progress through the level in 0..1.
    pub fn progress(&self) -> f32 {
        let (sx, _) = self.level.spawn;
        let (ex, _) = self.level.exit;
        ((self.p.x - sx) / (ex - sx).max(1.0)).clamp(0.0, 1.0)
    }

    /// Hearts the player may regain from this point (for UI).
    pub fn threats_alive(&self) -> usize {
        self.ents
            .iter()
            .filter(|e| e.alive && e.kind.is_threat())
            .count()
    }
}

/// Smooth square-ish wave in -1..1 with period `c` seconds (0 = constant 1).
pub fn wind_factor(time: f32, c: f32) -> f32 {
    if c <= 0.0 {
        return 1.0;
    }
    let s = (time * std::f32::consts::TAU / c).sin();
    (s * 3.0).clamp(-1.0, 1.0)
}

pub fn approach(v: f32, target: f32, delta: f32) -> f32 {
    if v < target {
        (v + delta).min(target)
    } else {
        (v - delta).max(target)
    }
}

impl Ent {
    pub fn basic(kind: EntKind, x: f32, y: f32, hw: f32, hh: f32) -> Ent {
        Ent {
            kind,
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            hw,
            hh,
            t: 0.0,
            hp: 1,
            state: 0,
            dir: 1.0,
            a: 0.0,
            b: 0.0,
            c: 0.0,
            home_x: x,
            home_y: y,
            alive: true,
        }
    }

    pub fn from_spawn(s: &EntSpawn) -> Ent {
        let (hw, hh) = match s.kind {
            EntKind::Skitter => (12.0, 8.0),
            EntKind::Bulwark => (16.0, 13.0),
            EntKind::Spitter => (13.0, 13.0),
            EntKind::Wisp => (11.0, 11.0),
            EntKind::Crystal => (8.0, 10.0),
            EntKind::EchoOrb | EntKind::HeartShard => (10.0, 10.0),
            EntKind::Chest => (16.0, 14.0),
            EntKind::Bloom => (16.0, 8.0),
            EntKind::Saw => (17.0, 17.0),
            EntKind::Geyser => (14.0, 6.0),
            EntKind::Wind | EntKind::Strike => (s.hw, s.hh),
            EntKind::Bolt => (5.0, 5.0),
        };
        let mut e = Ent::basic(s.kind, s.x, s.y, hw, hh);
        e.a = s.a;
        e.b = s.b;
        e.c = s.c;
        match s.kind {
            EntKind::Saw => {
                // a, b = other end; c = angular speed; dir = phase
                e.dir = s.hw;
            }
            EntKind::Skitter | EntKind::Bulwark => {
                e.dir = if s.a >= 0.0 { 1.0 } else { -1.0 };
            }
            EntKind::Wisp => e.dir = s.a,
            EntKind::Strike => {
                e.hw = s.hw;
                e.hh = s.hh;
                e.a = 0.0;
                e.b = 0.0;
                e.c = 0.0;
            }
            EntKind::Wind => {
                e.hw = s.hw;
                e.hh = s.hh;
            }
            _ => {}
        }
        e
    }
}
