//! Tactical AI. Enemies score every reachable tile x every usable ability x
//! every target and take the best plan. The same evaluator (with a different
//! team perspective) drives the squad autopilot used by the playtest bot.

use crate::battle::Battle;
use crate::map::{Map, N, Pos, Terrain};
use crate::units::{Ability, Kind, Target, Team, Unit};

#[derive(Clone, Debug, PartialEq)]
pub struct Plan {
    pub dest: Pos,
    pub action: Option<(Ability, Pos)>,
    pub score: f32,
}

pub fn plan(b: &Battle, id: u32) -> Plan {
    let u = b.unit(id);
    let mut dests = vec![u.pos];
    if !u.moved {
        dests.extend(b.move_targets(id));
    }
    let mut best = Plan {
        dest: u.pos,
        action: None,
        score: f32::NEG_INFINITY,
    };
    let f = fields(b, u);
    for dest in dests {
        let pos_score = position_score(b, u, dest);
        let mut best_action: Option<(Ability, Pos, f32)> = None;
        if !u.acted {
            for &a in u.kind.abilities() {
                for t in b.targets_from(id, a, dest) {
                    let s = action_score(b, u, a, dest, t);
                    if s > best_action.map(|x| x.2).unwrap_or(0.0) {
                        best_action = Some((a, t, s));
                    }
                }
            }
        }
        let total = match best_action {
            Some((_, _, s)) => pos_score + s,
            None => pos_score - approach_cost(u, &f, dest),
        };
        if total > best.score + 0.001 {
            best = Plan {
                dest,
                action: best_action.map(|(a, t, _)| (a, t)),
                score: total,
            };
        }
    }
    best
}

fn foes<'a>(b: &'a Battle, u: &Unit) -> impl Iterator<Item = &'a Unit> {
    let team = u.team();
    b.units.iter().filter(move |o| o.alive && o.team() != team)
}

fn position_score(b: &Battle, u: &Unit, dest: Pos) -> f32 {
    let mut s = 0.0;
    let lit = b.is_lit(dest);
    let ranged = matches!(u.kind, Kind::Wisp | Kind::Hexer | Kind::Ranger);
    match u.team() {
        Team::Dark => {
            if !lit {
                s += match u.kind {
                    Kind::Snuffer => 2.0,
                    Kind::HollowKing => 1.0,
                    _ => 4.0,
                };
            }
        }
        Team::Squad => {
            if lit {
                s += 1.0;
            }
            // Threat: damage enemies could plausibly bring here next turn.
            let threat: f32 = foes(b, u)
                .filter(|o| {
                    let reach = o.move_range
                        + o.kind
                            .abilities()
                            .iter()
                            .map(|a| a.info().max_range)
                            .max()
                            .unwrap_or(1);
                    o.pos.manhattan(dest) <= reach
                })
                .map(|o| {
                    o.kind
                        .abilities()
                        .iter()
                        .map(|a| a.info().damage)
                        .max()
                        .unwrap_or(1) as f32
                        + o.power as f32
                })
                .sum();
            let frail = if u.kind == Kind::Warden { 0.8 } else { 2.0 };
            s -= threat * frail;
            let allies = b
                .units
                .iter()
                .filter(|o| {
                    o.alive && o.team() == Team::Squad && o.id != u.id && o.pos.manhattan(dest) <= 2
                })
                .count() as f32;
            s += allies * 1.5;
            if let Some(p) = b.map.tile(dest).pickup {
                s += match p {
                    crate::map::Pickup::Ember => {
                        if u.hp < u.max_hp - 2 {
                            6.0
                        } else {
                            1.0
                        }
                    }
                    crate::map::Pickup::Lumen => 4.0,
                };
            }
        }
    }
    s += b.map.height(dest) as f32 * 0.6;
    if u.kind != Kind::HollowKing && !u.kind.flies() {
        let edges = dest
            .neighbors()
            .iter()
            .filter(|n| b.map.terrain(**n).lethal())
            .count() as f32;
        s -= edges * if u.team() == Team::Dark { 1.2 } else { 0.3 };
    }
    if ranged {
        let close = foes(b, u).filter(|o| o.pos.manhattan(dest) <= 1).count() as f32;
        s -= close * 5.0;
    }
    s
}

