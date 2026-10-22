//! Rules tests on hand-built scenarios.

use lanternfall::battle::{Battle, Config, Event, LONG_NIGHT, Member, Mods, Outcome, Phase};
use lanternfall::map::{Biome, Map, Pickup, Pos, Terrain};
use lanternfall::run::{ISLANDS, Run, Stage};
use lanternfall::units::{Ability, Cause, Kind, Team, Unit};

fn cfg() -> Config {
    Config {
        seed: 1,
        island: 1,
        biome: Biome::Moss,
        elite: false,
        boss: false,
        mods: Mods::default(),
        god_mode: false,
    }
}

/// A flat, empty island with the given units (ids follow the order).
fn arena(units: &[(Kind, Pos)]) -> Battle {
    let squad: Vec<Member> = Kind::SQUAD.iter().map(|k| Member::new(*k)).collect();
    let mut b = Battle::new(cfg(), &squad);
    for t in b.map.tiles.iter_mut() {
        t.terrain = Terrain::Ground;
        t.height = 0;
        t.lit = false;
        t.pickup = None;
        t.flare = 0;
    }
    b.units.clear();
    for (i, (k, p)) in units.iter().enumerate() {
        b.units.push(Unit::new(i as u32, *k, *p));
    }
    b.recompute_light();
    b
}

#[test]
fn shrouded_targets_take_half_damage_until_lit() {
    let mut b = arena(&[
        (Kind::Ranger, Pos::new(1, 5)),
        (Kind::Shade, Pos::new(5, 5)),
    ]);
    assert!(!b.is_lit(Pos::new(5, 5)));
    let c = b.calc(b.unit(0), Ability::Shoot, Pos::new(1, 5), b.unit(1));
    assert!(c.shrouded);
    assert_eq!(c.amount, 1);
    b.map.tile_mut(Pos::new(5, 5)).flare = 2;
    b.recompute_light();
    let c = b.calc(b.unit(0), Ability::Shoot, Pos::new(1, 5), b.unit(1));
    assert!(!c.shrouded);
    assert_eq!(c.amount, 3);
}

#[test]
fn bash_into_the_void_is_lethal() {
    let mut b = arena(&[
        (Kind::Warden, Pos::new(5, 4)),
        (Kind::Brute, Pos::new(5, 5)),
        (Kind::Shade, Pos::new(0, 0)),
    ]);
    b.map.tile_mut(Pos::new(5, 6)).terrain = Terrain::Chasm;
    let ev = b.act(0, Ability::Bash, Pos::new(5, 5)).unwrap();
    assert!(ev.iter().any(|e| matches!(
        e,
        Event::Fell {
            id: 1,
            water: false,
            ..
        }
    )));
    assert!(!b.unit(1).alive);
    assert_eq!(b.unit(1).death, Some(Cause::Void));
}

#[test]
fn bash_into_water_drowns_and_walls_hurt() {
    let mut b = arena(&[
        (Kind::Warden, Pos::new(5, 4)),
        (Kind::Shade, Pos::new(5, 5)),
        (Kind::Warden, Pos::new(2, 4)),
        (Kind::Brute, Pos::new(2, 5)),
    ]);
    b.map.tile_mut(Pos::new(5, 6)).terrain = Terrain::Water;
    b.map.tile_mut(Pos::new(2, 6)).terrain = Terrain::Rock;
    b.act(0, Ability::Bash, Pos::new(5, 5)).unwrap();
    assert_eq!(b.unit(1).death, Some(Cause::Water));
    let hp = b.unit(3).hp;
    b.act(2, Ability::Bash, Pos::new(2, 5)).unwrap();
    assert_eq!(b.unit(3).pos, Pos::new(2, 5));
    // 2 from the bash, +1 for slamming into the boulder.
    assert!(b.unit(3).hp <= hp - 3);
}

#[test]
fn the_king_cannot_be_shoved() {
    let mut b = arena(&[
        (Kind::Warden, Pos::new(5, 4)),
        (Kind::HollowKing, Pos::new(5, 5)),
    ]);
    b.map.tile_mut(Pos::new(5, 6)).terrain = Terrain::Chasm;
    b.act(0, Ability::Bash, Pos::new(5, 5)).unwrap();
    assert!(b.unit(1).alive);
    assert_eq!(b.unit(1).pos, Pos::new(5, 5));
}

#[test]
fn king_aura_swallows_lantern_light_but_not_flares() {
    let mut b = arena(&[
        (Kind::Warden, Pos::new(5, 4)),
        (Kind::HollowKing, Pos::new(5, 5)),
    ]);
    assert!(!b.is_lit(Pos::new(5, 5)));
    assert!(!b.is_lit(Pos::new(5, 4)), "aura covers adjacent tiles");
    b.map.tile_mut(Pos::new(5, 5)).flare = 2;
    b.recompute_light();
    assert!(b.is_lit(Pos::new(5, 5)));
}

