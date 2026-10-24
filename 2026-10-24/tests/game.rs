//! Rules of the game on hand-built levels: movement tuning, the rope,
//! momentum smashing, foes and hazards, rewind, the boss, generation.

use std::sync::Arc;

use cragswing::boss::BState;
use cragswing::game::Game;
use cragswing::level::*;
use cragswing::sim::*;
use cragswing::stagegen::*;

fn idle() -> Controls {
    Controls::default()
}

fn run_right() -> Controls {
    Controls {
        dir: 1,
        ..Controls::default()
    }
}

/// A wide flat floor at row 30 (top surface y = 960), air above and a pit below.
fn flat_level() -> Level {
    let mut l = Level::new(100, 50, Biome::Mossbluff, 1);
    l.fill(0, 30, 100, 34, ROCK);
    l.spawn = (3.0 * T, 30.0 * T - HH - 1.0);
    l.exit = (95.0 * T, 20.0 * T);
    l
}

fn world(mut l: Level) -> World {
    l.finalize();
    let mut w = World::new(Arc::new(l), Mods::default(), 4);
    for _ in 0..10 {
        w.step(&idle());
    }
    w
}

fn spawn_ent(l: &mut Level, kind: EntKind, x: f32, y: f32) {
    l.ents.push(EntSpawn::new(kind, x, y));
}

/// Hold jump for the whole rise and report the peak height gained in pixels.
fn jump_peak(hold_frames: usize) -> f32 {
    let mut w = world(flat_level());
    let y0 = w.p.y;
    let mut peak = 0.0f32;
    for f in 0..90 {
        let c = Controls {
            jump_pressed: f == 0,
            jump_held: f < hold_frames,
            ..idle()
        };
        w.step(&c);
        peak = peak.max(y0 - w.p.y);
    }
    peak
}

#[test]
fn jump_height_is_generous_and_variable() {
    let full = jump_peak(60);
    let tap = jump_peak(3);
    assert!((100.0..135.0).contains(&full), "full jump {full}");
    assert!(tap < full * 0.5, "tap {tap} vs full {full}");
    assert!(tap > 15.0, "even a tap should leave the ground: {tap}");
}

#[test]
fn flat_gaps_up_to_four_tiles_can_be_jumped() {
    for (gap, expect) in [(2, true), (4, true), (6, false)] {
        let mut l = Level::new(80, 50, Biome::Mossbluff, 1);
        l.fill(0, 30, 20, 34, ROCK);
        l.fill(20 + gap, 30, 70, 34, ROCK);
        l.spawn = (2.0 * T, 30.0 * T - HH - 1.0);
        let mut w = world(l);
        let mut jumped = false;
        let mut landed = false;
        for _ in 0..400 {
            let mut c = run_right();
            if !jumped && w.p.x + HW >= 20.0 * T && w.p.on_ground {
                c.jump_pressed = true;
                jumped = true;
            }
            c.jump_held = jumped && (w.p.vy < 0.0 || c.jump_pressed);
            w.step(&c);
            if jumped && w.p.on_ground && w.p.x > (20 + gap) as f32 * T {
                landed = true;
                break;
            }
            if w.p.y > 40.0 * T {
                break;
            }
        }
        assert_eq!(landed, expect, "gap {gap}");
    }
}

#[test]
fn coyote_time_and_jump_buffer_forgive_timing() {
    // run off a ledge and jump a few frames late
    let mut l = Level::new(60, 50, Biome::Mossbluff, 1);
    l.fill(0, 30, 20, 34, ROCK);
    l.spawn = (2.0 * T, 30.0 * T - HH - 1.0);
    let mut w = world(l.clone());
    while w.p.x < 20.0 * T + 2.0 {
        w.step(&run_right());
    }
    for _ in 0..4 {
        w.step(&run_right());
    }
    assert!(!w.p.on_ground);
    w.step(&Controls {
        dir: 1,
        jump_pressed: true,
        jump_held: true,
        ..idle()
    });
    assert!(w.p.vy < -300.0, "late jump should work: vy {}", w.p.vy);
    // far too late: no jump
    let mut w = world(l);
    while w.p.x < 20.0 * T + 2.0 {
        w.step(&run_right());
    }
    for _ in 0..14 {
        w.step(&run_right());
    }
    w.step(&Controls {
        dir: 1,
        jump_pressed: true,
        ..idle()
    });
    assert!(w.p.vy > 0.0, "air jump should not exist without the boon");

    // buffered jump: press while still falling, jump on landing
    let mut w = world(flat_level());
    w.p.y -= 40.0;
    w.p.on_ground = false;
    let mut pressed = false;
    let mut jumped = false;
    for _ in 0..40 {
        let near_floor = w.p.y + HH > 30.0 * T - 14.0;
        let c = Controls {
            jump_pressed: near_floor && !pressed,
            jump_held: near_floor,
            ..idle()
        };
        pressed |= near_floor;
        w.step(&c);
        if pressed && w.p.vy < -300.0 {
            jumped = true;
            break;
        }
    }
    assert!(
        jumped,
        "jump buffered before landing should fire on landing"
    );
}

