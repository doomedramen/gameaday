# AGENTS.md - GameADay Agent Instructions

## Purpose
AI makes one game every day. When the user says "make todays game", follow this procedure.

## Core Rules
- **Rust only** - Self-contained Cargo project in `YYYY-MM-DD/` at repo root
- **Brand new every day** - Never iterate, tweak, or expand yesterday's game. Completely different game/genre/mechanic.
- **Beat yesterday** - More impressive and advanced than the previous game. More systems, more polish, larger scope, better feel. Push ambition up every single day.
- **Playable & tested** - Buildable, runnable natively. Must pass e2e tests. Playable in one sitting with no bugs.
- **Fully autonomous** - No back-and-forth. Just make it.
- **Surprise** - Don't spoil WIP, just deliver a finished game.
- **State tracked** - All progress in repo files. Different AI each day must pick up from `gameaday-state.json`
- **Model recorded** - Must log `provider/model` (or model name) in `history[]` for every game

## Engine Guidance
Pick whatever fits best. `macroquad` is often great for fast 2D games. `minifb` is fine for tight pixel games. Bevy only if genuinely justified. No engine is fine for pure logic games. Test dependencies build before committing.

## Daily Flow
1. **Read state** - Load `gameaday-state.json`. Look at `last_game_folder` + `last_game_summary` to gauge the bar.
2. **Pick something new & bigger** - Choose a different concept that's more ambitious than yesterday.
3. **Build from scratch** - Create fresh `YYYY-MM-DD/` folder and build the whole thing.
4. **E2E test** - Build, test, run. Fix until solid and playable with no bugs.
5. **Update state** - Increment `day`, set `last_game_*`, append to `history` with `model` filled in.
6. **Report** - Brief description without spoiling too much. Include location and model used.

## Rules
- Never modify previous games
- Each game in separate date-named folder
- Must e2e pass before done
- Fully autonomous
- Day 1 can be simple/minimal but sets baseline
- Don't regress to simpler games
