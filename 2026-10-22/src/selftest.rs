//! Headless end-to-end checks and screenshot capture. Used by the shipped
//! binary (`--selftest`, `--screenshot <dir>`) and the integration tests.

use std::path::{Path, PathBuf};

use crate::app::{App, Input, Key, Screen};
use crate::battle::Phase;
use crate::bot::{self, Style};
use crate::gfx::canvas::{Canvas, H, W};
use crate::gfx::ui;
use crate::run::{ISLANDS, Node, Stage};
use crate::units::{Kind, Team};

const DT: f32 = 1.0 / 30.0;

fn temp_save(tag: &str) -> PathBuf {
    std::env::temp_dir()
        .join(format!("lanternfall-{tag}-{}", std::process::id()))
        .join("save.txt")
}

fn press(app: &mut App, k: Key) {
    app.update(
        DT,
        &Input {
            mouse: app.mouse,
            keys: vec![k],
            ..Input::default()
        },
    );
}

fn click(app: &mut App, at: (f32, f32)) {
    // Hover first so hit-testing sees the cursor, then click.
    app.update(
        DT,
        &Input {
            mouse: at,
            ..Input::default()
        },
    );
    app.update(
        DT,
        &Input {
            mouse: at,
            click: true,
            ..Input::default()
        },
    );
}

fn idle(app: &mut App, secs: f32) {
    let frames = (secs / DT).ceil() as usize;
    for _ in 0..frames {
        app.update(
            DT,
            &Input {
                mouse: app.mouse,
                ..Input::default()
            },
        );
    }
}

fn wait_player(app: &mut App, max_secs: f32) -> bool {
    let frames = (max_secs / DT) as usize;
    for _ in 0..frames {
        if app.player_can_act() {
            return true;
        }
        if app.screen != Screen::Game {
            return false;
        }
        if let Some(r) = app.run.as_ref()
            && r.stage != Stage::Battle
        {
            return false;
        }
        idle(app, DT);
    }
    false
}

fn distinct_colors(c: &Canvas) -> usize {
    let mut seen = std::collections::HashSet::new();
    for p in c.px.iter().step_by(37) {
        seen.insert(*p);
        if seen.len() > 400 {
            break;
        }
    }
    seen.len()
}

/// Play the real app with synthetic input until the run ends.
/// Returns (frames, won, islands reached).
fn drive_app(
    app: &mut App,
    god: bool,
    max_frames: usize,
    canvas: &mut Canvas,
) -> Result<(usize, bool, u32), String> {
    let mut frames = 0;
    let mut did_manual = false;
    while frames < max_frames {
        frames += 1;
        if frames % 20 == 0 {
            app.draw(canvas);
        }
        match app.screen {
            Screen::End => {
                let run = app.run.as_ref().ok_or("no run at end")?;
                return Ok((frames, run.won(), run.island));
            }
            Screen::Title => return Err("fell back to title unexpectedly".into()),
            Screen::Game => {}
        }
        if god && let Some(run) = app.run.as_mut() {
            run.god_mode = true;
            if let Some(b) = run.battle.as_mut() {
                b.cfg.god_mode = true;
            }
        }
        let stage = app.run.as_ref().map(|r| r.stage.clone()).ok_or("no run")?;
        match stage {
            Stage::Draft { options, .. } => {
                // Alternate between key and mouse selection.
                if frames % 2 == 0 {
                    press(app, Key::Num1);
                } else {
                    let r = ui::draft_rect(options.len() - 1, options.len());
                    click(app, r.center());
                }
                continue;
            }
            Stage::Route { options } => {
                let r = ui::route_rect(0, options.len());
                click(app, r.center());
                continue;
            }
            _ => {}
        }
        if app.player_can_act() {
            if !did_manual {
                manual_turn(app)?;
                did_manual = true;
            } else {
                app.autopilot_step();
            }
        } else {
            idle(app, DT);
        }
    }
    Err(format!("run did not finish within {max_frames} frames"))
}

