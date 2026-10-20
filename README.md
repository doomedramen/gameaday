# GameADay

AI makes one game per day.

## Usage
Say "make todays game" in any AI session (different model each day is fine). 

- Day 1: starts simple/minimal, no feedback questions.
- Subsequent days: asks about previous game, then makes a completely new game (not an increment).

Each game goes in `YYYY-MM-DD/` as a self-contained Rust project. Must be buildable, runnable, e2e tested, and playable in one sitting.

State tracked in `gameaday-state.json`.
