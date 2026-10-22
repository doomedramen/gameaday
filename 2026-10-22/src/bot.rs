//! Headless playtest driver: plays whole runs with the squad autopilot and
//! reports what happened, so tests can assert on pacing and progression.

use crate::ai;
use crate::battle::Phase;
use crate::run::{Run, Stage};
use crate::units::{Kind, Team};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Style {
    /// Squad autopilot.
    Smart,
    /// Never acts; just ends turns.
    Passive,
}

#[derive(Clone, Debug, Default)]
pub struct IslandStats {
    pub island: u32,
    pub boss: bool,
    pub enemies: usize,
    pub enemy_hp: i32,
    pub elites: usize,
    pub kinds: Vec<Kind>,
    pub rounds: u32,
}

#[derive(Clone, Debug, Default)]
pub struct Report {
    pub won: bool,
    pub lost: bool,
    pub final_island: u32,
    pub score: i64,
    pub islands: Vec<IslandStats>,
    pub steps: u32,
    pub stuck: bool,
    pub upgrades: usize,
    pub sanctuaries: u32,
    pub reinforcements_seen: bool,
    pub chains_seen: u32,
}

pub fn play_run(seed: u64, style: Style, god_mode: bool, max_steps: u32) -> Report {
    let mut run = Run::new(seed);
    run.god_mode = god_mode;
    if let Some(b) = run.battle.as_mut() {
        b.cfg.god_mode = god_mode;
    }
    let mut rep = Report::default();
    let mut current: Option<IslandStats> = None;
    loop {
        rep.steps += 1;
        if rep.steps > max_steps {
            rep.stuck = true;
            break;
        }
        match run.stage.clone() {
            Stage::Battle => {
                let Some(b) = run.battle.as_mut() else {
                    rep.stuck = true;
                    break;
                };
                if current.is_none() {
                    let dark: Vec<_> = b.units.iter().filter(|u| u.team() == Team::Dark).collect();
                    let mut kinds: Vec<Kind> = dark.iter().map(|u| u.kind).collect();
                    kinds.dedup();
                    current = Some(IslandStats {
                        island: b.cfg.island,
                        boss: b.cfg.boss,
                        enemies: dark.len(),
                        enemy_hp: dark.iter().map(|u| u.max_hp).sum(),
                        elites: dark.iter().filter(|u| u.elite).count(),
                        kinds,
                        rounds: 0,
                    });
                }
                if b.outcome.is_some() {
                    if let Some(mut s) = current.take() {
                        s.rounds = b.round;
                        rep.islands.push(s);
                    }
                    run.finish_battle();
                    continue;
                }
                match b.phase {
                    Phase::Player => {
                        if style == Style::Smart {
                            ai::autopilot_turn(b);
                            rep.chains_seen = rep.chains_seen.max(b.best_chain);
                        }
                        if b.outcome.is_none() {
                            b.end_player_turn();
                        }
                    }
                    Phase::Enemy => {
                        for e in b.enemy_step() {
                            if e == crate::battle::Event::Reinforcements {
                                rep.reinforcements_seen = true;
                            }
                        }
                    }
                    Phase::Over => {}
                }
            }
            Stage::Draft { .. } => {
                run.choose_upgrade(0);
            }
            Stage::Route { options } => {
                if options.contains(&crate::run::Node::Sanctuary) && options.len() > 1 {
                    // Alternate: take sanctuaries when hurt.
                    let hurt = run.squad.iter().any(|m| m.hp * 2 < m.max_hp);
                    let idx = if hurt {
                        options
                            .iter()
                            .position(|n| *n == crate::run::Node::Sanctuary)
                            .unwrap_or(0)
                    } else {
                        0
                    };
                    if options[idx] == crate::run::Node::Sanctuary {
                        rep.sanctuaries += 1;
                    }
                    run.choose_route(idx);
                } else {
                    run.choose_route(0);
                }
            }
            Stage::Victory => {
                rep.won = true;
                break;
            }
            Stage::Defeat { .. } => {
                rep.lost = true;
                break;
            }
        }
    }
    rep.final_island = run.island;
    rep.score = run.score;
    rep.upgrades = run.upgrades.len();
    rep
}
