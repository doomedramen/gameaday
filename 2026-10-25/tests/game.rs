//! Gameplay-level checks against the library.

use slipstream_cup::car::{Controls, Stats};
use slipstream_cup::race::{CHAMPION, Driver, Kind, Mode, PLAYER, Phase, ROSTER, Race};
use slipstream_cup::track::{BIOMES, Biome, HALF_W, Track, TrackSpec, WALL};

fn spec(seed: u64, biome: Biome) -> TrackSpec {
    TrackSpec {
        seed,
        biome,
        twisty: 0.5,
        hazard: 0.6,
        length: 13000.0,
    }
}

#[test]
fn generated_circuits_are_valid_loops() {
    for seed in 0..10 {
        for b in BIOMES {
            let t = Track::generate(spec(seed, b));
            assert!(t.attempts > 0, "fell back to oval");
            assert!((t.len - 13000.0).abs() < 600.0, "length {}", t.len);
            assert!(t.corners.len() >= 4, "only {} corners", t.corners.len());
            // the line stays on the road
            assert!(t.line.iter().all(|o| o.abs() < HALF_W));
            // every grid slot is on the road
            for k in 0..9 {
                let (p, _) = t.grid_pos(k);
                assert!(t.locate(p, None).lat.abs() < HALF_W);
            }
            // scenery never sits on the racing surface
            for pr in &t.props {
                assert!(
                    t.locate(pr.pos, None).lat.abs() > WALL,
                    "{} on track",
                    pr.sprite
                );
            }
        }
    }
}

#[test]
fn pace_notes_point_the_right_way() {
    for seed in 0..6 {
        let t = Track::generate(spec(seed, Biome::Timberline));
        for c in &t.corners {
            let a = t.angle_at(c.start_s);
            let b = t.angle_at(c.end_s);
            let turn = slipstream_cup::math::wrap_angle(b - a);
            // right turns rotate clockwise on screen (y down): angle grows
            assert_eq!(turn.signum(), c.dir, "corner at {} turns {turn}", c.start_s);
        }
    }
}

#[test]
fn same_seed_same_track() {
    let a = Track::generate(spec(5, Biome::DuneSea));
    let b = Track::generate(spec(5, Biome::DuneSea));
    assert_eq!(a.pts, b.pts);
    assert_eq!(a.patches.len(), b.patches.len());
}

fn run(race: &mut Race, secs: f32, c: Controls) {
    for _ in 0..(secs * 60.0) as usize {
        race.update(1.0 / 60.0, c);
        race.sounds.clear();
        race.skids.clear();
    }
}

#[test]
fn slipstream_fills_nitro() {
    let t = Track::generate(spec(3, Biome::Greenvale));
    let mut race = Race::new(t, Mode::Championship, 3, Stats::default(), &ROSTER, 0.95, 1);
    race.autopilot = true;
    run(&mut race, 30.0, Controls::default());
    assert!(
        race.stats_time_drafting > 0.5,
        "never drafted: {}",
        race.stats_time_drafting
    );
}

#[test]
fn every_rival_finishes_and_places_are_a_permutation() {
    let t = Track::generate(spec(11, Biome::GrandPrix));
    let mut rivals: Vec<Driver> = ROSTER.to_vec();
    rivals.push(CHAMPION);
    let mut race = Race::new(t, Mode::Championship, 2, Stats::default(), &rivals, 1.0, 9);
    race.autopilot = true;
    let mut secs = 0.0;
    while race.phase != Phase::Done {
        run(&mut race, 1.0, Controls::default());
        secs += 1.0;
        assert!(secs < 200.0, "race never ended");
    }
    let mut places: Vec<usize> = race.racers.iter().map(|r| r.place).collect();
    places.sort();
    assert_eq!(places, (1..=8).collect::<Vec<_>>());
    assert!(race.racers.iter().all(|r| r.finish.is_some()));
}

#[test]
fn bully_telegraphs_before_ramming() {
    // put the player alongside Brick and watch the state machine
    let t = Track::generate(spec(2, Biome::Greenvale));
    let brick = ROSTER
        .iter()
        .find(|d| d.kind == Kind::Bully)
        .copied()
        .unwrap();
    let mut race = Race::new(t, Mode::Championship, 3, Stats::default(), &[brick], 1.0, 4);
    run(
        &mut race,
        3.6,
        Controls {
            throttle: 1.0,
            ..Default::default()
        },
    );
    let mut saw_warn = false;
    let mut warned_before_ram = false;
    for _ in 0..900 {
        // keep the player next to the bully
        let bi = race
            .racers
            .iter()
            .position(|r| r.driver.kind == Kind::Bully)
            .unwrap();
        let pi = race.player_idx();
        let (bp, bh, bv) = (
            race.racers[bi].car.pos,
            race.racers[bi].car.heading,
            race.racers[bi].car.vel,
        );
        let side = race.racers[bi].car.right() * 70.0;
        race.racers[pi].car.pos = bp + side;
        race.racers[pi].car.heading = bh;
        race.racers[pi].car.vel = bv;
        race.racers[bi].ai.cd = race.racers[bi].ai.cd.min(0.1);
        run(
            &mut race,
            1.0 / 60.0,
            Controls {
                throttle: 1.0,
                ..Default::default()
            },
        );
        let ai = &race.racers[bi].ai;
        if ai.telegraphing() {
            saw_warn = true;
        }
        if ai.ramming() && saw_warn {
            warned_before_ram = true;
            break;
        }
    }
    assert!(warned_before_ram, "bully never telegraphed then rammed");
}

#[test]
fn early_throttle_stalls_and_late_throttle_launches() {
    let t = Track::generate(spec(4, Biome::Greenvale));
    let mut race = Race::new(t, Mode::TimeTrial, 1, Stats::default(), &[], 1.0, 1);
    run(
        &mut race,
        3.6,
        Controls {
            throttle: 1.0,
            ..Default::default()
        },
    );
    assert!(
        race.racers[0].stalled > 0.0 || race.t > 0.0 && race.racers[0].car.speed() < 150.0,
        "early throttle should stall"
    );

    let t = Track::generate(spec(4, Biome::Greenvale));
    let mut race = Race::new(t, Mode::TimeTrial, 1, Stats::default(), &[], 1.0, 1);
    run(&mut race, 3.2, Controls::default());
    run(
        &mut race,
        0.4,
        Controls {
            throttle: 1.0,
            ..Default::default()
        },
    );
    assert!(
        race.racers[0].car.boost_t > 0.0,
        "rocket start did not boost"
    );
}

#[test]
fn ghost_is_recorded_for_the_best_lap() {
    let t = Track::generate(spec(6, Biome::NeonNight));
    let mut race = Race::new(t, Mode::TimeTrial, 2, Stats::default(), &[], 1.0, 1);
    race.autopilot = true;
    while race.phase != Phase::Done && race.phase != Phase::Cooldown {
        run(&mut race, 1.0, Controls::default());
    }
    let g = race.best_ghost.as_ref().expect("ghost");
    let lap = race.best_lap.unwrap();
    assert!((g.len() as f32 / slipstream_cup::race::GHOST_HZ - lap).abs() < 0.5);
    let _ = PLAYER;
}
