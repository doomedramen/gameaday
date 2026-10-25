//! Headless self-test and screenshot mode. The autopilot (the same driver as
//! the rivals) plays generated circuits and whole Cups through the real
//! `App`, and the run asserts on pacing, difficulty, reachability and stuck
//! states.

use crate::app::{App, Input, Key, Screen, classic_spec};
use crate::campaign::{self, ROUNDS, Verdict};
use crate::canvas::{Canvas, H, W};
use crate::car::Controls;
use crate::race::{Mode, Phase, ROSTER, Race};
use crate::track::{BIOMES, Track, TrackSpec};
use std::fmt::Write as _;
use std::path::Path;

const DT: f32 = 1.0 / 60.0;

fn press(app: &mut App, k: Key) {
    let input = Input {
        down: vec![k],
        pressed: vec![k],
    };
    app.update(DT, &input);
    app.take_sounds();
}

fn idle(app: &mut App, frames: usize) {
    for _ in 0..frames {
        app.update(DT, &Input::default());
        app.take_sounds();
    }
}

/// Drive the current race to its end with the autopilot. Returns simulated
/// seconds.
fn autopilot_race(app: &mut App) -> Result<f32, String> {
    let mut t = 0.0;
    if let Some(r) = &mut app.race {
        r.autopilot = true;
    }
    let mut last_prog: Vec<f32> = Vec::new();
    let mut window = 0.0;
    while app.screen == Screen::Racing {
        app.update(DT, &Input::default());
        app.take_sounds();
        t += DT;
        window += DT;
        if let Some(r) = &app.race {
            if window >= 10.0 && r.phase == Phase::Running {
                let prog: Vec<f32> = r.racers.iter().map(|x| x.progress).collect();
                if !last_prog.is_empty() {
                    for (i, (a, b)) in last_prog.iter().zip(prog.iter()).enumerate() {
                        if r.racers[i].finish.is_none() && b - a < 300.0 {
                            return Err(format!(
                                "{} stuck: {:.0}px in 10s on {:?}",
                                r.racers[i].driver.name,
                                b - a,
                                r.track.spec
                            ));
                        }
                    }
                }
                last_prog = prog;
                window = 0.0;
            }
        }
        if t > 400.0 {
            return Err("race never ended".into());
        }
    }
    Ok(t)
}

pub struct CupLog {
    pub verdicts: Vec<Verdict>,
    pub rival_speed: Vec<f32>,
    pub seconds: f32,
    pub parts: Vec<campaign::Part>,
    pub venues: Vec<String>,
    pub champion: bool,
}