#[test]
fn wall_jumps_climb_a_shaft() {
    let mut l = Level::new(60, 60, Biome::Mossbluff, 1);
    l.fill(0, 50, 60, 54, ROCK);
    l.fill(10, 20, 12, 50, ROCK); // left wall
    l.fill(16, 20, 18, 50, ROCK); // right wall, four tiles apart
    l.spawn = (14.0 * T, 50.0 * T - HH - 1.0);
    let mut w = world(l);
    let y0 = w.p.y;
    let mut side = 1;
    let mut best = 0.0f32;
    for _ in 0..600 {
        let mut c = Controls {
            dir: side,
            ..idle()
        };
        if w.p.on_ground && w.p.x < 15.0 * T {
            c.dir = 1;
        }
        if w.p.wall != 0 {
            side = -w.p.wall;
            c.dir = side;
            c.jump_pressed = true;
        } else if w.p.on_ground {
            c.jump_pressed = true;
        }
        c.jump_held = c.jump_pressed || w.p.vy < 0.0;
        w.step(&c);
        best = best.max(y0 - w.p.y);
    }
    assert!(best > 9.0 * T, "wall jumping only climbed {best}px");
}

fn rope_world() -> World {
    let mut l = flat_level();
    l.anchors.push(Anchor {
        x: 12.0 * T,
        y: 22.0 * T,
        kind: AnchorKind::Ring,
        x2: 12.0 * T,
        y2: 22.0 * T,
        phase: 0.0,
    });
    l.spawn = (7.0 * T, 30.0 * T - HH - 1.0);
    world(l)
}

#[test]
fn rope_swings_gain_speed_and_never_stretch() {
    let mut w = rope_world();
    // jump, then hook the anchor at the top of the jump
    let aim = Some((12.0 * T, 22.0 * T));
    w.step(&Controls {
        jump_pressed: true,
        jump_held: true,
        ..idle()
    });
    let mut hooked = false;
    let mut peak_speed = 0.0f32;
    let mut max_stretch = 0.0f32;
    for f in 0..200 {
        let mut c = Controls {
            hook_held: hooked,
            aim,
            jump_held: f < 20,
            dir: 1,
            ..idle()
        };
        if !hooked && w.p.vy > -50.0 && f > 5 {
            c.hook_pressed = true;
            c.hook_held = true;
            hooked = true;
        }
        w.step(&c);
        if let Some(r) = &w.p.rope {
            let d = dist(w.p.x, w.p.y, r.ax, r.ay);
            max_stretch = max_stretch.max(d - r.len);
            peak_speed = peak_speed.max(w.p.speed);
        }
    }
    assert!(hooked, "hook never fired");
    assert!(peak_speed > 450.0, "swing too slow: {peak_speed}");
    assert!(max_stretch < 3.0, "rope stretched by {max_stretch}");
}

#[test]
fn hook_misses_without_an_anchor_and_ignores_walls() {
    let mut w = rope_world();
    w.step(&Controls {
        hook_pressed: true,
        hook_held: true,
        aim: Some((7.0 * T, 30.0 * T + 200.0)),
        ..idle()
    });
    assert!(w.p.rope.is_none());
    assert!(w.hook_miss.is_some());
}

