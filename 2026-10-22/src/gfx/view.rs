//! Presentation state: plays simulation events back as timed animation
//! steps, owns particles, floating text, screen shake and smoothed light.

use std::collections::{HashMap, VecDeque};

use super::iso::{pos_center, surface, tile_center};
use super::palette::Palette;
use crate::audio::Sfx;
use crate::battle::{Battle, Event, Fx, Outcome};
use crate::map::{Map, Pickup, Pos, Terrain};
use crate::rng::Rng;
use crate::units::{Kind, Team};

#[derive(Clone, Debug)]
pub struct UnitVis {
    pub x: f32,
    pub y: f32,
    pub h: f32,
    pub alpha: f32,
    pub flash: f32,
    pub lunge: (f32, f32),
    pub hop: f32,
    pub drop: f32,
    pub spawn: f32,
    pub hp: i32,
    pub visible: bool,
    pub guard_fx: f32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum PKind {
    Dot,
    Spark,
    Smoke,
    Ring,
}

#[derive(Clone, Debug)]
pub struct Particle {
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub life: f32,
    pub max: f32,
    pub color: u32,
    pub size: f32,
    pub additive: bool,
    pub gravity: f32,
    pub kind: PKind,
}

#[derive(Clone, Debug)]
pub struct FloatText {
    pub x: f32,
    pub y: f32,
    pub text: String,
    pub color: u32,
    pub life: f32,
    pub max: f32,
    pub scale: f32,
}

#[derive(Clone, Debug)]
pub struct Projectile {
    pub from: (f32, f32),
    pub to: (f32, f32),
    pub t: f32,
    pub fx: Fx,
}

#[derive(Clone, Debug)]
pub struct Banner {
    pub small: bool,
    pub text: String,
    pub sub: String,
    pub color: u32,
    pub life: f32,
    pub max: f32,
}

#[derive(Clone, Debug)]
enum Step {
    Walk { id: u32, path: Vec<Pos> },
    Lunge { id: u32, to: Pos, fx: Fx },
    Shoot { id: u32, from: Pos, to: Pos, fx: Fx },
    Ev(Event),
}

pub struct View {
    pub vis: HashMap<u32, UnitVis>,
    steps: VecDeque<Step>,
    cur: Option<(Step, f32, f32)>,
    pub particles: Vec<Particle>,
    pub texts: Vec<FloatText>,
    pub projectile: Option<Projectile>,
    pub shake: f32,
    pub flash: f32,
    pub flash_color: u32,
    pub banner: Option<Banner>,
    pub time: f32,
    pub glow: Vec<f32>,
    pub sfx: Vec<Sfx>,
    pub eclipse: f32,
    pub eclipse_at: (f32, f32),
    pub chain: Option<(u32, f32)>,
    rng: Rng,
    pending_spawn: Vec<u32>,
    ambient_acc: f32,
}

impl Default for View {
    fn default() -> Self {
        View::new()
    }
}

impl View {
    pub fn new() -> View {
        View {
            vis: HashMap::new(),
            steps: VecDeque::new(),
            cur: None,
            particles: vec![],
            texts: vec![],
            projectile: None,
            shake: 0.0,
            flash: 0.0,
            flash_color: 0xffffff,
            banner: None,
            time: 0.0,
            glow: vec![],
            sfx: vec![],
            eclipse: 0.0,
            eclipse_at: (0.0, 0.0),
            chain: None,
            rng: Rng::new(99),
            pending_spawn: vec![],
            ambient_acc: 0.0,
        }
    }

    /// Snap everything to the battle's current state.
    pub fn reset(&mut self, b: &Battle) {
        self.vis.clear();
        self.steps.clear();
        self.cur = None;
        self.projectile = None;
        self.pending_spawn.clear();
        self.texts.clear();
        self.chain = None;
        self.eclipse = 0.0;
        self.glow = b.glow.clone();
        self.sync(b, true);
    }

    pub fn busy(&self) -> bool {
        self.cur.is_some() || !self.steps.is_empty()
    }

    pub fn banner(&mut self, text: &str, sub: &str, color: u32, secs: f32) {
        self.banner = Some(Banner {
            small: false,
            text: text.to_string(),
            sub: sub.to_string(),
            color,
            life: secs,
            max: secs,
        });
    }

