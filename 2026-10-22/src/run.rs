//! A run: seven islands, upgrade drafts, route choices, the Hollow King.

use crate::battle::{Battle, Config, Member, Mods, Outcome};
use crate::map::Biome;
use crate::rng::Rng;
use crate::units::Kind;

pub const ISLANDS: u32 = 7;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Upgrade {
    TemperedLantern,
    IronHide,
    Fletching,
    EmberHeart,
    HeavyShield,
    KindledArrows,
    Wildfire,
    BattleHymn,
    SecondWind,
    SearingVolley,
    PyreKeeper,
    Swiftboots,
    Whetstone,
    EagleEye,
}

impl Upgrade {
    pub const ALL: [Upgrade; 14] = [
        Upgrade::TemperedLantern,
        Upgrade::IronHide,
        Upgrade::Fletching,
        Upgrade::EmberHeart,
        Upgrade::HeavyShield,
        Upgrade::KindledArrows,
        Upgrade::Wildfire,
        Upgrade::BattleHymn,
        Upgrade::SecondWind,
        Upgrade::SearingVolley,
        Upgrade::PyreKeeper,
        Upgrade::Swiftboots,
        Upgrade::Whetstone,
        Upgrade::EagleEye,
    ];

    pub fn name(self) -> &'static str {
        match self {
            Upgrade::TemperedLantern => "Tempered Lanterns",
            Upgrade::IronHide => "Iron Hide",
            Upgrade::Fletching => "Long Fletching",
            Upgrade::EmberHeart => "Ember Heart",
            Upgrade::HeavyShield => "Heavy Shield",
            Upgrade::KindledArrows => "Kindled Arrows",
            Upgrade::Wildfire => "Wildfire",
            Upgrade::BattleHymn => "Battle Hymn",
            Upgrade::SecondWind => "Second Wind",
            Upgrade::SearingVolley => "Searing Volley",
            Upgrade::PyreKeeper => "Pyre Keeper",
            Upgrade::Swiftboots => "Swiftboots",
            Upgrade::Whetstone => "Whetstone",
            Upgrade::EagleEye => "Eagle Eye",
        }
    }
    pub fn desc(self) -> &'static str {
        match self {
            Upgrade::TemperedLantern => "All lanterns +1 radius.",
            Upgrade::IronHide => "Warden +4 max HP.",
            Upgrade::Fletching => "Ranger +1 range.",
            Upgrade::EmberHeart => "Whole squad +2 max HP and heal 2.",
            Upgrade::HeavyShield => "Shield Bash shoves 2 tiles.",
            Upgrade::KindledArrows => "Ranger's Shoot sets the target tile alight for 2 rounds.",
            Upgrade::Wildfire => "Flare +1 damage and +1 light radius.",
            Upgrade::BattleHymn => "Kills heal the killer 1 HP.",
            Upgrade::SecondWind => "Mend heals +2 and is ready every turn.",
            Upgrade::SearingVolley => "Volley +1 damage.",
            Upgrade::PyreKeeper => "Braziers +1 radius; one extra starts lit.",
            Upgrade::Swiftboots => "Whole squad +1 move.",
            Upgrade::Whetstone => "Warden +1 damage.",
            Upgrade::EagleEye => "Ranger +1 damage.",
        }
    }
    /// Who it applies to, for the card art.
    pub fn owner(self) -> Option<Kind> {
        match self {
            Upgrade::IronHide | Upgrade::HeavyShield | Upgrade::Whetstone => Some(Kind::Warden),
            Upgrade::Fletching
            | Upgrade::KindledArrows
            | Upgrade::SearingVolley
            | Upgrade::EagleEye => Some(Kind::Ranger),
            Upgrade::Wildfire | Upgrade::SecondWind => Some(Kind::Lamplighter),
            _ => None,
        }
    }
    pub fn stackable(self) -> bool {
        matches!(
            self,
            Upgrade::IronHide
                | Upgrade::EmberHeart
                | Upgrade::BattleHymn
                | Upgrade::Whetstone
                | Upgrade::EagleEye
        )
    }
    pub fn min_island(self) -> u32 {
        match self {
            Upgrade::Swiftboots => 3,
            _ => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Node {
    Skirmish(Biome),
    Elite(Biome),
    Sanctuary,
    Throne,
}

impl Node {
    pub fn title(self) -> &'static str {
        match self {
            Node::Skirmish(_) => "Skirmish",
            Node::Elite(_) => "Elite Hunt",
            Node::Sanctuary => "Sanctuary",
            Node::Throne => "The Hollow Throne",
        }
    }
    pub fn desc(self) -> &'static str {
        match self {
            Node::Skirmish(_) => "A dark-held island. Clear it.",
            Node::Elite(_) => "Tougher foes, elites. Double score, two upgrade picks.",
            Node::Sanctuary => "A safe shore. Full heal, fallen rise, one upgrade.",
            Node::Throne => "Face the Hollow King.",
        }
    }
    pub fn biome(self) -> Biome {
        match self {
            Node::Skirmish(b) | Node::Elite(b) => b,
            Node::Sanctuary => Biome::Moss,
            Node::Throne => Biome::Hollow,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Stage {
    Battle,
    Draft { options: Vec<Upgrade>, picks: u8 },
    Route { options: Vec<Node> },
    Victory,
    Defeat { cause: String },
}

#[derive(Clone, Debug)]
pub struct Summary {
    pub island: u32,
    pub rounds: u32,
    pub kills: u32,
    pub battle_score: i64,
    pub bonus: i64,
    pub best_chain: u32,
    pub flawless: bool,
}

#[derive(Clone, Debug)]
pub struct Run {
    pub seed: u64,
    pub rng: Rng,
    /// 1-based island currently being played or chosen.
    pub island: u32,
    pub node: Node,
    pub squad: Vec<Member>,
    pub mods: Mods,
    pub upgrades: Vec<Upgrade>,
    pub score: i64,
    pub stage: Stage,
    pub battle: Option<Battle>,
    pub kills: u32,
    pub best_chain: u32,
    pub last_summary: Option<Summary>,
    pub god_mode: bool,
}

impl Run {
    pub fn new(seed: u64) -> Run {
        let mut rng = Rng::new(seed);
        let biome = *rng.pick(&Biome::NORMAL);
        let mut r = Run {
            seed,
            rng,
            island: 1,
            node: Node::Skirmish(biome),
            squad: Kind::SQUAD.iter().map(|k| Member::new(*k)).collect(),
            mods: Mods::default(),
            upgrades: vec![],
            score: 0,
            stage: Stage::Battle,
            battle: None,
            kills: 0,
            best_chain: 0,
            last_summary: None,
            god_mode: false,
        };
        r.start_battle();
        r
    }

    fn start_battle(&mut self) {
        let cfg = Config {
            seed: self
                .seed
                .wrapping_mul(31)
                .wrapping_add(self.island as u64 * 7919),
            island: self.island,
            biome: self.node.biome(),
            elite: matches!(self.node, Node::Elite(_)),
            boss: self.node == Node::Throne,
            mods: self.mods.clone(),
            god_mode: self.god_mode,
        };
        self.battle = Some(Battle::new(cfg, &self.squad));
        self.stage = Stage::Battle;
    }

    pub fn battle(&self) -> Option<&Battle> {
        self.battle.as_ref()
    }
    pub fn battle_mut(&mut self) -> Option<&mut Battle> {
        self.battle.as_mut()
    }

    /// Call when the battle reports an outcome. Advances the run.
    pub fn finish_battle(&mut self) {
        let Some(b) = self.battle.take() else {
            return;
        };
        match b.outcome {
            Some(Outcome::Victory) => {
                let flawless = b.fallen == 0;
                let mut bonus = 100 * self.island as i64;
                bonus += (10 - b.round as i64).max(0) * 15;
                if flawless {
                    bonus += 75;
                }
                if matches!(self.node, Node::Elite(_)) {
                    bonus = bonus * 3 / 2;
                }
                if self.node == Node::Throne {
                    bonus += 1000;
                }
                self.score += b.score + bonus;
                self.kills += b.kills;
                self.best_chain = self.best_chain.max(b.best_chain);
                self.last_summary = Some(Summary {
                    island: self.island,
                    rounds: b.round,
                    kills: b.kills,
                    battle_score: b.score,
                    bonus,
                    best_chain: b.best_chain,
                    flawless,
                });
                // Carry HP back; the fallen rise weakened.
                for (kind, hp, alive) in b.squad_members() {
                    if let Some(m) = self.squad.iter_mut().find(|m| m.kind == kind) {
                        m.hp = if alive {
                            (hp + m.max_hp * 4 / 10).min(m.max_hp)
                        } else {
                            (m.max_hp / 2).max(1)
                        };
                    }
                }
                if self.node == Node::Throne {
                    self.stage = Stage::Victory;
                } else {
                    let picks = if matches!(self.node, Node::Elite(_)) {
                        2
                    } else {
                        1
                    };
                    self.open_draft(picks);
                }
            }
            Some(Outcome::Defeat) => {
                self.score += b.score;
                self.kills += b.kills;
                self.stage = Stage::Defeat {
                    cause: b.defeat_cause.clone(),
                };
                self.battle = Some(b);
            }
            None => {
                self.battle = Some(b);
            }
        }
    }

    fn open_draft(&mut self, picks: u8) {
        let mut pool: Vec<Upgrade> = Upgrade::ALL
            .iter()
            .copied()
            .filter(|u| u.stackable() || !self.upgrades.contains(u))
            .filter(|u| u.min_island() <= self.island)
            .collect();
        self.rng.shuffle(&mut pool);
        pool.truncate(3);
        self.stage = Stage::Draft {
            options: pool,
            picks,
        };
    }

    pub fn choose_upgrade(&mut self, index: usize) -> bool {
        let Stage::Draft { options, picks } = self.stage.clone() else {
            return false;
        };
        let Some(&u) = options.get(index) else {
            return false;
        };
        self.apply(u);
        if picks > 1 {
            let mut rest = options.clone();
            rest.remove(index);
            // Refill to three for the second pick.
            let mut pool: Vec<Upgrade> = Upgrade::ALL
                .iter()
                .copied()
                .filter(|x| (x.stackable() || !self.upgrades.contains(x)) && !rest.contains(x))
                .filter(|x| x.min_island() <= self.island)
                .collect();
            self.rng.shuffle(&mut pool);
            while rest.len() < 3 {
                match pool.pop() {
                    Some(x) => rest.push(x),
                    None => break,
                }
            }
            self.stage = Stage::Draft {
                options: rest,
                picks: picks - 1,
            };
        } else {
            self.open_route();
        }
        true
    }

    fn apply(&mut self, u: Upgrade) {
        self.upgrades.push(u);
        let member = |s: &mut Vec<Member>, k: Kind| -> usize {
            s.iter().position(|m| m.kind == k).expect("squad member")
        };
        match u {
            Upgrade::TemperedLantern => self.squad.iter_mut().for_each(|m| m.lantern += 1),
            Upgrade::IronHide => {
                let i = member(&mut self.squad, Kind::Warden);
                self.squad[i].max_hp += 4;
                self.squad[i].hp += 4;
            }
            Upgrade::Fletching => {
                let i = member(&mut self.squad, Kind::Ranger);
                self.squad[i].range_bonus += 1;
            }
            Upgrade::EmberHeart => self.squad.iter_mut().for_each(|m| {
                m.max_hp += 2;
                m.hp = (m.hp + 4).min(m.max_hp);
            }),
            Upgrade::HeavyShield => self.mods.bash_push += 1,
            Upgrade::KindledArrows => self.mods.kindled_arrows = true,
            Upgrade::Wildfire => self.mods.flare_bonus += 1,
            Upgrade::BattleHymn => self.mods.kill_heal += 1,
            Upgrade::SecondWind => {
                self.mods.mend_bonus += 2;
                self.mods.mend_haste = true;
            }
            Upgrade::SearingVolley => self.mods.volley_bonus += 1,
            Upgrade::PyreKeeper => {
                self.mods.pyre_bonus += 1;
                self.mods.extra_lit += 1;
            }
            Upgrade::Swiftboots => self.squad.iter_mut().for_each(|m| m.move_range += 1),
            Upgrade::Whetstone => {
                let i = member(&mut self.squad, Kind::Warden);
                self.squad[i].power += 1;
            }
            Upgrade::EagleEye => {
                let i = member(&mut self.squad, Kind::Ranger);
                self.squad[i].power += 1;
            }
        }
    }

    fn open_route(&mut self) {
        let next = self.island + 1;
        if next >= ISLANDS {
            self.stage = Stage::Route {
                options: vec![Node::Throne],
            };
            return;
        }
        let current = self.node.biome();
        let mut biomes: Vec<Biome> = Biome::NORMAL
            .iter()
            .copied()
            .filter(|b| *b != current)
            .collect();
        self.rng.shuffle(&mut biomes);
        let a = Node::Skirmish(biomes[0]);
        let b = if next >= 3 && self.node != Node::Sanctuary && self.rng.chance(0.6) {
            Node::Sanctuary
        } else {
            Node::Elite(biomes[1])
        };
        self.stage = Stage::Route {
            options: vec![a, b],
        };
    }

    pub fn choose_route(&mut self, index: usize) -> bool {
        let Stage::Route { options } = self.stage.clone() else {
            return false;
        };
        let Some(&node) = options.get(index) else {
            return false;
        };
        self.island += 1;
        self.node = node;
        if node == Node::Sanctuary {
            for m in self.squad.iter_mut() {
                m.hp = m.max_hp;
            }
            self.open_draft(1);
        } else {
            self.start_battle();
        }
        true
    }

    pub fn over(&self) -> bool {
        matches!(self.stage, Stage::Victory | Stage::Defeat { .. })
    }

    pub fn won(&self) -> bool {
        self.stage == Stage::Victory
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn draft_offers_three_distinct() {
        let mut r = Run::new(5);
        r.open_draft(1);
        let Stage::Draft { options, .. } = &r.stage else {
            panic!()
        };
        assert_eq!(options.len(), 3);
        assert_ne!(options[0], options[1]);
        assert_ne!(options[1], options[2]);
    }

    #[test]
    fn upgrades_apply() {
        let mut r = Run::new(5);
        let lantern = r.squad[0].lantern;
        r.apply(Upgrade::TemperedLantern);
        assert_eq!(r.squad[0].lantern, lantern + 1);
        let hp = r.squad[0].max_hp;
        r.apply(Upgrade::IronHide);
        assert_eq!(r.squad[0].max_hp, hp + 4);
        r.apply(Upgrade::HeavyShield);
        assert_eq!(r.mods.bash_push, 1);
    }

    #[test]
    fn route_ends_at_throne() {
        let mut r = Run::new(9);
        r.island = ISLANDS - 1;
        r.open_route();
        assert_eq!(
            r.stage,
            Stage::Route {
                options: vec![Node::Throne]
            }
        );
        assert!(r.choose_route(0));
        assert_eq!(r.island, ISLANDS);
        assert!(r.battle().unwrap().cfg.boss);
    }
}
