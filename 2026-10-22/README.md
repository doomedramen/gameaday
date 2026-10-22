# Lanternfall — GameADay 2026-10-22

A turn-based tactics roguelite on floating isometric islands. Lead three
lantern-bearers (a Warden, a Ranger and a Lamplighter) across seven
procedurally generated islands taken by the dark, and end it at the Hollow
King's throne.

Light decides fights. Foes standing in darkness are **shrouded** and take half
damage, so your lanterns, braziers and flares matter as much as your blades.
And the void below every island is always hungry.

## Run

```sh
cargo run --release
```

Software-rendered at 1280x720 with real-time synthesised audio (CoreAudio via
`cpal`). Progress (best score, furthest island, runs, wins, tutorial) is saved
to `~/.lanternfall/save.txt` (override with `LANTERNFALL_SAVE`).

## Controls

| Action | Input |
| --- | --- |
| Select squad member | click it / its card, or `Tab` |
| Move | click a blue tile (`U` undoes a move) |
| Use ability | `1` `2` `3` or click it, then click a target |
| Quick attack | click a foe in range |
| Inspect a foe's reach | hover it |
| Toggle all danger zones | `D` |
| End turn | `Space` / `Enter` / End Turn button |
| Cancel | `Esc` / right click |
| Help, mute | `H`, `M` |

## How it plays

- Each squad member can **move once and act once** per turn, in any order.
- **Light:** each hero carries a lantern; braziers (kindled by the Lamplighter)
  and flares light more. Shrouded foes take half damage.
- **Shoves:** Shield Bash, Slam and Rend push. Into the void or water: instant
  death. Into walls or other units: extra damage.
- **Positioning:** +1 damage from higher ground or with a flanking ally; ranged
  attacks reach one tile further from high ground.
- **Pressure:** more shades every 4 rounds, the gloom dims lanterns at rounds 5
  and 9, and from round 14 the long night wounds the squad every round.
- **The run:** after each win, pick one of three boons (two after an Elite
  Hunt), then choose your next island: a Skirmish, an Elite Hunt or a
  Sanctuary. Island 7 is the throne.

Enemies: Shade, Wisp, Snuffer, Gloam Brute, Hexer, and the Hollow King.
Biomes: Mossmere, Ashen Reach, Rimeholt, Glasswild, the Hollow Throne.

## Tech

- `src/lib.rs`: everything engine-free. `map` (noise islands with
  connectivity checks), `battle` (rules, light, events), `ai` (tile × ability
  × target scoring), `run`, `save`, `bot` (playtest autopilot), `gfx`
  (CPU renderer: per-pixel lit isometric tiles, parallax sky, procedural
  sprites, particles, UI), `audio` (synth SFX and an adaptive score), `app`
  (screens, input, tutorial), `selftest`.
- `src/main.rs`: a thin `minifb` window and `cpal` audio adapter.

## Testing

```sh
cargo test --release
```

- Unit tests: RNG, map generation (300 seeds), save files, run flow, canvas,
  and every sound effect / music layer.
- `tests/game.rs`: rules on hand-built scenarios (shroud, shoves, the King's
  aura, kindle/snuff, undo, cooldowns, flanking, pickups, long night, AI,
  reinforcements, outcomes, fair spawns across 240 islands).
- `tests/binary_e2e.rs`: runs the shipped binary with `--selftest`, which:
  - plays 16 full runs with the autopilot, checking that difficulty ramps, new
    enemy types appear, nothing stalls, and wins are possible but rare
  - plays the real app through synthetic clicks and keys to a victory and a
    second full run, including undo, help and restart
  - checks that the save file persisted

  It then runs `--screenshot` and checks the PNGs.

```sh
cargo run --release -- --selftest
cargo run --release -- --screenshot ./screenshots
```