    pub fn toast(&mut self, text: &str, color: u32, secs: f32) {
        self.banner = Some(Banner {
            small: true,
            text: text.to_string(),
            sub: String::new(),
            color,
            life: secs,
            max: secs,
        });
    }

    pub fn feed(&mut self, events: Vec<Event>) {
        for e in events {
            match e {
                Event::Moved { id, path } => self.steps.push_back(Step::Walk { id, path }),
                Event::Attack { id, from, to, fx } => match fx {
                    Fx::Arrow | Fx::Volley | Fx::Bolt | Fx::Flare | Fx::Curse => {
                        self.steps.push_back(Step::Shoot { id, from, to, fx })
                    }
                    _ => self.steps.push_back(Step::Lunge { id, to, fx }),
                },
                Event::Spawned { id, .. } => {
                    self.pending_spawn.push(id);
                    self.steps.push_back(Step::Ev(e));
                }
                other => self.steps.push_back(Step::Ev(other)),
            }
        }
    }

    fn sync(&mut self, b: &Battle, force: bool) {
        for u in &b.units {
            if self.pending_spawn.contains(&u.id) {
                continue;
            }
            let h = surface(&b.map, u.pos);
            let e = self.vis.entry(u.id).or_insert_with(|| UnitVis {
                x: u.pos.x as f32,
                y: u.pos.y as f32,
                h,
                alpha: 1.0,
                flash: 0.0,
                lunge: (0.0, 0.0),
                hop: 0.0,
                drop: 0.0,
                spawn: 1.0,
                hp: u.hp,
                visible: u.alive,
                guard_fx: 0.0,
            });
            if force || u.alive {
                e.x = u.pos.x as f32;
                e.y = u.pos.y as f32;
                e.h = h;
                e.hop = 0.0;
            }
            if force {
                e.hp = u.hp;
                e.visible = u.alive;
                e.alpha = 1.0;
                e.drop = 0.0;
                e.spawn = 1.0;
            } else if u.alive {
                e.hp = u.hp;
                e.visible = true;
            }
        }
    }

    fn duration(step: &Step, b: &Battle) -> f32 {
        match step {
            Step::Walk { path, .. } => 0.11 * (path.len().saturating_sub(1)) as f32,
            Step::Lunge { .. } => 0.15,
            Step::Shoot { from, to, .. } => 0.12 + from.manhattan(*to) as f32 * 0.05,
            Step::Ev(e) => match e {
                Event::Damaged { boss, .. } => {
                    if *boss {
                        0.26
                    } else {
                        0.12
                    }
                }
                Event::Healed { .. } => 0.22,
                Event::Pushed { .. } => 0.15,
                Event::Bumped { .. } => 0.1,
                Event::Fell { .. } => 0.55,
                Event::Died { id } => {
                    if b.unit(*id).team() == Team::Squad {
                        0.6
                    } else {
                        0.32
                    }
                }
                Event::BrazierLit { .. } => 0.22,
                Event::BrazierSnuffed { .. } => 0.25,
                Event::Spawned { .. } => 0.3,
                Event::Eclipse { .. } => 0.8,
                Event::Guard { .. } => 0.12,
                Event::Weakened { .. } => 0.1,
                Event::Picked { .. } => 0.15,
                Event::Chain { .. } => 0.12,
                Event::RoundStart { .. } => 0.05,
                Event::Gloom => 0.9,
                Event::LongNight { .. } => 0.8,
                Event::Reinforcements => 0.5,
                Event::EnemyTurn => 0.45,
                Event::Outcome(_) => 0.5,
                Event::Moved { .. } | Event::Attack { .. } => 0.0,
            },
        }
    }

