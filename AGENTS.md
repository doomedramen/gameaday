# AGENTS.md - GameADay Agent Instructions

## Purpose
This project runs "GameADay" - an AI makes a game every day. When the user says "make todays game", follow the procedure.

## Core Rules
- ALWAYS use Rust
- Each game in its own folder named by date `YYYY-MM-DD/` at repo root
- Games must be self-contained Rust (native preferred). No browser/terminal requirement, but self-contained.
- Must be buildable and runnable
- Must pass e2e tests before marked as ready to play. Playable in one sitting with no bugs for player to report
- Fully autonomous after initial review (feedback) - no further user input during development
- Surprise to player - do not show unfinished details unnecessarily
- Different AI may run each day - read/write state from repo files
- **Try to make a BETTER game than yesterday.** Increase complexity/ambition each day - more systems, polish, mechanics. Push scope gradually but meaningfully.
- **Require more development effort each day.** Don't regress to simpler games; build on learnings and level up.

## Flow

### Start
When user says "make todays game" (or similar):
1. Read `gameaday-state.json` from repo root
2. If `day` is 0 or no previous game -> Day 1. Else -> Subsequent day.

### Day 1
- No feedback questions about previous games
- Start simple/minimal
- Create `YYYY-MM-DD/` folder with today's date (YYYY-MM-DD format)
- Create self-contained Rust game in that folder (Cargo project)
- Build, test e2e until it passes and is solid
- Update `gameaday-state.json`:
  - day becomes 1
  - last_game_date = today's date
  - last_game_folder = "YYYY-MM-DD"
  - add entry to history with summary AND model (MANDATORY - record provider/model or model name used)
- Report: game complete, location, brief description (without spoiling unnecessarily), and model used

### Subsequent Days (>1)
1. Ask feedback about previous game in conversational form. Questions:
- "How fun was yesterday's game (1-10)? Any moments that stood out?"
- "How were the visuals (1-10)? Too plain, just right, or overdone?"
- "Replayability (1-10)? Would you want to replay? Why?"
- "Controls/feel (1-10)? Tight, floaty, confusing, just right?"
- "Biggest gripe or wish?"
- "Anything from yesterday you'd want to see explored differently in a totally new game (not tweaked)?"
2. After getting answers, store feedback in `gameaday-state.json` (last_feedback)
3. Create `YYYY-MM-DD/` with today's date
4. Build a **COMPLETELY NEW** game - never an increment or improvement of previous. Similar themes/styles allowed if it's a different idea/game, not just "make previous better"
5. Fully autonomous during development
6. Buildable, runnable, e2e tests must pass
7. Update state: increment day, update last_game_*, add to history
8. Report done

## State
`gameaday-state.json` structure:
```json
{
  "day": number,
  "last_game_date": string | null,
  "last_game_folder": string | null,
  "last_game_summary": string | null,
  "last_feedback": {},
  "history": [
    {"date": "YYYY-MM-DD", "folder": "YYYY-MM-DD", "summary": "...", "model": "provider/model or name used"}
  ]
}
```

**MANDATORY**: When creating/updating a game, record the model used (provider/model ID if available, or name). Add `"model": "..."` to history entry and optionally to game folder metadata.

## Engine Guidance

Pick deliberately, not by habit. There is no required engine - choose per game and
record the choice in `game/<that day's>game.json`.

Network access to crates.io works; do not run cargo with `--offline` unless you
actually want to be limited to the small local cache.

### Options
- **macroquad** - recommended default when the game needs a window, sprites,
  text, or audio. Small dependency tree, fast compiles, built-in text/shape/audio
  drawing. Good for arcade, action, and 2D games.
- **bevy** - heavyweight ECS engine. Powerful, but cold builds take minutes and a
  version bump or dep conflict can consume an entire day's budget. Only pick it
  when the game genuinely needs large-scale ECS/systems machinery.
- **hand-rolled framebuffer (`minifb`)** - what day 2 used. Total control,
  minimal deps, but you pay for a software rasterizer, pixel font, and audio
  plumbing. Fine for pixel-art games; a poor use of effort for anything else.
- **no engine at all** - correct for pure-logic games (chess, puzzles, card
  games, roguelike deckbuilders, simulations). Just a library plus a thin
  renderer.

### Architecture rule (applies to every game, engine or not)
Keep the simulation pure and engine-free so it stays testable headlessly:

- `src/lib.rs` - all game logic, with no engine types in its public API.
  Rendering, input, and audio must not leak into it.
- `src/main.rs` - a thin adapter: reads input, steps the simulation, draws it,
  plays sound. Talks to the engine; the engine does not leak into `lib.rs`.
- Provide a headless mode (e.g. `--selftest`) that runs the simulation for a
  long simulated session without opening a window, and have an integration test
  drive the shipped binary through it.

This is what makes the e2e gate possible. Day 2's e2e test drove the real binary
headlessly through a boss wave, a death, and a restart - that only works because
the logic was separable from the engine.

### Verify, don't assume
- Confirm the crate actually resolves and builds *before* committing to it for
  the day. Do a throwaway build first.
- Prefer the newest stable release; check the version rather than assuming.
- If a dependency turns out to be a bad fit mid-build, abandon it early rather
  than fighting it all day.

### Boilerplate / template repos (optional)
These are starting points, not requirements. Using one, using a bare engine, or
using no engine at all is entirely the agent's call for each game. Only reach for
a template if it genuinely saves time for that day's game.

Bevy templates:
- https://github.com/TheBevyFlock/bevy_new_2d - opinionated 2D Bevy project
  structure (screens, menus, theme, audio, asset tracking, dev tools) with CI/CD
  and itch.io deploy. Scaffold with `bevy new my_game --template 2d`. MIT/CC0/Apache.
- https://github.com/benfrankel/pyri_new_jam - builds on Bevy New 2D with
  game-jam-oriented extras: more 3rd-party crates, bundled CC0 audio/fonts.
  Scaffold with `bevy new game -t benfrankel/pyri_new_jam`. CC0/0BSD.
- https://github.com/NiklasEi/bevy_game_template - Bevy template with out-of-box
  builds for Windows, Linux, macOS, Web, Android and iOS, plus release workflows.
  CC0. Heavier if you only need a native desktop build.

Notes when using any template:
- They assume Bevy, so they conflict with the "keep it light" advice above. Weigh
  the minutes spent on a Bevy cold build against the time saved.
- Templates bring their own structure and CI config; strip what you do not need
  (especially GitHub workflow files, which do not belong in a dated game folder).
- The architecture rule above still applies: keep the simulation pure and
  headlessly testable regardless of what the template scaffolds.

## Constraints
- Never modify previous games (read-only reference)
- Focus on creating today's game fresh
- E2E quality gate: game must be playable in one sitting without bugs
- Simplicity for Day 1 is important
- No user input after feedback phase
