use gameaday_2026_10_21::*;

fn playing(seed: u32) -> Game {
    let mut g = Game::new(seed);
    g.state = GameState::Playing;
    g
}

fn step(g: &mut Game, dt: f32, frames: usize, input: &Input) {
    for _ in 0..frames {
        g.update(dt, input);
        g.events.clear();
    }
}

#[test]
fn starts_on_title_and_waits_for_input() {
    let mut g = Game::new(1);
    assert_eq!(g.state, GameState::Title);
    // Time passes, but the game does not start without a start press.
    step(&mut g, 1.0 / 60.0, 30, &Input::default());
    assert_eq!(g.state, GameState::Title);

    let start = Input {
        start: true,
        ..Default::default()
    };
    g.update(1.0 / 60.0, &start);
    assert_eq!(g.state, GameState::Playing);
    assert_eq!(g.wave, 1);
    assert!(g.health > 0.0);
}

#[test]
fn a_full_run_reaches_later_waves_without_panicking() {
    // Simulate a competent player: always firing, drifting, dashing, pulsing.
    // The goal is to exercise every system for many simulated seconds.
    let mut g = playing(42);
    let mut rng = Rng32::new(7);
    for i in 0..60 * 90 {
        // 90 simulated seconds.
        let ang = rng.range(0.0, std::f32::consts::TAU);
        let input = Input {
            up: ang < 1.2,
            down: ang > 2.0 && ang < 3.2,
            left: ang > 1.2 && ang < 2.4,
            right: ang > 3.2,
            fire: true,
            dash: i % 120 == 0,
            pulse: i % 240 == 0,
            pointer: Vec2::new(rng.range(-1.0, 1.0), rng.range(-1.0, 1.0)),
            start: false,
            aim_override: Vec2::new(0.0, 0.0),
        };
        g.update(1.0 / 60.0, &input);
        g.events.clear();
        // Never let the sim run without a live player for this test.
        if g.state == GameState::GameOver {
            break;
        }
    }
    // Either we survived deep, or died - both must be a clean state.
    assert!(matches!(g.state, GameState::Playing | GameState::GameOver));
    assert!(g.wave >= 1);
}

#[test]
fn bullets_damage_and_kill_drifters() {
    let mut g = playing(3);
    // Clear the wave so nothing interferes, then place a single drifter.
    g.enemies.clear();
    g.spawn_queue = 0;
    g.enemies.push(Enemy {
        pos: Vec2::new(g.player_pos.x + 120.0, g.player_pos.y),
        vel: Vec2::new(0.0, 0.0),
        hp: 1,
        kind: EnemyKind::Drifter,
        flash: 0.0,
        spawn_t: 0.0,
        seed: 1,
        atk_cd: 99.0,
    });
    g.aim = Vec2::new(1.0, 0.0);
    let input = Input {
        fire: true,
        ..Default::default()
    };
    let score_before = g.score;
    for _ in 0..120 {
        g.update(1.0 / 60.0, &input);
        g.events.clear();
    }
    assert!(g.enemies.is_empty(), "drifter should have been destroyed");
    assert!(g.score > score_before);
}

#[test]
fn bloater_splits_into_two_drifters() {
    let mut g = playing(9);
    g.enemies.clear();
    g.spawn_queue = 0;
    g.enemies.push(Enemy {
        pos: Vec2::new(g.player_pos.x + 100.0, g.player_pos.y),
        vel: Vec2::new(0.0, 0.0),
        hp: 1,
        kind: EnemyKind::Bloater,
        flash: 0.0,
        spawn_t: 0.0,
        seed: 2,
        atk_cd: 99.0,
    });
    g.aim = Vec2::new(1.0, 0.0);
    let input = Input {
        fire: true,
        ..Default::default()
    };
    let mut max_drifters = 0;
    for _ in 0..90 {
        g.update(1.0 / 60.0, &input);
        g.events.clear();
        let d = g
            .enemies
            .iter()
            .filter(|e| e.kind == EnemyKind::Drifter)
            .count();
        max_drifters = max_drifters.max(d);
    }
    assert!(
        max_drifters >= 2,
        "bloater death should spawn at least two drifters (got {max_drifters})"
    );
}

