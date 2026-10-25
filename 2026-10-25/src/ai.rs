//! Rival drivers. Every AI uses the player's physics and inputs: it follows a
//! minimum-curvature racing line, brakes from a speed profile computed from
//! the car's own steering and grip limits, drafts, overtakes around slower
//! cars, uses nitro on straights and drifts the slow corners. Personalities
//! add blocking (Blocker, Champion), telegraphed ramming (Bully) and line
//! choice. The autopilot that playtests the game is this same driver.

use crate::car::{Controls, Stats};
use crate::math::{V2, wrap_angle};
use crate::race::{Kind, Race};
use crate::rng::Rng;
use crate::track::{HALF_W, STEP, Track, corner_speed};

#[derive(Clone, Debug)]
pub struct AiState {
    pub kind: Kind,
    /// Preferred lateral bias.
    pub lane: f32,
    pub phase: f32,
    pub base_top: f32,
    /// Overtake lane offset currently held, and for how long.
    pub pass_lat: f32,
    pub pass_t: f32,
    pub drift_hold: bool,
    /// Bully: cool-down, wind-up (telegraph) and ram timers.
    pub cd: f32,
    pub windup: f32,
    pub ram: f32,
    pub target: Option<usize>,
    pub near_flag: bool,
    pub t: f32,
}

impl AiState {
    pub fn new(kind: Kind, rng: &mut Rng) -> AiState {
        AiState {
            kind,
            lane: rng.range(-14.0, 14.0),
            phase: rng.range(0.0, std::f32::consts::TAU),
            base_top: 0.0,
            pass_lat: 0.0,
            pass_t: 0.0,
            drift_hold: false,
            cd: rng.range(3.0, 6.0),
            windup: 0.0,
            ram: 0.0,
            target: None,
            near_flag: false,
            t: 0.0,
        }
    }
    pub fn ramming(&self) -> bool {
        self.ram > 0.0
    }
    pub fn telegraphing(&self) -> bool {
        self.windup > 0.0
    }
    pub fn rammed(&mut self) {
        self.ram = 0.0;
        self.cd = 5.0;
    }
}

/// Per-sample target speed along the racing line for this car.
pub fn speed_profile(track: &Track, stats: &Stats, kind: Kind) -> Vec<f32> {
    let grip = track.spec.biome.road_grip();
    let caution = match kind {
        Kind::Rocket => 0.86,
        Kind::Champion | Kind::Player => 0.97,
        Kind::Ace => 0.95,
        Kind::Drifter => 0.93,
        _ => 0.91,
    };
    let g = stats.grip * grip * caution;
    let st = *stats;
    let mut v: Vec<f32> = track
        .line_curv
        .iter()
        .map(|k| {
            let r = 1.0 / k.abs().max(1e-5);
            corner_speed(r, g, |s| st.yaw_max(s) * caution, stats.top * 1.4)
        })
        .collect();
    // smooth the dips a little so the car doesn't lift on tiny wiggles
    let n = v.len();
    let src = v.clone();
    for i in 0..n {
        let a = src[(i + n - 1) % n];
        let b = src[(i + 1) % n];
        v[i] = src[i].min((a + b + src[i]) / 3.0 + 20.0);
    }
    v
}

