//! A run: five ascents and the Stormheart, with boons between them.

use std::sync::Arc;

use crate::game::Game;
use crate::level::Level;
use crate::rng::Rng;
use crate::save::{PERK_COUNT, Save};
use crate::sim::*;
use crate::stagegen::StageSpec;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Boon {
    LongLine,
    QuickReel,
    Locket,
    SecondWind,
    Momentum,
    EchoWell,
    Magnet,
    Gilded,
    SpringHeel,
    WallGrip,
    SwingBoost,
    StormWard,
    Glowcap,
    Rhythm,
    Phoenix,
}

impl Boon {
    pub const ALL: [Boon; 15] = [
        Boon::LongLine,
        Boon::QuickReel,
        Boon::Locket,
        Boon::SecondWind,
        Boon::Momentum,
        Boon::EchoWell,
        Boon::Magnet,
        Boon::Gilded,
        Boon::SpringHeel,
        Boon::WallGrip,
        Boon::SwingBoost,
        Boon::StormWard,
        Boon::Glowcap,
        Boon::Rhythm,
        Boon::Phoenix,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Boon::LongLine => "Long Line",
            Boon::QuickReel => "Quick Reel",
            Boon::Locket => "Heart Locket",
            Boon::SecondWind => "Second Wind",
            Boon::Momentum => "Momentum Mastery",
            Boon::EchoWell => "Echo Well",
            Boon::Magnet => "Magnet Charm",
            Boon::Gilded => "Gilded Satchel",
            Boon::SpringHeel => "Spring Heel",
            Boon::WallGrip => "Wall Grip",
            Boon::SwingBoost => "Swing Boost",
            Boon::StormWard => "Storm Ward",
            Boon::Glowcap => "Glowcap Lantern",
            Boon::Rhythm => "Rhythm Beads",
            Boon::Phoenix => "Phoenix Feather",
        }
    }

    pub fn desc(self) -> &'static str {
        match self {
            Boon::LongLine => "Your hook reaches 30% farther.",
            Boon::QuickReel => "Reel the rope in and out much faster.",
            Boon::Locket => "+1 max heart and heal a heart now.",
            Boon::SecondWind => "One extra jump in mid-air.",
            Boon::Momentum => "Smash glass and foes at a lower speed.",
            Boon::EchoWell => "Rewind meter holds 50% more.",
            Boon::Magnet => "Crystals fly to you from afar.",
            Boon::Gilded => "Crystals are worth 50% more.",
            Boon::SpringHeel => "Jump 12% higher.",
            Boon::WallGrip => "Slide slowly down walls.",
            Boon::SwingBoost => "Pumping the rope builds speed faster.",
            Boon::StormWard => "Ignore the first hit of every ascent.",
            Boon::Glowcap => "Your light reaches much farther in the dark.",
            Boon::Rhythm => "Combos last twice as long.",
            Boon::Phoenix => "Once per run, rise again at one heart.",
        }
    }

    pub fn apply(self, m: &mut Mods) {
        match self {
            Boon::LongLine => m.hook_range *= 1.3,
            Boon::QuickReel => m.reel *= 1.8,
            Boon::Locket => m.max_hp += 1,
            Boon::SecondWind => m.air_jumps += 1,
            Boon::Momentum => m.smash_v -= 70.0,
            Boon::EchoWell => m.echo_max *= 1.5,
            Boon::Magnet => m.magnet *= 2.2,
            Boon::Gilded => m.crystal_value *= 1.5,
            Boon::SpringHeel => m.jump_v *= 1.06,
            Boon::WallGrip => m.wall_slide = 45.0,
            Boon::SwingBoost => m.pump *= 1.6,
            Boon::StormWard => m.shield = true,
            Boon::Glowcap => m.lamp *= 1.7,
            Boon::Rhythm => m.chain_time *= 2.0,
            Boon::Phoenix => m.revive = true,
        }
    }

    /// Icon colour for the UI.
    pub fn color(self) -> u32 {
        match self {
            Boon::LongLine | Boon::QuickReel | Boon::SwingBoost => 0x5fd0e8,
            Boon::Locket | Boon::StormWard | Boon::Phoenix => 0xff6f6f,
            Boon::SecondWind | Boon::SpringHeel | Boon::WallGrip => 0x9be07a,
            Boon::Momentum => 0xffb347,
            Boon::EchoWell | Boon::Rhythm => 0xb48cff,
            Boon::Magnet | Boon::Gilded | Boon::Glowcap => 0xffe066,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Route {
    Steady,
    Daring,
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Debug)]
pub enum Medal {
    None,
    Bronze,
    Silver,
    Gold,
}

impl Medal {
    pub fn name(self) -> &'static str {
        match self {
            Medal::None => "no medal",
            Medal::Bronze => "Bronze",
            Medal::Silver => "Silver",
            Medal::Gold => "Gold",
        }
    }

    pub fn bonus(self) -> i64 {
        match self {
            Medal::None => 0,
            Medal::Bronze => 100,
            Medal::Silver => 250,
            Medal::Gold => 500,
        }
    }
}