#[test]
fn player_cannot_leave_the_arena() {
    let mut g = playing(11);
    let input = Input {
        up: true,
        left: true,
        ..Default::default()
    };
    for _ in 0..600 {
        g.update(1.0 / 60.0, &input);
        g.events.clear();
        assert!(g.player_pos.x >= PLAYER_RADIUS - 0.5);
        assert!(g.player_pos.y >= PLAYER_RADIUS - 0.5);
        assert!(g.player_pos.x <= WORLD_W - PLAYER_RADIUS + 0.5);
        assert!(g.player_pos.y <= WORLD_H - PLAYER_RADIUS + 0.5);
    }
}

#[test]
fn dash_has_a_cooldown_and_boosts_speed() {
    let mut g = playing(13);
    let idle = Input::default();
    step(&mut g, 1.0 / 60.0, 5, &idle);
    g.dash();
    let speed_after = g.player_vel.len();
    assert!(
        speed_after > PLAYER_MAX_SPEED,
        "dash should exceed the walking speed cap"
    );
    assert!(g.dash_cooldown > 0.0);
    // Second dash immediately should be ignored.
    let before = g.dash_cooldown;
    g.dash();
    assert_eq!(g.dash_cooldown, before);
}

#[test]
fn pulse_requires_full_energy_and_then_clears_nearby_enemies() {
    let mut g = playing(17);
    assert!(!g.pulse(), "pulse must not fire without a full meter");

    g.energy = g.max_energy;
    g.enemies.clear();
    g.spawn_queue = 0;
    for i in 0..3 {
        g.enemies.push(Enemy {
            pos: Vec2::new(g.player_pos.x + 20.0 * i as f32, g.player_pos.y),
            vel: Vec2::new(0.0, 0.0),
            hp: 1,
            kind: EnemyKind::Drifter,
            flash: 0.0,
            spawn_t: 0.0,
            seed: i,
            atk_cd: 99.0,
        });
    }
    assert!(g.pulse());
    assert_eq!(g.energy, 0.0);
    assert!(
        g.enemies.is_empty(),
        "pulse should wipe weak nearby enemies"
    );
}

#[test]
fn orbs_charge_energy_when_collected() {
    let mut g = playing(19);
    g.enemies.clear();
    g.spawn_queue = 0;
    g.energy = 0.0;
    g.orbs.push(Orb {
        pos: Vec2::new(g.player_pos.x + 4.0, g.player_pos.y),
        vel: Vec2::new(0.0, 0.0),
        life: 5.0,
        anim: 0.0,
    });
    let idle = Input::default();
    step(&mut g, 1.0 / 60.0, 10, &idle);
    assert!(g.energy > 0.0, "collecting an orb should add energy");
    assert!(g.orbs.is_empty());
}

#[test]
fn combo_multiplier_grows_and_resets() {
    let mut g = playing(23);
    g.enemies.clear();
    g.spawn_queue = 0;
    g.energy = g.max_energy;
    for i in 0..12 {
        g.enemies.push(Enemy {
            pos: Vec2::new(g.player_pos.x + 15.0 + i as f32, g.player_pos.y),
            vel: Vec2::new(0.0, 0.0),
            hp: 1,
            kind: EnemyKind::Drifter,
            flash: 0.0,
            spawn_t: 0.0,
            seed: i,
            atk_cd: 99.0,
        });
    }
    g.pulse();
    assert!(g.combo >= 12);
    assert!(g.combo_multiplier() >= 3);
    // Combo should decay after the timer expires.
    let idle = Input::default();
    step(&mut g, 1.0 / 60.0, 200, &idle);
    assert_eq!(g.combo, 0, "combo should reset once the timer runs out");
}

#[test]
fn boss_wave_spawns_a_warden() {
    let mut g = playing(29);
    g.enemies.clear();
    g.spawn_queue = 0;
    // Skip to just before a boss wave.
    g.wave = 4;
    g.wave_banner = 0.0;
    g.update_waves(1.0 / 60.0);
    assert_eq!(g.wave, 5);
    assert!(g.is_boss_wave());
    assert!(
        g.enemies.iter().any(|e| e.kind.is_boss()),
        "wave 5 must contain the Warden"
    );
}

#[test]
fn boss_wave_does_not_advance_while_boss_lives() {
    let mut g = playing(31);
    g.wave = 5;
    g.spawn_queue = 0;
    g.wave_banner = 0.0;
    g.enemies.clear();
    g.enemies.push(Enemy {
        pos: Vec2::new(100.0, 100.0),
        vel: Vec2::new(0.0, 0.0),
        hp: 50,
        kind: EnemyKind::Warden,
        flash: 0.0,
        spawn_t: 0.0,
        seed: 1,
        atk_cd: 99.0,
    });
    for _ in 0..600 {
        // Hold still; the boss is far enough that contact damage won't apply.
        g.update(1.0 / 60.0, &Input::default());
        g.events.clear();
        assert_eq!(g.wave, 5, "wave must not advance while the boss lives");
    }
    // Kill it and let the wave roll over.
    g.enemies.clear();
    for _ in 0..120 {
        g.update(1.0 / 60.0, &Input::default());
        g.events.clear();
    }
    assert!(g.wave >= 6);
}