/// One turn driven entirely through clicks and keys, checking the UI.
fn manual_turn(app: &mut App) -> Result<(), String> {
    press(app, Key::Tab);
    let id = app.selected.ok_or("TAB did not select a unit")?;
    let b = app.battle().ok_or("no battle")?;
    let moves = b.move_targets(id);
    let dest = *moves.first().ok_or("selected unit cannot move")?;
    let from = b.unit(id).pos;
    let at = app.tile_screen(dest).ok_or("tile not on screen")?;
    click(app, at);
    let b = app.battle().ok_or("no battle")?;
    if b.unit(id).pos != dest {
        return Err(format!("click move failed: {:?} -> {:?}", from, dest));
    }
    // Undo it (once the walk finishes), then move again.
    idle(app, 1.0);
    press(app, Key::U);
    if app.battle().ok_or("no battle")?.unit(id).pos != from {
        return Err("undo did not restore position".into());
    }
    let at = app.tile_screen(dest).ok_or("tile not on screen")?;
    app.selected = Some(id);
    click(app, at);
    idle(app, 1.0);
    // Help overlay opens and closes.
    press(app, Key::H);
    if !app.help {
        return Err("help did not open".into());
    }
    press(app, Key::H);
    if app.help {
        return Err("help did not close".into());
    }
    // End turn via the button.
    click(app, ui::END_TURN.center());
    let b = app.battle().ok_or("no battle")?;
    if b.phase == Phase::Player && b.outcome.is_none() {
        return Err("End Turn button did not end the turn".into());
    }
    Ok(())
}

pub fn run() -> Result<String, String> {
    let mut report = String::new();

    // 1. Simulation playtests across seeds.
    let mut per_island: Vec<Vec<i32>> = vec![vec![]; ISLANDS as usize + 1];
    let mut late_kinds = std::collections::HashSet::new();
    let mut early_kinds = std::collections::HashSet::new();
    let mut furthest = 0;
    let mut wins = 0;
    for seed in 0..16u64 {
        let r = bot::play_run(seed, Style::Smart, false, 40_000);
        if r.stuck {
            return Err(format!("seed {seed}: autopilot run got stuck"));
        }
        if !(r.won || r.lost) {
            return Err(format!("seed {seed}: run neither won nor lost"));
        }
        furthest = furthest.max(r.final_island);
        if r.won {
            wins += 1;
        }
        for i in &r.islands {
            per_island[i.island as usize].push(i.enemy_hp);
            if i.rounds > 40 {
                return Err(format!(
                    "seed {seed}: island {} took {} rounds",
                    i.island, i.rounds
                ));
            }
            for k in &i.kinds {
                if i.island <= 1 {
                    early_kinds.insert(*k);
                } else if i.island >= 3 {
                    late_kinds.insert(*k);
                }
            }
        }
    }
    if wins == 0 {
        return Err("the autopilot never wins unaided - too hard".into());
    }
    if wins > 12 {
        return Err(format!("the autopilot wins {wins}/16 - too easy"));
    }
    let mean = |v: &Vec<i32>| v.iter().sum::<i32>() as f32 / v.len().max(1) as f32;
    let early = mean(&per_island[1]);
    let mid = mean(&per_island[4]);
    if mid <= early * 1.5 {
        return Err(format!(
            "difficulty does not ramp: island1 hp {early}, island4 hp {mid}"
        ));
    }
    let new_kinds: Vec<Kind> = late_kinds.difference(&early_kinds).copied().collect();
    if new_kinds.len() < 3 {
        return Err(format!(
            "too few new enemy kinds after island 2: {new_kinds:?}"
        ));
    }
    report += &format!(
        "playtest: 16 autopilot runs, {wins} unaided wins, furthest island {furthest}, enemy hp island1 {early:.1} -> island4 {mid:.1}, new kinds {new_kinds:?}\n"
    );

    // 2. Victory and defeat are both reachable.
    let god = bot::play_run(3, Style::Smart, true, 40_000);
    if !god.won || god.islands.iter().filter(|i| i.boss).count() != 1 {
        return Err("god-mode run did not defeat the Hollow King".into());
    }
    let passive = bot::play_run(3, Style::Passive, false, 40_000);
    if !passive.lost {
        return Err("passive run did not lose".into());
    }
    report += &format!(
        "victory reachable (god run score {}), defeat reachable (passive lost on island {})\n",
        god.score, passive.final_island
    );

    // 3. The real app, driven with synthetic input, to victory.
    let save_path = temp_save("selftest");
    let _ = std::fs::remove_file(&save_path);
    let mut app = App::new(Some(save_path.clone()), 12345);
    let mut canvas = Canvas::new(W, H);
    idle(&mut app, 0.5);
    app.draw(&mut canvas);
    if distinct_colors(&canvas) < 50 {
        return Err("title screen looks blank".into());
    }
    click(&mut app, ui::title_rect(0).center());
    if app.screen != Screen::Game {
        return Err("New Run button did not start a run".into());
    }
    if app.tutorial != Some(0) {
        return Err("first run should start the tutorial".into());
    }
    if !wait_player(&mut app, 10.0) {
        return Err("never reached the player's turn".into());
    }
    let (frames, won, island) = drive_app(&mut app, true, 200_000, &mut canvas)?;
    if !won || island != ISLANDS {
        return Err(format!(
            "god-mode app run ended without victory (island {island})"
        ));
    }
    app.draw(&mut canvas);
    if distinct_colors(&canvas) < 50 {
        return Err("end screen looks blank".into());
    }
    report += &format!(
        "app e2e: victory in {frames} frames ({:.0} simulated s)\n",
        frames as f32 * DT
    );
    if !app.save.tutorial_done {
        return Err("tutorial was not completed".into());
    }

    // 4. Restart from the end screen, then lose a real run.
    press(&mut app, Key::Enter);
    if app.screen != Screen::Title {
        return Err("Enter on the end screen did not return to title".into());
    }
    press(&mut app, Key::Enter);
    if app.screen != Screen::Game || app.run.as_ref().is_none_or(|r| r.island != 1) {
        return Err("restart did not begin a fresh run".into());
    }
    if app.tutorial.is_some() {
        return Err("tutorial repeated after completion".into());
    }
    let (frames2, won2, island2) = drive_app(&mut app, false, 400_000, &mut canvas)?;
    report += &format!(
        "app e2e: second run {} on island {island2} after {frames2} frames\n",
        if won2 { "won" } else { "lost" }
    );

    // 5. Persistence.
    let reloaded = crate::save::Save::load(&save_path);
    if reloaded.runs != 2
        || reloaded.wins < 1
        || !reloaded.tutorial_done
        || reloaded.best_score <= 0
    {
        return Err(format!("save file not persisted correctly: {reloaded:?}"));
    }
    report += &format!("save: {reloaded:?}\n");
    let _ = std::fs::remove_dir_all(save_path.parent().unwrap_or(Path::new(".")));
    Ok(report)
}

