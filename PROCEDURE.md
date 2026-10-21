# GameADay Procedure

## Trigger
When user says "make todays game" (or similar), follow this procedure.

## On start
1. Read `gameaday-state.json` from repo root
2. Determine if this is day 1 (day == 0 or no previous game) or subsequent day

## Day 1
1. Do NOT ask feedback questions (no previous game)
2. Create folder `YYYY-MM-DD/` at repo root using today's date
3. Create a minimal/simple self-contained Rust project in that folder
4. Make it buildable and runnable
5. E2E test it thoroughly - must be playable in one sitting without bugs
6. Update `gameaday-state.json`: increment day to 1, set last_game_date, last_game_folder, add to history
7. Report done - game is ready, surprise for player

## Days > 1
1. Read last game info from state
2. Ask feedback questions about previous game (conversational):
   - Fun (1-10 + notes)?
   - Visuals (1-10 + notes)?
   - Replayability (1-10 + notes)?
   - Controls/Feel (1-10 + notes)?
   - Biggest gripe/wish (open)?
   - Anything to explore differently in a totally new game (not tweaked)?
3. Store feedback in state
4. Create folder `YYYY-MM-DD/` at repo root using today's date
5. Design and build a COMPLETELY NEW game (not an increment/improvement). Similar themes/styles OK if fundamentally different idea.
6. Self-contained Rust, native preferred
7. Buildable, runnable
8. E2E test thoroughly - playable in one sitting, no bugs
9. Fully autonomous after feedback - no further user input
10. Update state: increment day, set last game, save feedback, add to history
11. Report done - surprise for player

## Rules
- Always Rust
- Each game in separate date-named folder under root
- Never increment previous game
- Must e2e pass before done
- Fully autonomous after initial feedback (or immediately for day 1)
- Game is surprise to player
- Different AI each day - state must be in repo files
- Day 1 must be simple/minimal
- **Always try to beat yesterday** - make a better, more advanced game each day with more systems/polish
- **Increasing ambition** - each day's game should require more development effort than previous day's game
- **MANDATORY**: Record model used (provider/model or name) in state history for every game