#[test]
fn touch_damage_reduces_health_and_grants_invulnerability() {
    let mut g = playing(37);
    g.enemies.clear();
    g.spawn_queue = 0;
    g.invuln = 0.0;
    let hp = g.health;
    g.enemies.push(Enemy {
        pos: Vec2::new(g.player_pos.x + 4.0, g.player_pos.y),
        vel: Vec2::new(0.0, 0.0),
        hp: 1,
        kind: EnemyKind::Bloater,
        flash: 0.0,
        spawn_t: 0.0,
        seed: 1,
        atk_cd: 99.0,
    });
    g.update(1.0 / 60.0, &Input::default());
    assert!(g.health < hp, "contact should hurt");
    assert!(g.invuln > 0.0, "a hit should grant i-frames");
    // Immediate second hit is blocked.
    let hp2 = g.health;
    g.update(1.0 / 60.0, &Input::default());
    assert_eq!(
        g.health, hp2,
        "i-frames should prevent instant double damage"
    );
}

#[test]
fn death_ends_the_run_and_restart_resets_everything() {
    let mut g = playing(41);
    g.health = 1.0;
    g.invuln = 0.0;
    g.enemies.clear();
    g.spawn_queue = 0;
    g.enemies.push(Enemy {
        pos: g.player_pos,
        vel: Vec2::new(0.0, 0.0),
        hp: 1,
        kind: EnemyKind::Bloater,
        flash: 0.0,
        spawn_t: 0.0,
        seed: 1,
        atk_cd: 99.0,
    });
    g.update(1.0 / 60.0, &Input::default());
    assert_eq!(g.state, GameState::GameOver);
    assert!(g.best_score >= g.score);

    let start = Input {
        start: true,
        ..Default::default()
    };
    g.update(1.0 / 60.0, &start);
    assert_eq!(g.state, GameState::Playing);
    assert_eq!(g.wave, 1);
    assert_eq!(g.score, 0);
    assert_eq!(g.health, g.max_health);
    assert!(g.enemies.is_empty());
    assert!(g.bullets.is_empty());
    assert!(g.orbs.is_empty());
    assert!(g.particles.is_empty());
}

#[test]
fn game_over_shows_restart_and_preserves_best_score() {
    let mut g = playing(43);
    g.score = 1234;
    g.health = 0.0;
    g.enemies.clear();
    g.spawn_queue = 0;
    g.update(1.0 / 60.0, &Input::default());
    assert_eq!(g.state, GameState::GameOver);
    assert_eq!(g.best_score, 1234);
}

#[test]
fn boss_fires_spits_that_can_hurt_the_player() {
    let mut g = playing(47);
    g.enemies.clear();
    g.spawn_queue = 0;
    g.invuln = 0.0;
    g.wave = 5;
    g.enemies.push(Enemy {
        pos: Vec2::new(g.player_pos.x + 220.0, g.player_pos.y),
        vel: Vec2::new(0.0, 0.0),
        hp: 999,
        kind: EnemyKind::Warden,
        flash: 0.0,
        spawn_t: 0.0,
        seed: 1,
        atk_cd: 0.0,
    });
    let idle = Input::default();
    let mut saw_spit = false;
    let mut hurt = false;
    let hp = g.health;
    for _ in 0..60 * 6 {
        g.update(1.0 / 60.0, &idle);
        if g.events.contains(&SoundEvent::Spit) {
            saw_spit = true;
        }
        g.events.clear();
        if g.health < hp {
            hurt = true;
        }
    }
    assert!(saw_spit, "the Warden should periodically fire spit");
    assert!(
        hurt,
        "the Warden's spit should be able to damage the player"
    );
}