/// Par-based medals: gold at or under par, silver 1.45x, bronze 2.1x.
pub fn medal_for(time: f32, par: f32) -> Medal {
    if time <= par {
        Medal::Gold
    } else if time <= par * 1.45 {
        Medal::Silver
    } else if time <= par * 2.1 {
        Medal::Bronze
    } else {
        Medal::None
    }
}

#[derive(Clone, Debug)]
pub struct StageResult {
    pub stage: usize,
    pub time: f32,
    pub par: f32,
    pub medal: Medal,
    pub crystals: u32,
    pub damage: u32,
    pub falls: u32,
    pub rams: u32,
    pub smashes: u32,
    pub chests: u32,
    pub stage_score: i64,
    pub bonus: i64,
    pub new_best_time: bool,
}

pub struct PerkDef {
    pub name: &'static str,
    pub desc: &'static str,
    pub costs: &'static [u32],
}

pub const PERKS: [PerkDef; PERK_COUNT] = [
    PerkDef {
        name: "Stout Heart",
        desc: "+1 heart at the start of every run",
        costs: &[300, 900],
    },
    PerkDef {
        name: "Echo Reservoir",
        desc: "+25 rewind meter",
        costs: &[200, 450, 800],
    },
    PerkDef {
        name: "Long Reach",
        desc: "+8% hook range",
        costs: &[250, 500, 900],
    },
    PerkDef {
        name: "Crystal Sense",
        desc: "Crystals are drawn from farther away",
        costs: &[200, 550],
    },
    PerkDef {
        name: "Lucky Start",
        desc: "Begin each run with a random boon",
        costs: &[700],
    },
];

pub struct Run {
    pub seed: u64,
    pub heat: u8,
    /// 0..=5; 5 is the Stormheart.
    pub stage: usize,
    /// True while the training ascent is still to play.
    pub training: bool,
    pub boons: Vec<Boon>,
    pub mods: Mods,
    pub hp: i32,
    pub echo: f32,
    pub score: i64,
    pub crystals: u32,
    pub results: Vec<StageResult>,
    pub route: Route,
    pub pending_picks: u32,
    pub rng: Rng,
    pub total_time: f32,
    pub won: bool,
    pub explorer: bool,
    pub lucky: Option<Boon>,
}

impl Run {
    pub fn new(seed: u64, save: &Save, training: bool) -> Run {
        let mut mods = Mods {
            assist_aim: save.aim_assist,
            invincible: save.explorer,
            ..Mods::default()
        };
        mods.max_hp += save.perks[0] as i32;
        mods.echo_max += 25.0 * save.perks[1] as f32;
        mods.hook_range *= 1.0 + 0.08 * save.perks[2] as f32;
        mods.magnet *= 1.0 + 0.35 * save.perks[3] as f32;
        // Heat: harder ascents (see StageSpec::difficulty) and a leaner kit.
        if save.heat >= 2 {
            mods.max_hp -= 1;
        }
        if save.heat >= 3 {
            mods.echo_max *= 0.7;
        }
        let mut rng = Rng::new(seed ^ 0x9E37_79B9);
        let mut boons = vec![];
        let mut lucky = None;
        if save.perks[4] > 0 && !training {
            let b = *rng.pick(&Boon::ALL);
            b.apply(&mut mods);
            boons.push(b);
            lucky = Some(b);
        }
        let hp = mods.max_hp;
        let echo = mods.echo_max;
        Run {
            seed,
            heat: save.heat,
            stage: 0,
            training,
            boons,
            mods,
            hp,
            echo,
            score: 0,
            crystals: 0,
            results: vec![],
            route: Route::Steady,
            pending_picks: 0,
            rng,
            total_time: 0.0,
            won: false,
            explorer: save.explorer,
            lucky,
        }
    }

    pub fn spec(&self) -> StageSpec {
        let mut s = StageSpec::new(self.stage, self.seed.wrapping_add(self.stage as u64 * 7919));
        s.heat = self.heat;
        s.daring = self.route == Route::Daring && self.stage > 0 && self.stage < 5;
        if self.training {
            s.tutorial = true;
            s.stage = 0;
        }
        s
    }

    /// Mods for the stage about to start (route bonuses on top of boons).
    pub fn stage_mods(&self) -> Mods {
        let mut m = self.mods.clone();
        if self.route == Route::Daring && self.stage > 0 && self.stage < 5 {
            m.crystal_value *= 1.5;
        }
        m
    }

    pub fn make_game(&self, level: Arc<Level>) -> Game {
        Game::new(level, self.stage_mods(), self.hp, self.echo)
    }

    pub fn stage_label(&self) -> String {
        if self.training {
            "Training Ascent".to_string()
        } else if self.stage >= 5 {
            "Finale".to_string()
        } else {
            format!("Ascent {}", self.stage + 1)
        }
    }