#[test]
fn brittle_anchors_break_after_a_moment() {
    let mut l = flat_level();
    l.anchors.push(Anchor {
        x: 10.0 * T,
        y: 24.0 * T,
        kind: AnchorKind::Brittle,
        x2: 10.0 * T,
        y2: 24.0 * T,
        phase: 0.0,
    });
    l.spawn = (10.0 * T, 30.0 * T - HH - 1.0);
    let mut w = world(l);
    w.step(&Controls {
        hook_pressed: true,
        hook_held: true,
        aim: Some((10.0 * T, 24.0 * T)),
        ..idle()
    });
    assert!(w.p.rope.is_some());
    let mut broke_at = None;
    for f in 0..200 {
        w.step(&Controls {
            hook_held: true,
            aim: Some((10.0 * T, 24.0 * T)),
            ..idle()
        });
        if w.p.rope.is_none() {
            broke_at = Some(f);
            break;
        }
    }
    let f = broke_at.expect("brittle anchor held forever");
    assert!((70..110).contains(&f), "broke after {f} frames");
}

#[test]
fn glass_needs_speed_to_shatter() {
    let make = || {
        let mut l = flat_level();
        for y in 25..30 {
            l.set(20, y, GLASS);
        }
        l.spawn = (3.0 * T, 30.0 * T - HH - 1.0);
        world(l)
    };
    // walking pace: blocked
    let mut w = make();
    for _ in 0..300 {
        w.step(&run_right());
    }
    assert!(w.p.x < 20.0 * T, "walked through glass");
    assert_eq!(w.stats.smashes, 0);
    // at speed: smashes and carries on
    let mut w = make();
    w.p.x = 17.0 * T;
    for _ in 0..30 {
        w.p.vx = 600.0;
        w.step(&run_right());
        if w.p.x > 22.0 * T {
            break;
        }
    }
    assert!(w.stats.smashes >= 1, "fast hit did not break glass");
    assert!(w.p.x > 21.0 * T, "should pass through, at {}", w.p.x);
    assert!(w.events.iter().any(|e| e.kind == Ev::Smash));
}

#[test]
fn crumbling_floors_fall_and_return() {
    let mut l = flat_level();
    l.fill(10, 30, 14, 31, CRUMBLE);
    l.spawn = (11.0 * T, 30.0 * T - HH - 1.0);
    let mut w = world(l);
    let b = w.level.block_at(10, 30).unwrap();
    assert!(w.blocks[b].solid);
    let mut gone = false;
    for _ in 0..60 {
        w.step(&idle());
        if !w.blocks[b].solid {
            gone = true;
            break;
        }
    }
    assert!(gone, "crumble never collapsed under Wren");
    // step away, the tile comes back
    w.p.x = 40.0 * T;
    w.p.y = 30.0 * T - HH - 1.0;
    for _ in 0..240 {
        w.step(&idle());
    }
    assert!(w.blocks[b].solid, "crumble should return");
}

#[test]
fn stomps_ram_and_armour() {
    // stomp a skitter
    let mut l = flat_level();
    spawn_ent(&mut l, EntKind::Skitter, 10.0 * T, 30.0 * T - 8.0);
    l.spawn = (10.0 * T, 30.0 * T - 120.0);
    let mut w = world(l);
    for _ in 0..120 {
        w.step(&idle());
        if w.stats.kills > 0 {
            break;
        }
    }
    assert_eq!(w.stats.kills, 1, "dropping on a Skitter should stomp it");
    assert_eq!(w.stats.damage, 0);
    assert!(w.p.vy < 0.0, "stomp should bounce");

    // a bulwark shrugs off a stomp but falls to a ram
    let mut l = flat_level();
    spawn_ent(&mut l, EntKind::Bulwark, 10.0 * T, 30.0 * T - 13.0);
    l.spawn = (10.0 * T, 30.0 * T - 26.0 - HH - 24.0);
    let mut w = World::new(
        Arc::new({
            let mut l2 = l.clone();
            l2.finalize();
            l2
        }),
        Mods::default(),
        4,
    );
    // first contact: bounce off the shield, nobody hurt
    for _ in 0..12 {
        w.step(&idle());
    }
    assert_eq!(w.stats.kills, 0, "Bulwark must survive a gentle stomp");
    assert_eq!(w.stats.damage, 0, "stomping armour should not hurt");
    l.spawn = (3.0 * T, 30.0 * T - HH - 1.0);
    let mut w = world(l);
    w.p.x = 6.0 * T;
    w.p.y = 30.0 * T - HH - 1.0;
    for _ in 0..40 {
        w.p.vx = 620.0;
        w.step(&run_right());
        if w.stats.kills > 0 {
            break;
        }
    }
    assert_eq!(w.stats.rams, 1, "a fast charge rams the Bulwark");
}

