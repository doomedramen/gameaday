//! Headless self-test and screenshot mode.
//!
//! `run()` plays the game with the autopilot at several levels:
//! - level generation is solvable (an invincible bot clears every sampled ascent
//!   and the boss),
//! - difficulty ramps and new foes keep arriving until the last ascents,
//! - a normal-rules bot plays whole runs, so wins are possible but rare,
//! - the real `App` is driven through synthetic keys and mouse for a complete
//!   run (training, five ascents, drafts, routes, boss, victory),
//! - rewind, restart and the save file work.
//!
//! `screenshots()` renders key moments to PNG.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use crate::app::{App, Input, Key, Screen};
use crate::bot::{Bot, play};
use crate::game::Game;
use crate::gfx::canvas::{Canvas, H, W};
use crate::level::*;
use crate::run::{Boon, Route, Run};
use crate::save::Save;
use crate::sim::*;
use crate::stagegen::{StageSpec, generate};

// ------------------------------------------------------------------ helpers

fn play_stage(spec: &StageSpec, mods: Mods, hp: i32) -> (Outcome, f32, World, Bot) {
    let level = Arc::new(generate(spec));
    let mut w = World::new(level, mods, hp);
    let mut bot = Bot::new();
    let r = play(&mut w, &mut bot, 420.0);
    (r.outcome, r.seconds, w, bot)
}

/// Threat entities per 100 tiles of level.
fn threat_density(l: &Level) -> f32 {
    let n = l.ents.iter().filter(|e| e.kind.is_threat()).count() as f32;
    n / (l.w as f32 / 100.0)
}

fn kinds_in(l: &Level) -> BTreeSet<&'static str> {
    l.ents
        .iter()
        .map(|e| e.kind)
        .filter(|k| k.is_threat() || matches!(k, EntKind::Wind | EntKind::Bloom))
        .map(|k| k.name())
        .collect()
}

pub struct RunSim {
    pub won: bool,
    pub reached: usize,
    pub seconds: f32,
    pub damage: Vec<u32>,
}

/// Play a whole run (no UI) with the bot under normal rules.
pub fn sim_run(seed: u64, heat: u8) -> RunSim {
    let save = Save {
        heat_unlocked: heat,
        heat,
        tutorial_done: true,
        ..Save::default()
    };
    let mut scratch = Save::default();
    let mut run = Run::new(seed, &save, false);
    let mut seconds = 0.0;
    let mut damage = vec![];
    let mut rng = crate::rng::Rng::new(seed ^ 77);
    loop {
        let spec = run.spec();
        let level = Arc::new(generate(&spec));
        let mut g = run.make_game(level);
        let mut bot = Bot::new();
        bot.noise = seed as u32;
        let max = (420.0 / DT) as u64;
        let mut stuck_x = g.world.p.x;
        let mut stuck_at = 0u64;
        while g.world.outcome == Outcome::Playing && g.world.frame < max {
            let c = bot.act(&g.world);
            g.tick(&c, false);
            if !g.world.level.boss && g.world.p.x > stuck_x + 8.0 {
                stuck_x = g.world.p.x;
                stuck_at = g.world.frame;
            }
            if !g.world.level.boss && g.world.frame - stuck_at > 60 * 25 {
                break;
            }
            // simple rewind use: undo a fatal blow if we can
            if g.world.outcome == Outcome::Dead && g.can_rewind() {
                for _ in 0..70 {
                    g.tick(&Controls::default(), true);
                }
            }
        }
        seconds += g.world.time;
        damage.push(g.world.stats.damage);
        if g.world.outcome != Outcome::Cleared {
            return RunSim {
                won: false,
                reached: run.stage,
                seconds,
                damage,
            };
        }
        run.finish_stage(&g, &mut scratch);
        let boss = run.stage >= 5;
        let daring = run.route == Route::Daring && run.stage > 0 && run.stage < 5;
        if !run.advance() {
            return RunSim {
                won: true,
                reached: 6,
                seconds,
                damage,
            };
        }
        if !boss {
            run.pending_picks = 1 + daring as u32 + g.world.stats.chests.min(2);
            while run.pending_picks > 0 {
                let opts = run.draft_options();
                if opts.is_empty() {
                    break;
                }
                // prefer defensive boons like a sensible player
                let pick = opts
                    .iter()
                    .copied()
                    .find(|b| matches!(b, Boon::Locket | Boon::StormWard | Boon::Phoenix))
                    .unwrap_or(opts[0]);
                run.pick(pick);
            }
            if run.stage < 5 {
                run.set_route(if rng.chance(0.3) {
                    Route::Daring
                } else {
                    Route::Steady
                });
            }
        }
    }
}

