//! Unit kinds, abilities and their static data.

use crate::map::Pos;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Team {
    Squad,
    Dark,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Kind {
    Warden,
    Ranger,
    Lamplighter,
    Shade,
    Wisp,
    Snuffer,
    Brute,
    Hexer,
    HollowKing,
}

impl Kind {
    pub const SQUAD: [Kind; 3] = [Kind::Warden, Kind::Ranger, Kind::Lamplighter];
    pub const ENEMIES: [Kind; 6] = [
        Kind::Shade,
        Kind::Wisp,
        Kind::Snuffer,
        Kind::Brute,
        Kind::Hexer,
        Kind::HollowKing,
    ];

    pub fn team(self) -> Team {
        match self {
            Kind::Warden | Kind::Ranger | Kind::Lamplighter => Team::Squad,
            _ => Team::Dark,
        }
    }
    pub fn name(self) -> &'static str {
        match self {
            Kind::Warden => "Warden",
            Kind::Ranger => "Ranger",
            Kind::Lamplighter => "Lamplighter",
            Kind::Shade => "Shade",
            Kind::Wisp => "Wisp",
            Kind::Snuffer => "Snuffer",
            Kind::Brute => "Gloam Brute",
            Kind::Hexer => "Hexer",
            Kind::HollowKing => "The Hollow King",
        }
    }
    /// Name with a definite article, for log lines.
    pub fn the(self) -> String {
        let n = self.name();
        if n.starts_with("The ") {
            n.to_string()
        } else {
            format!("The {n}")
        }
    }

    pub fn blurb(self) -> &'static str {
        match self {
            Kind::Warden => "Shield-bearer. Shoves foes, guards allies.",
            Kind::Ranger => "Archer. Deadly at range, frail up close.",
            Kind::Lamplighter => "Keeper of flame. Flares, heals, kindles.",
            Kind::Shade => "Fast hunter. +1 damage when it strikes from darkness.",
            Kind::Wisp => "Flying caster. Bolts from range, flees melee.",
            Kind::Snuffer => "Hunts burning braziers and puts them out.",
            Kind::Brute => "Hulking mass. Slams and shoves.",
            Kind::Hexer => "Curses (weakens) and summons shades.",
            Kind::HollowKing => "Its aura drinks lantern light. Flares still burn.",
        }
    }
    pub fn base_hp(self) -> i32 {
        match self {
            Kind::Warden => 14,
            Kind::Ranger => 10,
            Kind::Lamplighter => 10,
            Kind::Shade => 4,
            Kind::Wisp => 3,
            Kind::Snuffer => 3,
            Kind::Brute => 9,
            Kind::Hexer => 5,
            Kind::HollowKing => 40,
        }
    }
    pub fn base_move(self) -> i32 {
        match self {
            Kind::Warden => 3,
            Kind::Ranger => 4,
            Kind::Lamplighter => 3,
            Kind::Shade => 4,
            Kind::Wisp => 3,
            Kind::Snuffer => 4,
            Kind::Brute => 2,
            Kind::Hexer => 3,
            Kind::HollowKing => 2,
        }
    }
    /// Flying units ignore height and may cross water and the void.
    pub fn flies(self) -> bool {
        matches!(self, Kind::Wisp)
    }
    pub fn abilities(self) -> &'static [Ability] {
        match self {
            Kind::Warden => &[Ability::Strike, Ability::Bash, Ability::Bulwark],
            Kind::Ranger => &[Ability::Shoot, Ability::Volley],
            Kind::Lamplighter => &[Ability::Flare, Ability::Mend, Ability::Kindle],
            Kind::Shade => &[Ability::Claw],
            Kind::Wisp => &[Ability::Bolt],
            Kind::Snuffer => &[Ability::Snuff, Ability::Nip],
            Kind::Brute => &[Ability::Slam],
            Kind::Hexer => &[Ability::Curse, Ability::Summon],
            Kind::HollowKing => &[Ability::Rend, Ability::Eclipse, Ability::Summon],
        }
    }
    /// Score value of defeating this enemy.
    pub fn value(self) -> i32 {
        match self {
            Kind::Shade => 10,
            Kind::Wisp => 12,
            Kind::Snuffer => 10,
            Kind::Brute => 25,
            Kind::Hexer => 25,
            Kind::HollowKing => 500,
            _ => 0,
        }
    }
    /// Spawn budget cost.
    pub fn cost(self) -> i32 {
        match self {
            Kind::Shade => 2,
            Kind::Wisp => 2,
            Kind::Snuffer => 2,
            Kind::Brute => 4,
            Kind::Hexer => 4,
            _ => 99,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Target {
    Enemy,
    Ally,
    Brazier,
    SelfOnly,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum Ability {
    Strike,
    Bash,
    Bulwark,
    Shoot,
    Volley,
    Flare,
    Mend,
    Kindle,
    Claw,
    Bolt,
    Snuff,
    Nip,
    Slam,
    Curse,
    Summon,
    Rend,
    Eclipse,
}

pub struct AbilityInfo {
    pub name: &'static str,
    pub desc: &'static str,
    pub min_range: i32,
    pub max_range: i32,
    pub damage: i32,
    pub cooldown: u8,
    pub target: Target,
    /// Ranged attacks benefit from high ground (+1 range).
    pub ranged: bool,
}

impl Ability {
    pub fn info(self) -> AbilityInfo {
        let a = |name, desc, min_range, max_range, damage, cooldown, target, ranged| AbilityInfo {
            name,
            desc,
            min_range,
            max_range,
            damage,
            cooldown,
            target,
            ranged,
        };
        use Target::*;
        match self {
            Ability::Strike => a("Strike", "Melee hit for 3.", 1, 1, 3, 0, Enemy, false),
            Ability::Bash => a(
                "Shield Bash",
                "Hit for 2 and shove 1 tile. Into the void or water: lethal. Into a wall: +1.",
                1,
                1,
                2,
                2,
                Enemy,
                false,
            ),
            Ability::Bulwark => a(
                "Bulwark",
                "Warden and adjacent allies take 1 less damage until your next turn.",
                0,
                0,
                0,
                3,
                SelfOnly,
                false,
            ),
            Ability::Shoot => a(
                "Shoot",
                "Arrow for 3 at range 2-4.",
                2,
                4,
                3,
                0,
                Enemy,
                true,
            ),
            Ability::Volley => a(
                "Volley",
                "2 damage to the target and every enemy next to it.",
                2,
                4,
                2,
                3,
                Enemy,
                true,
            ),
            Ability::Flare => a(
                "Flare",
                "2 damage and lights the area (radius 2) for 2 rounds. Burns through the King's aura.",
                1,
                3,
                2,
                0,
                Enemy,
                true,
            ),
            Ability::Mend => a(
                "Mend",
                "Heal an ally (or self) for 4.",
                0,
                2,
                0,
                2,
                Ally,
                false,
            ),
            Ability::Kindle => a(
                "Kindle",
                "Light an adjacent brazier (radius 3 light).",
                1,
                1,
                0,
                0,
                Brazier,
                false,
            ),
            Ability::Claw => a("Claw", "Melee for 2.", 1, 1, 2, 0, Enemy, false),
            Ability::Bolt => a("Bolt", "Ranged bolt for 2.", 2, 3, 2, 0, Enemy, true),
            Ability::Snuff => a("Snuff", "Extinguish a brazier.", 1, 1, 0, 0, Brazier, false),
            Ability::Nip => a("Nip", "Melee for 1.", 1, 1, 1, 0, Enemy, false),
            Ability::Slam => a("Slam", "Melee for 3 and shove.", 1, 1, 3, 0, Enemy, false),
            Ability::Curse => a(
                "Curse",
                "1 damage and Weaken (-1 damage) for 2 rounds.",
                2,
                4,
                1,
                0,
                Enemy,
                true,
            ),
            Ability::Summon => a(
                "Summon",
                "Call shades from the dark.",
                0,
                0,
                0,
                5,
                SelfOnly,
                false,
            ),
            Ability::Rend => a("Rend", "Melee for 4 and shove.", 1, 1, 4, 0, Enemy, false),
            Ability::Eclipse => a(
                "Eclipse",
                "Snuffs braziers within 3; 2 damage to every squad member in darkness.",
                0,
                0,
                2,
                3,
                SelfOnly,
                false,
            ),
        }
    }
    pub fn pushes(self) -> bool {
        matches!(self, Ability::Bash | Ability::Slam | Ability::Rend)
    }
    pub fn hotkey_label(index: usize) -> &'static str {
        ["1", "2", "3"][index.min(2)]
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Cause {
    Wounds,
    Void,
    Water,
}

#[derive(Clone, Debug)]
pub struct Unit {
    pub id: u32,
    pub kind: Kind,
    pub pos: Pos,
    pub hp: i32,
    pub max_hp: i32,
    pub move_range: i32,
    pub power: i32,
    pub range_bonus: i32,
    pub lantern: i32,
    pub moved: bool,
    pub acted: bool,
    pub cooldowns: [u8; 3],
    pub weakened: u8,
    pub guarded: bool,
    /// Shades that begin their turn in darkness ambush for +1.
    pub lurking: bool,
    pub alive: bool,
    pub elite: bool,
    pub death: Option<Cause>,
    /// Facing for sprites: +1 right, -1 left (screen space).
    pub facing: i32,
}

impl Unit {
    pub fn new(id: u32, kind: Kind, pos: Pos) -> Unit {
        Unit {
            id,
            kind,
            pos,
            hp: kind.base_hp(),
            max_hp: kind.base_hp(),
            move_range: kind.base_move(),
            power: 0,
            range_bonus: 0,
            lantern: if kind.team() == Team::Squad { 2 } else { 0 },
            moved: false,
            acted: false,
            cooldowns: [0; 3],
            weakened: 0,
            guarded: false,
            lurking: false,
            alive: true,
            elite: false,
            death: None,
            facing: 1,
        }
    }
    pub fn team(&self) -> Team {
        self.kind.team()
    }
    pub fn ability_index(&self, a: Ability) -> Option<usize> {
        self.kind.abilities().iter().position(|x| *x == a)
    }
    pub fn ready(&self, a: Ability) -> bool {
        self.ability_index(a)
            .is_some_and(|i| self.cooldowns[i] == 0)
    }
    pub fn make_elite(&mut self) {
        self.elite = true;
        self.max_hp += (self.max_hp + 1) / 2;
        self.hp = self.max_hp;
        self.power += 1;
    }
}
