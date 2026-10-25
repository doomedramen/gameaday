//! The Cup: five rounds with route choice, a parts draft, a garage of
//! permanent upgrades bought with prize money, qualifying targets, second
//! chances and a points table. The Grand Prix final adds the Champion.

use crate::car::Stats;
use crate::race::{CHAMPION, Driver, PLAYER, ROSTER};
use crate::rng::Rng;
use crate::track::{Biome, TrackSpec};

pub const ROUNDS: usize = 5;
pub const POINTS: [u32; 8] = [10, 8, 6, 5, 4, 3, 2, 1];
pub const PRIZE: [u32; 8] = [1500, 1100, 850, 650, 500, 380, 280, 200];
pub const LEVEL_COST: [u32; 3] = [600, 1100, 1700];

pub fn qualify_place(round: usize) -> usize {
    [5, 4, 3, 3, 1][round.min(ROUNDS - 1)]
}

pub fn skill(round: usize) -> f32 {
    [0.87, 0.93, 0.97, 1.0, 1.04][round.min(ROUNDS - 1)]
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Part {
    SlipFins,
    DraftMagnet,
    TurboSpool,
    PurpleHaze,
    BullBar,
    KerbSurfer,
    Recycler,
    OffRoad,
    Featherweight,
    Showboat,
    SelfRepair,
    Afterburner,
    LaunchControl,
    Scavenger,
}

pub const ALL_PARTS: [Part; 14] = [
    Part::SlipFins,
    Part::DraftMagnet,
    Part::TurboSpool,
    Part::PurpleHaze,
    Part::BullBar,
    Part::KerbSurfer,
    Part::Recycler,
    Part::OffRoad,
    Part::Featherweight,
    Part::Showboat,
    Part::SelfRepair,
    Part::Afterburner,
    Part::LaunchControl,
    Part::Scavenger,
];

impl Part {
    pub fn name(self) -> &'static str {
        match self {
            Part::SlipFins => "Slip Fins",
            Part::DraftMagnet => "Draft Magnet",
            Part::TurboSpool => "Turbo Spool",
            Part::PurpleHaze => "Purple Haze",
            Part::BullBar => "Bull Bar",
            Part::KerbSurfer => "Kerb Surfer",
            Part::Recycler => "Nitro Recycler",
            Part::OffRoad => "Rally Tyres",
            Part::Featherweight => "Featherweight",
            Part::Showboat => "Showboat",
            Part::SelfRepair => "Self-Repair Kit",
            Part::Afterburner => "Afterburner",
            Part::LaunchControl => "Launch Control",
            Part::Scavenger => "Scavenger",
        }
    }
    pub fn desc(self) -> &'static str {
        match self {
            Part::SlipFins => "Slipstream fills nitro twice as fast.",
            Part::DraftMagnet => "Catch a slipstream from 60% further back.",
            Part::TurboSpool => "Drift tiers charge 35% faster.",
            Part::PurpleHaze => "Mini-turbos last 50% longer.",
            Part::BullBar => "Half damage, and you shove harder.",
            Part::KerbSurfer => "Riding the kerbs at speed fills nitro.",
            Part::Recycler => "Nitro slowly refills on its own.",
            Part::OffRoad => "Grass, sand and mud barely slow you.",
            Part::Featherweight => "+20% acceleration, +20% damage taken.",
            Part::Showboat => "Style chains go to x8; style pays 25% more.",
            Part::SelfRepair => "Repairs 1.5% damage every second.",
            Part::Afterburner => "Nitro pushes 20% harder.",
            Part::LaunchControl => "Easier rocket starts with a longer boost.",
            Part::Scavenger => "Cans and wrenches give double.",
        }
    }
    pub fn color(self) -> u32 {
        match self {
            Part::SlipFins | Part::DraftMagnet => 0x9FD8FF,
            Part::TurboSpool | Part::PurpleHaze => 0xC86BFF,
            Part::BullBar | Part::SelfRepair => 0x6BE38A,
            Part::KerbSurfer | Part::Recycler | Part::Afterburner => 0x5BD8FF,
            Part::OffRoad | Part::Scavenger => 0xE6C07A,
            Part::Featherweight | Part::LaunchControl => 0xFF8A3D,
            Part::Showboat => 0xFFD34A,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Garage {
    pub levels: [u8; 4],
}

pub const GARAGE_SLOTS: [(&str, &str); 4] = [
    ("Engine", "+3.5% top speed, +7% acceleration"),
    ("Tyres", "+6% grip, sharper steering"),
    ("Armour", "-18% damage, heavier in contact"),
    ("Nitro Tank", "+0.6s of nitro, +6% nitro power"),
];

/// Player car stats from garage levels and parts.
pub fn build_stats(g: &Garage, parts: &[Part]) -> Stats {
    let mut s = Stats::default();
    let l = |i: usize| g.levels[i] as f32;
    s.top *= 1.0 + 0.035 * l(0);
    s.accel *= 1.0 + 0.07 * l(0);
    s.grip *= 1.0 + 0.06 * l(1);
    s.steer *= 1.0 + 0.025 * l(1);
    s.fragility *= 1.0 - 0.18 * l(2);
    s.mass += 0.1 * l(2);
    s.nitro_secs += 0.6 * l(3);
    s.nitro_power *= 1.0 + 0.06 * l(3);
    for p in parts {
        match p {
            Part::SlipFins => s.draft_rate *= 2.0,
            Part::DraftMagnet => s.draft_range *= 1.6,
            Part::TurboSpool => s.drift_rate *= 1.35,
            Part::PurpleHaze => s.turbo_len *= 1.5,
            Part::BullBar => {
                s.fragility *= 0.5;
                s.mass += 0.4;
            }
            Part::KerbSurfer => s.kerb_nitro = true,
            Part::Recycler => s.nitro_regen += 0.03,
            Part::OffRoad => s.offroad *= 2.2,
            Part::Featherweight => {
                s.accel *= 1.2;
                s.fragility *= 1.2;
            }
            Part::Showboat => s.style_cap = 8,
            Part::SelfRepair => s.regen += 1.5,
            Part::Afterburner => s.nitro_power *= 1.2,
            Part::LaunchControl => s.launch = true,
            Part::Scavenger => s.pickup_mult = 2.0,
        }
    }
    s
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Venue {
    pub spec: TrackSpec,
    pub laps: i32,
    pub label: &'static str,
    pub prize_mult: f32,
}

impl Venue {
    pub fn traits(&self) -> String {
        let t = if self.spec.twisty > 0.5 {
            "Twisty"
        } else if self.spec.twisty < 0.2 {
            "Flowing"
        } else {
            "Mixed"
        };
        let h = if self.spec.hazard > 0.7 {
            "lots of hazards"
        } else if self.spec.hazard < 0.35 {
            "few hazards"
        } else {
            "some hazards"
        };
        format!("{t}, {h}, {} laps", self.laps)
    }
}

const MODS: [(&str, f32, f32, f32); 4] = [
    ("Flowing", 0.12, 0.35, 1.0),
    ("Twisty", 0.62, 0.45, 1.15),
    ("Hazardous", 0.35, 0.95, 1.3),
    ("Endurance", 0.3, 0.5, 1.25),
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Qualified,
    SecondChance,
    Eliminated,
    Champion,
}

#[derive(Clone, Debug)]
pub struct RoundResult {
    pub biome: Biome,
    pub place: usize,
    pub verdict: Verdict,
    pub prize: u32,
    pub style: u32,
}

#[derive(Clone, Debug)]
pub struct Campaign {
    pub round: usize,
    pub cash: u32,
    pub garage: Garage,
    pub parts: Vec<Part>,
    pub chances: u32,
    pub points: Vec<(Driver, u32)>,
    pub venue: Venue,
    pub options: Vec<Venue>,
    pub draft: Vec<Part>,
    pub results: Vec<RoundResult>,
    pub style_total: u32,
    pub purchases: Vec<(usize, u32)>,
    pub rng: Rng,
    pub used: Vec<Biome>,
}

impl Campaign {
    pub fn new(seed: u64) -> Campaign {
        let mut rng = Rng::new(seed);
        let mut points: Vec<(Driver, u32)> = ROSTER.iter().map(|d| (*d, 0)).collect();
        points.push((PLAYER, 0));
        let spec = TrackSpec {
            seed: rng.next_u64(),
            biome: Biome::Greenvale,
            twisty: 0.25,
            hazard: 0.2,
            length: 13000.0,
        };
        Campaign {
            round: 0,
            cash: 0,
            garage: Garage::default(),
            parts: Vec::new(),
            chances: 2,
            points,
            venue: Venue {
                spec,
                laps: 3,
                label: "Season Opener",
                prize_mult: 1.0,
            },
            options: Vec::new(),
            draft: Vec::new(),
            results: Vec::new(),
            style_total: 0,
            purchases: Vec::new(),
            rng,
            used: vec![Biome::Greenvale],
        }
    }

    pub fn stats(&self) -> Stats {
        build_stats(&self.garage, &self.parts)
    }

    pub fn rivals(&self) -> Vec<Driver> {
        let mut v = ROSTER.to_vec();
        if self.round == ROUNDS - 1 {
            v.push(CHAMPION);
        }
        v
    }

    pub fn is_final(&self) -> bool {
        self.round == ROUNDS - 1
    }

    /// Apply a finished race. `order` is driver kinds by finishing place.
    pub fn finish_round(&mut self, order: &[Driver], style: u32) -> RoundResult {
        let place = order
            .iter()
            .position(|d| d.kind == crate::race::Kind::Player)
            .unwrap_or(order.len() - 1)
            + 1;
        let need = qualify_place(self.round);
        let showboat = self.parts.contains(&Part::Showboat);
        let prize = (PRIZE[(place - 1).min(7)] as f32 * self.venue.prize_mult) as u32
            + style / if showboat { 16 } else { 20 };
        self.cash += prize;
        self.style_total += style;
        for (i, d) in order.iter().enumerate() {
            if let Some(e) = self.points.iter_mut().find(|e| e.0.name == d.name) {
                e.1 += POINTS[i.min(7)];
            } else {
                self.points.push((*d, POINTS[i.min(7)]));
            }
        }
        let verdict = if place <= need {
            if self.is_final() {
                Verdict::Champion
            } else {
                Verdict::Qualified
            }
        } else if self.chances > 0 {
            self.chances -= 1;
            Verdict::SecondChance
        } else {
            Verdict::Eliminated
        };
        let r = RoundResult {
            biome: self.venue.spec.biome,
            place,
            verdict,
            prize,
            style,
        };
        self.results.push(r.clone());
        r
    }

    pub fn standings(&self) -> Vec<(Driver, u32)> {
        let mut v = self.points.clone();
        v.sort_by(|a, b| {
            b.1.cmp(&a.1).then(
                (a.0.kind != crate::race::Kind::Player)
                    .cmp(&(b.0.kind != crate::race::Kind::Player)),
            )
        });
        v
    }

    pub fn score(&self) -> u32 {
        let pts = self
            .points
            .iter()
            .find(|p| p.0.kind == crate::race::Kind::Player)
            .map(|p| p.1)
            .unwrap_or(0);
        pts * 100
            + self.style_total / 10
            + if self
                .results
                .last()
                .is_some_and(|r| r.verdict == Verdict::Champion)
            {
                2000
            } else {
                0
            }
    }

    /// Roll three parts the player doesn't own.
    pub fn roll_draft(&mut self) {
        let mut pool: Vec<Part> = ALL_PARTS
            .iter()
            .copied()
            .filter(|p| !self.parts.contains(p))
            .collect();
        self.rng.shuffle(&mut pool);
        pool.truncate(3);
        self.draft = pool;
    }

    pub fn take_part(&mut self, i: usize) {
        if let Some(p) = self.draft.get(i).copied() {
            self.parts.push(p);
        }
        self.draft.clear();
    }

    pub fn level_cost(&self, slot: usize) -> Option<u32> {
        let l = self.garage.levels[slot] as usize;
        LEVEL_COST.get(l).copied()
    }

    pub fn buy(&mut self, slot: usize) -> bool {
        match self.level_cost(slot) {
            Some(c) if c <= self.cash => {
                self.cash -= c;
                self.garage.levels[slot] += 1;
                self.purchases.push((slot, c));
                true
            }
            _ => false,
        }
    }

    /// Undo the most recent purchase made in this garage visit.
    pub fn undo(&mut self) -> bool {
        if let Some((slot, c)) = self.purchases.pop() {
            self.garage.levels[slot] -= 1;
            self.cash += c;
            true
        } else {
            false
        }
    }

    /// Advance to the next round: offers a route choice unless the next round
    /// is the final.
    pub fn next_round(&mut self) {
        self.round += 1;
        self.purchases.clear();
        if self.is_final() {
            let spec = TrackSpec {
                seed: self.rng.next_u64(),
                biome: Biome::GrandPrix,
                twisty: 0.45,
                hazard: 0.4,
                length: 14500.0,
            };
            self.venue = Venue {
                spec,
                laps: 4,
                label: "The Grand Prix",
                prize_mult: 2.0,
            };
            self.options.clear();
        } else {
            self.make_options();
        }
    }

    fn make_options(&mut self) {
        let mut biomes: Vec<Biome> = [
            Biome::Greenvale,
            Biome::DuneSea,
            Biome::Timberline,
            Biome::NeonNight,
        ]
        .into_iter()
        .filter(|b| !self.used.contains(b))
        .collect();
        if biomes.len() < 2 {
            biomes = vec![
                Biome::Greenvale,
                Biome::DuneSea,
                Biome::Timberline,
                Biome::NeonNight,
            ];
            biomes.retain(|b| Some(b) != self.used.last());
        }
        self.rng.shuffle(&mut biomes);
        let mut mods: Vec<usize> = (0..MODS.len()).collect();
        self.rng.shuffle(&mut mods);
        self.options = (0..2)
            .map(|k| {
                let m = MODS[mods[k]];
                let biome = biomes[k % biomes.len()];
                let spec = TrackSpec {
                    seed: self.rng.next_u64(),
                    biome,
                    twisty: m.1,
                    hazard: m.2 + self.round as f32 * 0.05,
                    length: 13500.0 + self.round as f32 * 300.0,
                };
                let laps = if m.0 == "Endurance" { 4 } else { 3 };
                Venue {
                    spec,
                    laps,
                    label: m.0,
                    prize_mult: m.3,
                }
            })
            .collect();
    }

    pub fn choose_route(&mut self, i: usize) {
        if let Some(v) = self.options.get(i).copied() {
            self.venue = v;
            self.used.push(v.spec.biome);
        }
        self.options.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn garage_buy_and_undo() {
        let mut c = Campaign::new(1);
        c.cash = 1000;
        assert!(c.buy(0));
        assert_eq!(c.cash, 400);
        assert!(!c.buy(0), "level 2 costs more than we have");
        assert!(c.undo());
        assert_eq!(c.cash, 1000);
        assert_eq!(c.garage.levels[0], 0);
        assert!(!c.undo());
    }

    #[test]
    fn upgrades_change_stats() {
        let base = build_stats(&Garage::default(), &[]);
        let g = Garage {
            levels: [3, 3, 3, 3],
        };
        let s = build_stats(&g, &[Part::SlipFins, Part::BullBar]);
        assert!(s.top > base.top && s.grip > base.grip && s.fragility < base.fragility * 0.5);
        assert_eq!(s.draft_rate, 2.0);
    }

    #[test]
    fn routes_and_final() {
        let mut c = Campaign::new(9);
        for r in 1..ROUNDS {
            c.next_round();
            if r < ROUNDS - 1 {
                assert_eq!(c.options.len(), 2);
                assert_ne!(c.options[0].spec.biome, c.options[1].spec.biome);
                c.choose_route(1);
            } else {
                assert_eq!(c.venue.spec.biome, Biome::GrandPrix);
                assert!(c.rivals().iter().any(|d| d.name == "Sable"));
            }
        }
    }

    #[test]
    fn second_chances_then_elimination() {
        let mut c = Campaign::new(3);
        let mut order: Vec<Driver> = ROSTER.to_vec();
        order.push(PLAYER); // last place
        assert_eq!(c.finish_round(&order, 0).verdict, Verdict::SecondChance);
        assert_eq!(c.finish_round(&order, 0).verdict, Verdict::SecondChance);
        assert_eq!(c.finish_round(&order, 0).verdict, Verdict::Eliminated);
    }
}
