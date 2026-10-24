//! Autopilot. Follows the level's traversal beats with scripted tactics
//! (run/jump, wall-jump shafts, rope swings whose timings are found by
//! simulating ahead) and dodges hazards by trying short macros in cloned
//! worlds. It is the solvability oracle for the generator and the player
//! for headless balance playtests.

use crate::level::*;
use crate::sim::*;

const MAX_ANCHORS: usize = 6;

#[derive(Clone, Debug)]
struct SwingPlan {
    jump_first: bool,
    lead: f32,
    hook_delay: [u32; MAX_ANCHORS],
    release_off: [f32; MAX_ANCHORS],
    pump: i8,
}

impl SwingPlan {
    fn default_for(n: usize) -> SwingPlan {
        let _ = n;
        SwingPlan {
            jump_first: true,
            lead: 0.0,
            hook_delay: [6; MAX_ANCHORS],
            release_off: [60.0; MAX_ANCHORS],
            pump: 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Phase {
    Approach,
    Edge,
    Hooked,
    Free,
}

#[derive(Clone, Debug)]
struct SwingRt {
    phase: Phase,
    t: u32,
    idx: usize,
}

#[derive(Clone, Copy, Debug)]
enum MacroKind {
    Hop,
    Wait,
    Back,
}

#[derive(Clone, Copy, Debug)]
struct Macro {
    kind: MacroKind,
    k: u32,
    t: u32,
}

#[derive(Clone)]
pub struct Bot {
    pub beat: usize,
    plan: Option<SwingPlan>,
    rt: SwingRt,
    mac: Option<Macro>,
    jump_hold: u32,
    hold: u32,
    hold_for: i64,
    shaft_side: i8,
    dodges: u32,
    last_falls: u32,
    retries: u32,
    pub searching: bool,
    pub frames: u64,
    pub noise: u32,
    pub boss: crate::boss::BossBrain,
}

impl Default for Bot {
    fn default() -> Bot {
        Bot::new()
    }
}

impl Bot {
    pub fn new() -> Bot {
        Bot {
            beat: 0,
            plan: None,
            rt: SwingRt {
                phase: Phase::Approach,
                t: 0,
                idx: 0,
            },
            mac: None,
            jump_hold: 0,
            hold: 40,
            hold_for: -1,
            shaft_side: 1,
            dodges: 0,
            last_falls: 0,
            retries: 0,
            searching: true,
            frames: 0,
            noise: 0,
            boss: Default::default(),
        }
    }

    pub fn act(&mut self, w: &World) -> Controls {
        self.frames += 1;
        if w.level.boss {
            return self.boss_controls(w);
        }
        self.sync_beat(w);
        if w.stats.falls != self.last_falls {
            self.last_falls = w.stats.falls;
            self.retries += 1;
            self.mac = None;
            self.plan = None;
            self.rt = SwingRt {
                phase: Phase::Approach,
                t: 0,
                idx: 0,
            };
        }
        if self.searching && self.mac.is_some() && self.critical(w) {
            self.mac = None;
        }
        let mut c = self.base(w);
        if self.searching {
            if let Some(m) = self.mac {
                c = apply_macro(&m, c, w);
                let mut m2 = m;
                m2.t += 1;
                self.mac = if m2.t > m2.k + 34 { None } else { Some(m2) };
            } else if !self.critical(w) && self.hazard_ahead(w) {
                if let Some(m) = self.choose_macro(w) {
                    if !matches!(m.kind, MacroKind::Hop) {
                        self.dodges += 1;
                    }
                    self.mac = Some(m);
                    c = apply_macro(&m, c, w);
                    self.mac = Some(Macro { t: 1, ..m });
                }
            }
        }
        // jump-hold bookkeeping so variable-height jumps are full height
        if c.jump_pressed {
            self.jump_hold = self.hold.max(1);
        }
        if self.jump_hold > 0 {
            self.jump_hold -= 1;
            if w.p.vy < 0.0 {
                c.jump_held = true;
            }
        }
        c
    }

    fn sync_beat(&mut self, w: &World) {
        let beats = &w.level.beats;
        let old = self.beat;
        let mut i = 0;
        while i < beats.len() {
            let b = &beats[i];
            let done = w.p.x > b.x1 - 4.0
                || (b.kind == BeatKind::Jump && w.p.on_ground && w.p.x + HW > b.x1 - T + 4.0);
            if !done {
                break;
            }
            i += 1;
        }
        self.beat = i;
        if self.beat != old {
            self.plan = None;
            self.rt = SwingRt {
                phase: Phase::Approach,
                t: 0,
                idx: 0,
            };
            self.shaft_side = 1;
            self.dodges = 0;
        }
    }

    // ---------------------------------------------------------------- base

    fn base(&mut self, w: &World) -> Controls {
        let mut c = Controls {
            dir: 1,
            ..Controls::default()
        };
        let Some(b) = w.level.beats.get(self.beat).cloned() else {
            // finish: walk to the exit gate
            let ex = w.level.exit.0;
            c.dir = if w.p.x < ex - 6.0 {
                1
            } else if w.p.x > ex + 6.0 {
                -1
            } else {
                0
            };
            return c;
        };
        match b.kind {
            BeatKind::Jump | BeatKind::Walk => {
                if (w.p.on_ground || w.p.coyote > 0.0)
                    && w.p.x + HW >= b.edge_x - 1.0
                    && w.p.x + HW < b.x1 - T + 4.0
                {
                    if self.hold_for != self.beat as i64 {
                        self.hold = if self.searching {
                            self.search_hold(w, &b)
                        } else {
                            40
                        };
                        self.hold_for = self.beat as i64;
                    }
                    c.jump_pressed = true;
                    c.jump_held = true;
                }
                if w.p.y < b.y1 - 20.0 && w.p.x > b.x1 {
                    c.dir = 1;
                }
                c
            }
            BeatKind::Shaft => self.shaft(w, &b, c),
            BeatKind::Swing => self.swing(w, &b, c),
            BeatKind::Boss => c,
        }
    }

    fn shaft(&mut self, w: &World, b: &Beat, mut c: Controls) -> Controls {
        let top = b.y1;
        let right_wall_x = b.x1 - T; // left face of the right wall
        let left_wall_x = b.edge_x + 2.0 * T; // right face of the left wall
        let in_shaft = w.p.x > b.edge_x - 4.0;
        if !in_shaft {
            c.dir = 1;
            return c;
        }
        // over the top platform: step out to the right
        if w.p.y + HH < top - 2.0 && w.p.x + HW > right_wall_x - 2.0 {
            c.dir = 1;
            c.jump_held = false;
            return c;
        }
        if w.p.on_ground {
            // bottom: run at the right wall and jump near it
            c.dir = 1;
            if w.p.x + HW > right_wall_x - 70.0 {
                c.jump_pressed = true;
                c.jump_held = true;
            }
            self.shaft_side = 1;
            return c;
        }
        if w.p.wall != 0 {
            // touching a wall: kick off toward the other wall
            self.shaft_side = -w.p.wall;
            c.dir = self.shaft_side;
            if w.p.vy > -120.0 {
                c.jump_pressed = true;
                c.jump_held = true;
            }
            return c;
        }
        // in the air: head for the wall we have not just left
        c.dir = self.shaft_side;
        c.jump_held = w.p.vy < 0.0;
        let _ = left_wall_x;
        if w.p.vy > 0.0 && w.p.wall_coyote > 0.0 {
            c.jump_pressed = true;
            c.jump_held = true;
        }
        c
    }

    fn swing(&mut self, w: &World, b: &Beat, mut c: Controls) -> Controls {
        let n = b.anchors.len().min(MAX_ANCHORS);
        if n == 0 {
            return c;
        }
        if self.plan.is_none() {
            if self.searching
                && self.rt.phase == Phase::Approach
                && w.p.on_ground
                && w.p.x + HW >= b.edge_x - 90.0
            {
                self.plan = Some(self.search_swing(w, b));
            } else if !self.searching {
                self.plan = Some(SwingPlan::default_for(n));
            } else {
                // approach until the planning point
                return c;
            }
        }
        let plan = self.plan.clone().unwrap();
        let rt = &mut self.rt;
        match rt.phase {
            Phase::Approach => {
                c.dir = 1;
                if w.p.x + HW >= b.edge_x - plan.lead {
                    rt.phase = Phase::Edge;
                    rt.t = 0;
                }
            }
            Phase::Edge => {}
            _ => {}
        }
        if rt.phase == Phase::Edge {
            c.dir = 1;
            if plan.jump_first && rt.t == 0 {
                c.jump_pressed = true;
                c.jump_held = true;
            } else if plan.jump_first && w.p.vy < 0.0 {
                c.jump_held = true;
            }
            let i = b.anchors[0];
            if rt.t >= plan.hook_delay[0] {
                c.hook_pressed = true;
                c.hook_held = true;
                c.aim = Some((w.anchors[i].x, w.anchors[i].y));
                rt.phase = Phase::Hooked;
                rt.t = 0;
            } else {
                rt.t += 1;
            }
            return c;
        }
        match rt.phase {
            Phase::Hooked => {
                let i = b.anchors[rt.idx];
                c.hook_held = true;
                c.dir = plan.pump;
                c.aim = Some((w.anchors[i].x, w.anchors[i].y));
                let a = w.anchors[i];
                rt.t += 1;
                let released = w.p.rope.is_none() && rt.t > 2;
                if released || w.p.x >= a.x + plan.release_off[rt.idx] || rt.t > 170 {
                    c.hook_held = false;
                    rt.phase = Phase::Free;
                    rt.t = 0;
                    rt.idx += 1;
                }
            }
            Phase::Free => {
                c.dir = 1;
                rt.t += 1;
                if rt.idx < n && !w.p.on_ground {
                    let i = b.anchors[rt.idx];
                    let a = w.anchors[i];
                    let d = dist(w.p.x, w.p.y, a.x, a.y);
                    if rt.t >= plan.hook_delay[rt.idx]
                        && a.alive
                        && d < w.mods.hook_range - 10.0
                        && a.y < w.p.y
                        && a.x > w.p.x - 60.0
                    {
                        c.hook_pressed = true;
                        c.hook_held = true;
                        c.aim = Some((a.x, a.y));
                        rt.phase = Phase::Hooked;
                        rt.t = 0;
                    }
                }
                // keep holding right and a little jump at the end for height
                if rt.idx >= n || w.p.on_ground {
                    c.jump_held = false;
                }
            }
            _ => {}
        }
        c
    }

    fn search_swing(&self, w: &World, b: &Beat) -> SwingPlan {
        let n = b.anchors.len().min(MAX_ANCHORS);
        let mut best = SwingPlan::default_for(n);
        if self.retries > 0 {
            // a previous attempt at this crossing failed: start the search
            // from a different guess so it can settle on another solution
            let mut r = 0x9E37_79B9u32
                ^ (self.retries.wrapping_mul(2_654_435_761))
                ^ (self.beat as u32).wrapping_mul(40503);
            let mut next = |m: u32| {
                r ^= r << 13;
                r ^= r >> 17;
                r ^= r << 5;
                r % m
            };
            best.jump_first = next(2) == 0;
            best.lead = [0.0, 24.0, 48.0][next(3) as usize];
            for i in 0..n {
                best.hook_delay[i] = [0u32, 4, 8, 12, 18, 26][next(6) as usize];
                best.release_off[i] =
                    [-40.0f32, -10.0, 20.0, 50.0, 80.0, 110.0, 150.0, 190.0][next(8) as usize];
            }
        }
        let mut best_score = self.eval_swing(w, b, &best);
        let jump_opts = [true, false];
        let leads = [0.0f32, 24.0, 48.0];
        let hooks = [0u32, 4, 8, 12, 18, 26];
        let offs = [-40.0f32, -10.0, 20.0, 50.0, 80.0, 110.0, 150.0, 190.0];
        // coordinate descent
        for pass in 0..2 {
            for &j in &jump_opts {
                let mut p = best.clone();
                p.jump_first = j;
                let s = self.eval_swing(w, b, &p);
                if s > best_score {
                    best_score = s;
                    best = p;
                }
            }
            for &l in &leads {
                let mut p = best.clone();
                p.lead = l;
                let s = self.eval_swing(w, b, &p);
                if s > best_score {
                    best_score = s;
                    best = p;
                }
            }
            for i in 0..n {
                for &h in &hooks {
                    let mut p = best.clone();
                    p.hook_delay[i] = h;
                    let s = self.eval_swing(w, b, &p);
                    if s > best_score {
                        best_score = s;
                        best = p;
                    }
                }
                for &o in &offs {
                    let mut p = best.clone();
                    p.release_off[i] = o;
                    let s = self.eval_swing(w, b, &p);
                    if s > best_score {
                        best_score = s;
                        best = p;
                    }
                }
            }
            if best_score >= 900_000.0 && pass == 0 {
                break;
            }
        }
        best
    }

    /// Simulate a plan from `w`; high score = landed safely on the far side.
    fn eval_swing(&self, w: &World, b: &Beat, plan: &SwingPlan) -> f32 {
        let mut sim = w.fork();
        let mut bot = self.clone();
        bot.searching = false;
        bot.plan = Some(plan.clone());
        bot.mac = None;
        bot.rt = SwingRt {
            phase: Phase::Approach,
            t: 0,
            idx: 0,
        };
        let hp0 = sim.p.hp;
        let falls0 = sim.stats.falls;
        let mut best_x = sim.p.x;
        for f in 0..520 {
            let c = bot.act_inner(&sim);
            sim.step(&c);
            sim.events.clear();
            best_x = best_x.max(sim.p.x);
            if sim.outcome == Outcome::Cleared {
                return 1_000_000.0 - f as f32 * 0.5;
            }
            if sim.stats.falls > falls0 || sim.p.hp < hp0 || sim.outcome != Outcome::Playing {
                return best_x - 1000.0 - f as f32;
            }
            if sim.p.on_ground && sim.p.x >= b.x1 - 12.0 && (sim.p.y - HH - b.y1).abs() < 70.0 {
                return 1_000_000.0 - f as f32 * 0.5;
            }
            if sim.p.on_ground && f > 40 && bot.rt.phase != Phase::Approach {
                // landed somewhere short
                return best_x - 500.0;
            }
        }
        best_x - 2000.0
    }

    /// `act` without macros or beat syncing, used inside simulations.
    fn act_inner(&mut self, w: &World) -> Controls {
        let mut c = self.base(w);
        if c.jump_pressed {
            self.jump_hold = self.hold.max(1);
        }
        if self.jump_hold > 0 {
            self.jump_hold -= 1;
            if w.p.vy < 0.0 {
                c.jump_held = true;
            }
        }
        c
    }

    /// How many frames to hold jump so the landing falls inside the target
    /// platform (short hops for small platforms).
    fn search_hold(&self, w: &World, b: &Beat) -> u32 {
        let next_edge = w
            .level
            .beats
            .get(self.beat + 1)
            .map(|n| n.edge_x)
            .unwrap_or(b.x1 + 400.0);
        let lo = b.x1 - T + 6.0;
        let hi = next_edge - 6.0;
        let center = (lo + hi) / 2.0;
        let mut best = (f32::INFINITY, 40u32);
        for h in [3u32, 5, 7, 9, 12, 16, 22, 40] {
            let mut sim = w.fork();
            let mut left = false;
            let mut landed_x = None;
            for f in 0..180 {
                let c = Controls {
                    dir: 1,
                    jump_pressed: f == 0,
                    jump_held: f < h as usize && (f == 0 || sim.p.vy < 0.0),
                    ..Controls::default()
                };
                sim.step(&c);
                sim.events.clear();
                if !sim.p.on_ground && f > 2 {
                    left = true;
                }
                if left && sim.p.on_ground {
                    landed_x = Some((sim.p.x, sim.p.y + HH));
                    break;
                }
                if sim.stats.falls > 0 || sim.outcome != Outcome::Playing {
                    break;
                }
            }
            if let Some((x, fy)) = landed_x
                && x >= lo
                && x <= hi
                && (fy - b.y1).abs() < 10.0
            {
                let score = (x - center).abs() + (40 - h.min(40)) as f32 * 0.02;
                if score < best.0 {
                    best = (score, h);
                }
            }
        }
        best.1
    }

    /// Phases where dodge macros would wreck the plan: the run-up and flight
    /// of a rope swing, and anything on the rope or inside a shaft.
    fn critical(&self, w: &World) -> bool {
        if w.p.rope.is_some() {
            return true;
        }
        // crumbling footing and the last stride before a take-off
        let ft = ((w.p.y + HH + 2.0) / T).floor() as i32;
        if w.p.on_ground && w.level.tile((w.p.x / T).floor() as i32, ft) == CRUMBLE {
            return true;
        }
        match w.level.beats.get(self.beat) {
            Some(b) if b.kind == BeatKind::Swing => {
                self.rt.phase != Phase::Approach || w.p.x + HW > b.edge_x - 200.0
            }
            Some(b) if b.kind == BeatKind::Shaft => w.p.x > b.edge_x - 40.0,
            _ => false,
        }
    }

    // -------------------------------------------------------------- hazards

    fn hazard_ahead(&self, w: &World) -> bool {
        let (px, py) = (w.p.x, w.p.y);
        for e in &w.ents {
            if !e.alive {
                continue;
            }
            let dx = e.x - px;
            let dy = e.y - py;
            match e.kind {
                // ground-bound things only matter on our own floor, ahead of us
                EntKind::Skitter
                | EntKind::Bulwark
                | EntKind::Spitter
                | EntKind::Geyser
                | EntKind::Bloom => {
                    if dy.abs() < 70.0 && dx > -40.0 && dx < 300.0 {
                        return true;
                    }
                }
                EntKind::Saw | EntKind::Wisp | EntKind::Bolt => {
                    if dx.abs() < 300.0 && dy.abs() < 260.0 {
                        return true;
                    }
                }
                EntKind::Strike => {
                    if e.state > 0 && (e.a - px).abs() < 260.0 {
                        return true;
                    }
                }
                _ => {}
            }
        }
        // spikes in the next few tiles on our own row
        let tx0 = (px / T).floor() as i32;
        let ty = ((py + HH - 4.0) / T).floor() as i32;
        for tx in tx0..tx0 + 6 {
            for dy in -1..=1 {
                if is_spike(w.level.tile(tx, ty + dy)) {
                    return true;
                }
            }
        }
        false
    }

    fn choose_macro(&mut self, w: &World) -> Option<Macro> {
        if w.p.rope.is_some() || !w.p.on_ground && w.p.since_ground > 0.3 {
            return None;
        }
        let mut cands: Vec<Option<Macro>> = vec![None];
        for k in (0..=30).step_by(3) {
            cands.push(Some(Macro {
                kind: MacroKind::Hop,
                k,
                t: 0,
            }));
        }
        // waiting and backing off are limited per beat so the bot cannot dither
        if self.dodges < 3 {
            for k in (8..=70).step_by(10) {
                cands.push(Some(Macro {
                    kind: MacroKind::Wait,
                    k,
                    t: 0,
                }));
            }
            for k in (8..=40).step_by(16) {
                cands.push(Some(Macro {
                    kind: MacroKind::Back,
                    k,
                    t: 0,
                }));
            }
        }
        // Prefer any clean option. If nothing is clean, push forward (taking
        // the hit is better than dithering): waiting or backing off only count
        // when they actually avoid the damage.
        let mut best_clean: Option<(f32, Option<Macro>)> = None;
        let mut best_forward: Option<(f32, Option<Macro>)> = None;
        for m in cands {
            let s = self.eval_macro(w, m);
            let clean = s > -500.0;
            if m.is_none() && clean {
                return None;
            }
            if clean && best_clean.is_none_or(|(bs, _)| s > bs + 0.001) {
                best_clean = Some((s, m));
            }
            let forward = matches!(
                m,
                None | Some(Macro {
                    kind: MacroKind::Hop,
                    ..
                })
            );
            if forward && best_forward.is_none_or(|(bs, _)| s > bs + 0.001) {
                best_forward = Some((s, m));
            }
        }
        best_clean.or(best_forward).and_then(|(_, m)| m)
    }

    fn eval_macro(&self, w: &World, m: Option<Macro>) -> f32 {
        let mut sim = w.fork();
        let mut bot = self.clone();
        bot.searching = false;
        bot.mac = None;
        let hp0 = sim.p.hp;
        let falls0 = sim.stats.falls;
        let mut mac = m;
        for _ in 0..150 {
            let mut c = bot.base(&sim);
            if bot.jump_hold > 0 {
                bot.jump_hold -= 1;
                if sim.p.vy < 0.0 {
                    c.jump_held = true;
                }
            }
            if c.jump_pressed {
                bot.jump_hold = bot.hold.max(1);
            }
            if let Some(mm) = mac {
                c = apply_macro(&mm, c, &sim);
                let mut m2 = mm;
                m2.t += 1;
                if m2.t > m2.k + 34 {
                    mac = None;
                } else {
                    mac = Some(m2);
                }
            }
            sim.step(&c);
            sim.events.clear();
            if sim.outcome == Outcome::Cleared {
                return 100_000.0 + sim.p.x * 0.001;
            }
            if sim.p.hp < hp0 || sim.stats.falls > falls0 || sim.outcome != Outcome::Playing {
                return -1000.0 + sim.p.x * 0.001;
            }
        }
        let wait_pen = match m {
            Some(Macro {
                kind: MacroKind::Wait,
                k,
                ..
            }) => k as f32 * 0.3,
            Some(Macro {
                kind: MacroKind::Back,
                ..
            }) => 40.0,
            _ => 0.0,
        };
        sim.p.x - wait_pen
    }

    // ----------------------------------------------------------------- boss

    fn boss_controls(&mut self, w: &World) -> Controls {
        crate::boss::bot_controls(self, w)
    }
}

fn apply_macro(m: &Macro, mut c: Controls, w: &World) -> Controls {
    match m.kind {
        MacroKind::Hop => {
            if m.t == m.k {
                c.jump_pressed = true;
                c.jump_held = true;
            } else if m.t > m.k && w.p.vy < 0.0 {
                c.jump_held = true;
            }
        }
        MacroKind::Wait => {
            if m.t < m.k {
                c.dir = 0;
                c.jump_pressed = false;
                c.jump_held = false;
            }
        }
        MacroKind::Back => {
            if m.t < m.k {
                c.dir = -1;
                c.jump_pressed = false;
                c.jump_held = false;
            }
        }
    }
    c
}

/// Result of an autopilot playthrough.
#[derive(Clone, Debug)]
pub struct Play {
    pub outcome: Outcome,
    pub seconds: f32,
    pub frames: u64,
    pub stuck: bool,
}

/// Run the bot on `w` until the stage ends, the player dies, or time runs out.
pub fn play(w: &mut World, bot: &mut Bot, max_seconds: f32) -> Play {
    let max_frames = (max_seconds / DT) as u64;
    let mut best_x = w.p.x;
    let mut last_progress = 0u64;
    let mut stuck = false;
    while w.outcome == Outcome::Playing && w.frame < max_frames {
        let c = bot.act(w);
        w.step(&c);
        w.events.clear();
        if w.p.x > best_x + 8.0 {
            best_x = w.p.x;
            last_progress = w.frame;
        }
        if !w.level.boss && w.frame - last_progress > 60 * 25 {
            stuck = true;
            break;
        }
    }
    Play {
        outcome: w.outcome,
        seconds: w.time,
        frames: w.frame,
        stuck,
    }
}
