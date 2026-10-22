# GameADay Procedure

## Trigger
When user says "make todays game" (or similar), follow this procedure.

## Steps
1. Read `gameaday-state.json` from repo root
2. Look at `last_game_folder` and `last_game_summary` to understand yesterday's game and the bar to beat
3. Pick a completely new game - different genre/mechanic, not an iteration. Make it more impressive and advanced than yesterday
4. Create folder `YYYY-MM-DD/` at repo root using today's date
5. Build a self-contained Rust game in that folder from scratch
6. Test thoroughly - buildable, runnable, e2e tests pass, playable in one sitting with no bugs
7. Update `gameaday-state.json`: increment `day`, set `last_game_date`, `last_game_folder`, `last_game_summary`, append entry to `history` with `model` (provider/model or name used)
8. Report done with location and brief description, include model used

## Rules
- Always Rust
- Each game in separate date-named folder
- Never increment previous game
- Must beat yesterday in ambition/polish/systems
- Must e2e pass before done
- Fully autonomous
- Surprise to player