    pub fn update(&mut self, dt: f32, b: &Battle, pal: &Palette) {
        self.time += dt;
        // Smooth light towards the simulation's light.
        if self.glow.len() != b.glow.len() {
            self.glow = b.glow.clone();
        }
        let k = (dt * 5.0).min(1.0);
        for (g, t) in self.glow.iter_mut().zip(b.glow.iter()) {
            *g += (t - *g) * k;
        }
        let mut budget = dt;
        let mut guard = 0;
        loop {
            guard += 1;
            if guard > 64 {
                break;
            }
            if self.cur.is_none() {
                let Some(step) = self.steps.pop_front() else {
                    break;
                };
                let dur = View::duration(&step, b);
                self.start(&step, b, pal);
                self.cur = Some((step, 0.0, dur));
            }
            let (step, mut t, dur) = self.cur.take().expect("cur");
            let used = budget.min((dur - t).max(0.0));
            t += used;
            budget -= used;
            self.progress(&step, if dur > 0.0 { t / dur } else { 1.0 }, b);
            if t >= dur {
                self.finish(&step, b);
            } else {
                self.cur = Some((step, t, dur));
                break;
            }
            if budget <= 0.0 && dur > 0.0 {
                break;
            }
        }
        if !self.busy() {
            self.sync(b, false);
        }
        self.tick_fx(dt, b, pal);
    }

    fn vis_pos(&self, id: u32) -> (f32, f32) {
        match self.vis.get(&id) {
            Some(v) => tile_center(v.x, v.y, v.h),
            None => (0.0, 0.0),
        }
    }

    fn start(&mut self, step: &Step, b: &Battle, pal: &Palette) {
        match step {
            Step::Walk { .. } => {}
            Step::Lunge { fx, .. } => self.sfx.push(match fx {
                Fx::Slash => Sfx::Slash,
                Fx::Bash => Sfx::Bash,
                Fx::Claw => Sfx::Claw,
                Fx::Slam | Fx::Rend => Sfx::Slam,
                _ => Sfx::Slash,
            }),
            Step::Shoot { id, from, to, fx } => {
                let mut a = pos_center(&b.map, *from);
                a.1 -= if b.unit(*id).kind == Kind::Wisp {
                    34.0
                } else {
                    22.0
                };
                let mut c = pos_center(&b.map, *to);
                c.1 -= 16.0;
                self.projectile = Some(Projectile {
                    from: a,
                    to: c,
                    t: 0.0,
                    fx: *fx,
                });
                self.sfx.push(match fx {
                    Fx::Arrow | Fx::Volley => Sfx::Arrow,
                    Fx::Flare => Sfx::Flare,
                    Fx::Bolt => Sfx::Bolt,
                    _ => Sfx::Curse,
                });
            }
            Step::Ev(e) => self.start_event(e, b, pal),
        }
    }

    fn progress(&mut self, step: &Step, p: f32, b: &Battle) {
        match step {
            Step::Walk { id, path } => {
                if path.len() < 2 {
                    return;
                }
                let segs = (path.len() - 1) as f32;
                let f = (p * segs).min(segs - 0.0001);
                let i = f.floor() as usize;
                let local = f - i as f32;
                let a = path[i];
                let c = path[i + 1];
                let ha = surface(&b.map, a);
                let hc = surface(&b.map, c);
                let before = self.vis.get(id).map(|v| (v.x, v.y)).unwrap_or_default();
                if let Some(v) = self.vis.get_mut(id) {
                    v.x = a.x as f32 + (c.x - a.x) as f32 * local;
                    v.y = a.y as f32 + (c.y - a.y) as f32 * local;
                    v.h = ha + (hc - ha) * local;
                    v.hop = (local * std::f32::consts::PI).sin() * 5.0;
                }
                // Footstep when crossing into a new tile.
                let crossed = (before.0.round() as i32, before.1.round() as i32)
                    != (
                        (a.x as f32 + (c.x - a.x) as f32 * local).round() as i32,
                        (a.y as f32 + (c.y - a.y) as f32 * local).round() as i32,
                    );
                if crossed {
                    self.sfx.push(Sfx::Step);
                }
            }
            Step::Lunge { id, to, .. } => {
                let (ax, ay) = self.vis_pos(*id);
                let (cx, cy) = pos_center(&b.map, *to);
                let (dx, dy) = (cx - ax, cy - ay);
                let len = (dx * dx + dy * dy).sqrt().max(1.0);
                let k = (p * std::f32::consts::PI).sin().max(0.0) * 12.0
                    + if p > 0.5 { 6.0 } else { 0.0 };
                if let Some(v) = self.vis.get_mut(id) {
                    v.lunge = (dx / len * k, dy / len * k);
                }
            }
            Step::Shoot { .. } => {
                if let Some(pr) = self.projectile.as_mut() {
                    pr.t = p;
                }
            }
            Step::Ev(Event::Pushed { id, from, to }) => {
                let hf = surface(&b.map, *from);
                let ht = if b.map.terrain(*to).lethal() {
                    hf
                } else {
                    surface(&b.map, *to)
                };
                if let Some(v) = self.vis.get_mut(id) {
                    let e = 1.0 - (1.0 - p) * (1.0 - p);
                    v.x = from.x as f32 + (to.x - from.x) as f32 * e;
                    v.y = from.y as f32 + (to.y - from.y) as f32 * e;
                    v.h = hf + (ht - hf) * e;
                }
            }
            Step::Ev(Event::Fell { id, .. }) => {
                if let Some(v) = self.vis.get_mut(id) {
                    v.drop = p * p * 260.0;
                    v.alpha = 1.0 - p;
                }
            }
            Step::Ev(Event::Died { id }) => {
                if let Some(v) = self.vis.get_mut(id) {
                    v.alpha = (1.0 - p).min(v.alpha);
                }
            }
            Step::Ev(Event::Spawned { id, .. }) => {
                if let Some(v) = self.vis.get_mut(id) {
                    v.spawn = p;
                }
            }
            Step::Ev(Event::Eclipse { .. }) => {
                self.eclipse = p;
            }
            _ => {}
        }
    }

