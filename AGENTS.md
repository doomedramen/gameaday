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
  - add entry to history with summary
- Report: game complete, location, brief description (without spoiling unnecessarily)

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
    {"date": "YYYY-MM-DD", "folder": "YYYY-MM-DD", "summary": "..."}
  ]
}
```

## Constraints
- Never modify previous games (read-only reference)
- Focus on creating today's game fresh
- E2E quality gate: game must be playable in one sitting without bugs
- Simplicity for Day 1 is important
- No user input after feedback phase
