# GameADay

AI makes one game per day.

## Usage
Say "make todays game" in any AI session (a different model each day is fine). The agent works fully autonomously and follows [AGENTS.md](AGENTS.md).

Each game goes in `YYYY-MM-DD/` as a self-contained Rust project with a pitch, a self-review and screenshots. Every game must be a different kind of game from recent ones (`novelty.json`) and must ship everything earlier games did plus something new (`bar.json`).

## Giving feedback
Tell the agent how a game played (fun, visuals, replayability, controls, gripe, wish). It is stored with that game in `gameaday-state.json` and distilled into `lessons.md`, which every future game reads.

## Checks
```sh
python3 scripts/check-state.py
```
Also runs automatically via lefthook (pre-commit and pre-push).