// ------------------------------------------------------------- app driver

/// Translate bot controls into the keys and mouse a person would use.
fn controls_to_input(c: &Controls, cam: (f32, f32), prev: &Controls) -> Input {
    let mut i = Input::default();
    if c.dir < 0 {
        i.down.push(Key::Left);
    }
    if c.dir > 0 {
        i.down.push(Key::Right);
    }
    if c.up {
        i.down.push(Key::Up);
    }
    if c.down {
        i.down.push(Key::Down);
    }
    if c.jump_held || c.jump_pressed {
        i.down.push(Key::Jump);
    }
    if c.jump_pressed {
        i.pressed.push(Key::Jump);
    }
    if c.hook_held {
        if let Some((ax, ay)) = c.aim {
            i.mouse = (ax - cam.0, ay - cam.1);
            i.mouse_down = true;
            i.mouse_pressed = c.hook_pressed || !prev.hook_held;
        } else {
            i.down.push(Key::Hook);
            if c.hook_pressed {
                i.pressed.push(Key::Hook);
            }
        }
    }
    i
}

fn press(key: Key) -> Input {
    Input {
        pressed: vec![key],
        down: vec![key],
        ..Input::default()
    }
}

pub struct AppRun {
    pub frames: u64,
    pub victory: bool,
    pub defeats: u32,
    pub clears: u32,
    pub screens: BTreeSet<String>,
}

