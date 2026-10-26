use the_last_glasshouse::{
    view::{H, View, W},
    *,
};
#[test]
fn real_hit_regions_play_a_complete_campaign() {
    let mut app = App::new(3);
    app.save = Save::default();
    let mut view = View::new();
    app.act(Action::Start);
    app.recorded = true; // isolate test from the player's on-disk records
    for step in 0..5000 {
        if matches!(app.game.screen, Screen::Won | Screen::Lost) {
            break;
        }
        let planned = bot_action(&app.game);
        view.render(&app);
        let action = if let Action::Compost(i) = planned {
            // Right click shares the card's region.
            let hit = view
                .hits
                .iter()
                .find(|h| h.action == Action::Card(i))
                .expect("compost card is clickable");
            let p = (hit.rect.x + hit.rect.w / 2., hit.rect.y + hit.rect.h / 2.);
            match view.action_at(p) {
                Action::Card(n) => Action::Compost(n),
                _ => panic!("card blocked"),
            }
        } else {
            let hit = view
                .hits
                .iter()
                .find(|h| h.action == planned)
                .unwrap_or_else(|| panic!("No hit for {planned:?} at {step}"));
            view.action_at((hit.rect.x + hit.rect.w / 2., hit.rect.y + hit.rect.h / 2.))
        };
        assert_eq!(planned, action);
        app.act(action);
        app.tick(1. / 60.);
    }
    assert_eq!(app.game.screen, Screen::Won);
    assert!(app.game.late_content);
    assert_eq!(view.pixels.len(), W * H);
    app.act(Action::Restart);
    assert_eq!(app.game.stage, 0);
    assert_eq!(app.game.hp, 100);
    assert_eq!(app.game.battle.turn, 1);
}
#[test]
fn pause_blocks_actions_and_settings_work() {
    let mut a = App::new(5);
    a.act(Action::Start);
    a.act(Action::Pause);
    let turn = a.game.battle.turn;
    a.act(Action::EndTurn);
    assert_eq!(a.game.battle.turn, turn);
    a.act(Action::Assist);
    assert!(a.game.easy);
    a.act(Action::Fullscreen);
    assert!(a.fullscreen_requested);
    a.act(Action::Mute);
    assert!(a.muted);
    a.act(Action::Motion);
    assert!(!a.motion);
    a.act(Action::Close);
    a.act(Action::EndTurn);
    assert_eq!(a.game.battle.turn, turn + 1);
}
#[test]
fn poison_prevents_a_dying_pests_bite() {
    let mut g = Game::new(1, false, false);
    g.start();
    g.battle.enemies[0].hp = 1;
    g.battle.enemies[0].poison = 2;
    g.battle.enemies[0].intent = Intent::Bite(99);
    let hp = g.hp;
    g.end_turn();
    assert_eq!(g.screen, Screen::Reward);
    assert_eq!(g.hp, hp);
}
#[test]
fn all_bosses_have_distinct_rules_and_the_queen_has_a_phase() {
    let mut g = Game::new(3, false, false);
    for stage in [5, 11, 17] {
        g.stage = stage;
        g.begin_battle();
        assert_eq!(g.battle.enemies[0].kind, 6 + stage / 6);
        g.battle.turn = 3;
        g.battle.enemies[0].hp = g.battle.enemies[0].max / 2;
        g.end_turn();
        if stage == 17 {
            // next planned turn 4: enraged bite
            assert!(g.battle.enemies[0].intent.damage() >= 14);
            g.battle.turn = 5;
            g.end_turn();
            assert!(matches!(g.battle.enemies[0].intent, Intent::Frost(3)));
        }
    }
}
#[test]
fn graft_and_removal_are_real_deck_changes() {
    let mut g = Game::new(10, false, false);
    g.screen = Screen::Route;
    g.route(2);
    let old = g.deck.len();
    g.upgrade(0);
    assert!(g.deck[0].gold);
    assert_eq!(g.deck.len(), old);
    assert_eq!(g.stage, 1);
    g.screen = Screen::Route;
    g.route(2);
    g.prune(0);
    assert_eq!(g.deck.len(), old - 1);
    assert_eq!(g.stage, 2);
}
#[test]
fn harvest_burst_retargets_after_a_kill() {
    let mut g = Game::new(3, false, false);
    g.stage = 4;
    g.begin_battle();
    g.battle.enemies[0].hp = 1;
    let before = g.battle.enemies[1].hp;
    g.battle.hand = vec![Card::new(24)];
    g.battle.energy = 3;
    g.battle.beds = [
        Some(Plant {
            card: Card::new(1),
            growth: 3,
        }),
        Some(Plant {
            card: Card::new(1),
            growth: 3,
        }),
        None,
    ];
    g.play(0, 0);
    assert!(g.battle.enemies[1].hp < before);
}