/// Update personality state. Returns true when a Bully starts a telegraph.
pub fn think(race: &mut Race, i: usize, dt: f32) -> bool {
    let pi = race.player_idx();
    let len = race.track.len;
    let player_prog = race.racers[pi].progress;
    let (my_prog, my_pos) = (race.racers[i].progress, race.racers[i].car.pos);
    let player_pos = race.racers[pi].car.pos;
    let r = &mut race.racers[i];
    let ai = &mut r.ai;
    ai.t += dt;
    if ai.base_top == 0.0 {
        ai.base_top = r.car.stats.top;
    }
    // gentle catch-up: never more than a few percent
    let gap = my_prog - player_prog;
    let k = if gap < -700.0 {
        1.035
    } else if gap > 900.0 {
        0.985
    } else {
        1.0
    };
    r.car.stats.top = ai.base_top * k;
    let mut started = false;
    if ai.kind == Kind::Bully && i != pi {
        ai.cd = (ai.cd - dt).max(0.0);
        if ai.ram > 0.0 {
            ai.ram -= dt;
            if ai.ram <= 0.0 {
                ai.cd = 4.5;
            }
        } else if ai.windup > 0.0 {
            ai.windup -= dt;
            if ai.windup <= 0.0 {
                ai.ram = 0.7;
            }
        } else if ai.cd <= 0.0 {
            let mut ds = player_prog - my_prog;
            if ds.abs() > len / 2.0 {
                ds -= ds.signum() * len;
            }
            if ds.abs() < 80.0 && my_pos.dist(player_pos) < 120.0 {
                ai.windup = 0.6;
                ai.target = Some(pi);
                started = true;
            }
        }
    }
    started
}

