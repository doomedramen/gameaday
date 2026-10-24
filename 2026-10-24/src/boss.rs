//! The Stormheart: a floating storm crystal. It can only be hurt after it
//! slams into the floor and is stunned, and only by something moving fast
//! enough to smash glass: a plunge from a ledge or a swing at speed.
//! Attacks are chosen by scoring the player's position and state.

use crate::bot::Bot;
use crate::level::*;
use crate::sim::*;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum BState {
    Intro,
    Idle,
    Fan,
    Rain,
    Gale,
    SlamTell,
    SlamFall,
    Stun,
    Recover,
    Summon,
    Dead,
}

#[derive(Clone, Debug)]
pub struct Boss {
    pub x: f32,
    pub y: f32,
    pub hp: i32,
    pub max_hp: i32,
    pub state: BState,
    pub t: f32,
    pub home: (f32, f32),
    pub floor: f32,
    /// Floor x the next slam lands on.
    pub spot: f32,
    pub since_slam: u32,
    pub last: BState,
    pub aim: f32,
    pub hit_t: f32,
    pub volleys: u32,
    pub contact_cd: f32,
}

pub const BOSS_R: f32 = 44.0;
pub const SLAM_SPOTS: [f32; 3] = [19.5, 38.5, 57.5];

impl Boss {
    pub fn new(level: &Level) -> Boss {
        let floor = 44.0 * T;
        Boss {
            x: level.pixel_w() / 2.0,
            y: 26.0 * T,
            hp: 3,
            max_hp: 3,
            state: BState::Intro,
            t: 0.0,
            home: (level.pixel_w() / 2.0, 26.0 * T),
            floor,
            spot: SLAM_SPOTS[1] * T,
            since_slam: 0,
            last: BState::Idle,
            aim: 0.0,
            hit_t: 0.0,
            volleys: 0,
            contact_cd: 0.0,
        }
    }

    pub fn phase(&self) -> u32 {
        (self.max_hp - self.hp + 1).clamp(1, 3) as u32
    }

    /// Stunned and open to attack.
    pub fn vulnerable(&self) -> bool {
        self.state == BState::Stun
    }
}