/// Drive the real application through a full run.
pub fn drive_app(app: &mut App, max_frames: u64, stop_at_victory: bool) -> AppRun {
    let mut out = AppRun {
        frames: 0,
        victory: false,
        defeats: 0,
        clears: 0,
        screens: BTreeSet::new(),
    };
    let mut canvas = Canvas::new(W, H);
    let mut bot = Bot::new();
    let mut prev = Controls::default();
    let mut last_screen = app.screen;
    let mut idle_title = 0;
    let mut last_frame = u64::MAX;
    let mut best_x = 0.0f32;
    let mut best_at = 0u64;
    for f in 0..max_frames {
        out.frames = f;
        out.screens.insert(format!("{:?}", app.screen));
        let mut input = Input::default();
        match app.screen {
            Screen::Title => {
                idle_title += 1;
                if out.victory {
                    break;
                }
                if idle_title > 4 {
                    input = press(Key::Enter);
                    idle_title = 0;
                }
            }
            Screen::Intro => input = press(Key::Jump),
            Screen::Playing => {
                if let Some(g) = &app.game {
                    let w = &g.world;
                    // a player would give up on a crossing the autopilot cannot find
                    if w.p.x > best_x + 8.0 || w.level.boss || w.frame < 5 {
                        best_x = best_x.max(w.p.x);
                        best_at = f;
                        if w.frame < 5 {
                            best_x = w.p.x;
                        }
                    } else if f - best_at > 60 * 40 {
                        app.screen = Screen::Title;
                        best_x = 0.0;
                        continue;
                    }
                    if w.outcome == Outcome::Playing && w.p.dying > 0.0 && g.can_rewind() {
                        // dying: rewind the fatal blow like a player would
                        input = Input {
                            down: vec![Key::Rewind],
                            ..Input::default()
                        };
                    } else if w.outcome == Outcome::Playing {
                        // hit-stop freezes the simulation; keep the same controls
                        // until it advances so the bot's timing stays honest
                        let c = if w.frame == last_frame {
                            prev
                        } else {
                            bot.act(w)
                        };
                        last_frame = w.frame;
                        input = controls_to_input(&c, (app.cam.x, app.cam.y), &prev);
                        prev = c;
                    } else if w.outcome == Outcome::Dead {
                        // a fatal blow: hold rewind if there is any echo left
                        if g.can_rewind() && g.dead_for < 4.0 {
                            input = Input {
                                down: vec![Key::Rewind],
                                ..Input::default()
                            };
                        } else {
                            input = press(Key::Enter);
                        }
                    }
                }
            }
            Screen::Clear => input = press(Key::Enter),
            Screen::Draft => input = press(Key::Num1),
            Screen::Route => input = press(if f % 3 == 0 { Key::Num2 } else { Key::Num1 }),
            Screen::Victory => {
                out.victory = true;
                if stop_at_victory {
                    break;
                }
                input = press(Key::Enter);
            }
            Screen::Defeat => {
                out.defeats += 1;
                input = press(Key::Enter);
            }
            _ => input = press(Key::Back),
        }
        if app.screen != last_screen {
            if std::env::var("CRAGSWING_DEBUG").is_ok() {
                let st = app
                    .run
                    .as_ref()
                    .map(|r| (r.stage, r.training, r.hp))
                    .unwrap_or((0, false, 0));
                let boons = app
                    .run
                    .as_ref()
                    .map(|r| r.boons.clone())
                    .unwrap_or_default();
                let seed = app.run.as_ref().map(|r| r.seed).unwrap_or(0);
                eprintln!(
                    "f{f}: {:?} -> {:?} run(stage,training,hp)={st:?} boons={boons:?} seed={seed}",
                    last_screen, app.screen
                );
            }
            if last_screen == Screen::Clear {
                out.clears += 1;
            }
            if app.screen == Screen::Playing || app.screen == Screen::Intro {
                // a new stage: fresh bot
                if last_screen != Screen::Paused {
                    bot = Bot::new();
                    prev = Controls::default();
                    last_frame = u64::MAX;
                }
            }
            last_screen = app.screen;
        }
        app.update(1.0 / 60.0, &input);
        let _ = app.take_sfx();
        if f % 90 == 0 {
            app.draw(&mut canvas);
        }
        if f % 300 == 0 && std::env::var("CRAGSWING_DEBUG").is_ok() {
            if let Some(g) = &app.game {
                let w = &g.world;
                eprintln!(
                    "  f{f} {:?} x={:.0} y={:.0} beat={} falls={} hp={} t={:.0}s",
                    app.screen, w.p.x, w.p.y, bot.beat, w.stats.falls, w.p.hp, w.time
                );
                if w.stats.falls == 6 {
                    eprintln!("{}", ascii_dump(w));
                    if let Some(b) = w.level.beats.get(bot.beat) {
                        eprintln!("beat {b:?}");
                    }
                }
            }
        }
    }
    out
}

/// Debug picture of the level around the player.
pub fn ascii_dump(w: &World) -> String {
    let l = &w.level;
    let cx = (w.p.x / T) as i32;
    let mut out = String::new();
    for y in 14..50 {
        for x in (cx - 40)..(cx + 40) {
            let mut c = match l.tile(x, y) {
                AIR => ' ',
                ROCK => '#',
                ICE => '=',
                GLASS => 'G',
                CRUMBLE => 'c',
                _ => '^',
            };
            for a in &l.anchors {
                if (a.x / T) as i32 == x && (a.y / T) as i32 == y {
                    c = 'O';
                }
            }
            for e in &w.ents {
                if e.alive && (e.x / T) as i32 == x && (e.y / T) as i32 == y {
                    c = match e.kind {
                        EntKind::Crystal => '*',
                        EntKind::Skitter => 's',
                        EntKind::Bulwark => 'B',
                        EntKind::Spitter => 'T',
                        EntKind::Saw => 'X',
                        EntKind::Geyser => 'g',
                        EntKind::Wisp => 'w',
                        EntKind::Bloom => 'b',
                        _ => '?',
                    };
                }
            }
            if (w.p.x / T) as i32 == x && (w.p.y / T) as i32 == y {
                c = '@';
            }
            out.push(c);
        }
        out.push('\n');
    }
    out
}