#[test]
fn squad_catches_the_ledge() {
    let mut b = arena(&[
        (Kind::Ranger, Pos::new(5, 5)),
        (Kind::Brute, Pos::new(5, 4)),
    ]);
    b.map.tile_mut(Pos::new(5, 6)).terrain = Terrain::Chasm;
    b.phase = Phase::Enemy;
    b.act(1, Ability::Slam, Pos::new(5, 5)).unwrap();
    assert!(b.unit(0).alive);
    assert_eq!(b.unit(0).pos, Pos::new(5, 5));
}

#[test]
fn kindle_and_snuff_change_light() {
    let mut b = arena(&[
        (Kind::Lamplighter, Pos::new(1, 1)),
        (Kind::Snuffer, Pos::new(7, 6)),
    ]);
    let br = Pos::new(7, 7);
    b.map.tile_mut(br).terrain = Terrain::Brazier;
    b.recompute_light();
    assert!(!b.is_lit(Pos::new(8, 9)));
    b.unit_mut(0).pos = Pos::new(6, 7);
    b.act(0, Ability::Kindle, br).unwrap();
    assert!(b.map.tile(br).lit);
    assert!(b.is_lit(Pos::new(8, 9)));
    b.phase = Phase::Enemy;
    b.act(1, Ability::Snuff, br).unwrap();
    assert!(!b.map.tile(br).lit);
}

#[test]
fn undo_restores_a_move_but_not_after_acting() {
    let mut b = arena(&[
        (Kind::Warden, Pos::new(5, 5)),
        (Kind::Shade, Pos::new(5, 8)),
    ]);
    b.move_unit(0, Pos::new(5, 7)).unwrap();
    assert!(b.can_undo());
    assert!(b.undo_move());
    assert_eq!(b.unit(0).pos, Pos::new(5, 5));
    assert!(!b.unit(0).moved);
    b.move_unit(0, Pos::new(5, 7)).unwrap();
    b.act(0, Ability::Strike, Pos::new(5, 8)).unwrap();
    assert!(!b.can_undo());
}

#[test]
fn cooldowns_skip_a_turn() {
    let mut b = arena(&[
        (Kind::Warden, Pos::new(5, 5)),
        (Kind::Brute, Pos::new(5, 6)),
    ]);
    b.unit_mut(1).hp = 99;
    b.unit_mut(1).max_hp = 99;
    b.act(0, Ability::Bash, Pos::new(5, 6)).unwrap();
    let back = |b: &mut Battle| {
        b.end_player_turn();
        while b.phase == Phase::Enemy {
            b.enemy_step();
        }
    };
    back(&mut b);
    let adj = b.unit(1).pos;
    if adj.manhattan(b.unit(0).pos) == 1 {
        assert!(b.targets(0, Ability::Bash).is_empty(), "still recharging");
    }
    back(&mut b);
    assert_eq!(b.unit(0).cooldowns[1], 0);
}

#[test]
fn flank_and_high_ground_add_damage() {
    let mut b = arena(&[
        (Kind::Warden, Pos::new(5, 5)),
        (Kind::Ranger, Pos::new(5, 7)),
        (Kind::Brute, Pos::new(5, 6)),
    ]);
    // Light the brute so shroud doesn't interfere.
    assert!(b.is_lit(Pos::new(5, 6)));
    let c = b.calc(b.unit(0), Ability::Strike, Pos::new(5, 5), b.unit(2));
    assert!(c.flank);
    assert_eq!(c.amount, 4);
    b.map.tile_mut(Pos::new(5, 5)).height = 1;
    let c = b.calc(b.unit(0), Ability::Strike, Pos::new(5, 5), b.unit(2));
    assert!(c.high_ground);
    assert_eq!(c.amount, 5);
}

#[test]
fn pickups_are_collected_along_the_path() {
    let mut b = arena(&[
        (Kind::Warden, Pos::new(5, 5)),
        (Kind::Shade, Pos::new(0, 0)),
    ]);
    b.map.tile_mut(Pos::new(5, 6)).pickup = Some(Pickup::Lumen);
    b.unit_mut(0).hp = 5;
    b.map.tile_mut(Pos::new(5, 7)).pickup = Some(Pickup::Ember);
    let lantern = b.unit(0).lantern;
    b.move_unit(0, Pos::new(5, 8)).unwrap();
    assert_eq!(b.unit(0).lantern, lantern + 1);
    assert_eq!(b.unit(0).hp, 8);
    assert!(b.map.tile(Pos::new(5, 6)).pickup.is_none());
}