#[test]
fn spikes_hurt_once_then_reset_to_safe_ground() {
    let mut l = flat_level();
    l.set(12, 29, SPIKE_UP);
    l.set(13, 29, SPIKE_UP);
    l.spawn = (3.0 * T, 30.0 * T - HH - 1.0);
    let mut w = world(l);
    for _ in 0..60 {
        w.step(&idle());
    }
    let hp = w.p.hp;
    for _ in 0..140 {
        w.step(&run_right());
        if w.p.hp < hp {
            break;
        }
    }
    assert_eq!(w.p.hp, hp - 1);
    assert!(
        w.p.x < 12.0 * T,
        "should be back before the spikes, at {}",
        w.p.x
    );
    assert!(w.p.invuln > 0.5, "grace period after a hit");
    w.step(&idle());
    assert_eq!(w.p.hp, hp - 1);
    assert_eq!(w.last_cause, "spikes");
}

#[test]
fn falling_into_a_pit_costs_no_heart() {
    let mut l = Level::new(60, 50, Biome::Mossbluff, 1);
    l.fill(0, 30, 20, 34, ROCK);
    l.spawn = (4.0 * T, 30.0 * T - HH - 1.0);
    let mut w = world(l);
    for _ in 0..90 {
        w.step(&idle());
    }
    let hp = w.p.hp;
    for _ in 0..400 {
        w.step(&run_right());
        if w.stats.falls > 0 {
            break;
        }
    }
    assert_eq!(w.stats.falls, 1);
    assert_eq!(w.p.hp, hp);
    assert!(w.p.x < 20.0 * T && w.p.on_ground || w.p.y < 31.0 * T);
}

#[test]
fn spitter_telegraphs_before_it_fires() {
    let mut l = flat_level();
    spawn_ent(&mut l, EntKind::Spitter, 25.0 * T, 30.0 * T - 13.0);
    l.spawn = (15.0 * T, 30.0 * T - HH - 1.0);
    l.finalize();
    let mut w = World::new(Arc::new(l), Mods::default(), 4);
    let mut charge_frame = None;
    let mut fire_frame = None;
    for f in 0..300 {
        w.step(&idle());
        for e in w.events.drain(..) {
            match e.kind {
                Ev::SpitterCharge if charge_frame.is_none() => charge_frame = Some(f),
                Ev::SpitterFire if fire_frame.is_none() => fire_frame = Some(f),
                _ => {}
            }
        }
    }
    let (c, fi) = (
        charge_frame.expect("no charge"),
        fire_frame.expect("no shot"),
    );
    assert!(fi - c >= 42, "telegraph only {} frames", fi - c);
}

#[test]
fn geysers_warn_then_burst() {
    let mut l = flat_level();
    let mut e = EntSpawn::new(EntKind::Geyser, 20.0 * T, 30.0 * T - 6.0);
    e.a = 4.0;
    e.b = 0.0;
    l.ents.push(e);
    l.spawn = (3.0 * T, 30.0 * T - HH - 1.0);
    let mut w = world(l);
    let mut states = vec![];
    for _ in 0..600 {
        w.step(&idle());
        let g = w.ents.iter().find(|e| e.kind == EntKind::Geyser).unwrap();
        if states.last() != Some(&g.state) {
            states.push(g.state);
        }
    }
    assert!(states.windows(3).any(|s| s == [0, 1, 2]), "{states:?}");
}

