//! Screens and menus driven through synthetic input.

use cragswing::app::{App, Input, Key, Screen};
use cragswing::gfx::canvas::{Canvas, H, W};
use cragswing::sim::Outcome;

fn press(k: Key) -> Input {
    Input {
        pressed: vec![k],
        down: vec![k],
        ..Input::default()
    }
}

fn click(x: f32, y: f32) -> Input {
    Input {
        mouse: (x, y),
        mouse_down: true,
        mouse_pressed: true,
        ..Input::default()
    }
}

fn frames(app: &mut App, n: usize) {
    let mut c = Canvas::new(W, H);
    for _ in 0..n {
        app.update(1.0 / 60.0, &Input::default());
        app.draw(&mut c);
    }
}

#[test]
fn the_title_leads_into_training_on_a_fresh_save() {
    let mut app = App::new(None, 1);
    frames(&mut app, 5);
    assert_eq!(app.screen, Screen::Title);
    app.update(1.0 / 60.0, &press(Key::Enter));
    assert_eq!(app.screen, Screen::Intro);
    assert!(
        app.run.as_ref().unwrap().training,
        "first play is the training ascent"
    );
    frames(&mut app, 220);
    assert_eq!(app.screen, Screen::Playing);
}

#[test]
fn workshop_purchases_spend_glimmer_and_persist() {
    let dir = std::env::temp_dir().join(format!("cragswing-app-{}", std::process::id()));
    let path = dir.join("save.txt");
    let mut app = App::new(Some(path.clone()), 2);
    app.save.glimmer = 500;
    app.screen = Screen::Workshop;
    frames(&mut app, 3);
    // the first row is Stout Heart (300): Enter buys the highlighted button
    app.update(1.0 / 60.0, &press(Key::Enter));
    assert_eq!(app.save.perks[0], 1);
    assert_eq!(app.save.glimmer, 200);
    frames(&mut app, 2);
    // a second Stout Heart costs 900: refused
    app.update(1.0 / 60.0, &press(Key::Enter));
    assert_eq!(app.save.perks[0], 1);
    assert_eq!(app.save.glimmer, 200);
    let loaded = cragswing::save::Save::load(&path);
    assert_eq!(loaded.perks[0], 1);
    assert_eq!(loaded.glimmer, 200);
    // the perk shows up in the next run
    app.screen = Screen::Title;
    app.save.tutorial_done = true;
    app.start_run(false);
    assert_eq!(app.run.as_ref().unwrap().mods.max_hp, 5);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn pause_menu_resumes_and_assists_toggle() {
    let mut app = App::new(None, 3);
    app.save.tutorial_done = true;
    app.start_run(false);
    frames(&mut app, 220);
    assert_eq!(app.screen, Screen::Playing);
    app.update(1.0 / 60.0, &press(Key::Pause));
    assert_eq!(app.screen, Screen::Paused);
    frames(&mut app, 2);
    // the paused simulation does not advance
    let f0 = app.game.as_ref().unwrap().world.frame;
    frames(&mut app, 30);
    assert_eq!(app.game.as_ref().unwrap().world.frame, f0);
    app.update(1.0 / 60.0, &press(Key::Pause));
    assert_eq!(app.screen, Screen::Playing);
    // explorer mode makes Wren invincible for the running stage too
    app.update(1.0 / 60.0, &press(Key::Pause));
    frames(&mut app, 2);
    // the toggles are the last buttons; click the explorer toggle by label position
    let mut hit = false;
    for y in (440..610).step_by(6) {
        app.update(1.0 / 60.0, &click(640.0, y as f32));
        frames(&mut app, 1);
        if app.save.explorer {
            hit = true;
            break;
        }
    }
    assert!(hit, "explorer toggle not reachable");
    assert!(app.game.as_ref().unwrap().world.mods.invincible);
}

#[test]
fn heat_is_locked_until_a_win() {
    let mut app = App::new(None, 4);
    frames(&mut app, 3);
    assert_eq!(app.save.heat_unlocked, 0);
    assert_eq!(app.save.heat, 0);
    app.save.heat_unlocked = 2;
    let mut c = Canvas::new(W, H);
    app.draw(&mut c);
    // heat button sits last in the assists column
    let mut seen = 0;
    for y in (260..460).step_by(4) {
        app.update(1.0 / 60.0, &click(560.0, y as f32));
        app.draw(&mut c);
        seen = seen.max(app.save.heat);
    }
    assert!((1..=2).contains(&seen), "heat {seen}");
}

#[test]
fn losing_all_hearts_ends_the_run_and_records_it() {
    let dir = std::env::temp_dir().join(format!("cragswing-def-{}", std::process::id()));
    let path = dir.join("save.txt");
    let mut app = App::new(Some(path.clone()), 5);
    app.save.tutorial_done = true;
    app.start_run(false);
    frames(&mut app, 220);
    {
        let g = app.game.as_mut().unwrap();
        g.echo = 0.0; // nothing to rewind with
        g.world.p.hp = 1;
        g.world.last_cause = "Sawblade";
        g.world.hurt(0.0, 0.0, false);
    }
    frames(&mut app, 400);
    assert_eq!(app.screen, Screen::Defeat);
    assert_eq!(app.game.as_ref().unwrap().world.outcome, Outcome::Dead);
    let saved = cragswing::save::Save::load(&path);
    assert_eq!(saved.runs, 1);
    assert_eq!(saved.wins, 0);
    app.update(1.0 / 60.0, &press(Key::Enter));
    assert_eq!(app.screen, Screen::Title);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn music_and_sound_follow_the_game_state() {
    let mut app = App::new(None, 6);
    frames(&mut app, 5);
    let title = app.music();
    app.save.tutorial_done = true;
    app.start_run(false);
    frames(&mut app, 220);
    let calm = app.music();
    assert_eq!(calm.theme, 0);
    // pretend we are flying fast near foes: intensity climbs
    {
        let g = app.game.as_mut().unwrap();
        g.world.p.speed = 700.0;
    }
    app.update(1.0 / 60.0, &Input::default());
    let fast = app.music();
    assert!(fast.intensity >= calm.intensity);
    assert!(title.intensity <= 0.3);
    let sfx = app.take_sfx();
    assert!(!sfx.is_empty(), "the banner and jingles should be queued");
}