impl World {
    pub(crate) fn update_boss(&mut self) {
        let Some(mut b) = self.boss.take() else {
            return;
        };
        b.t += DT;
        b.hit_t = (b.hit_t - DT).max(0.0);
        b.contact_cd = (b.contact_cd - DT).max(0.0);
        let phase = b.phase();
        let (px, py) = (self.p.x, self.p.y);
        match b.state {
            BState::Intro => {
                if b.t < DT * 1.5 {
                    self.emit(Ev::BossRoar, b.x, b.y, 0.0);
                }
                hover(&mut b, self.time);
                if b.t > 2.6 {
                    b.state = BState::Idle;
                    b.t = 0.0;
                }
            }
            BState::Idle => {
                // drift toward a point over the player's side of the arena
                let tx = b.home.0 + ((px - b.home.0) * 0.45).clamp(-500.0, 500.0);
                b.x += (tx - b.x) * 0.02;
                hover(&mut b, self.time);
                let wait = [1.5, 1.05, 0.7][(phase - 1) as usize];
                if b.t > wait {
                    self.choose_attack(&mut b);
                }
            }
            BState::Fan => {
                hover(&mut b, self.time);
                if b.t < 1.0 {
                    // aim follows the player until just before firing
                    if b.t < 0.7 {
                        b.aim = (py - b.y).atan2(px - b.x);
                    }
                } else if b.volleys > 0 {
                    b.volleys -= 1;
                    let n = [5, 7, 9][(phase - 1) as usize];
                    let spread = 0.95;
                    for i in 0..n {
                        let f = i as f32 / (n - 1) as f32 - 0.5;
                        let a = b.aim + f * spread;
                        let mut bolt = Ent::basic(EntKind::Bolt, b.x, b.y, 6.0, 6.0);
                        bolt.vx = a.cos() * 235.0;
                        bolt.vy = a.sin() * 235.0;
                        self.ents.push(bolt);
                    }
                    self.emit(Ev::BossAttack, b.x, b.y, 0.0);
                    b.t = 0.45;
                    if b.volleys > 0 {
                        b.aim = (py - b.y).atan2(px - b.x);
                    }
                }
                if b.volleys == 0 && b.t > 0.9 {
                    self.boss_idle(&mut b);
                }
            }
            BState::Rain => {
                hover(&mut b, self.time);
                if b.t < DT * 1.5 {
                    let n = [5, 7, 9][(phase - 1) as usize];
                    for i in 0..n {
                        let mut s = Ent::basic(EntKind::Strike, 0.0, 0.0, 0.0, 0.0);
                        let off = if i == 0 {
                            0.0
                        } else {
                            let k = ((i + 1) / 2) as f32;
                            k * 150.0 * if i % 2 == 0 { 1.0 } else { -1.0 }
                        };
                        s.a = (px + self.p.vx * 0.3 + off).clamp(3.5 * T, 72.5 * T);
                        s.c = b.floor - 10.0;
                        s.x = s.a;
                        s.y = s.c;
                        s.state = 1;
                        s.t = -(i as f32) * 0.16;
                        s.b = 0.0;
                        self.ents.push(s);
                    }
                    self.emit(Ev::StrikeWarn, px, b.floor, 0.0);
                }
                if b.t > 2.6 {
                    self.boss_idle(&mut b);
                }
            }
            BState::Gale => {
                hover(&mut b, self.time);
                if b.t < DT * 1.5 {
                    let dir = if px > b.x { 1.0 } else { -1.0 };
                    let mut wnd = Ent::basic(EntKind::Wind, b.home.0, 22.0 * T, 40.0 * T, 30.0 * T);
                    wnd.a = dir * 900.0;
                    wnd.dir = -1.0;
                    wnd.vx = 3.4;
                    self.ents.push(wnd);
                    self.emit(Ev::BossAttack, b.x, b.y, 1.0);
                }
                if b.t > 1.2 && b.volleys == 0 && phase >= 2 {
                    // follow the gust with a volley
                    b.volleys = 1;
                    b.state = BState::Fan;
                    b.t = 0.0;
                    b.aim = (py - b.y).atan2(px - b.x);
                } else if b.t > 3.4 {
                    self.boss_idle(&mut b);
                }
            }
            BState::SlamTell => {
                // glide over the chosen spot
                b.x += (b.spot - b.x) * 0.09;
                b.y += (b.floor - 430.0 - b.y) * 0.06;
                if b.t > 1.35 {
                    b.state = BState::SlamFall;
                    b.t = 0.0;
                    b.x = b.spot;
                }
            }
            BState::SlamFall => {
                let target = b.floor - 40.0;
                b.y += 1100.0 * DT;
                if b.y >= target {
                    b.y = target;
                    b.state = BState::Stun;
                    b.t = 0.0;
                    self.emit(Ev::BossStun, b.x, b.y, 0.0);
                    for dir in [-1.0f32, 1.0] {
                        let mut w =
                            Ent::basic(EntKind::Bolt, b.x + dir * 50.0, b.floor - 12.0, 8.0, 12.0);
                        w.vx = dir * 290.0;
                        w.state = 5;
                        self.ents.push(w);
                    }
                }
            }
            BState::Stun => {
                if b.t > 4.8 {
                    b.state = BState::Recover;
                    b.t = 0.0;
                }
            }
            BState::Recover => {
                b.y += (b.home.1 - b.y) * 0.05;
                b.x += (b.home.0 - b.x) * 0.03;
                if b.t > 1.4 {
                    self.boss_idle(&mut b);
                }
            }
            BState::Summon => {
                hover(&mut b, self.time);
                if b.t < DT * 1.5 {
                    for (i, wx) in [8.0f32, 68.0].iter().enumerate() {
                        let mut e = EntSpawn::new(EntKind::Wisp, wx * T, 18.0 * T);
                        e.a = i as f32 * 2.0;
                        self.ents.push(Ent::from_spawn(&e));
                    }
                    self.emit(Ev::BossAttack, b.x, b.y, 2.0);
                }
                if b.t > 1.4 {
                    self.boss_idle(&mut b);
                }
            }
            BState::Dead => {
                if b.t < DT * 1.5 {
                    self.emit(Ev::BossDown, b.x, b.y, 0.0);
                    self.stats.score += 1500;
                    // clear hostile projectiles
                    for e in &mut self.ents {
                        if matches!(e.kind, EntKind::Bolt | EntKind::Strike | EntKind::Wisp) {
                            e.alive = false;
                        }
                    }
                }
                b.y += (b.home.1 - 40.0 - b.y) * 0.01;
                if b.t > 2.4 {
                    self.gate_open = true;
                }
            }
        }
        // sweep up temporary entities
        self.boss_contact(&mut b);
        self.boss = Some(b);
        let _ = py;
    }

    fn boss_idle(&mut self, b: &mut Boss) {
        b.state = BState::Idle;
        b.t = 0.0;
        b.volleys = 0;
    }