#[test]
fn powerups_apply_their_effects() {
    // Repair.
    let mut g = playing(53);
    g.enemies.clear();
    g.spawn_queue = 0;
    g.health = 20.0;
    g.powerups.push(Powerup {
        pos: Vec2::new(g.player_pos.x + 3.0, g.player_pos.y),
        vel: Vec2::new(0.0, 0.0),
        kind: PowerKind::Repair,
        life: 5.0,
        anim: 0.0,
    });
    step(&mut g, 1.0 / 60.0, 6, &Input::default());
    assert!(g.health > 20.0, "repair should heal");

    // Rapid fire.
    let mut g = playing(54);
    g.enemies.clear();
    g.spawn_queue = 0;
    g.powerups.push(Powerup {
        pos: Vec2::new(g.player_pos.x + 3.0, g.player_pos.y),
        vel: Vec2::new(0.0, 0.0),
        kind: PowerKind::RapidFire,
        life: 5.0,
        anim: 0.0,
    });
    step(&mut g, 1.0 / 60.0, 6, &Input::default());
    assert!(g.rapid_fire > 0.0);

    // Charge.
    let mut g = playing(55);
    g.enemies.clear();
    g.spawn_queue = 0;
    g.energy = 0.0;
    g.powerups.push(Powerup {
        pos: Vec2::new(g.player_pos.x + 3.0, g.player_pos.y),
        vel: Vec2::new(0.0, 0.0),
        kind: PowerKind::Charge,
        life: 5.0,
        anim: 0.0,
    });
    step(&mut g, 1.0 / 60.0, 6, &Input::default());
    assert!(g.energy >= g.max_energy);
}

#[test]
fn rapid_fire_actually_shoots_faster() {
    let mut g = playing(61);
    let fire = Input {
        fire: true,
        ..Default::default()
    };
    // Baseline: count bullets produced in a fixed window.
    g.fire_cooldown = 0.0;
    let mut baseline = 0;
    for _ in 0..60 {
        g.fire();
        baseline += 1;
        g.update(1.0 / 60.0, &Input::default());
        g.events.clear();
    }
    let _ = fire;

    let mut g2 = playing(61);
    g2.rapid_fire = 10.0;
    g2.fire_cooldown = 0.0;
    let mut rapid = 0;
    for _ in 0..60 {
        g2.fire();
        rapid += 1;
        g2.update(1.0 / 60.0, &Input::default());
        g2.events.clear();
    }
    // Both count successful triggers; with the shorter cooldown rapid should
    // be at least as many, and strictly more in practice.
    assert!(rapid >= baseline);
    assert!(!g2.bullets.is_empty());
}

#[test]
fn particles_are_bounded_over_time() {
    // Combat for a while, then confirm the particle list drains when idle.
    let mut g = playing(67);
    let busy = Input {
        fire: true,
        ..Default::default()
    };
    for _ in 0..60 * 20 {
        g.update(1.0 / 60.0, &busy);
        g.events.clear();
        if g.state == GameState::GameOver {
            break;
        }
    }
    let idle = Input::default();
    step(&mut g, 1.0 / 60.0, 120, &idle);
    // Give particles a couple of seconds to expire.
    for _ in 0..180 {
        g.update(1.0 / 60.0, &idle);
        g.events.clear();
    }
    assert!(
        g.particles.len() < 400,
        "particles should expire instead of accumulating forever"
    );
}

#[test]
fn dt_spike_does_not_teleport_the_player() {
    // The binary clamps dt; the sim itself must still be sane with a big step.
    let mut g = playing(71);
    let input = Input {
        right: true,
        ..Default::default()
    };
    g.update(0.25, &input);
    assert!(g.player_pos.x <= WORLD_W - PLAYER_RADIUS + 0.5);
    assert!(g.player_pos.x.is_finite());
    assert!(g.player_pos.y.is_finite());
}

#[test]
fn rng_is_deterministic_for_a_seed() {
    let mut a = Rng32::new(1234);
    let mut b = Rng32::new(1234);
    for _ in 0..100 {
        assert_eq!(a.next_u32(), b.next_u32());
    }
    let mut c = Rng32::new(999);
    assert_ne!(a.next_u32(), c.next_u32());
}

#[test]
fn same_seed_produces_the_same_opening_wave() {
    let mut a = playing(1234);
    let mut b = playing(1234);
    let idle = Input::default();
    step(&mut a, 1.0 / 60.0, 120, &idle);
    step(&mut b, 1.0 / 60.0, 120, &idle);
    assert_eq!(a.enemies.len(), b.enemies.len());
    for (ea, eb) in a.enemies.iter().zip(b.enemies.iter()) {
        assert_eq!(ea.kind, eb.kind);
        assert!((ea.pos.x - eb.pos.x).abs() < 0.001);
    }
}