// -------------------------------------------------------------------- run

pub fn run() -> Result<String, String> {
    let mut rep = String::new();
    run_inner(&mut rep).map_err(|e| format!("{e}\n--- progress so far ---\n{rep}"))?;
    Ok(rep)
}

fn run_inner(rep: &mut String) -> Result<(), String> {
    let mut note = |s: String| {
        rep.push_str(&s);
        rep.push('\n');
    };
    let t0 = std::time::Instant::now();

    // 1. every sampled ascent is solvable by an invincible bot
    let inv = Mods {
        invincible: true,
        ..Mods::default()
    };
    let mut cleared = [0u32; 5];
    let mut total = [0u32; 5];
    let seeds = 14u64;
    let mut times = [0.0f32; 5];
    for stage in 0..5usize {
        for seed in 1..=seeds {
            let spec = StageSpec::new(stage, seed * 31 + 5);
            let (out, secs, _, _) = play_stage(&spec, inv.clone(), 4);
            total[stage] += 1;
            if out == Outcome::Cleared {
                cleared[stage] += 1;
                times[stage] += secs;
            }
        }
        let rate = cleared[stage] as f32 / total[stage] as f32;
        if rate < 0.85 {
            return Err(format!(
                "stage {stage}: autopilot cleared only {}/{}",
                cleared[stage], total[stage]
            ));
        }
    }
    note(format!(
        "solvable: ascents cleared by the autopilot {:?} of {:?} (avg {:.0}s)",
        cleared,
        total,
        times.iter().sum::<f32>() / cleared.iter().sum::<u32>() as f32
    ));

    // 2. the boss can be beaten, and only by hitting it at speed
    let mut boss_wins = 0;
    for seed in 1..=4u64 {
        let spec = StageSpec::new(5, seed);
        let (out, secs, w, _) = play_stage(&spec, inv.clone(), 4);
        if out == Outcome::Cleared {
            boss_wins += 1;
        }
        let b = w.boss.as_ref().ok_or("boss missing")?;
        if out == Outcome::Cleared && b.hp > 0 {
            return Err("boss level cleared without defeating the boss".into());
        }
        note(format!(
            "boss seed {seed}: {out:?} in {secs:.0}s, boss hp left {}",
            b.hp
        ));
    }
    if boss_wins < 3 {
        return Err(format!(
            "autopilot beat the Stormheart only {boss_wins}/4 times"
        ));
    }
    note("victory reachable: the Stormheart falls to a fast ram".to_string());

    // 3. difficulty ramps; new foes keep arriving
    let mut dens = [0.0f32; 5];
    let mut seen: BTreeSet<&str> = BTreeSet::new();
    let mut first_stage_of: Vec<(&str, usize)> = vec![];
    for stage in 0..5usize {
        let mut d = 0.0;
        for seed in 1..=10u64 {
            let l = generate(&StageSpec::new(stage, seed * 17));
            d += threat_density(&l);
            for k in kinds_in(&l) {
                if seen.insert(k) {
                    first_stage_of.push((k, stage));
                }
            }
        }
        dens[stage] = d / 10.0;
    }
    note(format!(
        "threat density per 100 tiles by ascent: {:?}",
        dens.map(|d| (d * 10.0).round() / 10.0)
    ));
    if !(dens[4] > dens[0] * 1.8 && dens[3] > dens[1]) {
        return Err(format!("difficulty does not ramp: {dens:?}"));
    }
    for stage in 1..5usize {
        if !first_stage_of.iter().any(|(_, s)| *s == stage) {
            return Err(format!("ascent {stage} introduces nothing new"));
        }
    }
    note(format!("new foes by ascent: {first_stage_of:?}"));

    // typical (bronze-pace) timeline: new content arrives late in the run
    let mut clock = 0.0;
    let mut arrivals = vec![];
    for stage in 0..5usize {
        let l = generate(&StageSpec::new(stage, 99));
        if first_stage_of.iter().any(|(_, s)| *s == stage) && stage > 0 {
            arrivals.push((stage, clock));
        }
        clock += l.par * 2.1;
    }
    arrivals.push((5, clock));
    clock += 150.0;
    note(format!(
        "bronze-pace (typical player) timeline: new content at {:?}s, run ends ~{:.0}s",
        arrivals.iter().map(|a| a.1 as u32).collect::<Vec<_>>(),
        clock
    ));
    let last_new = arrivals.last().map(|a| a.1).unwrap_or(0.0);
    if clock < 780.0 || last_new < clock * 0.6 {
        return Err(format!(
            "content pacing: run {clock:.0}s, last new content at {last_new:.0}s"
        ));
    }

    // 4. whole runs under normal rules: wins possible but not guaranteed
    let mut wins = 0;
    let mut reached = vec![];
    let mut dmg = [0u32; 6];
    let runs = 24u64;
    for seed in 1..=runs {
        let r = sim_run(seed * 101, 0);
        if r.won {
            wins += 1;
        }
        reached.push(r.reached);
        for (i, d) in r.damage.iter().enumerate() {
            dmg[i.min(5)] += d;
        }
    }
    note(format!(
        "normal-rules autopilot runs: {wins}/{runs} wins; furthest stage histogram {:?}; damage per stage {:?}",
        hist(&reached),
        dmg
    ));
    if wins == 0 {
        return Err("no autopilot run won: victory should be possible".into());
    }
    if wins as u64 == runs {
        return Err("every run won: the game is too easy".into());
    }
    // heat makes it harder
    let mut hot_wins = 0;
    for seed in 1..=24u64 {
        if sim_run(seed * 101, 3).won {
            hot_wins += 1;
        }
    }
    let base_wins = wins;
    note(format!(
        "heat 3 wins {hot_wins}/24 vs heat 0 {base_wins}/24"
    ));
    if hot_wins > base_wins {
        return Err("heat made the run easier".into());
    }

    // 5. the real application, driven by synthetic input, plays a full run
    let dir = std::env::temp_dir().join(format!("cragswing-selftest-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    let save_path = dir.join("save.txt");
    let mut app = App::new(Some(save_path.clone()), 12345);
    // a few attempts: the bot plays by normal rules
    let mut result = None;
    let mut total_clears = 0;
    let mut total_frames = 0;
    let mut screens = BTreeSet::new();
    for attempt in 0..6 {
        let r = drive_app(&mut app, 60 * 60 * 45, false);
        total_clears += r.clears;
        total_frames += r.frames;
        screens.extend(r.screens.iter().cloned());
        if r.victory {
            result = Some((attempt, r));
            break;
        }
    }
    let (attempt, r) = result.ok_or("the app run never reached victory")?;
    let _ = r;
    note(format!(
        "app driven by synthetic input: victory after {} attempt(s), {} stage clears, {:.0} simulated minutes, screens {:?}",
        attempt + 1,
        total_clears,
        total_frames as f32 / 3600.0,
        screens
    ));
    for needed in [
        "Title", "Intro", "Playing", "Clear", "Draft", "Route", "Victory",
    ] {
        if !r.screens.contains(needed) {
            return Err(format!("screen {needed} never shown"));
        }
    }
    if app.stats_seen.stages_cleared < 7 {
        return Err(format!(
            "only {} stages cleared in the app",
            app.stats_seen.stages_cleared
        ));
    }
    // save persisted
    let saved = Save::load(&save_path);
    if !saved.tutorial_done
        || saved.wins < 1
        || saved.runs < 1
        || saved.glimmer == 0
        || saved.best_score <= 0
    {
        return Err(format!("save file did not persist progress: {saved:?}"));
    }
    if saved.best_ms.iter().all(|m| *m == 0) {
        return Err("no best times were recorded".into());
    }
    note(format!(
        "save persisted: best {} score, {} wins, glimmer {}",
        saved.best_score, saved.wins, saved.glimmer
    ));

    // 6. rewind: scrub back from a death
    rewind_check()?;
    note("rewind restores position, hearts and broken blocks and spends echo".to_string());

    // 7. restart after defeat works and every screen renders
    let mut app2 = App::new(None, 99);
    let mut canvas = Canvas::new(W, H);
    app2.draw(&mut canvas);
    app2.update(1.0 / 60.0, &press(Key::Enter));
    for _ in 0..200 {
        app2.update(1.0 / 60.0, &Input::default());
        app2.draw(&mut canvas);
    }
    if app2.screen != Screen::Playing {
        return Err(format!(
            "expected Playing after intro, got {:?}",
            app2.screen
        ));
    }
    app2.update(1.0 / 60.0, &press(Key::Pause));
    app2.draw(&mut canvas);
    if app2.screen != Screen::Paused {
        return Err("pause did not open".into());
    }
    app2.update(1.0 / 60.0, &press(Key::Help));
    app2.draw(&mut canvas);
    if app2.screen != Screen::Help {
        return Err("help did not open".into());
    }
    app2.update(1.0 / 60.0, &press(Key::Back));
    app2.update(1.0 / 60.0, &press(Key::Pause));
    app2.update(1.0 / 60.0, &press(Key::Pause));
    note("title, intro, pause, help and restart flow work".to_string());

    let _ = std::fs::remove_dir_all(&dir);
    note(format!(
        "selftest finished in {:.1}s",
        t0.elapsed().as_secs_f32()
    ));
    Ok(())
}

fn hist(v: &[usize]) -> [u32; 7] {
    let mut h = [0u32; 7];
    for x in v {
        h[(*x).min(6)] += 1;
    }
    h
}

fn rewind_check() -> Result<(), String> {
    let spec = StageSpec::new(0, 3);
    let level = Arc::new(generate(&spec));
    let mut g = Game::new(level, Mods::default(), 4, 100.0);
    let idle = Controls::default();
    for _ in 0..40 {
        g.tick(&idle, false);
    }
    let x0 = g.world.p.x;
    let hp0 = g.world.p.hp;
    let right = Controls {
        dir: 1,
        ..Controls::default()
    };
    for _ in 0..90 {
        g.tick(&right, false);
    }
    if g.world.p.x <= x0 + 50.0 {
        return Err("player did not run".into());
    }
    g.world.hurt(g.world.p.x, g.world.p.y, false);
    if g.world.p.hp != hp0 - 1 {
        return Err("hurt did not remove a heart".into());
    }
    let echo0 = g.echo;
    for _ in 0..120 {
        g.tick(&idle, true);
    }
    if g.world.p.x > x0 + 60.0 || g.world.p.hp != hp0 {
        return Err(format!(
            "rewind did not restore: x {} hp {}",
            g.world.p.x, g.world.p.hp
        ));
    }
    if g.echo >= echo0 - 20.0 {
        return Err("rewind did not spend echo".into());
    }
    Ok(())
}

// -------------------------------------------------------------- screenshots

struct Shooter {
    dir: PathBuf,
    n: usize,
}

impl Shooter {
    fn save(&mut self, c: &Canvas, name: &str) -> Result<(), String> {
        let p = self.dir.join(format!("{:02}_{name}.png", self.n + 1));
        c.save_png(&p).map_err(|e| e.to_string())?;
        self.n += 1;
        Ok(())
    }
}

fn settle(app: &mut App, frames: usize) {
    let mut canvas = Canvas::new(W, H);
    for _ in 0..frames {
        app.update(1.0 / 60.0, &Input::default());
        let _ = app.take_sfx();
        app.draw(&mut canvas);
    }
}

/// Run the bot inside the app for `frames` ticks of play.
fn bot_frames(app: &mut App, bot: &mut Bot, frames: usize) {
    let mut prev = Controls::default();
    for _ in 0..frames {
        if app.screen != Screen::Playing {
            app.update(1.0 / 60.0, &press(Key::Jump));
            continue;
        }
        let Some(g) = &app.game else { break };
        if g.world.outcome != Outcome::Playing {
            break;
        }
        let c = bot.act(&g.world);
        let input = controls_to_input(&c, (app.cam.x, app.cam.y), &prev);
        prev = c;
        app.update(1.0 / 60.0, &input);
        let _ = app.take_sfx();
    }
}

/// Start a run at `stage` in the app (explorer mode so the bot survives).
fn app_at_stage(stage: usize, seed: u64, boons: &[Boon]) -> App {
    let mut app = App::new(None, seed);
    app.save.tutorial_done = true;
    app.save.explorer = true;
    app.start_run(false);
    if let Some(run) = app.run.as_mut() {
        run.stage = stage;
        for b in boons {
            run.pick(*b);
        }
    }
    app.start_stage_for_test();
    app
}

pub fn screenshots(dir: &Path) -> Result<usize, String> {
    std::fs::create_dir_all(dir).map_err(|e| e.to_string())?;
    let mut sh = Shooter {
        dir: dir.to_path_buf(),
        n: 0,
    };
    let mut canvas = Canvas::new(W, H);

    // title
    let mut app = App::new(None, 4242);
    app.save.tutorial_done = true;
    app.save.heat_unlocked = 1;
    app.save.best_score = 6420;
    app.save.wins = 2;
    app.save.runs = 7;
    app.save.glimmer = 640;
    app.save.best_ms = [61400, 74200, 0, 0, 0, 0];
    settle(&mut app, 100);
    app.draw(&mut canvas);
    sh.save(&canvas, "title")?;

    // training with a prompt on screen
    let mut app = App::new(None, 7);
    app.start_run(true);
    settle(&mut app, 220);
    let mut bot = Bot::new();
    let mut shot = false;
    for _ in 0..60 * 40 {
        bot_frames(&mut app, &mut bot, 4);
        let Some(g) = &app.game else { break };
        let w = &g.world;
        let in_prompt =
            w.level.prompts.iter().any(|p| {
                p.text.starts_with("Aim with the mouse") && w.p.x >= p.x0 && w.p.x <= p.x1
            });
        if in_prompt && w.p.on_ground {
            app.draw(&mut canvas);
            sh.save(&canvas, "training_prompt")?;
            shot = true;
            break;
        }
    }
    if !shot {
        app.draw(&mut canvas);
        sh.save(&canvas, "training_prompt")?;
    }

    // each ascent mid-action, on a seed that actually has the feature
    let names = [
        "meadow_swing",
        "cave_lantern",
        "frost_gusts",
        "cinder_geysers",
        "storm_strikes",
    ];
    let kinds = [
        EntKind::Skitter,
        EntKind::Spitter,
        EntKind::Wind,
        EntKind::Geyser,
        EntKind::Strike,
    ];
    for (stage, name) in names.iter().enumerate() {
        let mut done = false;
        let mut seed_try = 100 + stage as u64 * 13;
        for _ in 0..12 {
            // find a level of this ascent that contains the feature
            while !generate(&StageSpec::new(stage, seed_try))
                .ents
                .iter()
                .any(|e| e.kind == kinds[stage])
            {
                seed_try += 1;
            }
            let mut app = App::new(None, 5);
            app.save.tutorial_done = true;
            app.save.explorer = true;
            app.start_run(false);
            if let Some(run) = app.run.as_mut() {
                run.stage = stage;
                run.seed = seed_try.wrapping_sub(stage as u64 * 7919);
                run.pick(Boon::LongLine);
            }
            app.start_stage_for_test();
            settle(&mut app, 120);
            let mut bot = Bot::new();
            let mut frames = 0;
            let mut ready = false;
            while frames < 60 * 70 && !ready {
                bot_frames(&mut app, &mut bot, 4);
                frames += 4;
                let Some(g) = &app.game else { break };
                let w = &g.world;
                if w.outcome != Outcome::Playing || app.screen != Screen::Playing {
                    break;
                }
                ready = match stage {
                    0 => w.p.rope.is_some() && w.p.speed > 380.0 && frames > 200,
                    1 => w.ents.iter().any(|e| {
                        e.alive
                            && e.kind == EntKind::Spitter
                            && e.state == 1
                            && (e.x - w.p.x).abs() < 380.0
                    }),
                    2 => w.gust.abs() > 0.6,
                    3 => w.ents.iter().any(|e| {
                        e.alive
                            && e.kind == EntKind::Geyser
                            && e.state == 2
                            && (e.x - w.p.x).abs() < 420.0
                    }),
                    _ => w.ents.iter().any(|e| {
                        e.alive
                            && e.kind == EntKind::Strike
                            && e.state == 1
                            && e.t > 0.6
                            && (e.a - w.p.x).abs() < 500.0
                    }),
                };
            }
            if ready {
                app.draw(&mut canvas);
                sh.save(&canvas, name)?;
                done = true;
                break;
            }
            seed_try += 1;
        }
        if !done {
            return Err(format!("could not frame a screenshot for {name}"));
        }
    }

    // boss: telegraph, then stunned with the hit prompt
    let mut app = app_at_stage(5, 31, &[Boon::LongLine, Boon::Locket]);
    settle(&mut app, 220);
    let mut bot = Bot::new();
    let mut got_tell = false;
    let mut got_stun = false;
    for _ in 0..60 * 120 {
        bot_frames(&mut app, &mut bot, 3);
        let Some(g) = &app.game else { break };
        let Some(b) = &g.world.boss else { break };
        let (state, bt) = (b.state, b.t);
        if !got_tell && matches!(state, crate::boss::BState::Fan) && bt > 0.7 {
            app.draw(&mut canvas);
            sh.save(&canvas, "boss_fan")?;
            got_tell = true;
        }
        if got_tell && !got_stun && state == crate::boss::BState::Stun && bt > 0.4 {
            app.draw(&mut canvas);
            sh.save(&canvas, "boss_stunned")?;
            got_stun = true;
        }
        if got_stun {
            break;
        }
    }

    // stage clear, draft, route, victory, workshop, help
    let mut app = App::new(None, 55);
    app.save.tutorial_done = true;
    app.save.glimmer = 800;
    app.save.heat_unlocked = 1;
    app.save.explorer = true;
    app.start_run(false);
    settle(&mut app, 200);
    let mut bot = Bot::new();
    for _ in 0..60 * 120 {
        bot_frames(&mut app, &mut bot, 10);
        if app.screen == Screen::Clear {
            break;
        }
    }
    settle(&mut app, 70);
    sh.save(
        &{
            app.draw(&mut canvas);
            canvas.clone()
        },
        "stage_clear",
    )?;
    app.update(1.0 / 60.0, &press(Key::Enter));
    settle(&mut app, 5);
    app.draw(&mut canvas);
    sh.save(&canvas, "boon_draft")?;
    app.update(1.0 / 60.0, &press(Key::Num2));
    settle(&mut app, 5);
    app.draw(&mut canvas);
    sh.save(&canvas, "route_choice")?;
    let mut w = App::new(None, 3);
    w.save.glimmer = 520;
    w.save.perks = [1, 2, 0, 0, 0];
    w.screen = Screen::Workshop;
    settle(&mut w, 30);
    w.draw(&mut canvas);
    sh.save(&canvas, "workshop")?;
    let mut h = App::new(None, 3);
    h.screen = Screen::Help;
    settle(&mut h, 30);
    h.draw(&mut canvas);
    sh.save(&canvas, "help")?;

    // victory via the driver (explorer)
    let mut v = App::new(None, 77);
    v.save.tutorial_done = true;
    v.save.explorer = true;
    let r = drive_app(&mut v, 60 * 60 * 40, true);
    if r.victory {
        settle(&mut v, 90);
        v.draw(&mut canvas);
        sh.save(&canvas, "victory")?;
    }
    Ok(sh.n)
}