/// Play a whole Cup through the app. `sandbag` makes the player idle at the
/// start of every race (to prove elimination is reachable).
fn play_cup(app: &mut App, sandbag: bool) -> Result<CupLog, String> {
    let mut log = CupLog {
        verdicts: vec![],
        rival_speed: vec![],
        seconds: 0.0,
        parts: vec![],
        venues: vec![],
        champion: false,
    };
    if app.screen != Screen::Title {
        return Err(format!("expected title, got {:?}", app.screen));
    }
    app.sel = 0;
    press(app, Key::Enter);
    if app.screen != Screen::Intro {
        return Err(format!("expected intro, got {:?}", app.screen));
    }
    press(app, Key::Enter);
    for _guard in 0..20 {
        if app.screen != Screen::PreRace {
            return Err(format!("expected pre-race, got {:?}", app.screen));
        }
        if let Some(c) = &app.campaign {
            log.venues
                .push(format!("{}:{}", c.venue.spec.biome.name(), c.venue.label));
        }
        press(app, Key::Enter);
        if sandbag {
            // sit on the grid for 25s while the field leaves
            for _ in 0..(25.0 / DT) as usize {
                app.update(DT, &Input::default());
                app.take_sounds();
            }
            log.seconds += 25.0;
        }
        log.seconds += autopilot_race(app)?;
        // rival pace this round (px/s over their whole race)
        if let Some(r) = &app.race {
            let mut sp = 0.0f32;
            let mut n = 0.0f32;
            for x in &r.racers {
                if !x.is_player() && !x.lap_times.is_empty() {
                    sp +=
                        r.track.len / (x.lap_times.iter().sum::<f32>() / x.lap_times.len() as f32);
                    n += 1.0;
                }
            }
            log.rival_speed.push(sp / n.max(1.0));
        }
        if app.screen != Screen::Results {
            return Err(format!("expected results, got {:?}", app.screen));
        }
        idle(app, 30);
        let v = app
            .last_result
            .as_ref()
            .map(|r| r.verdict)
            .ok_or("no result")?;
        log.verdicts.push(v);
        press(app, Key::Enter);
        match v {
            Verdict::Champion => {
                log.champion = true;
                if app.screen != Screen::Victory {
                    return Err("no victory screen".into());
                }
                idle(app, 70);
                press(app, Key::Enter);
                return Ok(log);
            }
            Verdict::Eliminated => {
                if app.screen != Screen::GameOver {
                    return Err("no game over screen".into());
                }
                idle(app, 70);
                press(app, Key::Enter);
                return Ok(log);
            }
            Verdict::SecondChance => continue,
            Verdict::Qualified => {
                if app.screen != Screen::Standings {
                    return Err("no standings".into());
                }
                idle(app, 20);
                press(app, Key::Enter);
                if app.screen != Screen::Draft {
                    return Err("no draft".into());
                }
                idle(app, 40);
                press(app, Key::Right);
                press(app, Key::Enter);
                idle(app, 40);
                if app.screen != Screen::Garage {
                    return Err(format!("no garage, got {:?}", app.screen));
                }
                if let Some(c) = &app.campaign {
                    log.parts = c.parts.clone();
                }
                // buy what we can: cheapest first, undo once to prove it works
                for slot in [0usize, 1, 3, 2, 0, 1] {
                    app.sel = slot;
                    press(app, Key::Enter);
                }
                let cash_before = app.campaign.as_ref().map(|c| c.cash).unwrap_or(0);
                let bought = app
                    .campaign
                    .as_ref()
                    .map(|c| c.purchases.len())
                    .unwrap_or(0);
                press(app, Key::Back);
                if bought > 0 {
                    let after = app.campaign.as_ref().map(|c| c.cash).unwrap_or(0);
                    if after <= cash_before {
                        return Err("undo did not refund".into());
                    }
                    app.sel = 0;
                    press(app, Key::Enter);
                }
                app.sel = 5;
                press(app, Key::Enter);
                if app.screen == Screen::Route {
                    press(app, Key::Right);
                    press(app, Key::Enter);
                }
            }
        }
    }
    Err("cup did not finish".into())
}