    /// Fold a cleared stage into the run.
    pub fn finish_stage(&mut self, game: &Game, save: &mut Save) -> StageResult {
        let w = &game.world;
        let time = w.time;
        let par = w.level.par;
        let medal = medal_for(time, par);
        let mut bonus = medal.bonus();
        if w.stats.damage == 0 {
            bonus += 200;
        }
        let stage_score = w.stats.score + bonus;
        let idx = if self.training {
            usize::MAX
        } else {
            self.stage
        };
        let new_best = !self.explorer && idx < 6 && save.record_time(idx, time);
        let res = StageResult {
            stage: self.stage,
            time,
            par,
            medal,
            crystals: w.stats.crystals,
            damage: w.stats.damage,
            falls: w.stats.falls,
            rams: w.stats.rams,
            smashes: w.stats.smashes,
            chests: w.stats.chests,
            stage_score,
            bonus,
            new_best_time: new_best,
        };
        if !self.training {
            self.score += stage_score;
            self.crystals += w.stats.crystals;
            self.total_time += time;
            self.results.push(res.clone());
        }
        self.hp = w.p.hp.max(1);
        self.echo = game.echo;
        // keep revive/shield state
        self.mods.revive = w.mods.revive;
        res
    }

    /// Three distinct boons not yet owned.
    pub fn draft_options(&mut self) -> Vec<Boon> {
        let mut pool: Vec<Boon> = Boon::ALL
            .iter()
            .copied()
            .filter(|b| !self.boons.contains(b))
            .collect();
        self.rng.shuffle(&mut pool);
        pool.truncate(3);
        pool
    }

    pub fn pick(&mut self, b: Boon) {
        b.apply(&mut self.mods);
        self.boons.push(b);
        if b == Boon::Locket {
            self.hp = (self.hp + 1).min(self.mods.max_hp);
        }
        self.pending_picks = self.pending_picks.saturating_sub(1);
    }

    /// Move on after clearing the current stage; returns false when the run
    /// is finished (boss beaten).
    pub fn advance(&mut self) -> bool {
        if self.training {
            self.training = false;
            self.stage = 0;
            return true;
        }
        if self.stage >= 5 {
            self.won = true;
            return false;
        }
        self.stage += 1;
        true
    }

    /// Choose the route for the coming stage (heals on Steady).
    pub fn set_route(&mut self, r: Route) {
        self.route = r;
        if r == Route::Steady {
            self.hp = (self.hp + 1).min(self.mods.max_hp);
        }
    }

    /// Final score including the boss win bonus.
    pub fn final_score(&self) -> i64 {
        self.score + if self.won { 2000 } else { 0 }
    }

    /// Glimmer earned from this run for the Workshop.
    pub fn glimmer_earned(&self) -> u32 {
        self.crystals + if self.won { 200 } else { 0 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn medals_by_par() {
        assert_eq!(medal_for(50.0, 60.0), Medal::Gold);
        assert_eq!(medal_for(80.0, 60.0), Medal::Silver);
        assert_eq!(medal_for(120.0, 60.0), Medal::Bronze);
        assert_eq!(medal_for(200.0, 60.0), Medal::None);
    }

    #[test]
    fn boons_change_mods_and_drafts_are_distinct() {
        let mut m = Mods::default();
        Boon::LongLine.apply(&mut m);
        Boon::Locket.apply(&mut m);
        Boon::Phoenix.apply(&mut m);
        assert!(m.hook_range > 500.0 && m.max_hp == 5 && m.revive);
        let save = Save::default();
        let mut run = Run::new(3, &save, false);
        for _ in 0..30 {
            let d = run.draft_options();
            assert_eq!(d.len(), 3);
            assert!(d[0] != d[1] && d[1] != d[2] && d[0] != d[2]);
        }
        // every boon eventually drafted until the pool empties
        for _ in 0..15 {
            let d = run.draft_options();
            if let Some(b) = d.first() {
                run.pick(*b);
            }
        }
        assert_eq!(run.boons.len(), 15);
        assert!(run.draft_options().is_empty());
    }

    #[test]
    fn perks_apply_at_run_start() {
        let save = Save {
            perks: [2, 3, 1, 0, 1],
            ..Save::default()
        };
        let run = Run::new(9, &save, false);
        assert_eq!(
            run.mods.max_hp,
            6 + run.boons.contains(&Boon::Locket) as i32
        );
        assert!(run.mods.echo_max >= 175.0);
        assert_eq!(run.boons.len(), 1);
    }

    #[test]
    fn stage_flow() {
        let save = Save::default();
        let mut run = Run::new(5, &save, true);
        assert!(run.spec().tutorial);
        assert!(run.advance());
        assert_eq!(run.stage, 0);
        for _ in 0..5 {
            assert!(run.advance());
        }
        assert_eq!(run.stage, 5);
        assert!(!run.advance());
        assert!(run.won);
    }
}