#[test]
fn shield_and_phoenix_blunt_the_first_blows() {
    let mut l = flat_level();
    l.set(12, 29, SPIKE_UP);
    let mods = Mods {
        shield: true,
        revive: true,
        ..Mods::default()
    };
    l.finalize();
    let mut w = World::new(Arc::new(l), mods, 1);
    w.p.x = 12.0 * T + 16.0;
    w.p.y = 30.0 * T - HH - 2.0;
    w.step(&idle());
    assert_eq!(w.p.hp, 1, "shield absorbs the first hit");
    assert_eq!(w.stats.damage, 0);
    for _ in 0..100 {
        w.step(&idle());
    }
    w.hurt(0.0, 0.0, false);
    assert_eq!(w.p.hp, 1, "phoenix keeps Wren alive at one heart");
    assert!(w.p.dying <= 0.0);
    for _ in 0..100 {
        w.step(&idle());
    }
    w.hurt(0.0, 0.0, false);
    assert!(w.p.dying > 0.0, "second lethal blow kills");
}

#[test]
fn rewind_undoes_damage_and_broken_glass_and_costs_echo() {
    let mut l = flat_level();
    for y in 25..30 {
        l.set(20, y, GLASS);
    }
    l.finalize();
    let mut g = Game::new(Arc::new(l), Mods::default(), 4, 100.0);
    for _ in 0..30 {
        g.tick(&idle(), false);
    }
    g.world.p.x = 17.0 * T;
    let hp = g.world.p.hp;
    for _ in 0..30 {
        g.world.p.vx = 600.0;
        g.tick(&run_right(), false);
    }
    assert!(g.world.stats.smashes >= 1);
    g.world.hurt(0.0, 0.0, false);
    assert_eq!(g.world.p.hp, hp - 1);
    let echo = g.echo;
    for _ in 0..90 {
        g.tick(&idle(), true);
    }
    assert!(g.echo < echo - 15.0, "rewind should cost echo");
    assert_eq!(g.world.p.hp, hp, "hearts come back");
    let b = g.world.level.block_at(20, 27).unwrap();
    assert!(g.world.blocks[b].solid, "glass is whole again");
    // with no echo left rewinding stops
    g.echo = 0.0;
    let x = g.world.p.x;
    g.tick(&run_right(), true);
    g.tick(&run_right(), true);
    assert!(g.world.p.x != x || g.world.frame > 0);
}

#[test]
fn echo_orbs_and_checkpoints_cannot_be_farmed_by_rewinding() {
    let mut l = flat_level();
    spawn_ent(&mut l, EntKind::EchoOrb, 8.0 * T, 30.0 * T - 20.0);
    l.finalize();
    let mut g = Game::new(Arc::new(l), Mods::default(), 4, 20.0);
    for _ in 0..90 {
        g.tick(&run_right(), false);
    }
    let after_first = g.echo;
    assert!(after_first > 40.0, "orb should refill echo: {after_first}");
    // spend a little echo rewinding past the pickup, then collect it again
    for _ in 0..60 {
        g.tick(&idle(), true);
    }
    let spent = g.echo;
    for _ in 0..90 {
        g.tick(&run_right(), false);
    }
    assert!(
        g.echo < spent + 12.0,
        "orb paid out twice: {} -> {}",
        spent,
        g.echo
    );
}

#[test]
fn the_stormheart_only_yields_to_speed_while_stunned() {
    let level = generate(&StageSpec::new(5, 1));
    let mut w = World::new(Arc::new(level), Mods::default(), 4);
    for _ in 0..5 {
        w.step(&idle());
    }
    // not stunned: even a fast hit does nothing (and hurts)
    {
        let b = w.boss.as_mut().unwrap();
        b.state = BState::Idle;
        b.t = 0.0;
        b.x = 38.0 * T;
        b.y = 30.0 * T;
    }
    w.p.x = 38.0 * T;
    w.p.y = 30.0 * T;
    w.p.vx = 600.0;
    w.step(&idle());
    assert_eq!(
        w.boss.as_ref().unwrap().hp,
        3,
        "boss is invulnerable when not stunned"
    );
    assert!(w.p.hp < 4, "touching the awake boss hurts");

    // stunned, slow contact: harmless bounce, no damage to the boss
    let mut w = World::new(
        Arc::new(generate(&StageSpec::new(5, 1))),
        Mods::default(),
        4,
    );
    {
        let b = w.boss.as_mut().unwrap();
        b.state = BState::Stun;
        b.t = 0.0;
        b.x = 38.0 * T;
        b.y = 43.0 * T;
    }
    w.p.x = 36.0 * T;
    w.p.y = 43.0 * T;
    for _ in 0..30 {
        w.p.vx = 150.0;
        w.step(&run_right());
    }
    assert_eq!(w.boss.as_ref().unwrap().hp, 3, "slow bumps do nothing");
    assert_eq!(w.p.hp, 4, "stunned boss does not hurt");

    // stunned, fast: it cracks
    let mut w = World::new(
        Arc::new(generate(&StageSpec::new(5, 1))),
        Mods::default(),
        4,
    );
    {
        let b = w.boss.as_mut().unwrap();
        b.state = BState::Stun;
        b.t = 0.0;
        b.x = 38.0 * T;
        b.y = 43.0 * T;
    }
    w.p.x = 32.0 * T;
    w.p.y = 43.0 * T;
    for _ in 0..60 {
        w.p.vx = 700.0;
        w.step(&run_right());
        if w.boss.as_ref().unwrap().hp < 3 {
            break;
        }
    }
    assert_eq!(
        w.boss.as_ref().unwrap().hp,
        2,
        "a fast hit must crack the shell"
    );
}