/// Multi-source BFS walking distance (ignoring units). Unreachable = 99.
pub fn distance_field(b: &Battle, seeds: &[Pos], flies: bool) -> Vec<i32> {
    let mut d = vec![99; (N * N) as usize];
    let mut q = std::collections::VecDeque::new();
    for s in seeds {
        if s.in_bounds() {
            d[Map::idx(*s)] = 0;
            q.push_back(*s);
        }
    }
    while let Some(p) = q.pop_front() {
        let dp = d[Map::idx(p)];
        for n in p.neighbors() {
            if !n.in_bounds() || d[Map::idx(n)] <= dp + 1 {
                continue;
            }
            let t = b.map.tile(n);
            if t.terrain.blocks() {
                continue;
            }
            if !flies && (!t.terrain.walkable() || (t.height - b.map.height(p)).abs() > 1) {
                continue;
            }
            d[Map::idx(n)] = dp + 1;
            q.push_back(n);
        }
    }
    d
}

fn brazier_sides(b: &Battle, lit: bool) -> Vec<Pos> {
    b.map
        .braziers()
        .filter(|p| b.map.tile(*p).lit == lit)
        .flat_map(|p| p.neighbors())
        .filter(|n| b.map.walkable(*n))
        .collect()
}

/// Precomputed fields for one unit's planning.
struct Fields {
    foes: Vec<i32>,
    goal: Option<Vec<i32>>,
}

fn fields(b: &Battle, u: &Unit) -> Fields {
    let flies = u.kind.flies();
    let foe_pos: Vec<Pos> = foes(b, u).map(|o| o.pos).collect();
    let goal = match (u.team(), u.kind) {
        (Team::Dark, Kind::Snuffer) => Some(brazier_sides(b, true)),
        (Team::Squad, Kind::Lamplighter) => Some(brazier_sides(b, false)),
        _ => None,
    }
    .filter(|g| !g.is_empty())
    .map(|g| distance_field(b, &g, flies));
    Fields {
        foes: distance_field(b, &foe_pos, flies),
        goal,
    }
}

fn approach_cost(u: &Unit, f: &Fields, dest: Pos) -> f32 {
    let foe = f.foes[Map::idx(dest)];
    if let Some(g) = &f.goal {
        let gd = g[Map::idx(dest)];
        return match u.team() {
            Team::Dark => gd as f32 * 3.0,
            Team::Squad => gd.min(foe) as f32 * 2.0,
        };
    }
    let ideal = match u.kind {
        Kind::Wisp | Kind::Hexer | Kind::Ranger => 3,
        Kind::Lamplighter => 2,
        _ => 1,
    };
    (foe - ideal).abs() as f32 * 2.5
}