#[test]
fn long_night_wounds_the_squad() {
    let mut b = arena(&[
        (Kind::Warden, Pos::new(1, 1)),
        (Kind::Shade, Pos::new(9, 9)),
    ]);
    b.round = LONG_NIGHT - 1;
    let hp = b.unit(0).hp;
    b.end_player_turn();
    let mut saw = false;
    while b.phase == Phase::Enemy {
        for e in b.enemy_step() {
            if matches!(e, Event::LongNight { .. }) {
                saw = true;
            }
        }
    }
    assert!(saw);
    assert!(b.unit(0).hp < hp);
}

#[test]
fn enemies_attack_when_they_can() {
    let mut b = arena(&[
        (Kind::Ranger, Pos::new(5, 5)),
        (Kind::Shade, Pos::new(5, 8)),
    ]);
    let hp = b.unit(0).hp;
    b.end_player_turn();
    while b.phase == Phase::Enemy {
        b.enemy_step();
    }
    assert!(b.unit(0).hp < hp, "the shade should close in and claw");
}

#[test]
fn outcomes_trigger() {
    let mut b = arena(&[(Kind::Warden, Pos::new(5, 5)), (Kind::Wisp, Pos::new(5, 6))]);
    b.unit_mut(1).hp = 1;
    let ev = b.act(0, Ability::Strike, Pos::new(5, 6)).unwrap();
    assert!(ev.contains(&Event::Outcome(Outcome::Victory)));
    assert_eq!(b.phase, Phase::Over);

    let mut b = arena(&[
        (Kind::Ranger, Pos::new(5, 5)),
        (Kind::Brute, Pos::new(5, 6)),
    ]);
    b.unit_mut(0).hp = 1;
    b.phase = Phase::Enemy;
    let ev = b.act(1, Ability::Slam, Pos::new(5, 5)).unwrap();
    assert!(ev.contains(&Event::Outcome(Outcome::Defeat)));
    assert!(!b.defeat_cause.is_empty());
}

#[test]
fn reinforcements_arrive() {
    let mut b = arena(&[
        (Kind::Warden, Pos::new(0, 0)),
        (Kind::Shade, Pos::new(10, 10)),
    ]);
    b.unit_mut(1).hp = 99;
    let before = b.alive(Team::Dark).count();
    for _ in 0..4 {
        b.end_player_turn();
        while b.phase == Phase::Enemy {
            b.enemy_step();
        }
        if b.outcome.is_some() {
            break;
        }
    }
    assert!(b.round >= 4);
    assert!(b.alive(Team::Dark).count() > before);
}

#[test]
fn battles_are_deterministic() {
    let squad: Vec<Member> = Kind::SQUAD.iter().map(|k| Member::new(*k)).collect();
    let a = Battle::new(cfg(), &squad);
    let b = Battle::new(cfg(), &squad);
    assert_eq!(a.map.tiles, b.map.tiles);
    let pa: Vec<(Kind, Pos)> = a.units.iter().map(|u| (u.kind, u.pos)).collect();
    let pb: Vec<(Kind, Pos)> = b.units.iter().map(|u| (u.kind, u.pos)).collect();
    assert_eq!(pa, pb);
}

#[test]
fn every_island_spawns_a_fair_fight() {
    for seed in 0..40u64 {
        for island in 1..=6 {
            let mut c = cfg();
            c.seed = seed;
            c.island = island;
            c.biome = Biome::NORMAL[(seed % 4) as usize];
            let squad: Vec<Member> = Kind::SQUAD.iter().map(|k| Member::new(*k)).collect();
            let b = Battle::new(c, &squad);
            assert_eq!(b.alive(Team::Squad).count(), 3);
            assert!(b.alive(Team::Dark).count() >= 3);
            for u in &b.units {
                assert!(b.map.walkable(u.pos));
                assert_eq!(b.units.iter().filter(|o| o.pos == u.pos).count(), 1);
            }
            let reach = b.map.flood(b.unit(0).pos);
            for u in b.alive(Team::Dark) {
                assert!(
                    reach[Map::idx(u.pos)],
                    "seed {seed} island {island}: enemy unreachable"
                );
            }
        }
    }
}

#[test]
fn run_progresses_through_draft_and_route() {
    let mut r = Run::new(4);
    r.god_mode = true;
    r.battle.as_mut().unwrap().cfg.god_mode = true;
    let b = r.battle.as_mut().unwrap();
    for u in b.units.iter_mut().filter(|u| u.team() == Team::Dark) {
        u.alive = false;
    }
    b.outcome = Some(Outcome::Victory);
    r.finish_battle();
    assert!(matches!(r.stage, Stage::Draft { .. }));
    assert!(r.score > 0);
    assert!(r.choose_upgrade(0));
    assert!(matches!(r.stage, Stage::Route { .. }));
    assert!(r.choose_route(0));
    assert_eq!(r.island, 2);
    assert!(r.island < ISLANDS);
}