pub fn run() -> Result<String, String> {
    let mut rep = String::new();
    let t0 = std::time::Instant::now();

    // 1) Generator oracle: the autopilot laps every generated circuit.
    let mut laps = 0;
    for seed in 0..12u64 {
        for (k, b) in BIOMES.iter().enumerate() {
            let spec = TrackSpec {
                seed: seed * 31 + k as u64,
                biome: *b,
                twisty: (seed % 5) as f32 * 0.17,
                hazard: 0.3 + (seed % 3) as f32 * 0.3,
                length: 13000.0 + (seed % 4) as f32 * 500.0,
            };
            let track = Track::generate(spec);
            if track.attempts == 0 {
                return Err(format!("generator fell back to an oval for {spec:?}"));
            }
            let mut race = Race::new(
                track,
                Mode::TimeTrial,
                1,
                campaign::build_stats(&Default::default(), &[]),
                &[],
                1.0,
                seed,
            );
            race.autopilot = true;
            race.countdown = 0.0;
            let mut t = 0.0;
            while race.phase != Phase::Done && race.phase != Phase::Cooldown {
                race.update(DT, Controls::default());
                race.sounds.clear();
                race.skids.clear();
                t += DT;
                if t > 60.0 {
                    return Err(format!("autopilot could not lap {spec:?}"));
                }
            }
            if race.player_wall_hits > 6 {
                return Err(format!(
                    "autopilot hit the wall {} times on {spec:?}",
                    race.player_wall_hits
                ));
            }
            laps += 1;
        }
    }
    writeln!(rep, "generator: autopilot lapped {laps} generated circuits").ok();

    // 2) Full Cups through the real app.
    let save_path =
        std::env::temp_dir().join(format!("slipstream-selftest-{}.txt", std::process::id()));
    let _ = std::fs::remove_file(&save_path);
    let mut app = App::new(Some(save_path.clone()), 42);
    let mut session = 0.0;
    let mut champion = false;
    let mut seen_parts: Vec<campaign::Part> = Vec::new();
    let mut seen_venues: Vec<String> = Vec::new();
    let mut new_after_15 = 0;
    let mut speeds: Vec<Vec<f32>> = Vec::new();
    for cup in 0..8 {
        let log = play_cup(&mut app, false)?;
        for p in &log.parts {
            if !seen_parts.contains(p) {
                seen_parts.push(*p);
                if session > 15.0 * 60.0 {
                    new_after_15 += 1;
                }
            }
        }
        for v in &log.venues {
            if !seen_venues.contains(v) {
                seen_venues.push(v.clone());
                if session > 15.0 * 60.0 {
                    new_after_15 += 1;
                }
            }
        }
        session += log.seconds;
        champion |= log.champion;
        writeln!(
            rep,
            "cup {cup}: {:?} ({:.0}s racing)",
            log.verdicts, log.seconds
        )
        .ok();
        if log
            .verdicts
            .iter()
            .filter(|v| **v != Verdict::SecondChance)
            .count()
            == ROUNDS
        {
            speeds.push(log.rival_speed.clone());
        }
        if session > 22.0 * 60.0 && champion && cup >= 2 {
            break;
        }
    }
    if !champion {
        return Err("no Cup won by the autopilot: victory not reachable".into());
    }
    writeln!(rep, "victory reachable").ok();
    if new_after_15 == 0 {
        return Err("nothing new appeared after 15 simulated minutes".into());
    }
    writeln!(
        rep,
        "new venues/parts after 15 min: {new_after_15} (session {:.1} min)",
        session / 60.0
    )
    .ok();
    // difficulty: rivals are measurably faster in later rounds
    if let Some(s) = speeds.first() {
        let early = s[0];
        let late = s[s.len() - 1];
        if late < early * 1.04 {
            return Err(format!(
                "rival pace did not rise: {early:.0} -> {late:.0} px/s"
            ));
        }
        writeln!(
            rep,
            "difficulty rises: rival pace {early:.0} -> {late:.0} px/s"
        )
        .ok();
    }

    // 3) Elimination is reachable.
    let log = play_cup(&mut app, true)?;
    if log.verdicts.last() != Some(&Verdict::Eliminated) {
        return Err(format!(
            "sandbagging should be eliminated, got {:?}",
            log.verdicts
        ));
    }
    if log
        .verdicts
        .iter()
        .filter(|v| **v == Verdict::SecondChance)
        .count()
        != 2
    {
        return Err("second chances not spent before elimination".into());
    }
    writeln!(rep, "defeat reachable after 2 second chances").ok();

    // 4) Time trial: ghost recorded, persisted, raced; restart works.
    app.sel = 1;
    press(&mut app, Key::Enter);
    if app.screen != Screen::TtSelect {
        return Err("no time trial select".into());
    }
    if app.save.records.iter().any(|r| r.par.is_none()) {
        return Err("medal pars not computed".into());
    }
    press(&mut app, Key::Enter);
    autopilot_race(&mut app)?;
    if app.screen != Screen::TtResults || !app.tt_new_record {
        return Err("time trial did not record".into());
    }
    if app.save.records[0].ghost.len() < 100 {
        return Err("ghost too short".into());
    }
    press(&mut app, Key::Restart);
    if app.screen != Screen::Racing || app.race.as_ref().map(|r| r.ghost.is_some()) != Some(true) {
        return Err("restart did not start a ghost race".into());
    }
    idle(&mut app, 300);
    press(&mut app, Key::Restart);
    if app.race.as_ref().map(|r| r.phase) != Some(Phase::Countdown) {
        return Err("R did not restart".into());
    }
    writeln!(
        rep,
        "time trial: ghost {} frames, medal {}, restart works",
        app.save.records[0].ghost.len(),
        app.tt_medal
    )
    .ok();
    let reloaded = crate::save::Save::load(&save_path);
    if reloaded.records[0].ghost.len() != app.save.records[0].ghost.len() || reloaded.cups_won == 0
    {
        return Err("save file did not persist ghost/cups".into());
    }
    writeln!(
        rep,
        "save file persists ghost, records and cups won ({})",
        reloaded.cups_won
    )
    .ok();
    let _ = std::fs::remove_file(&save_path);

    // 5) Pause menu and quit back to title.
    press(&mut app, Key::Back);
    if !app.paused {
        return Err("pause did not open".into());
    }
    app.pause_sel = 3;
    press(&mut app, Key::Enter);
    if app.screen != Screen::Title {
        return Err("quit to title failed".into());
    }
    writeln!(rep, "app driven by synthetic input: pause, quit, menus").ok();
    writeln!(rep, "selftest took {:.1}s", t0.elapsed().as_secs_f32()).ok();
    Ok(rep)
}

// ---- screenshots -----------------------------------------------------------

fn snap(
    app: &mut App,
    cv: &mut Canvas,
    dir: &Path,
    name: &str,
    n: &mut usize,
) -> Result<(), String> {
    app.draw(cv);
    crate::pngout::write_rgb(&dir.join(format!("{:02}_{name}.png", *n + 1)), W, H, &cv.px)?;
    *n += 1;
    Ok(())
}