    fn finish(&mut self, step: &Step, b: &Battle) {
        match step {
            Step::Walk { id, path } => {
                if let (Some(v), Some(last)) = (self.vis.get_mut(id), path.last()) {
                    v.x = last.x as f32;
                    v.y = last.y as f32;
                    v.h = surface(&b.map, *last);
                    v.hop = 0.0;
                }
            }
            Step::Shoot { .. } => {
                self.projectile = None;
            }
            Step::Ev(Event::Died { id }) => {
                if let Some(v) = self.vis.get_mut(id) {
                    v.visible = false;
                }
            }
            Step::Ev(Event::Fell { id, .. }) => {
                if let Some(v) = self.vis.get_mut(id) {
                    v.visible = false;
                }
            }
            Step::Ev(Event::Eclipse { .. }) => {
                self.eclipse = 0.0;
            }
            _ => {}
        }
    }

    fn start_event(&mut self, e: &Event, b: &Battle, pal: &Palette) {
        match e {
            Event::Damaged {
                id,
                amount,
                hp,
                shrouded,
                boss,
            } => {
                let (x, y) = self.vis_pos(*id);
                let squad = b.unit(*id).team() == Team::Squad;
                if let Some(v) = self.vis.get_mut(id) {
                    v.hp = *hp;
                    v.flash = 1.0;
                }
                let col = if squad { 0xff5a4a } else { 0xffe08a };
                self.text(
                    x,
                    y - 46.0,
                    &format!("-{amount}"),
                    col,
                    if *boss { 3.0 } else { 2.0 },
                );
                if *shrouded {
                    self.text(x, y - 64.0, "SHROUDED", 0xb090ff, 1.0);
                }
                let n = if *boss { 36 } else { 14 };
                let spark = if squad { 0xff7060 } else { 0xfff0c0 };
                for _ in 0..n {
                    self.burst(x, y - 18.0, spark, 160.0, 0.4, PKind::Spark, true);
                }
                if b.unit(*id).team() == Team::Dark {
                    for _ in 0..6 {
                        self.burst(x, y - 16.0, 0x2a1838, 60.0, 0.7, PKind::Smoke, false);
                    }
                }
                self.shake += if *boss {
                    9.0
                } else if squad {
                    5.0
                } else {
                    3.0
                };
                if *boss {
                    self.flash = 0.35;
                    self.flash_color = 0xffffff;
                    self.sfx.push(Sfx::BossHit);
                } else if squad {
                    self.sfx.push(Sfx::Hurt);
                    self.flash = 0.18;
                    self.flash_color = 0xff2020;
                } else {
                    self.sfx.push(Sfx::Hit);
                }
            }
            Event::Healed { id, amount, hp } => {
                let (x, y) = self.vis_pos(*id);
                if let Some(v) = self.vis.get_mut(id) {
                    v.hp = *hp;
                }
                self.text(x, y - 46.0, &format!("+{amount}"), 0x8affa0, 2.0);
                for _ in 0..16 {
                    self.burst(x, y - 10.0, 0x7aff9a, 50.0, 0.9, PKind::Dot, true);
                }
                self.sfx.push(Sfx::Heal);
            }
            Event::Bumped { at, .. } => {
                let (x, y) = pos_center(&b.map, *at);
                for _ in 0..10 {
                    self.burst(x, y - 6.0, 0xb0a090, 70.0, 0.5, PKind::Smoke, false);
                }
                self.shake += 4.0;
                self.sfx.push(Sfx::Bash);
            }
            Event::Fell { at, water, .. } => {
                if *water {
                    let (x, y) = pos_center(&b.map, *at);
                    for _ in 0..24 {
                        self.burst(x, y, pal.water_hi, 110.0, 0.7, PKind::Dot, false);
                    }
                    self.sfx.push(Sfx::Splash);
                } else {
                    self.sfx.push(Sfx::Fall);
                }
            }
            Event::Died { id } => {
                let (x, y) = self.vis_pos(*id);
                let u = b.unit(*id);
                if u.team() == Team::Squad {
                    for _ in 0..40 {
                        self.burst(x, y - 20.0, 0xffd080, 120.0, 1.2, PKind::Dot, true);
                    }
                    self.sfx.push(Sfx::SquadDeath);
                    self.shake += 6.0;
                } else {
                    let big = u.kind == Kind::HollowKing;
                    let n = if big { 120 } else { 26 };
                    for _ in 0..n {
                        self.burst(
                            x,
                            y - 20.0,
                            0x1a0e24,
                            if big { 200.0 } else { 90.0 },
                            1.1,
                            PKind::Smoke,
                            false,
                        );
                    }
                    for _ in 0..n / 2 {
                        self.burst(x, y - 20.0, pal.accent, 140.0, 0.8, PKind::Spark, true);
                    }
                    self.ring(x, y, 0xffe0a0);
                    if big {
                        self.shake += 18.0;
                        self.flash = 0.8;
                        self.flash_color = 0xffffff;
                    }
                    self.sfx.push(Sfx::Death);
                }
            }
            Event::BrazierLit { at } => {
                let (x, y) = pos_center(&b.map, *at);
                for _ in 0..30 {
                    self.burst(x, y - 26.0, 0xffa040, 120.0, 0.8, PKind::Spark, true);
                }
                self.ring(x, y - 10.0, 0xffc060);
                self.sfx.push(Sfx::Kindle);
            }
            Event::BrazierSnuffed { at } => {
                let (x, y) = pos_center(&b.map, *at);
                for _ in 0..22 {
                    self.burst(x, y - 26.0, 0x6a6070, 50.0, 1.4, PKind::Smoke, false);
                }
                self.sfx.push(Sfx::Snuff);
            }
            Event::Spawned { id, at } => {
                self.pending_spawn.retain(|x| x != id);
                let u = b.unit(*id);
                let h = surface(&b.map, *at);
                self.vis.insert(
                    *id,
                    UnitVis {
                        x: at.x as f32,
                        y: at.y as f32,
                        h,
                        alpha: 1.0,
                        flash: 0.0,
                        lunge: (0.0, 0.0),
                        hop: 0.0,
                        drop: 0.0,
                        spawn: 0.0,
                        hp: u.max_hp,
                        visible: true,
                        guard_fx: 0.0,
                    },
                );
                let (x, y) = pos_center(&b.map, *at);
                for _ in 0..20 {
                    self.burst(x, y - 10.0, 0x3a1850, 60.0, 0.9, PKind::Smoke, false);
                }
                self.sfx.push(Sfx::Summon);
            }
            Event::Eclipse { at } => {
                self.eclipse_at = pos_center(&b.map, *at);
                self.shake += 14.0;
                self.flash = 0.6;
                self.flash_color = 0x000000;
                self.sfx.push(Sfx::Eclipse);
            }
            Event::Guard { id } => {
                if let Some(v) = self.vis.get_mut(id) {
                    v.guard_fx = 1.0;
                }
                self.sfx.push(Sfx::Guard);
            }
            Event::Weakened { id } => {
                let (x, y) = self.vis_pos(*id);
                self.text(x, y - 62.0, "WEAKENED", 0xc070ff, 1.0);
            }
            Event::Picked { at, kind, .. } => {
                let (x, y) = pos_center(&b.map, *at);
                let c = match kind {
                    Pickup::Ember => 0xff9040,
                    Pickup::Lumen => 0xfff0a0,
                };
                for _ in 0..20 {
                    self.burst(x, y - 14.0, c, 80.0, 0.7, PKind::Spark, true);
                }
                self.text(x, y - 70.0, kind.name(), c, 1.0);
                self.sfx.push(Sfx::Pickup);
            }
            Event::Chain { count } => {
                self.chain = Some((*count, 1.4));
                self.sfx.push(Sfx::Chain(*count));
            }
            Event::RoundStart { round } => {
                if b.phase == crate::battle::Phase::Player {
                    self.toast(&format!("ROUND {round} - YOUR MOVE"), 0xffe6a0, 1.2);
                    self.sfx.push(Sfx::Turn);
                }
            }
            Event::Gloom => {
                self.banner("THE GLOOM DEEPENS", "Lanterns dim by 1", 0xb090ff, 1.8);
                self.sfx.push(Sfx::Gloom);
            }
            Event::LongNight { damage } => {
                self.banner(
                    "THE LONG NIGHT",
                    &format!("The dark bites the squad for {damage}"),
                    0xff6060,
                    1.6,
                );
                self.flash = 0.4;
                self.flash_color = 0x400010;
                self.sfx.push(Sfx::Gloom);
            }
            Event::Reinforcements => {
                self.banner(
                    "THE DARK STIRS",
                    "More shades crawl from the edges",
                    0xd080ff,
                    1.4,
                );
                self.sfx.push(Sfx::Summon);
            }
            Event::EnemyTurn => {
                self.toast("THE DARK MOVES", 0xc8a0ff, 1.0);
                self.sfx.push(Sfx::Banner);
            }
            Event::Outcome(o) => match o {
                Outcome::Victory => {
                    self.banner("ISLAND CLEANSED", "", 0xffe6a0, 2.4);
                    self.sfx.push(Sfx::Victory);
                }
                Outcome::Defeat => {
                    self.banner("THE LIGHT GOES OUT", "", 0xff6a6a, 2.4);
                    self.sfx.push(Sfx::Defeat);
                }
            },
            Event::Pushed { .. } | Event::Moved { .. } | Event::Attack { .. } => {}
        }
    }

