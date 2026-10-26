# The Last Glasshouse

GameADay, 26 October 2026. An original botanical deckbuilder in Rust.
Plant a hand of seeds, tend three garden beds and harvest them against pests
across eighteen encounters and four seasons.

## Run

```sh
cargo run --release
```

All artwork and fonts are embedded; no downloads or network access at runtime.
The native window is resizable. On macOS the green window control and the
visible Fullscreen button use native fullscreen. The fullscreen shortcut is F.
The native fullscreen adapter is macOS-specific; other platforms can resize
the window. The built local macOS app, if present, is in
`target/The Last Glasshouse.app` and is intentionally not committed.

## Controls

| Action | Input |
| --- | --- |
| Play a tending card | Click it, or 1–9 |
| Plant / water | Select the card, then click a bed |
| Harvest a mature plant | Click its bed with no card selected |
| Select an enemy target | Click the enemy |
| Compost a card for +1 action | Right-click it, once per turn |
| End turn | Space or the End turn button |
| Undo current-turn decisions | U or Undo decision |
| Cancel selected card | Escape or right-click outside the hand |
| Help / field guide | H / D or their buttons |
| Pause and settings | Escape / P or Pause / settings |
| Fullscreen / mute | F / M or settings buttons |

Every enemy turn grows beds by one and refreshes the hand and actions.
Shelter absorbs incoming bites and resets each turn. Mature neighbouring
plants add harvest damage. Poison ticks before enemy actions. Cards move
through draw and discard piles; cards marked Exhaust leave circulation for
that encounter. A maximum of nine cards fits in the hand.

After encounters, choose a card or skip it. Between rooms choose healing,
a relic with a harder encounter, or a potting bench to remove or upgrade a
card. Relics persist through the run. Three keepers have different intentions;
the final keeper has a second phase. Gentle mode reduces incoming bites,
and motion and audio can be disabled independently.

Best score, completed runs, wins and collection records are saved atomically
at `~/.last-glasshouse/save.txt`; override with `GLASSHOUSE_SAVE`.
A win unlocks an alternate starting deck. An unfinished run is not saved.

## Validation

```sh
cargo fmt --check
cargo clippy --offline --all-targets -- -D warnings
cargo test --offline --release
cargo build --offline --release
cargo run --release -- --selftest
cargo run --release -- --screenshot screenshots
```

The headless selftest plays 48 seeded full campaigns and checks rising enemy
pressure, late seasonal content, bounded encounters, reachable loss and
restart. Simulated minutes are an explicit decision-pacing estimate for this
untimed game, not a timed human playtest. The binary integration test also
exports ten PNGs. A second integration test completes a campaign using the
renderer’s actual hit regions and the app’s input actions. Additional tests
cover poison timing, harvest retargeting, deck edits, boss phases, settings,
undo, invalid inputs, saves and audio limits.

`src/lib.rs` owns rules, campaign, app actions, persistence and bot;
`src/view.rs` is its software-rendering module; `src/audio.rs` is its synthesis
module. `src/main.rs` only adapts the window, mouse/keyboard, audio device
and macOS fullscreen API. No engine types enter the rules.

## Review and status

The player found the game boring, far too easy, and manual harvesting
unmotivated. This game did **not** beat the best previous game. It is frozen
after play; feedback is recorded in the repository state and general lessons
for future games. See `REVIEW.md` for the honest assessment.

## Credits

Illustrations: **Twemoji**, copyright Twitter, Inc. and other contributors,
[CC BY 4.0](https://creativecommons.org/licenses/by/4.0/), from
[Twemoji v17.0.2](https://github.com/jdecked/twemoji/tree/v17.0.2/assets).
Unmodified SVG and PNG sources are vendored in `assets/art/`; the renderer
rasterizes SVGs, scales, layers and animates the artwork.

Typography: **Fraunces**, Undercase Type / Fraunces Project Authors, and
**Lato**, Łukasz Dziedzic / Lato Project Authors; both SIL Open Font License 1.1.
Sources and full licences are in [assets/CREDITS.md](assets/CREDITS.md).
Original code, layout and synthesised music/sound were created for this game.

Model: **openai/gpt-6**.