/// Advance a race with the autopilot until `pred` holds (or a time limit).
fn run_until(app: &mut App, secs: f32, pred: impl Fn(&Race) -> bool) {
    if let Some(r) = &mut app.race {
        r.autopilot = true;
    }
    let mut t = 0.0;
    while t < secs {
        app.update(DT, &Input::default());
        app.take_sounds();
        t += DT;
        if app.screen != Screen::Racing {
            break;
        }
        if let Some(r) = &app.race {
            if pred(r) {
                break;
            }
        }
    }
}

pub fn screenshots(dir: &Path) -> Result<usize, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut cv = Canvas::new(W, H);
    let mut n = 0;
    let mut app = App::new(None, 2026);
    idle(&mut app, 90);
    snap(&mut app, &mut cv, dir, "title", &mut n)?;

    app.sel = 0;
    press(&mut app, Key::Enter);
    idle(&mut app, 30);
    snap(&mut app, &mut cv, dir, "the_field", &mut n)?;
    press(&mut app, Key::Enter);
    snap(&mut app, &mut cv, dir, "round_one", &mut n)?;
    press(&mut app, Key::Enter);
    idle(&mut app, 100);
    snap(&mut app, &mut cv, dir, "countdown", &mut n)?;
    // a rocket start: throttle in the last moment before green
    for _ in 0..400 {
        let cd = app.race.as_ref().map(|r| r.countdown).unwrap_or(0.0);
        let mut inp = Input::default();
        if cd <= 0.25 {
            inp.down.push(Key::Up);
        }
        app.update(DT, &inp);
        app.take_sounds();
        if app.race.as_ref().is_some_and(|r| r.t > 0.5) {
            break;
        }
    }
    snap(&mut app, &mut cv, dir, "rocket_start", &mut n)?;
    // busiest moment: the pack into the first bends
    run_until(&mut app, 30.0, |r| {
        let p = r.player().car.pos;
        r.racers
            .iter()
            .filter(|x| x.car.pos.dist(p) < 300.0)
            .count()
            >= 4
            && r.t > 4.0
    });
    snap(&mut app, &mut cv, dir, "busiest_pack", &mut n)?;
    // a held drift through the next tight bend (line-following steering)
    run_until(&mut app, 60.0, |r| {
        r.next_corner()
            .is_some_and(|(d, c)| c.severity >= 2 && d < 40.0)
            && r.t > 8.0
    });
    for _ in 0..150 {
        let (Some(race), Some(view)) = (&mut app.race, &mut app.view) else {
            break;
        };
        let pi = race.player_idx();
        let dir = race.next_corner().map(|c| c.1.dir).unwrap_or(1.0);
        let mut c = crate::ai::drive(race, pi, DT);
        c.throttle = 1.0;
        c.drift = true;
        if !race.player().car.drifting {
            c.steer = dir;
        }
        race.autopilot = false;
        race.update(DT, c);
        race.sounds.clear();
        view.update(race, DT);
        if race.player().car.drift_tier() >= 2 {
            break;
        }
    }
    snap(&mut app, &mut cv, dir, "drift_sparks", &mut n)?;
    run_until(&mut app, 200.0, |r| r.phase == Phase::Cooldown);
    idle(&mut app, 50);
    snap(&mut app, &mut cv, dir, "finish_line", &mut n)?;
    run_until(&mut app, 30.0, |_| false);
    while app.screen == Screen::Racing {
        app.update(DT, &Input::default());
        app.take_sounds();
    }
    idle(&mut app, 60);
    snap(&mut app, &mut cv, dir, "results", &mut n)?;
    if app.last_result.as_ref().map(|r| r.verdict) == Some(Verdict::Qualified) {
        press(&mut app, Key::Enter);
        idle(&mut app, 60);
        snap(&mut app, &mut cv, dir, "standings", &mut n)?;
        press(&mut app, Key::Enter);
        idle(&mut app, 50);
        press(&mut app, Key::Right);
        snap(&mut app, &mut cv, dir, "parts_draft", &mut n)?;
        press(&mut app, Key::Enter);
        idle(&mut app, 40);
        app.sel = 0;
        press(&mut app, Key::Enter);
        press(&mut app, Key::Down);
        snap(&mut app, &mut cv, dir, "garage", &mut n)?;
        app.sel = 5;
        press(&mut app, Key::Enter);
        idle(&mut app, 30);
        snap(&mut app, &mut cv, dir, "route_choice", &mut n)?;
    }

    // a tour of the venues mid-race, with the field
    for (k, biome) in BIOMES.iter().enumerate().skip(1) {
        let mut c = campaign::Campaign::new(77 + k as u64);
        c.round = if k == 4 { ROUNDS - 1 } else { k };
        c.venue = campaign::Venue {
            spec: TrackSpec {
                seed: 900 + k as u64,
                biome: *biome,
                twisty: 0.4,
                hazard: 0.8,
                length: 13500.0,
            },
            laps: 3,
            label: "Tour",
            prize_mult: 1.0,
        };
        app.campaign = Some(c);
        app.screen = Screen::Route;
        app.sel = 0;
        let opts = app.campaign.as_ref().map(|c| c.options.len()).unwrap_or(0);
        if opts == 0 {
            // jump straight to the pre-race screen for this venue
            app.campaign.as_mut().unwrap().options = vec![app.campaign.as_ref().unwrap().venue];
        }
        press(&mut app, Key::Enter);
        if k == 4 {
            snap(&mut app, &mut cv, dir, "grand_prix_card", &mut n)?;
        }
        press(&mut app, Key::Enter);
        let at = [0.0, 24.0, 30.0, 38.0, 1.5][k];
        run_until(&mut app, at, |_| false);
        // prefer a frame with rivals in view (the Champion at the Grand Prix)
        run_until(&mut app, 25.0, |r| {
            let p = r.player().car.pos;
            let near = r
                .racers
                .iter()
                .filter(|x| !x.is_player() && x.car.pos.dist(p) < 320.0);
            if r.track.spec.biome == crate::track::Biome::GrandPrix {
                r.racers.iter().any(|x| {
                    x.driver.kind == crate::race::Kind::Champion && x.car.pos.dist(p) < 260.0
                })
            } else {
                near.count() >= 2
            }
        });
        let name = [
            "",
            "dune_sea",
            "timberline",
            "neon_night",
            "grand_prix_grid",
        ][k];
        snap(&mut app, &mut cv, dir, name, &mut n)?;
        if k == 3 {
            app.paused = true;
            snap(&mut app, &mut cv, dir, "pause_menu", &mut n)?;
            app.paused = false;
        }
        app.race = None;
        app.view = None;
    }

    // victory screen
    {
        let mut c = campaign::Campaign::new(5);
        c.round = ROUNDS - 1;
        let mut order = vec![crate::race::PLAYER];
        order.extend(ROSTER.iter().copied());
        order.push(crate::race::CHAMPION);
        c.style_total = 5400;
        c.finish_round(&order, 2400);
        app.campaign = Some(c);
        app.screen = Screen::Victory;
        app.screen_t = 0.0;
        idle(&mut app, 100);
        snap(&mut app, &mut cv, dir, "victory", &mut n)?;
    }
    // defeat screen
    {
        let mut c = campaign::Campaign::new(6);
        c.round = 2;
        c.chances = 0;
        let mut order: Vec<_> = ROSTER.to_vec();
        order.push(crate::race::PLAYER);
        c.finish_round(&order, 300);
        app.campaign = Some(c);
        app.screen = Screen::GameOver;
        app.screen_t = 0.0;
        idle(&mut app, 30);
        snap(&mut app, &mut cv, dir, "cup_over", &mut n)?;
    }
    // time trial with a ghost
    app.campaign = None;
    app.screen = Screen::Title;
    app.sel = 1;
    press(&mut app, Key::Enter);
    app.sel = 3;
    snap(&mut app, &mut cv, dir, "time_trial_select", &mut n)?;
    app.sel = 0;
    press(&mut app, Key::Enter);
    run_until(&mut app, 200.0, |r| r.phase == Phase::Done);
    while app.screen == Screen::Racing {
        app.update(DT, &Input::default());
        app.take_sounds();
    }
    press(&mut app, Key::Restart);
    run_until(&mut app, 8.5, |_| false);
    if let Some(r) = &mut app.race {
        // fall a little behind the ghost so both are visible
        r.autopilot = false;
    }
    for _ in 0..40 {
        app.update(
            DT,
            &Input {
                down: vec![Key::Up],
                pressed: vec![],
            },
        );
        app.take_sounds();
    }
    let _ = classic_spec(0);
    snap(&mut app, &mut cv, dir, "time_trial_ghost", &mut n)?;
    app.race = None;
    app.view = None;
    app.screen = Screen::Help;
    app.help_page = 0;
    snap(&mut app, &mut cv, dir, "how_to_play", &mut n)?;
    app.screen = Screen::Settings;
    snap(&mut app, &mut cv, dir, "settings", &mut n)?;
    Ok(n)
}