    /// Pick the next attack by scoring what each would do to this player.
    fn choose_attack(&mut self, b: &mut Boss) {
        let phase = b.phase();
        let (px, py) = (self.p.x, self.p.y);
        let high = py < 36.0 * T;
        let roped = self.p.rope.is_some();
        let far = (px - b.x).abs() > 12.0 * T;
        let wisps = self
            .ents
            .iter()
            .filter(|e| e.alive && e.kind == EntKind::Wisp)
            .count();
        let mut options: Vec<(BState, f32)> = vec![
            (
                BState::Fan,
                1.0 + if far { 1.2 } else { 0.0 } + if high { 0.5 } else { 0.0 },
            ),
            (
                BState::Rain,
                1.0 + if !high { 1.3 } else { 0.0 } + if py > 40.0 * T { 0.6 } else { 0.0 },
            ),
            (BState::SlamTell, 0.7 + b.since_slam as f32 * 1.4),
        ];
        if phase >= 2 {
            options.push((
                BState::Gale,
                0.6 + if roped { 1.8 } else { 0.0 } + if high { 0.8 } else { 0.0 },
            ));
        }
        if phase >= 3 && wisps < 2 {
            options.push((BState::Summon, 1.1));
        }
        // never repeat the same attack; always slam within three attacks
        options.retain(|(s, _)| *s != b.last);
        if b.since_slam >= 2 {
            options.retain(|(s, _)| *s == BState::SlamTell);
        }
        let total: f32 = options.iter().map(|o| o.1).sum();
        let mut pick = self.rng.f32() * total;
        let mut chosen = options[0].0;
        for (s, w) in &options {
            if pick < *w {
                chosen = *s;
                break;
            }
            pick -= *w;
        }
        b.last = chosen;
        b.state = chosen;
        b.t = 0.0;
        match chosen {
            BState::SlamTell => {
                b.since_slam = 0;
                // land on the nearest spot that is not right on top of the
                // player, so they must move but can still get there
                let mut best = SLAM_SPOTS[1] * T;
                let mut best_score = f32::INFINITY;
                for s in SLAM_SPOTS {
                    let sx = s * T;
                    let d = (sx - px).abs();
                    let score = if d < 7.0 * T { 5000.0 - d } else { d } + self.rng.f32() * 3.0 * T;
                    if score < best_score {
                        best_score = score;
                        best = sx;
                    }
                }
                b.spot = best;
                self.emit(Ev::BossAttack, b.spot, b.floor, 3.0);
            }
            BState::Fan => {
                b.since_slam += 1;
                b.volleys = if phase >= 3 { 2 } else { 1 };
                b.aim = (py - b.y).atan2(px - b.x);
                self.emit(Ev::SpitterCharge, b.x, b.y, 1.0);
            }
            BState::Rain => b.since_slam += 1,
            BState::Gale => {
                b.since_slam += 1;
                b.volleys = 0;
            }
            _ => b.since_slam += 1,
        }
    }

    fn boss_contact(&mut self, b: &mut Boss) {
        if self.p.dying > 0.0 || b.state == BState::Dead || b.contact_cd > 0.0 {
            return;
        }
        let (px, py) = (self.p.x, self.p.y);
        // closest point of the player box to the boss centre
        let cx = b.x.clamp(px - HW, px + HW);
        let cy = b.y.clamp(py - HH, py + HH);
        if dist(cx, cy, b.x, b.y) > BOSS_R {
            return;
        }
        if b.vulnerable() && self.p.speed >= self.mods.smash_v * 0.98 {
            b.hp -= 1;
            b.hit_t = 0.6;
            b.contact_cd = 0.5;
            self.p.rope = None;
            self.p.vy = -540.0;
            self.p.vx = if px < b.x { -240.0 } else { 240.0 };
            self.p.invuln = self.p.invuln.max(0.6);
            self.stats.score += 500;
            self.bump_chain(3, b.x, b.y);
            self.emit(Ev::BossHit, b.x, b.y, b.hp as f32);
            // reward drops
            for i in 0..5 {
                let mut c =
                    EntSpawn::new(EntKind::Crystal, b.x + (i as f32 - 2.0) * 26.0, b.y - 50.0);
                c.a = 0.0;
                self.ents.push(Ent::from_spawn(&c));
            }
            if b.hp <= 0 {
                b.state = BState::Dead;
                b.t = 0.0;
            } else {
                b.state = BState::Recover;
                b.t = 0.0;
            }
        } else if b.vulnerable() {
            // too slow: bounce off the stunned shell
            self.p.vy = -380.0;
            self.p.vx = if px < b.x { -260.0 } else { 260.0 };
            b.contact_cd = 0.3;
            self.emit(Ev::Bounce, b.x, b.y, 0.0);
        } else {
            self.hurt_by("the Stormheart", b.x, b.y, false);
            b.contact_cd = 0.4;
        }
    }
}

