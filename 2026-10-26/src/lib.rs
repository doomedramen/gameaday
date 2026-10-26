//! Engine-free game rules, campaign, deterministic bot and persistence.
use std::fs;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Card {
    pub id: usize,
    pub gold: bool,
}
impl Card {
    pub fn description(self) -> String {
        if !self.gold {
            return CARDS[self.id].text.into();
        }
        let s = CARDS[self.id];
        match self.id {
            0 => "Grows in 2. Harvest: 15 damage. Starts +1 growth.".into(),
            1 => "Grows in 3. Harvest: 22 damage. Starts +1 growth.".into(),
            2 => "Grows in 3. Harvest: 14 damage, +1 action.".into(),
            3 => "Grows in 2. Harvest: 13 shelter, heal 3.".into(),
            4 => "Grows in 2. Harvest: 13 damage, 4 poison.".into(),
            5 => "Grows in 3. Harvest: 28 damage, 12 shelter.".into(),
            6 => "Grows in 4. Harvest: 35 damage, draw 2.".into(),
            7 => "Grows in 3. Harvest: 17 damage, grow neighbours.".into(),
            8 => "Grows in 2. Harvest: 11 shelter, draw 2.".into(),
            9 => "Grows in 4. Harvest: 29 damage, 23 shelter.".into(),
            10 => "Grow a bed by 3. Draw a card.".into(),
            11 => "Gain 12 shelter. Lasts this enemy turn.".into(),
            12 => "Deal 11 damage to the selected pest.".into(),
            13 => "Grow ALL beds by 2. Gain 7 shelter.".into(),
            15 => "Grow a bed by 4. Gain 8 shelter.".into(),
            16 => "This turn: harvests deal +11 damage.".into(),
            17 => "Give ALL pests 8 poison.".into(),
            18 => "Grow ALL beds by 3. Lose 2 health.".into(),
            19 => "Deal 24 damage. Lose all shelter.".into(),
            20 => "Gain 24 shelter. Draw a card.".into(),
            21 => "Deal 9 damage to ALL pests. Draw 1.".into(),
            22 => "Grow ALL beds by 3. Heal 10.".into(),
            23 => "Gain 6 shelter per occupied bed.".into(),
            24 => "Harvest ALL mature beds with +9 damage.".into(),
            25 => "Draw 3 cards. Exhaust.".into(),
            _ => s.text.into(),
        }
    }
    pub fn new(id: usize) -> Self {
        Self { id, gold: false }
    }
}
#[derive(Clone, Copy)]
pub struct Spec {
    pub name: &'static str,
    pub icon: &'static str,
    pub cost: i32,
    pub text: &'static str,
    pub grow: i32,
    pub hit: i32,
    pub guard: i32,
    pub heal: i32,
}
macro_rules! card {
    ($n:expr,$i:expr,$c:expr,$t:expr,$g:expr,$h:expr,$b:expr,$r:expr) => {
        Spec {
            name: $n,
            icon: $i,
            cost: $c,
            text: $t,
            grow: $g,
            hit: $h,
            guard: $b,
            heal: $r,
        }
    };
}
pub const CARDS: [Spec; 28] = [
    card!(
        "Sweet Pea",
        "seed",
        1,
        "Grows in 2. Harvest: 10 damage.",
        2,
        10,
        0,
        0
    ),
    card!(
        "Briar Rose",
        "rose",
        1,
        "Grows in 3. Harvest: 17 damage.",
        3,
        17,
        0,
        0
    ),
    card!(
        "Sunflower",
        "sunflower",
        1,
        "Grows in 3. Harvest: 9 damage, +1 action.",
        3,
        9,
        0,
        0
    ),
    card!(
        "Clover",
        "clover",
        1,
        "Grows in 2. Harvest: 8 shelter, heal 3.",
        2,
        0,
        8,
        3
    ),
    card!(
        "Inkcap",
        "mushroom",
        1,
        "Grows in 2. Harvest: 8 damage, 4 poison.",
        2,
        8,
        0,
        0
    ),
    card!(
        "Desert Spire",
        "cactus",
        2,
        "Grows in 3. Harvest: 23 damage, 7 shelter.",
        3,
        23,
        7,
        0
    ),
    card!(
        "Moon Lotus",
        "lotus",
        2,
        "Grows in 4. Harvest: 30 damage, draw 2.",
        4,
        30,
        0,
        0
    ),
    card!(
        "Cherry Bough",
        "cherry",
        1,
        "Grows in 3. Harvest: 12 damage, grow neighbours.",
        3,
        12,
        0,
        0
    ),
    card!(
        "Silver Fern",
        "herb",
        1,
        "Grows in 2. Harvest: 6 shelter, draw 2.",
        2,
        0,
        6,
        0
    ),
    card!(
        "Old Oak",
        "tree",
        2,
        "Grows in 4. Harvest: 24 damage, 18 shelter.",
        4,
        24,
        18,
        0
    ),
    card!(
        "Water",
        "water",
        1,
        "Grow a bed by 2. Draw a card.",
        0,
        0,
        0,
        0
    ),
    card!(
        "Glass Ward",
        "jar",
        1,
        "Gain 9 shelter. Lasts this enemy turn.",
        0,
        0,
        9,
        0
    ),
    card!(
        "Secateurs",
        "shears",
        1,
        "Deal 8 damage to the selected pest.",
        0,
        8,
        0,
        0
    ),
    card!(
        "Rainshower",
        "water",
        1,
        "Grow ALL beds by 1. Gain 4 shelter.",
        0,
        0,
        4,
        0
    ),
    card!(
        "Sunbreak",
        "sun",
        0,
        "Gain 2 actions. Draw a card. Exhaust.",
        0,
        0,
        0,
        0
    ),
    card!(
        "Mulch",
        "leaf",
        1,
        "Grow a bed by 3. Gain 5 shelter.",
        0,
        0,
        5,
        0
    ),
    card!(
        "Wild Honey",
        "bee",
        1,
        "This turn: harvests deal +8 damage.",
        0,
        0,
        0,
        0
    ),
    card!(
        "Spore Cloud",
        "mushroom",
        1,
        "Give ALL pests 5 poison.",
        0,
        0,
        0,
        0
    ),
    card!(
        "Grafting",
        "shears",
        1,
        "Grow ALL beds by 2. Lose 2 health.",
        0,
        0,
        0,
        0
    ),
    card!(
        "Glass Cannon",
        "spark",
        2,
        "Deal 21 damage. Lose all shelter.",
        0,
        21,
        0,
        0
    ),
    card!(
        "Winter Coat",
        "snow",
        2,
        "Gain 21 shelter. Draw a card.",
        0,
        0,
        21,
        0
    ),
    card!(
        "Ladybird",
        "ladybug",
        1,
        "Deal 6 damage to ALL pests. Draw 1.",
        0,
        6,
        0,
        0
    ),
    card!(
        "Second Spring",
        "cherry",
        2,
        "Grow ALL beds by 2. Heal 7.",
        0,
        0,
        0,
        7
    ),
    card!(
        "Deep Roots",
        "tree",
        1,
        "Gain 5 shelter per occupied bed.",
        0,
        0,
        0,
        0
    ),
    card!(
        "Bloomstorm",
        "butterfly",
        2,
        "Harvest ALL mature beds with +6 damage.",
        0,
        0,
        0,
        0
    ),
    card!("Pruning", "herb", 0, "Draw 2 cards. Exhaust.", 0, 0, 0, 0),
    card!(
        "Hot Compost",
        "fire",
        0,
        "Gain 1 action. Grow ALL beds by 1. Exhaust.",
        0,
        0,
        0,
        0
    ),
    card!(
        "Dewdrop",
        "water",
        0,
        "Grow a bed by 1. Heal 2. Exhaust.",
        0,
        0,
        0,
        2
    ),
];
#[derive(Clone, Debug)]
pub struct Plant {
    pub card: Card,
    pub growth: i32,
}
impl Plant {
    pub fn mature(&self) -> bool {
        self.growth >= CARDS[self.card.id].grow
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Intent {
    Bite(i32),
    Prune(i32),
    Shell(i32),
    Tax(i32),
    Frost(i32),
}
impl Intent {
    pub fn label(self) -> String {
        match self {
            Self::Bite(n) => format!("Bite  {n}"),
            Self::Prune(n) => format!("Prune {n} growth"),
            Self::Shell(n) => format!("Armour {n}"),
            Self::Tax(n) => format!("Bite {n} + tax"),
            Self::Frost(n) => format!("Frost: ALL -{n}"),
        }
    }
    pub fn damage(self) -> i32 {
        match self {
            Self::Bite(n) | Self::Tax(n) => n,
            _ => 0,
        }
    }
}
pub const FOES: [(&str, &str); 9] = [
    ("Glass Aphid", "beetle"),
    ("Copper Snail", "snail"),
    ("Root Cricket", "locust"),
    ("Silk Weaver", "spider"),
    ("Honey Thief", "bee"),
    ("Ash Moth", "butterfly"),
    ("Keeper of Rain", "snail"),
    ("Keeper of Ash", "spider"),
    ("The White Queen", "snow"),
];
#[derive(Clone, Debug)]
pub struct Enemy {
    pub kind: usize,
    pub hp: i32,
    pub max: i32,
    pub armour: i32,
    pub poison: i32,
    pub intent: Intent,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Screen {
    Title,
    Battle,
    Reward,
    Route,
    Prune,
    Won,
    Lost,
}
pub const SEASONS: [(&str, &str, &str); 4] = [
    (
        "The Rain Rooms",
        "SPRING",
        "Rain feeds every new planting +1 growth.",
    ),
    (
        "The Amber House",
        "SUMMER",
        "Every harvest deals +3 damage.",
    ),
    (
        "The Rust Orchard",
        "AUTUMN",
        "Composting grows every occupied bed +1.",
    ),
    (
        "The White Conservatory",
        "WINTER",
        "First harvest each turn grants 6 shelter.",
    ),
];
pub const RELICS: [(&str, &str, &str); 9] = [
    (
        "Brass Watering Can",
        "water",
        "New plantings start with +1 growth.",
    ),
    (
        "Bee Hotel",
        "bee",
        "Each mature neighbour adds +6 harvest damage.",
    ),
    ("Thick Glass", "jar", "Start each turn with 4 shelter."),
    ("Pocket Almanac", "herb", "Draw one extra card each turn."),
    (
        "Warm Soil",
        "fire",
        "First harvest each turn gives +1 action.",
    ),
    ("Copper Roots", "tree", "Every harvest heals 2."),
    ("Rain Bell", "water", "Water grows an additional +1."),
    ("Prism Lens", "spark", "Attacks ignore enemy armour."),
    ("Seed Library", "seed", "Start each turn with +1 action."),
];
#[derive(Clone, Debug)]
pub struct Rng(pub u64);
impl Rng {
    pub fn next_u64(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    pub fn range(&mut self, n: usize) -> usize {
        (self.next_u64() as usize) % n
    }
    pub fn shuffle<T>(&mut self, s: &mut [T]) {
        for i in (1..s.len()).rev() {
            let j = self.range(i + 1);
            s.swap(i, j);
        }
    }
}
#[derive(Clone, Debug)]
pub struct Battle {
    pub hand: Vec<Card>,
    pub draw: Vec<Card>,
    pub discard: Vec<Card>,
    pub exhausted: usize,
    pub beds: [Option<Plant>; 3],
    pub enemies: Vec<Enemy>,
    pub energy: i32,
    pub shield: i32,
    pub turn: usize,
    pub composted: bool,
    pub honey: i32,
    pub harvests: usize,
    pub selected: Option<usize>,
    pub target: usize,
    pub damage_dealt: i32,
}
#[derive(Clone, Debug)]
pub struct Snapshot {
    battle: Battle,
    hp: i32,
    score: i32,
    rng: Rng,
    total_harvests: usize,
    peak_chain: usize,
}
#[derive(Clone, Debug)]
pub struct Game {
    pub screen: Screen,
    pub seed: u64,
    pub rng: Rng,
    pub stage: usize,
    pub hp: i32,
    pub max_hp: i32,
    pub score: i32,
    pub deck: Vec<Card>,
    pub relics: Vec<usize>,
    pub battle: Battle,
    pub rewards: [Card; 3],
    pub message: String,
    pub event: u64,
    pub impact: i32,
    pub history: Vec<Snapshot>,
    pub risky: bool,
    pub easy: bool,
    pub tutorial: usize,
    pub seen_cards: u64,
    pub seen_foes: u32,
    pub turns: usize,
    pub late_content: bool,
    pub total_harvests: usize,
    pub peak_chain: usize,
}
impl Default for Battle {
    fn default() -> Self {
        Self {
            hand: vec![],
            draw: vec![],
            discard: vec![],
            exhausted: 0,
            beds: [None, None, None],
            enemies: vec![],
            energy: 3,
            shield: 0,
            turn: 1,
            composted: false,
            honey: 0,
            harvests: 0,
            selected: None,
            target: 0,
            damage_dealt: 0,
        }
    }
}
impl Game {
    pub fn new(seed: u64, easy: bool, alternate: bool) -> Self {
        let ids = if alternate {
            vec![0, 2, 3, 4, 8, 10, 10, 13, 11, 11, 12, 12]
        } else {
            vec![0, 0, 0, 1, 3, 10, 10, 10, 11, 11, 12, 12]
        };
        let deck = ids.into_iter().map(Card::new).collect();
        Self {
            screen: Screen::Title,
            seed: seed.max(1),
            rng: Rng(seed.max(1)),
            stage: 0,
            hp: 100,
            max_hp: 100,
            score: 0,
            deck,
            relics: vec![],
            battle: Battle::default(),
            rewards: [Card::new(0); 3],
            message: String::new(),
            event: 0,
            impact: 0,
            history: vec![],
            risky: false,
            easy,
            tutorial: 0,
            seen_cards: 0,
            seen_foes: 0,
            turns: 0,
            late_content: false,
            total_harvests: 0,
            peak_chain: 0,
        }
    }
    pub fn season(&self) -> usize {
        (self.stage / 5).min(3)
    }
    pub fn has(&self, r: usize) -> bool {
        self.relics.contains(&r)
    }
    pub fn start(&mut self) {
        self.begin_battle();
    }
    pub fn notify(&mut self, s: impl Into<String>, impact: i32) {
        self.message = s.into();
        self.event += 1;
        self.impact = impact;
    }
    fn draw_cards(&mut self, n: usize) {
        for _ in 0..n {
            if self.battle.hand.len() >= 9 {
                break;
            }
            if self.battle.draw.is_empty() {
                self.battle.draw = std::mem::take(&mut self.battle.discard);
                self.rng.shuffle(&mut self.battle.draw);
            }
            if let Some(c) = self.battle.draw.pop() {
                self.seen_cards |= 1 << c.id;
                self.battle.hand.push(c);
            }
        }
    }
    pub fn begin_battle(&mut self) {
        self.screen = Screen::Battle;
        self.history.clear();
        self.battle = Battle::default();
        self.battle.draw = self.deck.clone();
        self.rng.shuffle(&mut self.battle.draw);
        let boss = matches!(self.stage, 5 | 11 | 17);
        let count = if boss {
            if self.stage == 17 { 3 } else { 2 }
        } else if self.stage < 3 {
            1
        } else if self.stage < 10 {
            2
        } else {
            3
        };
        for k in 0..count {
            let kind = if boss && k == 0 {
                6 + self.stage / 6
            } else {
                let pool = if self.stage < 3 {
                    2
                } else if self.stage < 7 {
                    3
                } else if self.stage < 12 {
                    5
                } else {
                    6
                };
                self.rng.range(pool)
            };
            let hp = if kind >= 6 {
                42 + self.stage as i32 * 4
            } else {
                14 + self.stage as i32 * 2 + if kind == 1 { 5 } else { 0 }
            } + if self.risky { 5 } else { 0 };
            self.battle.enemies.push(Enemy {
                kind,
                hp,
                max: hp,
                armour: 0,
                poison: 0,
                intent: Intent::Bite(3),
            });
            self.seen_foes |= 1 << kind;
        }
        if self.stage >= 15 {
            self.late_content = true;
        }
        self.battle.energy = 3 + i32::from(self.has(8));
        self.battle.shield = if self.has(2) { 4 } else { 0 };
        self.draw_cards(6 + usize::from(self.has(3)));
        self.plan_intents();
        self.notify(
            if self.stage == 0 {
                "Plant a seed in an empty bed. Rain gives it a head start.".into()
            } else {
                format!(
                    "Encounter {} / 18 — {}",
                    self.stage + 1,
                    SEASONS[self.season()].0
                )
            },
            0,
        );
    }
    pub fn checkpoint(&mut self) {
        self.history.push(Snapshot {
            battle: self.battle.clone(),
            hp: self.hp,
            score: self.score,
            rng: self.rng.clone(),
            total_harvests: self.total_harvests,
            peak_chain: self.peak_chain,
        });
        if self.history.len() > 32 {
            self.history.remove(0);
        }
    }
    pub fn undo(&mut self) {
        if self.screen != Screen::Battle {
            return;
        }
        if let Some(s) = self.history.pop() {
            self.battle = s.battle;
            self.hp = s.hp;
            self.score = s.score;
            self.rng = s.rng;
            self.total_harvests = s.total_harvests;
            self.peak_chain = s.peak_chain;
            self.notify(
                "Decision undone. Committed enemy turns cannot be undone.",
                0,
            );
        }
    }
    pub fn select(&mut self, i: usize) {
        if self.screen != Screen::Battle || i >= self.battle.hand.len() {
            return;
        }
        let c = self.battle.hand[i];
        if self.battle.energy < CARDS[c.id].cost {
            self.notify("Not enough actions. Compost a card or end the turn.", 0);
            return;
        }
        self.battle.selected = Some(i);
        if !Self::targeted(c.id) {
            self.play(i, 0);
        }
    }
    pub fn targeted(id: usize) -> bool {
        id < 10 || matches!(id, 10 | 15 | 27)
    }
    pub fn bed(&mut self, i: usize) {
        if i >= 3 || self.screen != Screen::Battle {
            return;
        }
        if let Some(c) = self.battle.selected {
            self.play(c, i);
        } else {
            self.harvest(i);
        }
    }
    pub fn compost(&mut self, i: usize) {
        if self.screen != Screen::Battle || i >= self.battle.hand.len() {
            return;
        }
        if self.battle.composted {
            self.notify("Compost once per turn. Your next turn refreshes it.", 0);
            return;
        }
        self.checkpoint();
        self.battle.discard.push(self.battle.hand.remove(i));
        self.battle.selected = None;
        self.battle.energy += 1;
        self.battle.composted = true;
        if self.season() == 2 {
            self.grow_all(1);
        }
        self.notify("Composted: +1 action. The card returns next shuffle.", 1);
    }
    fn grow_all(&mut self, n: i32) {
        for p in self.battle.beds.iter_mut().flatten() {
            p.growth = (p.growth + n).min(CARDS[p.card.id].grow);
        }
    }
    fn hurt(&mut self, target: usize, n: i32) {
        let ignore = self.has(7);
        if let Some(e) = self.battle.enemies.get_mut(target) {
            let damage = if ignore { n } else { (n - e.armour).max(0) };
            e.hp -= damage;
            self.battle.damage_dealt += damage;
        }
    }
    fn hurt_all(&mut self, n: i32) {
        for i in 0..self.battle.enemies.len() {
            self.hurt(i, n);
        }
    }
    pub fn play(&mut self, i: usize, bed: usize) -> bool {
        if self.screen != Screen::Battle || i >= self.battle.hand.len() || bed >= 3 {
            return false;
        }
        let c = self.battle.hand[i];
        let spec = CARDS[c.id];
        if self.battle.energy < spec.cost {
            self.notify("Not enough actions. Right-click a card to compost it.", 0);
            return false;
        }
        if c.id < 10 && self.battle.beds[bed].is_some() {
            self.notify(
                "That bed is occupied. Harvest it when its growth ring is full.",
                0,
            );
            return false;
        }
        if matches!(c.id, 10 | 15 | 27) && self.battle.beds[bed].is_none() {
            self.notify("Water needs a planted bed. Choose a plant first.", 0);
            return false;
        }
        self.checkpoint();
        self.battle.energy -= spec.cost;
        self.battle.hand.remove(i);
        self.battle.selected = None;
        let bonus = if c.gold { 3 } else { 0 };
        if matches!(c.id, 14 | 25 | 26 | 27) {
            self.battle.exhausted += 1;
        } else {
            self.battle.discard.push(c);
        }
        match c.id {
            0..=9 => {
                let growth =
                    i32::from(self.season() == 0) + i32::from(self.has(0)) + i32::from(c.gold);
                self.battle.beds[bed] = Some(Plant { card: c, growth });
                self.tutorial = self.tutorial.max(1);
            }
            10 | 15 | 27 => {
                let n = match c.id {
                    10 => 2 + i32::from(self.has(6)),
                    15 => 3,
                    _ => 1,
                } + i32::from(c.gold);
                if let Some(p) = &mut self.battle.beds[bed] {
                    p.growth = (p.growth + n).min(CARDS[p.card.id].grow);
                }
                if c.id == 10 {
                    self.draw_cards(1);
                }
                self.battle.shield += spec.guard + bonus;
                self.hp = (self.hp + spec.heal).min(self.max_hp);
                self.tutorial = self.tutorial.max(2);
            }
            11 | 20 => {
                self.battle.shield += spec.guard + bonus;
                if c.id == 20 {
                    self.draw_cards(1);
                }
            }
            12 | 19 => {
                if c.id == 19 {
                    self.battle.shield = 0;
                }
                self.hurt(self.battle.target, spec.hit + bonus);
            }
            13 => {
                self.grow_all(1 + i32::from(c.gold));
                self.battle.shield += 4 + bonus;
            }
            14 => {
                self.battle.energy += 2;
                self.draw_cards(1);
            }
            16 => self.battle.honey += 8 + bonus,
            17 => {
                for e in &mut self.battle.enemies {
                    e.poison += 5 + bonus;
                }
            }
            18 => {
                self.grow_all(2 + i32::from(c.gold));
                self.hp -= 2;
            }
            21 => {
                self.hurt_all(6 + bonus);
                self.draw_cards(1);
            }
            22 => {
                self.grow_all(2 + i32::from(c.gold));
                self.hp = (self.hp + 7 + bonus).min(self.max_hp);
            }
            23 => {
                self.battle.shield +=
                    (5 + i32::from(c.gold)) * self.battle.beds.iter().flatten().count() as i32
            }
            24 => {
                self.battle.honey += 6 + bonus;
                for b in 0..3 {
                    self.harvest_inner(b);
                }
                self.battle.honey -= 6 + bonus;
            }
            25 => self.draw_cards(2 + usize::from(c.gold)),
            26 => {
                self.battle.energy += 1;
                self.grow_all(1);
            }
            _ => {}
        }
        self.notify(
            format!("{}{}", spec.name, if c.gold { " +" } else { "" }),
            if spec.hit > 0 { 2 } else { 1 },
        );
        self.check_outcome();
        true
    }
    fn harvest_inner(&mut self, i: usize) -> bool {
        if !self
            .battle
            .beds
            .get(i)
            .and_then(Option::as_ref)
            .is_some_and(Plant::mature)
        {
            return false;
        }
        let neighbours = (0_usize..3)
            .filter(|&j| {
                j.abs_diff(i) == 1 && self.battle.beds[j].as_ref().is_some_and(Plant::mature)
            })
            .count();
        if self.battle.enemies[self.battle.target].hp <= 0
            && let Some(target) = self.battle.enemies.iter().position(|e| e.hp > 0)
        {
            self.battle.target = target;
        }
        let p = self.battle.beds[i].take().unwrap();
        let spec = CARDS[p.card.id];
        let bonus = if p.card.gold { 5 } else { 0 };
        let hit = spec.hit
            + if spec.hit > 0 {
                neighbours as i32 * if self.has(1) { 6 } else { 3 }
                    + self.battle.honey
                    + if self.season() == 1 { 3 } else { 0 }
                    + bonus
            } else {
                0
            };
        self.hurt(self.battle.target, hit);
        self.battle.shield += spec.guard + if spec.guard > 0 { bonus } else { 0 };
        self.hp = (self.hp + spec.heal + if self.has(5) { 2 } else { 0 }).min(self.max_hp);
        match p.card.id {
            2 => self.battle.energy += 1,
            4 => {
                if let Some(e) = self.battle.enemies.get_mut(self.battle.target) {
                    e.poison += 4;
                }
            }
            6 | 8 => self.draw_cards(2),
            7 => self.grow_all(1),
            _ => {}
        }
        if self.battle.harvests == 0 {
            if self.has(4) {
                self.battle.energy += 1;
            }
            if self.season() == 3 {
                self.battle.shield += 6;
            }
        }
        self.battle.harvests += 1;
        self.total_harvests += 1;
        self.peak_chain = self.peak_chain.max(self.battle.harvests);
        self.score += 10 * (1 + neighbours as i32) * self.battle.harvests as i32;
        self.tutorial = self.tutorial.max(3);
        true
    }
    pub fn harvest(&mut self, i: usize) {
        if self.screen != Screen::Battle {
            return;
        }
        if !self
            .battle
            .beds
            .get(i)
            .and_then(Option::as_ref)
            .is_some_and(Plant::mature)
        {
            self.notify(
                "Not ripe yet. Water adds growth; ending a turn grows every bed.",
                0,
            );
            return;
        }
        self.checkpoint();
        let before = self.battle.damage_dealt;
        self.harvest_inner(i);
        let n = self.battle.damage_dealt - before;
        self.notify(
            format!("Harvest!  {n} damage  /  chain {}", self.battle.harvests),
            3,
        );
        self.check_outcome();
    }
    fn plan_intents(&mut self) {
        let mature = self.battle.beds.iter().flatten().any(Plant::mature);
        for e in &mut self.battle.enemies {
            let n = 3
                + self.stage as i32 / 3
                + if e.kind >= 6 { 3 } else { 0 }
                + i32::from(self.risky)
                + if e.kind == 8 && e.hp * 2 <= e.max {
                    4
                } else {
                    0
                };
            e.intent = if e.kind == 8 && self.battle.turn.is_multiple_of(3) {
                Intent::Frost(if e.hp * 2 <= e.max { 3 } else { 2 })
            } else if e.kind == 6 && self.battle.turn.is_multiple_of(3) {
                Intent::Shell(7)
            } else if e.kind == 6 && self.battle.turn % 3 == 1 && self.battle.turn > 1 {
                Intent::Bite(n + 5)
            } else if e.kind == 7 && self.battle.turn.is_multiple_of(2) {
                Intent::Tax(n)
            } else if e.kind == 7 && mature {
                Intent::Prune(2)
            } else if e.kind == 1 && self.battle.turn.is_multiple_of(3) {
                Intent::Shell(4 + self.stage as i32 / 4)
            } else if (e.kind == 2 || e.kind == 3) && mature && self.battle.turn.is_multiple_of(2) {
                Intent::Prune(1)
            } else if e.kind == 4 && self.battle.turn.is_multiple_of(2) {
                Intent::Tax(n)
            } else if e.kind == 8 && self.battle.turn.is_multiple_of(3) {
                Intent::Prune(2)
            } else {
                Intent::Bite(n)
            };
        }
    }
    pub fn end_turn(&mut self) {
        if self.screen != Screen::Battle {
            return;
        }
        self.history.clear();
        self.battle.selected = None;
        let mut damage = 0;
        let mut tax = false;
        let intents: Vec<(usize, Intent)> = self
            .battle
            .enemies
            .iter()
            .enumerate()
            .filter(|(_, e)| e.hp > 0)
            .map(|(i, e)| (i, e.intent))
            .collect();
        for e in &mut self.battle.enemies {
            e.hp -= e.poison;
            if e.poison > 0 {
                e.poison -= 1;
            }
        }
        if self.check_outcome() {
            return;
        }
        for (i, intent) in intents {
            if self.battle.enemies.get(i).is_some_and(|e| e.hp <= 0) {
                continue;
            }
            match intent {
                Intent::Bite(n) => damage += n,
                Intent::Tax(n) => {
                    damage += n;
                    tax = true;
                }
                Intent::Shell(n) => {
                    if let Some(e) = self.battle.enemies.get_mut(i) {
                        e.armour = n;
                    }
                }
                Intent::Frost(n) => {
                    for p in self.battle.beds.iter_mut().flatten() {
                        p.growth = (p.growth - n).max(0);
                    }
                }
                Intent::Prune(n) => {
                    if let Some(p) = self
                        .battle
                        .beds
                        .iter_mut()
                        .flatten()
                        .max_by_key(|p| CARDS[p.card.id].hit + p.growth * 2)
                    {
                        p.growth = (p.growth - n).max(0);
                    }
                }
            }
        }
        if self.easy {
            damage = (damage * 2 + 2) / 3;
        }
        let wound = (damage - self.battle.shield).max(0);
        self.hp -= wound;
        self.battle.turn += 1;
        self.turns += 1;
        if self.battle.turn > 18 {
            self.hp -= ((self.battle.turn - 18) * 3) as i32;
        }
        self.battle.discard.append(&mut self.battle.hand);
        self.grow_all(1);
        self.battle.energy = 3 + i32::from(self.has(8)) - i32::from(tax);
        self.battle.shield = if self.has(2) { 4 } else { 0 };
        self.battle.honey = 0;
        self.battle.composted = false;
        self.battle.harvests = 0;
        self.draw_cards(6 + usize::from(self.has(3)));
        self.plan_intents();
        self.tutorial = self.tutorial.max(4);
        self.notify(
            if wound > 0 {
                format!("{wound} health lost. Beds grew. New hand, fresh actions.")
            } else {
                "All damage sheltered. Beds grew. New hand, fresh actions.".into()
            },
            if wound > 0 { 4 } else { 1 },
        );
        self.check_outcome();
    }
    fn check_outcome(&mut self) -> bool {
        if self
            .battle
            .enemies
            .iter()
            .any(|e| e.kind == 8 && e.hp * 2 <= e.max)
        {
            self.plan_intents();
        }
        if self.hp <= 0 {
            self.hp = 0;
            self.screen = Screen::Lost;
            self.notify("The glasshouse fell. Your health reached zero.", 4);
            return true;
        }
        if self.battle.enemies.iter().all(|e| e.hp <= 0) {
            self.score += 100 + self.stage as i32 * 15 + self.hp;
            self.history.clear();
            if self.stage == 17 {
                self.screen = Screen::Won;
                self.notify("Spring will come again. You saved the last glasshouse.", 5);
            } else {
                self.screen = Screen::Reward;
                self.hp = (self.hp + 4).min(self.max_hp);
                let max_id = if self.stage < 4 {
                    18
                } else if self.stage < 9 {
                    24
                } else {
                    28
                };
                let mut ids = Vec::new();
                while ids.len() < 3 {
                    let n = self.rng.range(max_id);
                    if !ids.contains(&n) {
                        ids.push(n)
                    }
                }
                for (i, id) in ids.into_iter().enumerate() {
                    self.rewards[i] = Card {
                        id,
                        gold: self.stage >= 8 && CARDS[id].cost > 0 && self.rng.range(3) == 0,
                    };
                    self.seen_cards |= 1 << id;
                }
                if matches!(self.stage, 5 | 11) {
                    self.grant_relic();
                }
                self.notify(
                    "The room is safe. Heal 4; choose a cutting for your deck.",
                    5,
                );
            }
            true
        } else {
            if self.battle.enemies[self.battle.target].hp <= 0 {
                self.battle.target = self.battle.enemies.iter().position(|e| e.hp > 0).unwrap();
            }
            false
        }
    }
    fn grant_relic(&mut self) {
        let remaining: Vec<_> = (0..RELICS.len()).filter(|r| !self.has(*r)).collect();
        if !remaining.is_empty() {
            let r = remaining[self.rng.range(remaining.len())];
            self.relics.push(r);
        }
    }
    pub fn reward(&mut self, i: usize) {
        if self.screen != Screen::Reward {
            return;
        }
        if i < 3 {
            self.deck.push(self.rewards[i]);
        }
        self.screen = Screen::Route;
        self.notify("Choose the next doorway. Your deck and health carry on.", 0);
    }
    pub fn route(&mut self, i: usize) {
        if self.screen != Screen::Route {
            return;
        }
        self.risky = i == 1;
        if i == 0 {
            self.hp = (self.hp + 14).min(self.max_hp);
        }
        if i == 1 {
            self.grant_relic();
        }
        if i == 2 {
            self.screen = Screen::Prune;
            return;
        }
        self.stage += 1;
        self.begin_battle();
    }
    pub fn prune(&mut self, i: usize) {
        if self.screen == Screen::Prune && i < self.deck.len() && self.deck.len() > 8 {
            self.deck.remove(i);
            self.stage += 1;
            self.begin_battle();
        }
    }
    pub fn upgrade(&mut self, i: usize) {
        if self.screen == Screen::Prune && i < self.deck.len() {
            if self.deck[i].gold || matches!(self.deck[i].id, 14 | 26 | 27) {
                self.notify("That cutting is already at its best. Choose another.", 0);
                return;
            }
            self.deck[i].gold = true;
            self.stage += 1;
            self.begin_battle();
        }
    }
    pub fn pressure(&self) -> i32 {
        self.battle
            .enemies
            .iter()
            .map(|e| e.max + e.intent.damage() * 3)
            .sum()
    }
    pub fn expected_damage(&self) -> i32 {
        let n: i32 = self
            .battle
            .enemies
            .iter()
            .filter(|e| e.hp > 0)
            .map(|e| e.intent.damage())
            .sum();
        let n = if self.easy { (n * 2 + 2) / 3 } else { n };
        (n - self.battle.shield).max(0)
    }
}

/// Plan from visible state. Both headless rules tests and UI click replay use these same actions.
pub fn bot_action(g: &Game) -> Action {
    match g.screen {
        Screen::Title => Action::Start,
        Screen::Reward => Action::Reward(
            (0..3)
                .max_by_key(|&i| {
                    let c = g.rewards[i];
                    let plants = g.deck.iter().filter(|c| c.id < 10).count();
                    let score = match c.id {
                        3 | 22 => {
                            if g.hp < 60 {
                                40
                            } else {
                                20
                            }
                        }
                        13 | 14 | 17 | 20 | 21 | 26 => 30,
                        0..=9 => {
                            if plants < 7 {
                                26
                            } else {
                                8
                            }
                        }
                        _ => 15,
                    };
                    score + i32::from(c.gold) * 6
                })
                .unwrap(),
        ),
        Screen::Route => Action::Route(if g.hp < 78 {
            0
        } else if g.relics.len() < 5 {
            1
        } else {
            0
        }),
        Screen::Prune => Action::Prune(0),
        Screen::Battle => {
            let empty = (0..3).find(|&i| g.battle.beds[i].is_none());
            let growing = (0..3)
                .filter(|&i| g.battle.beds[i].is_some())
                .min_by_key(|&i| {
                    let p = g.battle.beds[i].as_ref().unwrap();
                    CARDS[p.card.id].grow - p.growth
                });
            if let Some(i) = g.battle.selected {
                return Action::Bed(if g.battle.hand[i].id < 10 {
                    empty.unwrap()
                } else {
                    growing.unwrap()
                });
            }
            let target = g
                .battle
                .enemies
                .iter()
                .enumerate()
                .filter(|(_, e)| e.hp > 0)
                .min_by_key(|(_, e)| e.hp)
                .map(|(i, _)| i)
                .unwrap();
            if target != g.battle.target {
                return Action::Target(target);
            }
            if let Some(i) = (0..3)
                .filter(|&i| g.battle.beds[i].as_ref().is_some_and(Plant::mature))
                .max_by_key(|&i| {
                    let p = g.battle.beds[i].as_ref().unwrap();
                    CARDS[p.card.id].hit + CARDS[p.card.id].guard
                })
            {
                return Action::Bed(i);
            }
            let best = g
                .battle
                .hand
                .iter()
                .enumerate()
                .filter_map(|(i, c)| {
                    let spec = CARDS[c.id];
                    if spec.cost > g.battle.energy {
                        return None;
                    }
                    let score = match c.id {
                        0..=9 => {
                            if empty.is_some() {
                                24 + spec.hit / 4 + spec.guard / 3
                            } else {
                                return None;
                            }
                        }
                        10 | 15 | 27 => {
                            if growing.is_some() {
                                33
                            } else {
                                return None;
                            }
                        }
                        11 | 20 | 23 => {
                            if g.expected_damage() > 0 {
                                20 + g.expected_damage().min(spec.guard.max(10))
                            } else {
                                1
                            }
                        }
                        12 | 19 => 23,
                        13 | 18 | 22 | 26 => {
                            if growing.is_some() {
                                32
                            } else if c.id == 22 && g.hp < 80 {
                                23
                            } else if c.id == 26 {
                                9
                            } else {
                                1
                            }
                        }
                        14 | 25 => 40,
                        16 | 24 => 1,
                        17 => 29,
                        21 => 31,
                        _ => 1,
                    };
                    Some((score, i))
                })
                .max();
            if let Some((score, i)) = best
                && score > 3
            {
                return Action::Card(i);
            }
            if !g.battle.composted && !g.battle.hand.is_empty() {
                return Action::Compost(
                    g.battle
                        .hand
                        .iter()
                        .position(|c| c.id < 10 && empty.is_none())
                        .unwrap_or(g.battle.hand.len() - 1),
                );
            }
            Action::EndTurn
        }
        _ => Action::None,
    }
}
pub fn bot_step(g: &mut Game) {
    match bot_action(g) {
        Action::Start => g.start(),
        Action::Reward(i) => g.reward(i),
        Action::Route(i) => g.route(i),
        Action::Prune(i) => g.prune(i),
        Action::Card(i) => g.select(i),
        Action::Bed(i) => g.bed(i),
        Action::Target(i) => g.battle.target = i,
        Action::Compost(i) => g.compost(i),
        Action::EndTurn => g.end_turn(),
        _ => {}
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Save {
    pub best: i32,
    pub wins: usize,
    pub runs: usize,
    pub seen_cards: u64,
    pub seen_foes: u32,
}
impl Save {
    pub fn path() -> PathBuf {
        std::env::var_os("GLASSHOUSE_SAVE")
            .map(PathBuf::from)
            .unwrap_or_else(|| {
                let base = std::env::var_os("HOME")
                    .map(PathBuf::from)
                    .unwrap_or_else(std::env::temp_dir);
                base.join(".last-glasshouse/save.txt")
            })
    }
    pub fn load(path: &Path) -> Self {
        let s = fs::read_to_string(path).unwrap_or_default();
        let v: Vec<_> = s
            .split_whitespace()
            .filter_map(|s| s.parse::<u64>().ok())
            .collect();
        if v.len() != 6 || v[0] != 1 {
            return Self::default();
        }
        Self {
            best: v[1].min(i32::MAX as u64) as i32,
            wins: v[2] as usize,
            runs: v[3] as usize,
            seen_cards: v[4],
            seen_foes: v[5] as u32,
        }
    }
    pub fn write(&self, path: &Path) -> std::io::Result<()> {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)?;
        }
        let tmp = path.with_extension("tmp");
        fs::write(
            &tmp,
            format!(
                "1 {} {} {} {} {}\n",
                self.best, self.wins, self.runs, self.seen_cards, self.seen_foes
            ),
        )?;
        fs::rename(tmp, path)
    }
    pub fn record(&mut self, g: &Game) {
        self.best = self.best.max(g.score);
        self.runs += 1;
        self.wins += usize::from(g.screen == Screen::Won);
        self.seen_cards |= g.seen_cards;
        self.seen_foes |= g.seen_foes;
    }
}

pub fn selftest() -> Result<String, String> {
    let mut wins = 0;
    let mut losses = 0;
    let mut max_turns = 0;
    let mut late = 0;
    let mut actions = 0;
    let mut simulated_seconds = 0;
    let mut late_runs = 0;
    let mut min_late_seconds = usize::MAX;
    for seed in 1..=48 {
        let mut g = Game::new(seed, false, seed % 2 == 0);
        g.start();
        let initial = g.pressure();
        let mut run_actions = 0;
        let mut last_stage = 0;
        let mut stagnation = 0;
        while !matches!(g.screen, Screen::Won | Screen::Lost) {
            let old_event = g.event;
            bot_step(&mut g);
            actions += 1;
            run_actions += 1;
            simulated_seconds += 8;
            if g.event == old_event && g.stage == last_stage {
                stagnation += 1
            } else {
                stagnation = 0
            }
            if stagnation > 20 || actions > 300_000 {
                return Err("Bot stalled".into());
            }
            last_stage = g.stage;
            max_turns = max_turns.max(g.battle.turn);
            if g.stage >= 15 && g.screen == Screen::Battle {
                if run_actions * 4 + g.stage * 30 < 900 {
                    return Err("Late content too early in paced playtest".into());
                }
                if g.pressure() <= initial {
                    return Err("Difficulty did not rise".into());
                }
                if g.late_content {
                    late += 1;
                }
            }
            if g.battle.turn > 45 {
                return Err("Unbounded encounter".into());
            }
        }
        if g.late_content {
            let seconds = run_actions * 4 + g.stage * 30;
            if seconds < 900 {
                return Err("Late seasonal content arrived before 15 simulated minutes".into());
            }
            min_late_seconds = min_late_seconds.min(seconds);
            late_runs += 1;
        }
        if g.screen == Screen::Won {
            wins += 1
        } else {
            losses += 1
        }
        if g.hp < 0 || g.battle.energy < 0 {
            return Err("Invalid resource".into());
        }
        let mut restart = Game::new(seed, false, false);
        restart.start();
        if restart.hp != 100 || restart.stage != 0 || restart.battle.turn != 1 {
            return Err("Restart failed".into());
        }
    }
    let mut idle = Game::new(99, false, false);
    idle.start();
    for _ in 0..60 {
        idle.end_turn();
    }
    if idle.screen != Screen::Lost {
        return Err("Loss unreachable".into());
    }
    if wins < 20 || late == 0 || late_runs < 20 || simulated_seconds < 900 {
        return Err(format!("Balance/content gate: wins={wins}, late={late}"));
    }
    Ok(format!(
        "SELFTEST OK: {wins}/48 wins, {losses} bot losses; idle loss reachable; {actions} legal decisions; {simulated_seconds}s simulated decision time; late content={late} actions across {late_runs} runs (minimum full-run pacing {min_late_seconds}s at 4s/decision + 30s/door); max encounter turns={max_turns}; ramp, restart, no stalls verified"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn planting_growth_and_pollination() {
        let mut g = Game::new(1, false, false);
        g.start();
        g.battle.hand = vec![Card::new(0), Card::new(0), Card::new(10)];
        assert!(g.play(0, 0));
        assert!(g.play(0, 1));
        assert!(g.play(0, 0));
        assert!(g.battle.beds[0].as_ref().unwrap().mature());
        g.battle.beds[1].as_mut().unwrap().growth = 2;
        let before = g.battle.enemies[0].hp;
        g.harvest(0);
        assert_eq!(before - g.battle.enemies[0].hp, 13);
    }
    #[test]
    fn illegal_actions_are_free() {
        let mut g = Game::new(2, false, false);
        g.start();
        g.battle.hand = vec![Card::new(10)];
        let energy = g.battle.energy;
        assert!(!g.play(0, 0));
        assert_eq!(g.battle.energy, energy);
        assert_eq!(g.battle.hand.len(), 1);
    }
    #[test]
    fn undo_stops_at_enemy_turn() {
        let mut g = Game::new(3, false, false);
        g.start();
        g.battle.hand = vec![Card::new(0)];
        g.play(0, 0);
        g.undo();
        assert!(g.battle.beds[0].is_none());
        assert_eq!(g.battle.hand.len(), 1);
        g.play(0, 0);
        g.end_turn();
        let hp = g.hp;
        g.undo();
        assert_eq!(g.hp, hp);
        assert!(g.history.is_empty());
    }
    #[test]
    fn compost_once_and_no_negative_energy() {
        let mut g = Game::new(4, false, false);
        g.start();
        g.compost(0);
        let n = g.battle.hand.len();
        g.compost(0);
        assert_eq!(g.battle.hand.len(), n);
        assert_eq!(g.battle.energy, 4);
    }
    #[test]
    fn save_roundtrip() {
        let p = std::env::temp_dir().join(format!("glasshouse-test-{}.txt", std::process::id()));
        let s = Save {
            best: 123,
            wins: 4,
            runs: 9,
            seen_cards: 1234,
            seen_foes: 255,
        };
        s.write(&p).unwrap();
        assert_eq!(Save::load(&p), s);
        fs::write(&p, "broken").unwrap();
        assert_eq!(Save::load(&p), Save::default());
        let _ = fs::remove_file(p);
    }
    #[test]
    fn every_card_executes() {
        for id in 0..28 {
            let mut g = Game::new(10, false, false);
            g.start();
            g.battle.enemies[0].hp = 500;
            g.battle.hand = vec![Card { id, gold: true }];
            g.battle.energy = 9;
            if id >= 10 {
                g.battle.beds[0] = Some(Plant {
                    card: Card::new(0),
                    growth: 1,
                });
            }
            assert!(g.play(0, 0), "card {id}");
        }
    }
    #[test]
    fn generated_campaigns() {
        let s = selftest().unwrap();
        assert!(s.contains("SELFTEST OK"));
    }
}

pub mod audio;
pub mod view;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Action {
    Start,
    Alternate,
    Daily,
    Card(usize),
    Compost(usize),
    Bed(usize),
    Target(usize),
    EndTurn,
    Undo,
    Reward(usize),
    Route(usize),
    Prune(usize),
    Upgrade(usize),
    Relic(usize),
    Restart,
    Pause,
    Help,
    Codex,
    Close,
    Mute,
    Fullscreen,
    Assist,
    Motion,
    Quit,
    None,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Overlay {
    None,
    Pause,
    Help,
    Codex,
}
pub struct App {
    pub game: Game,
    pub save: Save,
    pub overlay: Overlay,
    pub muted: bool,
    pub motion: bool,
    pub fullscreen_requested: bool,
    pub quit: bool,
    pub time: f32,
    pub event_time: f32,
    pub last_event: u64,
    pub recorded: bool,
    pub cursor: (f32, f32),
    pub hover: Action,
    pub sound_event: u64,
    pub save_error: Option<String>,
}
impl App {
    pub fn new(seed: u64) -> Self {
        Self {
            game: Game::new(seed, false, false),
            save: Save::load(&Save::path()),
            overlay: Overlay::None,
            muted: false,
            motion: true,
            fullscreen_requested: false,
            quit: false,
            time: 0.,
            event_time: 0.,
            last_event: 0,
            recorded: false,
            cursor: (-1., -1.),
            hover: Action::None,
            sound_event: 0,
            save_error: None,
        }
    }
    pub fn act(&mut self, a: Action) {
        match a {
            Action::Mute => self.muted = !self.muted,
            Action::Motion => self.motion = !self.motion,
            Action::Fullscreen => self.fullscreen_requested = true,
            Action::Quit => self.quit = true,
            Action::Pause => {
                self.overlay = if self.overlay == Overlay::Pause {
                    Overlay::None
                } else {
                    Overlay::Pause
                }
            }
            Action::Help => {
                self.overlay = if self.overlay == Overlay::Help {
                    Overlay::None
                } else {
                    Overlay::Help
                }
            }
            Action::Codex => {
                self.overlay = if self.overlay == Overlay::Codex {
                    Overlay::None
                } else {
                    Overlay::Codex
                }
            }
            Action::Close => {
                self.overlay = Overlay::None;
                self.game.battle.selected = None;
            }
            Action::Assist => {
                self.game.easy = !self.game.easy;
            }
            _ if self.overlay != Overlay::None => {}
            Action::Start | Action::Daily | Action::Alternate | Action::Restart => {
                let seed = if a == Action::Daily {
                    20261026
                } else {
                    self.game
                        .seed
                        .wrapping_mul(6364136223846793005)
                        .wrapping_add(1)
                };
                let easy = self.game.easy;
                self.game = Game::new(seed, easy, a == Action::Alternate && self.save.wins > 0);
                self.game.start();
                self.recorded = false;
            }
            Action::Card(i) => self.game.select(i),
            Action::Compost(i) => self.game.compost(i),
            Action::Bed(i) => self.game.bed(i),
            Action::Target(i) => {
                if self.game.screen == Screen::Battle
                    && self.game.battle.enemies.get(i).is_some_and(|e| e.hp > 0)
                {
                    self.game.battle.target = i;
                    self.game.notify(
                        format!("Target: {}", FOES[self.game.battle.enemies[i].kind].0),
                        0,
                    );
                }
            }
            Action::EndTurn => self.game.end_turn(),
            Action::Undo => self.game.undo(),
            Action::Reward(i) => self.game.reward(i),
            Action::Route(i) => self.game.route(i),
            Action::Prune(i) => self.game.prune(i),
            Action::Upgrade(i) => self.game.upgrade(i),
            Action::Relic(_) => {}
            Action::None => {}
        }
        self.refresh();
    }
    pub fn refresh(&mut self) {
        if self.game.event != self.last_event {
            self.last_event = self.game.event;
            self.event_time = self.time;
            if self.game.impact > 0 {
                self.sound_event += 1;
            }
        }
        if matches!(self.game.screen, Screen::Won | Screen::Lost) && !self.recorded {
            self.save.record(&self.game);
            self.save_error = self.save.write(&Save::path()).err().map(|e| e.to_string());
            self.recorded = true;
        }
    }
    pub fn tick(&mut self, dt: f32) {
        self.time += dt.min(0.05);
        self.refresh();
    }
}