#[test]
fn every_generated_ascent_is_well_formed() {
    for stage in 0..5usize {
        for seed in 1..=40u64 {
            let spec = StageSpec::new(stage, seed * 977);
            let a = generate(&spec);
            let b = generate(&spec);
            assert_eq!(a.tiles, b.tiles, "generation must be deterministic");
            assert_eq!(a.beats.len(), b.beats.len());
            let (sx, sy) = a.spawn;
            // spawn is not inside rock and stands on something
            let w = World::new(Arc::new(a.clone()), Mods::default(), 4);
            assert!(
                !w.box_hits(sx, sy, HW, HH),
                "stage {stage} seed {seed}: spawn in rock"
            );
            assert!(
                w.box_hits(sx, sy + 3.0, HW, HH),
                "spawn should rest on the floor"
            );
            // beats advance rightwards
            for pair in a.beats.windows(2) {
                assert!(pair[1].edge_x >= pair[0].edge_x - 1.0, "beats out of order");
            }
            // nothing spawns inside solid rock
            for e in &a.ents {
                if matches!(e.kind, EntKind::Wind | EntKind::Strike) {
                    continue;
                }
                let t = a.tile((e.x / T) as i32, (e.y / T) as i32);
                assert!(
                    !matches!(t, ROCK | ICE),
                    "stage {stage} seed {seed}: {:?} inside rock at {},{}",
                    e.kind,
                    e.x,
                    e.y
                );
            }
            // exit is open air and beyond the spawn
            assert!(a.exit.0 > sx + 3000.0);
            assert!(
                !w.box_hits(a.exit.0, a.exit.1, 8.0, 8.0),
                "stage {stage} seed {seed}: exit inside rock"
            );
            // the level has the content its ascent promises
            assert!(a.ents.iter().any(|e| e.kind == EntKind::Crystal));
            if stage >= 1 {
                assert!(!a.anchors.is_empty(), "later ascents need ropes");
            }
        }
    }
}

#[test]
fn difficulty_scales_with_stage_heat_and_route() {
    let base: Vec<f32> = (0..5).map(|s| StageSpec::new(s, 1).difficulty()).collect();
    assert!(base.windows(2).all(|w| w[1] > w[0]), "{base:?}");
    let mut hot = StageSpec::new(2, 1);
    hot.heat = 3;
    assert!(hot.difficulty() > base[2] + 0.4);
    let mut daring = StageSpec::new(2, 1);
    daring.daring = true;
    assert!(daring.difficulty() > base[2]);
}

#[test]
fn tutorial_level_forces_a_momentum_smash() {
    let mut spec = StageSpec::new(0, 5);
    spec.tutorial = true;
    let l = generate(&spec);
    let glass = l.tiles.iter().filter(|t| **t == GLASS).count();
    assert!(glass >= 6, "training must contain the glass door");
    assert!(!l.prompts.is_empty());
    // there is no way round: nothing but glass between the swing and the exit
    let mut w = World::new(Arc::new(l), Mods::default(), 4);
    assert!(!w.level.boss);
    for _ in 0..10 {
        w.step(&idle());
    }
    assert_eq!(w.outcome, Outcome::Playing);
}
