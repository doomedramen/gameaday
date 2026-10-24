# AGENTS.md - GameADay Agent Instructions

This is the single source of truth. README.md and PROCEDURE.md point here.

## Purpose
AI makes one game every day. When the user says "make todays game", follow the Daily Flow below.

## Core Rules
- **Rust only** - Self-contained Cargo project in `YYYY-MM-DD/` at repo root.
- **Brand new every day** - Never iterate, tweak, or expand a previous game. Never modify previous games.
- **Played games are frozen** - Once the user has played a game it is never changed, whatever the feedback says (fun, feel, balance, art). The only exception is a game-breaking bug (crash, unusable input or UI, unwinnable or softlocked), fixed minimally. Feedback goes to `history[].feedback` and `lessons.md` only.
- **Fresh art every day** - Do not reuse the previous days' procedural look. Use third-party assets (itch.io, OpenGameArt, Kenney and similar packs; fonts, sprites, tilesets, audio) wherever they fit, and pick a different style from recent games. Check the licence allows redistribution, vendor the files into the game folder under `assets/`, and credit author, source URL and licence in the game's `README.md` and `assets/CREDITS.md`. Record the art style in `PITCH.md` and make sure it differs visibly from the last two games.
- **Beat the best so far** - Not just yesterday: every game must be more impressive than the best game in `history[]`. More systems, more content, more polish, better feel.
- **Never too similar** - Novelty rules in `novelty.json` are enforced by `scripts/check-state.py`.
- **Ratchet, don't regress** - Every capability in `bar.json` ships again (or is explained as not applicable), plus at least 2 new ones.
- **Playable & tested** - Buildable, runnable natively, e2e tests pass, playable in one sitting with no bugs.
- **Fully autonomous** - No back-and-forth. Just make it. Surprise the player; don't spoil WIP.
- **State tracked** - Everything a different model needs tomorrow lives in repo files.
- **No recreations** - Original games within a genre; never a copy of a specific existing game.

## Repo Files
| File | What it is |
| --- | --- |
| `gameaday-state.json` | `day` + `history[]`. The last entry is yesterday's game. |
| `novelty.json` | Genre pool, tag vocabulary for 7 dimensions, cooldown rules. |
| `bar.json` | Every capability any game has shipped. Only grows. |
| `lessons.md` | General design lessons distilled from player feedback. Read it every day. |
| `scripts/check-state.py` | Validates all of the above. Runs in pre-commit and pre-push. |

## Daily Flow
1. **Read** `AGENTS.md`, `gameaday-state.json`, `bar.json`, `lessons.md`, `novelty.json`. Look at the best-rated previous game's folder (README, screenshots) to calibrate the bar.
2. **Pick** a genre and tags that pass the novelty rules (genre not used in 14 days, core verb not in 7, at least 4 of 7 tag dimensions differ from yesterday). Prefer genres never used. If the honest pitch is "previous game X but with Y", pick again.
3. **Pitch** - write `YYYY-MM-DD/PITCH.md` before code:
   - one-paragraph concept and the 7 tags
   - the closest previous game and 3 ways today's game *plays* differently
   - which `bar.json` capabilities ship, which are not applicable (and why)
   - at least 2 new "firsts" (e.g. save file, tutorial, procedural generation, meta-progression, AI opponent, level editor, dynamic music, lighting, multiple biomes)
   - which `lessons.md` entries apply and how the design honours them
   - art style and asset sources (must look different from the last two games; see Fresh art every day)
   - focus axis: the weakest-rated axis in recent feedback (fun / visuals / replayability / controls) gets extra effort
4. **Build** in phases: playable core -> content -> dedicated polish pass. Polish checklist: juice on every key event, audio mix (important events loudest), onboarding/controls on screen, readable UI, clear end states, no dead time.
5. **Test**
   - Architecture: all logic in `src/lib.rs` with no engine types; `src/main.rs` is a thin adapter.
   - `--selftest`: headless long session driven by an integration test against the shipped binary.
   - **Playtest assertions** in that run: difficulty measurably rises, new content/mechanics still appear after ~15 simulated minutes, win/lose reachable, no stuck states, restart works.
   - `--screenshot <dir>`: renders at least 4 key moments (title, early play, busiest moment, end state) to PNG. Commit them in `YYYY-MM-DD/screenshots/`.
6. **Look** at every screenshot yourself. Fix anything plain, unreadable, cluttered, or ugly, then regenerate.
7. **Review** - write `YYYY-MM-DD/REVIEW.md`: honest 1-10 self-score per axis vs the best previous game, known weaknesses. If it doesn't beat the best so far, keep working.
8. **Update state**
   - Append a `history[]` entry (schema below); increment `day`.
   - Add the new firsts to `bar.json` with `first_date` = today.
   - Add any new tag vocabulary to `novelty.json` (with description; never remove or rename).
   - Run `python3 scripts/check-state.py` until it passes.
9. **Report** - brief, non-spoiling description, folder, and model used.

## Feedback
The agent never asks for feedback. If the user gives feedback on a game (in the prompt or later):
- store it verbatim in that game's `history[].feedback` (`fun`, `visuals`, `replayability`, `controls`, `gripe`, `wish`)
- distil it into **general** rules in `lessons.md` (never "fix game X")

## History Entry Schema
```json
{
  "date": "YYYY-MM-DD",
  "folder": "YYYY-MM-DD",
  "title": "Name",
  "summary": "Name - one line of what it is and its main systems",
  "model": "provider/model",
  "tags": {
    "genre": "...", "core_verb": "...", "perspective": "...", "pacing": "...",
    "input": "...", "structure": "...", "win_condition": "..."
  },
  "capabilities": ["every bar.json id this game ships"],
  "not_applicable": { "bar-id": "why it doesn't fit this genre" },
  "firsts": ["bar.json ids that debut today (min 2)"],
  "feedback": {}
}
```

## Engine Guidance
Pick whatever fits best. `macroquad` is often great for fast 2D games. `minifb` is fine for tight pixel games. Bevy only if genuinely justified. No engine is fine for pure logic games. Test that dependencies build before committing to one. PNG output for screenshots can use the `png` crate or a hand-written encoder.

## Quality Gates
- pre-commit: `cargo fmt --check`, `clippy -D warnings`, unit tests; `check-state.py` when state files are staged.
- pre-push: full `cargo test --release` (incl. binary e2e), release build, `check-state.py`.
- Rules in `novelty.json` / `bar.json` / `check-state.py` apply to games from `novelty.json` `enforced_from` onward; earlier games are grandfathered.