/// Steering, throttle, drift and nitro for racer `i`.
pub fn drive(race: &Race, i: usize, _dt: f32) -> Controls {
    let track = &race.track;
    let me = &race.racers[i];
    let car = &me.car;
    let ai = &me.ai;
    let sp = car.speed();
    let len = track.len;
    let pi = race.player_idx();
    let is_player = i == pi;

    // --- choose the lateral target
    let look = 70.0 + sp * 0.36;
    let s_t = me.s + look;
    let it = track.idx_at(s_t);
    let wobble = if is_player {
        0.0
    } else {
        (ai.t * 0.6 + ai.phase).sin() * 18.0 * (1.08 - race.skill).max(0.0) * 5.0
    };
    let mut lat_t = track.line[it] + if is_player { 0.0 } else { ai.lane } + wobble;

    // overtaking: steer round slower cars ahead
    let mut blocked_by: Option<usize> = None;
    for (j, o) in race.racers.iter().enumerate() {
        if j == i {
            continue;
        }
        let mut ds = o.s - me.s;
        if ds < -len / 2.0 {
            ds += len;
        } else if ds > len / 2.0 {
            ds -= len;
        }
        if ds > 0.0
            && ds < 70.0 + sp * 0.4
            && (o.lat - lat_t).abs() < 48.0
            && o.car.speed() < sp + 40.0
        {
            blocked_by = Some(j);
            break;
        }
    }
    if let Some(j) = blocked_by {
        let o = &race.racers[j];
        let lim = HALF_W - 30.0;
        let left = o.lat + 60.0;
        let right = o.lat - 60.0;
        let can_l = left < lim;
        let can_r = right > -lim;
        // prefer the inside of the next corner
        let inside_left = track.curv[it] < 0.0;
        lat_t = match (can_l, can_r) {
            (true, true) => {
                if inside_left {
                    left
                } else {
                    right
                }
            }
            (true, false) => left,
            (false, true) => right,
            _ => lat_t,
        };
    }

    // defending: cover the line of a chaser in the mirrors
    let defends = matches!(ai.kind, Kind::Blocker | Kind::Champion) && !is_player;
    if defends {
        let p = &race.racers[pi];
        let mut ds = p.s - me.s;
        if ds < -len / 2.0 {
            ds += len;
        } else if ds > len / 2.0 {
            ds -= len;
        }
        if ds < -40.0 && ds > -260.0 && track.curv[me.idx].abs() < 1.0 / 900.0 {
            let k = if ai.kind == Kind::Blocker { 0.85 } else { 0.55 };
            lat_t = lat_t + (p.lat - lat_t) * k;
        }
    }

    // bully: wind-up holds station, the ram steers into the target
    if ai.kind == Kind::Bully && !is_player {
        if let Some(t) = ai.target {
            let o = &race.racers[t];
            if ai.windup > 0.0 {
                lat_t = me.lat + (me.lat - o.lat).signum() * 10.0;
            } else if ai.ram > 0.0 {
                lat_t = o.lat - (me.lat - o.lat).signum() * 30.0;
            }
        }
    }

    // steer round oil and mud ahead
    for patch in &track.patches {
        if patch.kind == crate::track::PatchKind::Sand && !is_player {
            continue;
        }
        let rel = patch.pos - car.pos;
        let along = rel.dot(car.fwd());
        if along > 0.0 && along < 120.0 + sp * 0.6 {
            let ploc = track.locate(patch.pos, Some(me.idx));
            let clear = patch.r + 26.0;
            if (ploc.lat - lat_t).abs() < clear {
                let up = ploc.lat + clear;
                let down = ploc.lat - clear;
                lat_t = if (up - lat_t).abs() < (down - lat_t).abs() && up < HALF_W - 22.0
                    || down < -(HALF_W - 22.0)
                {
                    up
                } else {
                    down
                };
            }
        }
    }
    let lim = HALF_W - 22.0;
    let lat_t = lat_t.clamp(-lim, lim);
    let target = track.point_at(s_t, lat_t);
    let to = target - car.pos;
    let err = wrap_angle(to.angle() - car.heading);
    let mut steer = (err * 2.8 - car.yaw * 0.06).clamp(-1.0, 1.0);
    // reversing out of trouble
    let backwards = car.fwd().dot(track.tan[me.idx]) < -0.2;

    // --- speed control from the profile
    let decel = car.stats.brake * 0.55;
    let horizon = (sp * sp / (2.0 * decel) + 80.0).min(900.0);
    let samples = (horizon / STEP) as usize + 1;
    let mut desired = f32::MAX;
    for k in 0..samples {
        let idx = (me.idx + k) % track.n;
        let d = (k as f32 * STEP - 10.0).max(0.0);
        let v = me.vmax[idx];
        let allow = (v * v + 2.0 * decel * d).sqrt();
        desired = desired.min(allow);
    }
    let mut throttle = if sp > desired + 25.0 {
        -1.0
    } else if sp > desired + 5.0 {
        0.0
    } else {
        1.0
    };
    if backwards || me.lat.abs() > HALF_W + 40.0 && sp < 60.0 {
        // stuck facing the wrong way or beached: reverse-turn back
        if backwards {
            throttle = 0.6;
            steer = if err > 0.0 { 1.0 } else { -1.0 };
        }
    }

    // --- drift slow corners
    let drift_k = match ai.kind {
        Kind::Drifter => 1.0 / 450.0,
        Kind::Champion | Kind::Ace => 1.0 / 300.0,
        Kind::Player => 1.0 / 380.0,
        _ => f32::MAX,
    };
    let here_k = track.line_curv[me.idx].abs();
    let ahead_k = track.line_curv[(me.idx + 6) % track.n].abs();
    let mid_k = track.line_curv[(me.idx + 3) % track.n].abs();
    let keep = here_k.max(mid_k).max(ahead_k) > drift_k * 0.55;
    let drift = (car.drifting && keep && sp > 200.0)
        || (!car.drifting && ahead_k > drift_k && sp > 250.0 && steer.abs() > 0.3);

    // --- nitro on straights or to complete a pass
    let straight_ahead = (0..40).all(|k| me.vmax[(me.idx + k) % track.n] > car.stats.top * 1.05);
    let smart = matches!(
        ai.kind,
        Kind::Champion | Kind::Ace | Kind::Rocket | Kind::Player
    );
    let nitro = car.nitro > 0.25
        && straight_ahead
        && sp > 320.0
        && (smart || car.nitro > 0.7 || blocked_by.is_some());

    Controls {
        throttle,
        steer,
        drift,
        nitro,
    }
}

/// True for the frames a Bully's telegraph is showing over its car.
pub fn warning_over(race: &Race, i: usize) -> bool {
    race.racers[i].ai.telegraphing()
}

/// Is the point inside the track's reachable area (for tests).
pub fn on_track(track: &Track, p: V2) -> bool {
    track.locate(p, None).lat.abs() < crate::track::WALL
}