fn hover(b: &mut Boss, time: f32) {
    let ty = b.home.1 + (time * 1.3).sin() * 18.0;
    b.y += (ty - b.y) * 0.05;
}

// ------------------------------------------------------------------- autopilot

#[derive(Clone, Copy, Debug)]
pub struct Seg {
    pub dir: i8,
    pub dur: u16,
    pub jump: bool,
    pub hook: Option<u8>,
}

#[derive(Clone, Debug, Default)]
pub struct BossBrain {
    pub script: Vec<Seg>,
    pub idx: usize,
    pub t: u16,
    pub rng: u32,
    pub cooldown: u32,
}

fn next_rand(s: &mut u32) -> u32 {
    *s ^= *s << 13;
    *s ^= *s >> 17;
    *s ^= *s << 5;
    *s
}

fn random_script(rng: &mut u32, anchors: usize, aggressive: bool) -> Vec<Seg> {
    let n = 2 + (next_rand(rng) % 4) as usize;
    let mut v = Vec::new();
    for _ in 0..n {
        let r = next_rand(rng);
        v.push(Seg {
            dir: [-1, 0, 1][(r % 3) as usize],
            dur: 4 + ((r >> 4) % if aggressive { 46 } else { 30 }) as u16,
            jump: (r >> 12) % 3 == 0,
            hook: if aggressive && anchors > 0 && (r >> 16) % 4 == 0 {
                Some(((r >> 20) % anchors as u32) as u8)
            } else {
                None
            },
        });
    }
    v
}

fn script_controls(w: &World, seg: &Seg, t: u16) -> Controls {
    let mut c = Controls {
        dir: seg.dir,
        ..Controls::default()
    };
    if t == 0 {
        if seg.jump {
            c.jump_pressed = true;
        }
        if let Some(a) = seg.hook {
            c.hook_pressed = true;
            let an = w.anchors[a as usize];
            c.aim = Some((an.x, an.y));
        }
    }
    if seg.jump && w.p.vy < 0.0 && t < 22 {
        c.jump_held = true;
    }
    if let Some(a) = seg.hook {
        c.hook_held = t + 1 < seg.dur;
        let an = w.anchors[a as usize];
        c.aim = Some((an.x, an.y));
    }
    c
}

/// Simulate a script; positive scores are better.
pub fn eval(w: &World, script: &[Seg], horizon: u32, want_hit: bool) -> f32 {
    let mut sim = w.fork();
    let hp0 = sim.p.hp;
    let boss0 = sim.boss.as_ref().map(|b| b.hp).unwrap_or(0);
    let mut idx = 0;
    let mut t = 0u16;
    let mut near_best = f32::INFINITY;
    for f in 0..horizon {
        let c = if idx < script.len() {
            let c = script_controls(&sim, &script[idx], t);
            t += 1;
            if t >= script[idx].dur {
                idx += 1;
                t = 0;
            }
            c
        } else {
            Controls::default()
        };
        sim.step(&c);
        sim.events.clear();
        if sim.p.hp < hp0 || sim.p.dying > 0.0 {
            return -1000.0 + f as f32 * 0.1;
        }
        if let Some(b) = &sim.boss {
            if b.hp < boss0 {
                return 1000.0 - f as f32 * 0.5;
            }
            if want_hit && b.vulnerable() {
                let d = dist(sim.p.x, sim.p.y, b.x, b.y);
                near_best = near_best.min(d);
            }
        }
        if sim.p.on_ground && f > horizon / 2 && idx >= script.len() && !want_hit {
            break;
        }
    }
    if want_hit {
        -near_best * 0.05
    } else {
        // staying alive; prefer ending near the middle of the arena
        -((sim.p.x - 38.0 * T).abs()) * 0.002
    }
}