fn action_score(b: &Battle, u: &Unit, a: Ability, from: Pos, t: Pos) -> f32 {
    let info = a.info();
    match info.target {
        Target::Enemy => {
            let Some(victim) = b.unit_at(t) else {
                return 0.0;
            };
            let calc = b.calc(u, a, from, victim);
            let mut s = calc.amount.min(victim.hp) as f32 * 10.0;
            if calc.amount >= victim.hp {
                s += 40.0 + victim.kind.value() as f32 * 0.2;
            }
            s += match victim.kind {
                Kind::Lamplighter => 6.0,
                Kind::Ranger => 4.0,
                Kind::HollowKing => 8.0,
                Kind::Hexer => 6.0,
                Kind::Snuffer => 4.0,
                _ => 0.0,
            };
            if a.pushes() && victim.kind != Kind::HollowKing && calc.amount < victim.hp {
                let dir = from.dir_to(t);
                let land = t.add(dir.0, dir.1);
                let terr = b.map.terrain(land);
                if terr.lethal() && victim.team() == Team::Dark && !victim.kind.flies() {
                    s += 60.0;
                } else if terr.blocks() || b.unit_at(land).is_some() || terr.lethal() {
                    s += 8.0;
                }
            }
            if a == Ability::Volley {
                let extra = t
                    .neighbors()
                    .iter()
                    .filter(|n| b.unit_at(**n).is_some_and(|o| o.team() != u.team()))
                    .count();
                s += extra as f32 * 14.0;
            }
            if a == Ability::Flare && b.shrouded(victim) {
                s += 10.0;
            }
            if a == Ability::Curse && victim.weakened == 0 {
                s += 12.0;
            }
            s
        }
        Target::Ally => {
            let ally = if t == from {
                u
            } else {
                match b.unit_at(t) {
                    Some(a) => a,
                    None => return 0.0,
                }
            };
            let missing = ally.max_hp - ally.hp;
            if missing < 3 {
                0.0
            } else {
                15.0 + missing as f32 * 4.0
            }
        }
        Target::Brazier => {
            if a == Ability::Kindle {
                30.0
            } else {
                45.0
            }
        }
        Target::SelfOnly => match a {
            Ability::Summon => 35.0,
            Ability::Bulwark => {
                let allies = b
                    .units
                    .iter()
                    .filter(|o| o.alive && o.team() == Team::Squad && o.pos.manhattan(from) <= 1)
                    .count() as f32;
                let danger = foes(b, u).filter(|o| o.pos.manhattan(from) <= 4).count() as f32;
                if danger > 0.0 {
                    allies * 6.0 + danger * 2.0
                } else {
                    0.0
                }
            }
            Ability::Eclipse => {
                let braziers = b
                    .map
                    .braziers()
                    .filter(|p| b.map.tile(*p).lit && p.chebyshev(from) <= 3)
                    .count() as f32;
                let dark_squad = b
                    .units
                    .iter()
                    .filter(|o| o.alive && o.team() == Team::Squad && !b.is_lit(o.pos))
                    .count() as f32;
                15.0 + braziers * 12.0 + dark_squad * 18.0
            }
            _ => 0.0,
        },
    }
}

/// Squad autopilot for a whole turn: repeatedly execute the single best
/// plan across all members until nobody has anything useful left.
pub fn autopilot_turn(b: &mut Battle) {
    for _ in 0..12 {
        if b.outcome.is_some() {
            return;
        }
        let best = b
            .units
            .iter()
            .filter(|u| u.alive && u.team() == Team::Squad && !(u.moved && u.acted))
            .map(|u| (u.id, plan(b, u.id)))
            .filter(|(id, p)| p.action.is_some() || p.dest != b.unit(*id).pos)
            .max_by(|a, b| a.1.score.total_cmp(&b.1.score));
        let Some((id, p)) = best else {
            return;
        };
        let mut did = false;
        if p.dest != b.unit(id).pos && b.move_unit(id, p.dest).is_ok() {
            did = true;
        }
        if let Some((a, t)) = p.action
            && b.act(id, a, t).is_ok()
        {
            did = true;
        }
        if !did {
            // Avoid spinning on an unusable plan.
            let u = b.unit_mut(id);
            u.moved = true;
            u.acted = true;
        }
    }
}

/// Squad autopilot for one unit: move then act using the same evaluator.
/// Returns false if the unit had nothing worth doing.
pub fn autopilot_unit(b: &mut Battle, id: u32) -> bool {
    let u = b.unit(id);
    if !u.alive || u.team() != Team::Squad || (u.moved && u.acted) {
        return false;
    }
    let p = plan(b, id);
    let mut did = false;
    if p.dest != b.unit(id).pos && b.move_unit(id, p.dest).is_ok() {
        did = true;
    }
    if let Some((a, t)) = p.action
        && b.act(id, a, t).is_ok()
    {
        did = true;
    }
    did
}

/// Whether a tile is one where pushing a unit would kill it.
pub fn deadly_edge(map: &Map, p: Pos) -> bool {
    p.neighbors().iter().any(|n| {
        let t = map.terrain(*n);
        t == Terrain::Chasm || t == Terrain::Water
    })
}