    fn text(&mut self, x: f32, y: f32, s: &str, color: u32, scale: f32) {
        self.texts.push(FloatText {
            x,
            y,
            text: s.to_string(),
            color,
            life: 1.1,
            max: 1.1,
            scale,
        });
    }

    fn ring(&mut self, x: f32, y: f32, color: u32) {
        self.particles.push(Particle {
            x,
            y,
            vx: 0.0,
            vy: 0.0,
            life: 0.45,
            max: 0.45,
            color,
            size: 40.0,
            additive: true,
            gravity: 0.0,
            kind: PKind::Ring,
        });
    }

    #[allow(clippy::too_many_arguments)]
    fn burst(
        &mut self,
        x: f32,
        y: f32,
        color: u32,
        speed: f32,
        life: f32,
        kind: PKind,
        additive: bool,
    ) {
        let a = self.rng.f32() * std::f32::consts::TAU;
        let s = speed * (0.3 + self.rng.f32() * 0.7);
        let l = life * (0.5 + self.rng.f32() * 0.5);
        self.particles.push(Particle {
            x,
            y,
            vx: a.cos() * s,
            vy: a.sin() * s * 0.6 - speed * 0.25,
            life: l,
            max: l,
            color,
            size: match kind {
                PKind::Smoke => 4.0 + self.rng.f32() * 6.0,
                PKind::Spark => 1.5 + self.rng.f32() * 1.5,
                _ => 1.5 + self.rng.f32() * 2.0,
            },
            additive,
            gravity: match kind {
                PKind::Smoke => -30.0,
                PKind::Spark => 180.0,
                _ => 20.0,
            },
            kind,
        });
    }

