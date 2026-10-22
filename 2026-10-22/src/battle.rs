//! One island battle: turn structure, light, movement, abilities, damage.
//!
//! The simulation mutates state immediately and returns `Event`s describing
//! what happened, which the renderer turns into animation and sound.

use crate::ai;
use crate::map::{Biome, Map, N, Pickup, Pos, Terrain};
use crate::rng::Rng;
use crate::units::{Ability, Cause, Kind, Target, Team, Unit};

/// Modifiers granted by run upgrades.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Mods {
    pub kill_heal: i32,
    pub kindled_arrows: bool,
    pub bash_push: i32,
    pub pyre_bonus: i32,
    pub flare_bonus: i32,
    pub mend_bonus: i32,
    pub mend_haste: bool,
    pub volley_bonus: i32,
    pub extra_lit: i32,
}

/// Persistent per-member stats carried between battles.
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub kind: Kind,
    pub hp: i32,
    pub max_hp: i32,
    pub move_range: i32,
    pub power: i32,
    pub range_bonus: i32,
    pub lantern: i32,
}

impl Member {
    pub fn new(kind: Kind) -> Member {
        Member {
            kind,
            hp: kind.base_hp(),
            max_hp: kind.base_hp(),
            move_range: kind.base_move(),
            power: 0,
            range_bonus: 0,
            lantern: 2,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Config {
    pub seed: u64,
    pub island: u32,
    pub biome: Biome,
    pub elite: bool,
    pub boss: bool,
    pub mods: Mods,
    /// Debug/test only: multiplies squad damage.
    pub god_mode: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Phase {
    Player,
    Enemy,
    Over,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    Victory,
    Defeat,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Fx {
    Slash,
    Bash,
    Arrow,
    Volley,
    Flare,
    Bolt,
    Curse,
    Claw,
    Slam,
    Rend,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    Moved {
        id: u32,
        path: Vec<Pos>,
    },
    Attack {
        id: u32,
        from: Pos,
        to: Pos,
        fx: Fx,
    },
    Damaged {
        id: u32,
        amount: i32,
        hp: i32,
        shrouded: bool,
        boss: bool,
    },
    Healed {
        id: u32,
        amount: i32,
        hp: i32,
    },
    Pushed {
        id: u32,
        from: Pos,
        to: Pos,
    },
    Bumped {
        id: u32,
        at: Pos,
    },
    Fell {
        id: u32,
        at: Pos,
        water: bool,
    },
    Died {
        id: u32,
    },
    BrazierLit {
        at: Pos,
    },
    BrazierSnuffed {
        at: Pos,
    },
    Spawned {
        id: u32,
        at: Pos,
    },
    Eclipse {
        at: Pos,
    },
    Guard {
        id: u32,
    },
    Weakened {
        id: u32,
    },
    Picked {
        id: u32,
        at: Pos,
        kind: Pickup,
    },
    Chain {
        count: u32,
    },
    RoundStart {
        round: u32,
    },
    Gloom,
    LongNight {
        damage: i32,
    },
    Reinforcements,
    EnemyTurn,
    Outcome(Outcome),
}

/// Breakdown of a damage calculation, shown in the targeting tooltip.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Calc {
    pub amount: i32,
    pub shrouded: bool,
    pub flank: bool,
    pub high_ground: bool,
    pub dark_fury: bool,
    pub guarded: bool,
    pub weakened: bool,
}

pub const GLOOM_ROUNDS: [u32; 2] = [5, 9];
pub const REINFORCE_EVERY: u32 = 4;
pub const MAX_ENEMIES: usize = 12;
/// From this round the dark itself wounds the squad, escalating.
pub const LONG_NIGHT: u32 = 14;

#[derive(Clone, Debug)]
pub struct Battle {
    pub cfg: Config,
    pub map: Map,
    pub units: Vec<Unit>,
    pub round: u32,
    pub phase: Phase,
    pub rng: Rng,
    /// Gameplay light: tile is lit.
    pub lit: Vec<bool>,
    /// Visual light intensity 0..1 per tile.
    pub glow: Vec<f32>,
    pub chain: u32,
    pub best_chain: u32,
    pub score: i64,
    pub kills: u32,
    pub fallen: u32,
    pub outcome: Option<Outcome>,
    pub defeat_cause: String,
    pub log: Vec<String>,
    pub enemy_queue: Vec<u32>,
    /// Snapshot for undoing the last move.
    pub undo: Option<Box<Battle>>,
}

impl Battle {
    pub fn new(cfg: Config, squad: &[Member]) -> Battle {
        let mut rng = Rng::new(cfg.seed ^ 0xBA77_1E00);
        let roster = if cfg.boss {
            vec![]
        } else {
            enemy_roster(&mut rng, cfg.island, cfg.elite)
        };
        let slots = if cfg.boss { 6 } else { roster.len() };
        let map = Map::generate(cfg.seed, cfg.biome, slots.max(3));
        let mut b = Battle {
            cfg,
            map,
            units: vec![],
            round: 1,
            phase: Phase::Player,
            rng,
            lit: vec![false; (N * N) as usize],
            glow: vec![0.0; (N * N) as usize],
            chain: 0,
            best_chain: 0,
            score: 0,
            kills: 0,
            fallen: 0,
            outcome: None,
            defeat_cause: String::new(),
            log: vec![],
            enemy_queue: vec![],
            undo: None,
        };
        for (i, m) in squad.iter().enumerate() {
            let pos = b.map.player_spawns[i % b.map.player_spawns.len()];
            let id = b.units.len() as u32;
            let mut u = Unit::new(id, m.kind, pos);
            u.max_hp = m.max_hp;
            u.hp = m.hp.clamp(1, m.max_hp);
            u.move_range = m.move_range;
            u.power = m.power;
            u.range_bonus = m.range_bonus;
            u.lantern = m.lantern;
            u.facing = -1;
            b.units.push(u);
        }
        if b.cfg.boss {
            let spawns = b.map.enemy_spawns.clone();
            let king_at = *spawns
                .iter()
                .min_by_key(|p| (p.x + p.y, p.x))
                .expect("spawns");
            b.spawn(Kind::HollowKing, king_at, false);
            let mut rest = spawns.into_iter().filter(|p| *p != king_at);
            for k in [Kind::Shade, Kind::Shade, Kind::Wisp, Kind::Hexer] {
                if let Some(p) = rest.next() {
                    b.spawn(k, p, false);
                }
            }
        } else {
            let spawns = b.map.enemy_spawns.clone();
            for ((k, elite), p) in roster.into_iter().zip(spawns) {
                b.spawn(k, p, elite);
            }
        }
        // Pyre Keeper: extra braziers start lit.
        let mut extra = b.cfg.mods.extra_lit;
        let braziers: Vec<Pos> = b.map.braziers().collect();
        for p in braziers {
            if extra > 0 && !b.map.tile(p).lit {
                b.map.tile_mut(p).lit = true;
                extra -= 1;
            }
        }
        b.recompute_light();
        b.say(format!("{} - island {}", b.cfg.biome.name(), b.cfg.island));
        b
    }

    fn spawn(&mut self, kind: Kind, pos: Pos, elite: bool) -> u32 {
        let id = self.units.len() as u32;
        let mut u = Unit::new(id, kind, pos);
        if elite {
            u.make_elite();
        }
        u.facing = 1;
        self.units.push(u);
        id
    }

    pub fn say(&mut self, s: String) {
        self.log.push(s);
        if self.log.len() > 40 {
            self.log.remove(0);
        }
    }

    // ---------------------------------------------------------------- queries

    pub fn unit(&self, id: u32) -> &Unit {
        &self.units[id as usize]
    }
    pub fn unit_mut(&mut self, id: u32) -> &mut Unit {
        &mut self.units[id as usize]
    }
    pub fn unit_at(&self, p: Pos) -> Option<&Unit> {
        self.units.iter().find(|u| u.alive && u.pos == p)
    }
    pub fn alive(&self, team: Team) -> impl Iterator<Item = &Unit> {
        self.units
            .iter()
            .filter(move |u| u.alive && u.team() == team)
    }
    pub fn is_lit(&self, p: Pos) -> bool {
        p.in_bounds() && self.lit[Map::idx(p)]
    }
    pub fn boss(&self) -> Option<&Unit> {
        self.units.iter().find(|u| u.kind == Kind::HollowKing)
    }
    pub fn shrouded(&self, u: &Unit) -> bool {
        u.team() == Team::Dark && !self.is_lit(u.pos)
    }

    // ------------------------------------------------------------------ light

    pub fn recompute_light(&mut self) {
        let mut best = vec![f32::NEG_INFINITY; (N * N) as usize];
        let aura: Vec<Pos> = self
            .units
            .iter()
            .filter(|u| u.alive && u.kind == Kind::HollowKing)
            .map(|u| u.pos)
            .collect();
        let mut sources: Vec<(Pos, i32, bool)> = vec![];
        for u in self
            .units
            .iter()
            .filter(|u| u.alive && u.team() == Team::Squad)
        {
            sources.push((u.pos, u.lantern, true));
        }
        for p in Map::all_pos() {
            let t = self.map.tile(p);
            if t.terrain == Terrain::Brazier && t.lit {
                sources.push((p, 3 + self.cfg.mods.pyre_bonus, false));
            }
            if t.flare > 0 {
                sources.push((p, 2 + self.cfg.mods.flare_bonus, false));
            }
        }
        for p in Map::all_pos() {
            let in_aura = aura.iter().any(|a| a.chebyshev(p) <= 1);
            let i = Map::idx(p);
            for &(s, r, lantern) in &sources {
                if lantern && in_aura {
                    continue;
                }
                let dx = (s.x - p.x) as f32;
                let dy = (s.y - p.y) as f32;
                let d = (dx * dx + dy * dy).sqrt();
                // Margin: positive means inside the lit radius.
                let m = (r as f32 + 0.5 - d) / (r as f32 + 0.5);
                if m > best[i] {
                    best[i] = m;
                }
            }
        }
        for (i, &m) in best.iter().enumerate() {
            self.lit[i] = m >= 0.0;
            self.glow[i] = if m >= 0.0 {
                0.3 + 0.7 * m.min(1.0)
            } else {
                (0.3 + m * 1.2).max(0.0)
            };
        }
    }

    // --------------------------------------------------------------- movement

    /// Movement field for a unit: (cost, previous) per tile, or None.
    pub fn move_field(&self, id: u32) -> Vec<Option<(i32, Pos)>> {
        let u = self.unit(id);
        let mut field: Vec<Option<(i32, Pos)>> = vec![None; (N * N) as usize];
        field[Map::idx(u.pos)] = Some((0, u.pos));
        let mut frontier = vec![u.pos];
        let flies = u.kind.flies();
        for step in 1..=u.move_range {
            let mut next = vec![];
            for p in frontier {
                for n in p.neighbors() {
                    if !n.in_bounds() || field[Map::idx(n)].is_some() {
                        continue;
                    }
                    let t = self.map.tile(n);
                    if t.terrain.blocks() {
                        continue;
                    }
                    if !flies {
                        if !t.terrain.walkable() {
                            continue;
                        }
                        let from_h = self.map.height(p);
                        if (t.height - from_h).abs() > 1 {
                            continue;
                        }
                    }
                    if let Some(o) = self.unit_at(n)
                        && o.team() != u.team()
                    {
                        continue;
                    }
                    field[Map::idx(n)] = Some((step, p));
                    next.push(n);
                }
            }
            frontier = next;
        }
        field
    }

    /// Tiles the unit may end a move on.
    pub fn move_targets(&self, id: u32) -> Vec<Pos> {
        let u = self.unit(id);
        if u.moved || !u.alive {
            return vec![];
        }
        let field = self.move_field(id);
        Map::all_pos()
            .filter(|p| {
                *p != u.pos
                    && field[Map::idx(*p)].is_some()
                    && self.map.walkable(*p)
                    && self.unit_at(*p).is_none()
            })
            .collect()
    }

    pub fn path_to(&self, id: u32, dest: Pos) -> Option<Vec<Pos>> {
        let field = self.move_field(id);
        field[Map::idx(dest)]?;
        let start = self.unit(id).pos;
        let mut path = vec![dest];
        let mut cur = dest;
        while cur != start {
            let (_, prev) = field[Map::idx(cur)]?;
            cur = prev;
            path.push(cur);
        }
        path.reverse();
        Some(path)
    }

    pub fn move_unit(&mut self, id: u32, dest: Pos) -> Result<Vec<Event>, &'static str> {
        if self.phase == Phase::Over {
            return Err("battle over");
        }
        if !self.move_targets(id).contains(&dest) {
            return Err("can't move there");
        }
        let path = self.path_to(id, dest).ok_or("no path")?;
        if self.unit(id).team() == Team::Squad {
            let mut snap = self.clone();
            snap.undo = None;
            self.undo = Some(Box::new(snap));
        }
        let mut ev = vec![Event::Moved {
            id,
            path: path.clone(),
        }];
        let from = self.unit(id).pos;
        {
            let u = self.unit_mut(id);
            u.pos = dest;
            u.moved = true;
            u.facing = facing_for(from, dest, u.facing);
        }
        if self.unit(id).team() == Team::Squad {
            for p in path.iter().skip(1) {
                if let Some(k) = self.map.tile(*p).pickup {
                    self.map.tile_mut(*p).pickup = None;
                    ev.push(Event::Picked {
                        id,
                        at: *p,
                        kind: k,
                    });
                    match k {
                        Pickup::Ember => {
                            self.heal(id, 3, &mut ev);
                        }
                        Pickup::Lumen => {
                            self.unit_mut(id).lantern += 1;
                            let n = self.unit(id).kind.name();
                            self.say(format!("{n}'s lantern burns brighter"));
                        }
                    }
                }
            }
        }
        self.recompute_light();
        Ok(ev)
    }

    pub fn can_undo(&self) -> bool {
        self.phase == Phase::Player && self.undo.is_some()
    }

    pub fn undo_move(&mut self) -> bool {
        if !self.can_undo() {
            return false;
        }
        let snap = self.undo.take().expect("checked");
        *self = *snap;
        true
    }

    // -------------------------------------------------------------- abilities

    /// Effective max range of an ability from `from` against `to`.
    fn in_range(&self, u: &Unit, a: Ability, from: Pos, to: Pos) -> bool {
        let info = a.info();
        let d = from.manhattan(to);
        let mut max = info.max_range;
        if info.ranged {
            max += u.range_bonus;
            if self.map.height(from) > self.map.height(to) {
                max += 1;
            }
        }
        d >= info.min_range && d <= max
    }

    /// Valid targets for ability `a` if unit `id` stood at `from`.
    pub fn targets_from(&self, id: u32, a: Ability, from: Pos) -> Vec<Pos> {
        let u = self.unit(id);
        let Some(idx) = u.ability_index(a) else {
            return vec![];
        };
        if u.cooldowns[idx] > 0 {
            return vec![];
        }
        let info = a.info();
        match info.target {
            Target::SelfOnly => {
                if a == Ability::Summon && self.alive(Team::Dark).count() >= MAX_ENEMIES {
                    return vec![];
                }
                vec![from]
            }
            Target::Enemy => self
                .units
                .iter()
                .filter(|t| t.alive && t.team() != u.team())
                .filter(|t| self.in_range(u, a, from, t.pos))
                .map(|t| t.pos)
                .collect(),
            Target::Ally => self
                .units
                .iter()
                .filter(|t| t.alive && t.team() == u.team())
                .map(|t| if t.id == id { from } else { t.pos })
                .filter(|p| self.in_range(u, a, from, *p))
                .collect(),
            Target::Brazier => self
                .map
                .braziers()
                .filter(|p| {
                    let lit = self.map.tile(*p).lit;
                    (a == Ability::Kindle && !lit) || (a == Ability::Snuff && lit)
                })
                .filter(|p| self.in_range(u, a, from, *p))
                .collect(),
        }
    }

    pub fn targets(&self, id: u32, a: Ability) -> Vec<Pos> {
        let u = self.unit(id);
        if u.acted || !u.alive {
            return vec![];
        }
        self.targets_from(id, a, u.pos)
    }

    /// Damage `attacker` (standing at `from`) would deal to `target` with `a`.
    pub fn calc(&self, attacker: &Unit, a: Ability, from: Pos, target: &Unit) -> Calc {
        let info = a.info();
        let mut c = Calc::default();
        let mut dmg = info.damage + attacker.power;
        match a {
            Ability::Flare => dmg += self.cfg.mods.flare_bonus,
            Ability::Volley => dmg += self.cfg.mods.volley_bonus,
            _ => {}
        }
        if attacker.weakened > 0 {
            dmg -= 1;
            c.weakened = true;
        }
        if attacker.kind == Kind::Shade && attacker.lurking {
            dmg += 1;
            c.dark_fury = true;
        }
        if attacker.team() == Team::Squad
            && self.units.iter().any(|o| {
                o.alive
                    && o.team() == Team::Squad
                    && o.id != attacker.id
                    && o.pos.manhattan(target.pos) == 1
                    && o.pos != from
            })
        {
            dmg += 1;
            c.flank = true;
        }
        if self.map.height(from) > self.map.height(target.pos) {
            dmg += 1;
            c.high_ground = true;
        }
        dmg = dmg.max(1);
        if self.shrouded(target) {
            dmg = (dmg / 2).max(1);
            c.shrouded = true;
        }
        if target.guarded {
            dmg = (dmg - 1).max(1);
            c.guarded = true;
        }
        if self.cfg.god_mode && attacker.team() == Team::Squad {
            dmg *= 10;
        }
        c.amount = dmg;
        c
    }

    pub fn act(&mut self, id: u32, a: Ability, target: Pos) -> Result<Vec<Event>, &'static str> {
        if self.phase == Phase::Over {
            return Err("battle over");
        }
        if !self.targets(id, a).contains(&target) {
            return Err("invalid target");
        }
        self.undo = None;
        let mut ev = vec![];
        let idx = self.unit(id).ability_index(a).expect("has ability");
        let mut cd = a.info().cooldown;
        if a == Ability::Mend && self.cfg.mods.mend_haste {
            cd = cd.saturating_sub(1);
        }
        {
            let from = self.unit(id).pos;
            let u = self.unit_mut(id);
            u.acted = true;
            u.cooldowns[idx] = cd;
            if target != from {
                u.facing = facing_for(from, target, u.facing);
            }
        }
        let from = self.unit(id).pos;
        let name = self.unit(id).kind.name();
        match a {
            Ability::Strike
            | Ability::Claw
            | Ability::Nip
            | Ability::Shoot
            | Ability::Bolt
            | Ability::Bash
            | Ability::Slam
            | Ability::Rend
            | Ability::Flare
            | Ability::Curse => {
                let fx = match a {
                    Ability::Strike => Fx::Slash,
                    Ability::Claw | Ability::Nip => Fx::Claw,
                    Ability::Shoot => Fx::Arrow,
                    Ability::Bolt => Fx::Bolt,
                    Ability::Bash => Fx::Bash,
                    Ability::Slam => Fx::Slam,
                    Ability::Rend => Fx::Rend,
                    Ability::Flare => Fx::Flare,
                    _ => Fx::Curse,
                };
                ev.push(Event::Attack {
                    id,
                    from,
                    to: target,
                    fx,
                });
                if a == Ability::Flare {
                    self.map.tile_mut(target).flare = 2;
                    self.recompute_light();
                }
                if a == Ability::Shoot && self.cfg.mods.kindled_arrows {
                    let t = self.map.tile_mut(target);
                    t.flare = t.flare.max(2);
                    self.recompute_light();
                }
                let victim = self.unit_at(target).map(|t| t.id).expect("target unit");
                let calc = self.calc(self.unit(id), a, from, self.unit(victim));
                self.hurt(victim, calc.amount, Some(id), Cause::Wounds, &mut ev);
                if a == Ability::Curse && self.unit(victim).alive {
                    self.unit_mut(victim).weakened = 2;
                    ev.push(Event::Weakened { id: victim });
                }
                if a.pushes() && self.unit(victim).alive {
                    let dir = from.dir_to(target);
                    let dist = if a == Ability::Bash {
                        1 + self.cfg.mods.bash_push
                    } else {
                        1
                    };
                    self.push(victim, dir, dist, Some(id), &mut ev);
                }
            }
            Ability::Volley => {
                ev.push(Event::Attack {
                    id,
                    from,
                    to: target,
                    fx: Fx::Volley,
                });
                let mut hit: Vec<u32> = vec![];
                for p in std::iter::once(target).chain(target.neighbors()) {
                    if let Some(t) = self.unit_at(p)
                        && t.team() == Team::Dark
                    {
                        hit.push(t.id);
                    }
                }
                for v in hit {
                    let calc = self.calc(self.unit(id), a, from, self.unit(v));
                    self.hurt(v, calc.amount, Some(id), Cause::Wounds, &mut ev);
                }
            }
            Ability::Mend => {
                let ally = self.unit_at(target).map(|t| t.id).unwrap_or(id);
                let amount = 4 + self.cfg.mods.mend_bonus;
                self.heal(ally, amount, &mut ev);
            }
            Ability::Kindle => {
                self.map.tile_mut(target).lit = true;
                ev.push(Event::BrazierLit { at: target });
                self.say(format!("{name} kindles a brazier"));
                self.recompute_light();
            }
            Ability::Snuff => {
                self.map.tile_mut(target).lit = false;
                ev.push(Event::BrazierSnuffed { at: target });
                self.say(format!(
                    "{} snuffs out a brazier!",
                    self.unit(id).kind.the()
                ));
                self.recompute_light();
            }
            Ability::Bulwark => {
                let allies: Vec<u32> = self
                    .alive(Team::Squad)
                    .filter(|o| o.pos.manhattan(from) <= 1)
                    .map(|o| o.id)
                    .collect();
                for o in allies {
                    self.unit_mut(o).guarded = true;
                    ev.push(Event::Guard { id: o });
                }
                self.say("The Warden raises a bulwark".to_string());
            }
            Ability::Summon => {
                let count = if self.unit(id).kind == Kind::HollowKing {
                    2
                } else {
                    1
                };
                let mut spots: Vec<Pos> = Map::all_pos()
                    .filter(|p| {
                        self.map.walkable(*p)
                            && self.unit_at(*p).is_none()
                            && p.manhattan(from) <= 3
                    })
                    .collect();
                spots.sort_by_key(|p| (self.is_lit(*p), p.manhattan(from), p.x, p.y));
                for p in spots.into_iter().take(count) {
                    if self.alive(Team::Dark).count() >= MAX_ENEMIES {
                        break;
                    }
                    let sid = self.spawn(Kind::Shade, p, false);
                    self.unit_mut(sid).acted = true;
                    self.unit_mut(sid).moved = true;
                    ev.push(Event::Spawned { id: sid, at: p });
                }
                self.say(format!(
                    "{} calls shades from the dark",
                    self.unit(id).kind.the()
                ));
            }
            Ability::Eclipse => {
                ev.push(Event::Eclipse { at: from });
                let braziers: Vec<Pos> = self
                    .map
                    .braziers()
                    .filter(|p| p.chebyshev(from) <= 3 && self.map.tile(*p).lit)
                    .collect();
                for p in braziers {
                    self.map.tile_mut(p).lit = false;
                    ev.push(Event::BrazierSnuffed { at: p });
                }
                self.recompute_light();
                let victims: Vec<u32> = self
                    .alive(Team::Squad)
                    .filter(|u| !self.is_lit(u.pos))
                    .map(|u| u.id)
                    .collect();
                for v in victims {
                    let mut dmg = 2;
                    if self.unit(v).guarded {
                        dmg -= 1;
                    }
                    self.hurt(v, dmg, Some(id), Cause::Wounds, &mut ev);
                }
                self.say("ECLIPSE! The King swallows the light".to_string());
            }
        }
        self.recompute_light();
        self.check_outcome(&mut ev);
        Ok(ev)
    }

    fn heal(&mut self, id: u32, amount: i32, ev: &mut Vec<Event>) {
        let u = self.unit_mut(id);
        let before = u.hp;
        u.hp = (u.hp + amount).min(u.max_hp);
        let healed = u.hp - before;
        let hp = u.hp;
        if healed > 0 {
            ev.push(Event::Healed {
                id,
                amount: healed,
                hp,
            });
        }
    }

    fn hurt(&mut self, id: u32, amount: i32, by: Option<u32>, cause: Cause, ev: &mut Vec<Event>) {
        if !self.unit(id).alive {
            return;
        }
        if self.cfg.god_mode && self.unit(id).team() == Team::Squad {
            return;
        }
        let shrouded = self.shrouded(self.unit(id));
        let boss = self.unit(id).kind == Kind::HollowKing;
        let hp = {
            let u = self.unit_mut(id);
            u.hp -= amount;
            u.hp
        };
        ev.push(Event::Damaged {
            id,
            amount,
            hp: hp.max(0),
            shrouded,
            boss,
        });
        if hp <= 0 {
            self.kill(id, by, cause, ev);
        }
    }

    fn kill(&mut self, id: u32, by: Option<u32>, cause: Cause, ev: &mut Vec<Event>) {
        {
            let u = self.unit_mut(id);
            u.alive = false;
            u.hp = 0;
            u.death = Some(cause);
        }
        ev.push(Event::Died { id });
        let kind = self.unit(id).kind;
        if kind.team() == Team::Squad {
            self.fallen += 1;
            let how = match cause {
                Cause::Wounds => "falls",
                Cause::Void => "is lost to the void",
                Cause::Water => "sinks beneath the water",
            };
            self.say(format!("{} {how}!", kind.the()));
            if self.defeat_cause.is_empty() || self.alive(Team::Squad).count() == 0 {
                let killer = by
                    .map(|k| match self.unit(k).kind {
                        Kind::HollowKing => "the Hollow King".to_string(),
                        other => format!("a {}", other.name()),
                    })
                    .unwrap_or_else(|| "the dark".to_string());
                self.defeat_cause = format!("{} fell to {killer}", kind.the());
            }
            return;
        }
        self.kills += 1;
        let by_squad = by.is_some_and(|k| self.unit(k).team() == Team::Squad);
        if by_squad && self.phase == Phase::Player {
            self.chain += 1;
            self.best_chain = self.best_chain.max(self.chain);
            if self.chain >= 2 {
                ev.push(Event::Chain { count: self.chain });
            }
        }
        let mult = self.chain.max(1) as i64;
        let gained = kind.value() as i64 * mult * if self.cfg.elite { 2 } else { 1 };
        self.score += gained;
        if let Some(k) = by
            && self.unit(k).team() == Team::Squad
            && self.unit(k).alive
            && self.cfg.mods.kill_heal > 0
        {
            self.heal(k, self.cfg.mods.kill_heal, ev);
        }
        let how = match cause {
            Cause::Wounds => "destroyed",
            Cause::Void => "hurled into the void",
            Cause::Water => "drowned",
        };
        self.say(format!("{} {how} (+{gained})", kind.name()));
    }

    fn push(&mut self, id: u32, dir: (i32, i32), dist: i32, by: Option<u32>, ev: &mut Vec<Event>) {
        if self.unit(id).kind == Kind::HollowKing {
            // The King cannot be moved; the blow lands harder instead.
            ev.push(Event::Bumped {
                id,
                at: self.unit(id).pos,
            });
            self.hurt(id, 1, None, Cause::Wounds, ev);
            return;
        }
        for _ in 0..dist {
            if !self.unit(id).alive {
                return;
            }
            let from = self.unit(id).pos;
            let to = from.add(dir.0, dir.1);
            let terrain = self.map.terrain(to);
            let team = self.unit(id).team();
            if let Some(other) = self.unit_at(to).map(|o| o.id) {
                ev.push(Event::Bumped { id, at: to });
                self.hurt(id, 1, None, Cause::Wounds, ev);
                self.hurt(other, 1, None, Cause::Wounds, ev);
                return;
            }
            if terrain.blocks() {
                ev.push(Event::Bumped { id, at: to });
                self.hurt(id, 1, None, Cause::Wounds, ev);
                return;
            }
            if terrain.lethal() {
                if team == Team::Squad || self.unit(id).kind.flies() {
                    // Squad members (and hovering wisps) catch the ledge.
                    ev.push(Event::Bumped { id, at: to });
                    self.hurt(id, 2, None, Cause::Wounds, ev);
                    return;
                }
                let water = terrain == Terrain::Water;
                ev.push(Event::Pushed { id, from, to });
                self.unit_mut(id).pos = to;
                ev.push(Event::Fell { id, at: to, water });
                let cause = if water { Cause::Water } else { Cause::Void };
                self.kill(id, by, cause, ev);
                return;
            }
            if (self.map.height(to) - self.map.height(from)) > 1 {
                ev.push(Event::Bumped { id, at: to });
                self.hurt(id, 1, None, Cause::Wounds, ev);
                return;
            }
            ev.push(Event::Pushed { id, from, to });
            self.unit_mut(id).pos = to;
        }
    }

    fn check_outcome(&mut self, ev: &mut Vec<Event>) {
        if self.outcome.is_some() {
            return;
        }
        if self.alive(Team::Squad).count() == 0 {
            self.outcome = Some(Outcome::Defeat);
            self.phase = Phase::Over;
            ev.push(Event::Outcome(Outcome::Defeat));
        } else if self.alive(Team::Dark).count() == 0 {
            self.outcome = Some(Outcome::Victory);
            self.phase = Phase::Over;
            ev.push(Event::Outcome(Outcome::Victory));
            self.say("The island is cleansed!".to_string());
        }
    }

    // ------------------------------------------------------------------ turns

    pub fn end_player_turn(&mut self) -> Vec<Event> {
        let mut ev = vec![];
        if self.phase != Phase::Player {
            return ev;
        }
        self.undo = None;
        self.phase = Phase::Enemy;
        self.chain = 0;
        let mut queue: Vec<u32> = vec![];
        let ids: Vec<u32> = self.alive(Team::Dark).map(|u| u.id).collect();
        for id in ids {
            let u = self.unit_mut(id);
            u.moved = false;
            u.acted = false;
            for c in u.cooldowns.iter_mut() {
                *c = c.saturating_sub(1);
            }
            queue.push(id);
        }
        for id in queue.iter().copied() {
            let dark = !self.is_lit(self.unit(id).pos);
            self.unit_mut(id).lurking = dark;
        }
        // Snuffers go first so the light shifts before everyone else moves.
        queue.sort_by_key(|id| (self.unit(*id).kind != Kind::Snuffer, *id));
        queue.reverse();
        self.enemy_queue = queue;
        ev.push(Event::EnemyTurn);
        ev
    }

    /// Run one enemy's turn. Returns its events; when no enemies remain
    /// queued the round ends and the player phase begins.
    pub fn enemy_step(&mut self) -> Vec<Event> {
        let mut ev = vec![];
        if self.phase != Phase::Enemy {
            return ev;
        }
        while let Some(id) = self.enemy_queue.pop() {
            let u = self.unit(id);
            if !u.alive || u.acted {
                continue;
            }
            let plan = ai::plan(self, id);
            if plan.dest != self.unit(id).pos
                && let Ok(mut e) = self.move_unit(id, plan.dest)
            {
                ev.append(&mut e);
            }
            if let Some((a, t)) = plan.action
                && let Ok(mut e) = self.act(id, a, t)
            {
                ev.append(&mut e);
            }
            self.unit_mut(id).acted = true;
            self.check_outcome(&mut ev);
            return ev;
        }
        self.start_round(&mut ev);
        ev
    }

    fn start_round(&mut self, ev: &mut Vec<Event>) {
        if self.phase == Phase::Over {
            return;
        }
        self.round += 1;
        for p in Map::all_pos() {
            let t = self.map.tile_mut(p);
            t.flare = t.flare.saturating_sub(1);
        }
        for u in self.units.iter_mut().filter(|u| u.alive) {
            u.weakened = u.weakened.saturating_sub(1);
        }
        ev.push(Event::RoundStart { round: self.round });
        if GLOOM_ROUNDS.contains(&self.round) {
            for u in self
                .units
                .iter_mut()
                .filter(|u| u.alive && u.team() == Team::Squad)
            {
                u.lantern = (u.lantern - 1).max(1);
            }
            ev.push(Event::Gloom);
            self.say("The gloom deepens - lanterns dim".to_string());
        }
        if self.round.is_multiple_of(REINFORCE_EVERY) && !self.cfg.boss {
            self.reinforce(ev);
        }
        if self.round >= LONG_NIGHT {
            let damage = 1 + (self.round - LONG_NIGHT) as i32 / 2;
            ev.push(Event::LongNight { damage });
            self.say(format!("The long night bites for {damage}"));
            let ids: Vec<u32> = self.alive(Team::Squad).map(|u| u.id).collect();
            for id in ids {
                self.hurt(id, damage, None, Cause::Wounds, ev);
            }
            self.check_outcome(ev);
            if self.phase == Phase::Over {
                return;
            }
        }
        let ids: Vec<u32> = self.alive(Team::Squad).map(|u| u.id).collect();
        for id in ids {
            let u = self.unit_mut(id);
            u.moved = false;
            u.acted = false;
            u.guarded = false;
            for c in u.cooldowns.iter_mut() {
                *c = c.saturating_sub(1);
            }
        }
        self.recompute_light();
        self.phase = Phase::Player;
    }

    fn reinforce(&mut self, ev: &mut Vec<Event>) {
        let count = 1 + self.cfg.island as usize / 4;
        let squad: Vec<Pos> = self.alive(Team::Squad).map(|u| u.pos).collect();
        let mut spots: Vec<Pos> = Map::all_pos()
            .filter(|p| {
                self.map.walkable(*p)
                    && self.unit_at(*p).is_none()
                    && squad.iter().all(|s| s.manhattan(*p) >= 4)
            })
            .collect();
        self.rng.shuffle(&mut spots);
        spots.sort_by_key(|p| self.is_lit(*p));
        let mut spawned = 0;
        for p in spots.into_iter().take(count) {
            if self.alive(Team::Dark).count() >= MAX_ENEMIES {
                break;
            }
            let kind = if self.cfg.island >= 3 && self.rng.chance(0.25) {
                Kind::Wisp
            } else {
                Kind::Shade
            };
            let id = self.spawn(kind, p, false);
            ev.push(Event::Spawned { id, at: p });
            spawned += 1;
        }
        if spawned > 0 {
            ev.push(Event::Reinforcements);
            self.say("More shades crawl from the dark!".to_string());
        }
    }

    /// Living squad members as persistent members (for the run).
    pub fn squad_members(&self) -> Vec<(Kind, i32, bool)> {
        self.units
            .iter()
            .filter(|u| u.team() == Team::Squad)
            .map(|u| (u.kind, u.hp, u.alive))
            .collect()
    }
}

fn facing_for(from: Pos, to: Pos, current: i32) -> i32 {
    // Screen x grows with (x - y).
    let dx = (to.x - to.y) - (from.x - from.y);
    if dx > 0 {
        1
    } else if dx < 0 {
        -1
    } else {
        current
    }
}

/// Enemy roster for a normal island: escalating budget, unlocking kinds.
pub fn enemy_roster(rng: &mut Rng, island: u32, elite: bool) -> Vec<(Kind, bool)> {
    let mut pool = vec![Kind::Shade, Kind::Wisp];
    let newest = match island {
        0 | 1 => None,
        2 => Some(Kind::Snuffer),
        3 => Some(Kind::Brute),
        _ => Some(Kind::Hexer),
    };
    if island >= 2 {
        pool.push(Kind::Snuffer);
    }
    if island >= 3 {
        pool.push(Kind::Brute);
    }
    if island >= 4 {
        pool.push(Kind::Hexer);
    }
    let mut budget = 4 + island as i32 * 3 + if elite { 4 } else { 0 };
    let mut out: Vec<(Kind, bool)> = vec![];
    if let Some(k) = newest {
        out.push((k, false));
        budget -= k.cost();
    }
    out.push((Kind::Shade, false));
    budget -= 2;
    while budget >= 2 && out.len() < 9 {
        let mut k = *rng.pick(&pool);
        if k == Kind::Wisp && out.iter().filter(|e| e.0 == Kind::Wisp).count() >= 2 {
            k = Kind::Shade;
        }
        if k.cost() <= budget {
            out.push((k, false));
            budget -= k.cost();
        } else {
            out.push((Kind::Shade, false));
            budget -= 2;
        }
    }
    let elites = if elite {
        2
    } else if island >= 5 {
        1
    } else {
        0
    };
    for e in out.iter_mut().rev().take(elites) {
        e.1 = true;
    }
    out
}
