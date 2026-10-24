//! One stage in play: the simulation plus time rewind. Snapshots of the whole
//! world are kept every other frame; holding rewind scrubs them backwards,
//! spending Echo. Echo pickups are claimed once so rewinding cannot farm them.

use std::collections::VecDeque;
use std::sync::Arc;

use crate::level::Level;
use crate::sim::*;

const SNAP_EVERY: u64 = 2;
const HISTORY: usize = 170;
/// Echo spent per second of rewinding.
pub const REWIND_COST: f32 = 26.0;
pub const ECHO_REGEN: f32 = 2.2;

pub struct Game {
    pub world: World,
    history: VecDeque<World>,
    pub echo: f32,
    pub echo_max: f32,
    pub rewinding: bool,
    /// Seconds spent rewinding (visual effect strength).
    pub rewind_fx: f32,
    claimed_orbs: Vec<(i32, i32)>,
    claimed_cp: usize,
    /// Seconds the world has been Dead (for the rewind-or-give-up window).
    pub dead_for: f32,
    pub rewound_total: f32,
}

impl Game {
    pub fn new(level: Arc<Level>, mods: Mods, hp: i32, echo: f32) -> Game {
        let echo_max = mods.echo_max;
        Game {
            world: World::new(level, mods, hp),
            history: VecDeque::with_capacity(HISTORY + 2),
            echo: echo.min(echo_max),
            echo_max,
            rewinding: false,
            rewind_fx: 0.0,
            claimed_orbs: vec![],
            claimed_cp: 0,
            dead_for: 0.0,
            rewound_total: 0.0,
        }
    }

    /// Anything left to scrub back through?
    pub fn can_rewind(&self) -> bool {
        self.echo > 1.0 && !self.history.is_empty()
    }

    /// Advance one 1/60 s tick. Returns this tick's events.
    pub fn tick(&mut self, c: &Controls, rewind: bool) -> Vec<Event> {
        let mut events = Vec::new();
        let wants = rewind
            && self.can_rewind()
            && (self.world.outcome == Outcome::Playing || self.world.outcome == Outcome::Dead);
        if wants {
            self.rewinding = true;
            self.rewind_fx = (self.rewind_fx + DT * 6.0).min(1.0);
            // pop one snapshot per tick (two frames of history)
            if let Some(snap) = self.history.pop_back() {
                let keep_events = std::mem::take(&mut self.world.events);
                self.world = snap;
                self.world.events = keep_events;
                self.world.outcome = Outcome::Playing;
            }
            self.echo = (self.echo - REWIND_COST * DT).max(0.0);
            self.rewound_total += DT;
            self.dead_for = 0.0;
            return events;
        }
        self.rewinding = false;
        self.rewind_fx = (self.rewind_fx - DT * 4.0).max(0.0);
        if self.world.outcome == Outcome::Dead {
            self.dead_for += DT;
            return events;
        }
        if self.world.outcome != Outcome::Playing {
            return events;
        }
        if self.world.frame.is_multiple_of(SNAP_EVERY) {
            let mut snap = self.world.fork();
            snap.events.clear();
            self.history.push_back(snap);
            if self.history.len() > HISTORY {
                self.history.pop_front();
            }
        }
        self.world.step(c);
        events.append(&mut self.world.events);
        for e in &events {
            match e.kind {
                Ev::EchoOrb => {
                    let key = (e.x as i32, e.y as i32);
                    if !self.claimed_orbs.contains(&key) {
                        self.claimed_orbs.push(key);
                        self.echo = (self.echo + 35.0).min(self.echo_max);
                    }
                }
                Ev::Checkpoint => {
                    let idx = e.v as usize;
                    if idx > self.claimed_cp {
                        self.claimed_cp = idx;
                        self.echo = (self.echo + 40.0).min(self.echo_max);
                    }
                }
                _ => {}
            }
        }
        self.echo = (self.echo + ECHO_REGEN * DT).min(self.echo_max);
        events
    }
}