pub fn bot_controls(bot: &mut Bot, w: &World) -> Controls {
    let Some(b) = w.boss.as_ref() else {
        return Controls::default();
    };
    let brain = &mut bot.boss;
    if brain.rng == 0 {
        brain.rng = 0x1234_5678 ^ (bot.noise.wrapping_mul(2_654_435_761));
    }
    // follow the script we committed to
    if brain.idx < brain.script.len() {
        let seg = brain.script[brain.idx];
        let c = script_controls(w, &seg, brain.t);
        brain.t += 1;
        if brain.t >= seg.dur {
            brain.idx += 1;
            brain.t = 0;
        }
        return c;
    }
    brain.cooldown = brain.cooldown.saturating_sub(1);
    if b.state == BState::Dead {
        // walk to the gate
        let ex = w.level.exit.0;
        return Controls {
            dir: if w.p.x < ex - 6.0 {
                1
            } else if w.p.x > ex + 6.0 {
                -1
            } else {
                0
            },
            ..Controls::default()
        };
    }
    let n_anchors = w.anchors.len();
    if b.vulnerable() && b.t < 3.6 {
        // go for the hit: skim-swing into the shell from the anchor above it
        let spot_ix = SLAM_SPOTS
            .iter()
            .position(|s| (s * T - b.spot).abs() < 2.0)
            .unwrap_or(1);
        let toward: i8 = if w.p.x < b.x { 1 } else { -1 };
        let mut best: Option<(f32, Vec<Seg>)> = None;
        'search: for pre in [0u16, 5, 10, 16, 24] {
            for dur in [20u16, 26, 32, 38, 44, 52, 60, 70, 82] {
                for off in [0u16, 8, 20] {
                    let s = vec![
                        Seg {
                            dir: toward,
                            dur: pre.max(1),
                            jump: false,
                            hook: None,
                        },
                        Seg {
                            dir: toward,
                            dur: dur + off,
                            jump: false,
                            hook: Some(spot_ix as u8),
                        },
                        Seg {
                            dir: toward,
                            dur: 30,
                            jump: false,
                            hook: None,
                        },
                    ];
                    let sc = eval(w, &s, 160, true);
                    if best.as_ref().is_none_or(|(bs, _)| sc > *bs) {
                        best = Some((sc, s));
                    }
                    if sc > 500.0 {
                        break 'search;
                    }
                }
            }
        }
        if best.as_ref().is_none_or(|(sc, _)| *sc < 500.0) {
            for _ in 0..300 {
                let s = random_script(&mut brain.rng, n_anchors, true);
                let sc = eval(w, &s, 150, true);
                if best.as_ref().is_none_or(|(bs, _)| sc > *bs) {
                    best = Some((sc, s));
                }
                if sc > 500.0 {
                    break;
                }
            }
        }
        if let Some((_, s)) = best {
            brain.script = s;
            brain.idx = 0;
            brain.t = 0;
            return Controls::default();
        }
    }
    // otherwise stay out of trouble near the slam spot, dodging by search
    let target_x = if matches!(b.state, BState::SlamTell | BState::SlamFall | BState::Stun) {
        // stand six tiles off, on the arena-centre side
        if b.spot < 38.0 * T {
            b.spot + 6.0 * T
        } else {
            b.spot - 6.0 * T
        }
    } else {
        38.0 * T
    };
    let mut base = Controls::default();
    if (w.p.x - target_x).abs() > 40.0 {
        base.dir = if w.p.x < target_x { 1 } else { -1 };
        // hop over steps and low walls in the way
        let ahead = w.p.x + base.dir as f32 * (HW + 6.0);
        if w.p.on_ground && w.box_hits(ahead, w.p.y, 2.0, HH - 4.0) {
            base.jump_pressed = true;
            base.jump_held = true;
        }
    }
    let threatened = w.ents.iter().any(|e| {
        e.alive
            && matches!(e.kind, EntKind::Bolt | EntKind::Strike | EntKind::Wisp)
            && (e.x - w.p.x).abs() < 340.0
            && (e.y - w.p.y).abs() < 300.0
    }) || matches!(b.state, BState::SlamFall);
    if threatened && brain.cooldown == 0 {
        let mut best: Option<(f32, Vec<Seg>)> = None;
        let mut empty = vec![Seg {
            dir: base.dir,
            dur: 12,
            jump: false,
            hook: None,
        }];
        let sc0 = eval(w, &empty, 70, false);
        best = best.or(Some((sc0, std::mem::take(&mut empty))));
        if sc0 < -500.0 {
            for _ in 0..90 {
                let s = random_script(&mut brain.rng, 0, false);
                let sc = eval(w, &s, 70, false);
                if best.as_ref().is_none_or(|(bs, _)| sc > *bs) {
                    best = Some((sc, s));
                }
                if sc > -1.0 {
                    break;
                }
            }
        }
        brain.cooldown = 6;
        if let Some((_, s)) = best {
            brain.script = s;
            brain.idx = 0;
            brain.t = 0;
        }
    }
    base
}