    /// Ambient particles: brazier embers, lantern motes, biome atmosphere.
    fn tick_fx(&mut self, dt: f32, b: &Battle, pal: &Palette) {
        self.ambient_acc += dt;
        while self.ambient_acc > 0.05 {
            self.ambient_acc -= 0.05;
            for p in Map::all_pos() {
                let t = b.map.tile(p);
                if t.terrain == Terrain::Brazier && t.lit && self.rng.chance(0.5) {
                    let (x, y) = pos_center(&b.map, p);
                    let l = 0.8 + self.rng.f32() * 0.8;
                    self.particles.push(Particle {
                        x: x + (self.rng.f32() - 0.5) * 10.0,
                        y: y - 30.0,
                        vx: (self.rng.f32() - 0.5) * 14.0,
                        vy: -30.0 - self.rng.f32() * 30.0,
                        life: l,
                        max: l,
                        color: 0xffa040,
                        size: 1.2 + self.rng.f32(),
                        additive: true,
                        gravity: -10.0,
                        kind: PKind::Dot,
                    });
                }
                if t.flare > 0 && self.rng.chance(0.35) {
                    let (x, y) = pos_center(&b.map, p);
                    self.burst(x, y - 8.0, 0xfff0b0, 30.0, 1.0, PKind::Dot, true);
                }
            }
            // Biome motes drifting across the whole scene.
            if self.rng.chance(0.7) {
                let (x, y) = (self.rng.f32() * 1280.0, 120.0 + self.rng.f32() * 520.0);
                let (vx, vy, grav) = match b.cfg.biome {
                    crate::map::Biome::Frost => (-12.0, 25.0, 0.0),
                    crate::map::Biome::Ash => (6.0, -18.0, -4.0),
                    _ => (8.0, -6.0, 0.0),
                };
                let l = 3.0 + self.rng.f32() * 2.0;
                self.particles.push(Particle {
                    x,
                    y,
                    vx: vx + (self.rng.f32() - 0.5) * 10.0,
                    vy,
                    life: l,
                    max: l,
                    color: pal.mote,
                    size: 1.0 + self.rng.f32() * 1.2,
                    additive: true,
                    gravity: grav,
                    kind: PKind::Dot,
                });
            }
        }
        for p in self.particles.iter_mut() {
            p.life -= dt;
            p.vy += p.gravity * dt;
            p.x += p.vx * dt;
            p.y += p.vy * dt;
            if p.kind == PKind::Smoke {
                p.size += dt * 6.0;
                p.vx *= 1.0 - dt * 2.0;
            }
        }
        self.particles.retain(|p| p.life > 0.0);
        if self.particles.len() > 1500 {
            let excess = self.particles.len() - 1500;
            self.particles.drain(..excess);
        }
        for t in self.texts.iter_mut() {
            t.life -= dt;
            t.y -= dt * 28.0;
        }
        self.texts.retain(|t| t.life > 0.0);
        self.shake = (self.shake - dt * 30.0).max(0.0) * (1.0 - dt * 4.0).max(0.0);
        self.flash = (self.flash - dt * 1.8).max(0.0);
        if let Some(bn) = self.banner.as_mut() {
            bn.life -= dt;
            if bn.life <= 0.0 {
                self.banner = None;
            }
        }
        if let Some((_, t)) = self.chain.as_mut() {
            *t -= dt;
            if *t <= 0.0 {
                self.chain = None;
            }
        }
        let decay = (1.0 - dt * 10.0).max(0.0);
        for v in self.vis.values_mut() {
            v.flash = (v.flash - dt * 5.0).max(0.0);
            v.guard_fx = (v.guard_fx - dt * 2.0).max(0.0);
            if !matches!(self.cur, Some((Step::Lunge { .. }, _, _))) {
                v.lunge.0 *= decay;
                v.lunge.1 *= decay;
            }
        }
    }

    /// Shake offset for this frame.
    pub fn shake_offset(&self) -> (f32, f32) {
        if self.shake <= 0.01 {
            return (0.0, 0.0);
        }
        let t = self.time * 60.0;
        (
            (t * 1.7).sin() * self.shake * 0.6,
            (t * 2.3).cos() * self.shake * 0.5,
        )
    }

    pub fn take_sfx(&mut self) -> Vec<Sfx> {
        std::mem::take(&mut self.sfx)
    }
}