// ------------------------------------------------------------- screenshots

fn shot(
    app: &mut App,
    canvas: &mut Canvas,
    dir: &Path,
    name: &str,
    count: &mut usize,
) -> Result<(), String> {
    app.draw(canvas);
    canvas
        .save_png(&dir.join(format!("{name}.png")))
        .map_err(|e| e.to_string())?;
    *count += 1;
    Ok(())
}

fn new_app(tag: &str, seed: u64) -> App {
    let p = temp_save(tag);
    let _ = std::fs::remove_file(&p);
    let mut app = App::new(Some(p), seed);
    app.save.tutorial_done = true;
    app
}

/// Advance the battle with autopilot turns until `cond` holds.
fn play_until(app: &mut App, max_frames: usize, mut cond: impl FnMut(&App) -> bool) -> bool {
    for _ in 0..max_frames {
        if cond(app) {
            return true;
        }
        if app.player_can_act() {
            app.autopilot_step();
        } else {
            idle(app, DT);
        }
        if app.run.as_ref().is_none_or(|r| r.stage != Stage::Battle) {
            return cond(app);
        }
    }
    false
}

pub fn screenshots(dir: &Path) -> Result<usize, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut n = 0;
    let mut canvas = Canvas::new(W, H);

    // Title
    let mut app = new_app("shots", 2026);
    app.mouse = (640.0, 300.0);
    idle(&mut app, 1.2);
    shot(&mut app, &mut canvas, dir, "01_title", &mut n)?;

    // Battle start with the tutorial, a selection and a hovered move path.
    let mut app = new_app("shots", 77);
    app.save.tutorial_done = false;
    press(&mut app, Key::Enter);
    idle(&mut app, 3.0);
    press(&mut app, Key::Tab);
    if let Some(id) = app.selected
        && let Some(b) = app.battle()
    {
        let moves = b.move_targets(id);
        if let Some(far) = moves.iter().max_by_key(|p| b.unit(id).pos.manhattan(**p)) {
            let at = app.tile_screen(*far).unwrap_or((640.0, 360.0));
            app.update(
                DT,
                &Input {
                    mouse: at,
                    ..Input::default()
                },
            );
        }
    }
    idle(&mut app, 0.2);
    shot(&mut app, &mut canvas, dir, "02_battle_move", &mut n)?;

    // Targeting with damage preview.
    let mut app = new_app("shots", 91);
    press(&mut app, Key::Enter);
    wait_player(&mut app, 8.0);
    let found = play_until(&mut app, 6000, |a| {
        a.player_can_act()
            && a.battle().is_some_and(|b| {
                b.alive(Team::Squad)
                    .any(|u| !u.acted && !b.targets(u.id, u.kind.abilities()[0]).is_empty())
            })
    });
    if found && let Some(b) = app.battle() {
        let pick = b
            .alive(Team::Squad)
            .find(|u| !u.acted && !b.targets(u.id, u.kind.abilities()[0]).is_empty())
            .map(|u| (u.id, b.targets(u.id, u.kind.abilities()[0])[0]));
        if let Some((id, t)) = pick {
            app.selected = Some(id);
            press(&mut app, Key::Num1);
            let at = app.tile_screen(t).unwrap_or((640.0, 360.0));
            app.update(
                DT,
                &Input {
                    mouse: (at.0, at.1 - 4.0),
                    ..Input::default()
                },
            );
        }
    }
    shot(&mut app, &mut canvas, dir, "03_targeting", &mut n)?;

    // Busiest moment: lots of particles mid-combat on a later island.
    let mut app = new_app("shots", 4242);
    press(&mut app, Key::Enter);
    if let Some(run) = app.run.as_mut() {
        run.island = 4;
        run.stage = Stage::Route {
            options: vec![Node::Elite(crate::map::Biome::Ash)],
        };
    }
    app.choose_route(0);
    wait_player(&mut app, 8.0);
    let mut best = 0;
    let mut best_canvas = Canvas::new(W, H);
    for _ in 0..4000 {
        if app.player_can_act() {
            app.autopilot_step();
        } else {
            idle(&mut app, DT);
        }
        let fx = app
            .view
            .particles
            .iter()
            .filter(|p| p.kind != crate::gfx::view::PKind::Dot || p.max < 1.5)
            .count();
        let score = fx
            + app.view.texts.len() * 40
            + if app.view.projectile.is_some() {
                150
            } else {
                0
            };
        if score > best && app.view.banner.is_none() {
            best = score;
            app.draw(&mut best_canvas);
        }
        if app.run.as_ref().is_none_or(|r| r.stage != Stage::Battle) {
            break;
        }
    }
    best_canvas
        .save_png(&dir.join("04_combat.png"))
        .map_err(|e| e.to_string())?;
    n += 1;

    // Frost biome skirmish overview with the danger overlay.
    let mut app = new_app("shots", 555);
    press(&mut app, Key::Enter);
    if let Some(run) = app.run.as_mut() {
        run.island = 2;
        run.stage = Stage::Route {
            options: vec![Node::Skirmish(crate::map::Biome::Frost)],
        };
    }
    app.choose_route(0);
    wait_player(&mut app, 8.0);
    idle(&mut app, 2.5);
    press(&mut app, Key::D);
    app.mouse = (1100.0, 400.0);
    idle(&mut app, 0.1);
    shot(&mut app, &mut canvas, dir, "05_frost_danger", &mut n)?;

    // The Hollow King.
    let mut app = new_app("shots", 31337);
    press(&mut app, Key::Enter);
    if let Some(run) = app.run.as_mut() {
        run.island = ISLANDS - 1;
        run.stage = Stage::Route {
            options: vec![Node::Throne],
        };
    }
    app.choose_route(0);
    wait_player(&mut app, 8.0);
    play_until(&mut app, 3000, |a| {
        a.battle().is_some_and(|b| b.round >= 3) && a.player_can_act()
    });
    idle(&mut app, 0.3);
    shot(&mut app, &mut canvas, dir, "06_hollow_king", &mut n)?;

    // Draft and route screens after a win.
    let mut app = new_app("shots", 8080);
    press(&mut app, Key::Enter);
    if let Some(run) = app.run.as_mut() {
        run.god_mode = true;
        if let Some(b) = run.battle.as_mut() {
            b.cfg.god_mode = true;
        }
    }
    play_until(&mut app, 20000, |a| {
        a.run
            .as_ref()
            .is_some_and(|r| matches!(r.stage, Stage::Draft { .. }))
    });
    app.mouse = ui::draft_rect(1, 3).center();
    idle(&mut app, 0.3);
    shot(&mut app, &mut canvas, dir, "07_draft", &mut n)?;
    press(&mut app, Key::Num2);
    idle(&mut app, 0.2);
    if let Some(run) = app.run.as_mut() {
        run.island = 3;
        run.stage = Stage::Route {
            options: vec![Node::Skirmish(crate::map::Biome::Crystal), Node::Sanctuary],
        };
    }
    app.mouse = ui::route_rect(0, 2).center();
    idle(&mut app, 0.3);
    shot(&mut app, &mut canvas, dir, "08_route", &mut n)?;

    // Victory screen.
    let mut app = new_app("shots", 9);
    press(&mut app, Key::Enter);
    let mut c2 = Canvas::new(W, H);
    drive_app(&mut app, true, 200_000, &mut c2)?;
    idle(&mut app, 0.5);
    shot(&mut app, &mut canvas, dir, "09_victory", &mut n)?;

    // Help.
    let mut app = new_app("shots", 10);
    press(&mut app, Key::H);
    shot(&mut app, &mut canvas, dir, "10_help", &mut n)?;

    let _ = std::fs::remove_dir_all(temp_save("shots").parent().unwrap_or(Path::new(".")));
    Ok(n)
}
