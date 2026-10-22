# Lanternfall: self-review (day 3)

Compared against the best previous game, **Nebula Drift** (day 2). Player
ratings for it were fun 7-8, visuals 4-6, replayability 7-8, controls 7-8.

| Axis | Nebula Drift (player) | Lanternfall (self) | Why |
| --- | --- | --- | --- |
| Fun | 7-8 | 7-8 | Real decisions every turn: light, shoves into the void, flanking and high ground. Brutal but fair; the autopilot wins ~15% of runs unaided, so a thinking player should do better. |
| Visuals | 4-6 | 7 | Per-pixel lighting that smoothly follows lanterns and braziers, floating islands with tapered cliffs over a parallax cloud sea, five biome palettes, animated procedural sprites, glow and particles. |
| Replayability | 7-8 | 8 | Procedural islands, 14 boons, route choices, elites, five biomes, a persistent best score. |
| Controls | 7-8 | 7 | Mouse-first with hotkeys, undo, quick-attack, threat preview on hover, damage breakdown tooltip, and a guided first run. |

Scope versus day 2: about 3x the code. New this time: tactical AI,
procedural generation, a meta structure (run, draft, route), a save file,
adaptive music, a tutorial, undo, and a headless app driver that plays the
real UI through synthetic input.

## How it was checked
- `cargo test --release`: 37 tests, including the shipped-binary selftest
  and screenshot mode.
- I looked at all 10 screenshots and iterated: moved the title menu off the
  island, smoothed the cliff undersides, raised ambient light (the boss island
  was nearly black), changed the per-round banner into a small toast, and fixed
  log grammar.
- Balance was tuned with the autopilot, which:
  - fixed endless stalemates by adding the long night
  - nerfed Wisps after they dealt about half of all squad damage
  - flattened a difficulty spike on islands 4-5
- The windowed build was launched and ran stably (title screen at ~40% of one
  core, about 7.8 ms per battle frame).

## Known weaknesses
- Balance was tuned against a greedy bot, not humans. Players may find
  islands 4-5 a wall, or find the game easier than the bot does.
- I could only verify the music numerically (layers add energy, nothing
  clips, every effect is audible). I never heard the actual mix.
- The sprites are small relative to the tiles, and the 8x8 bitmap font is
  readable but plain.
- No settings menu, the window can't be resized, and audio needs an f32
  output device (other formats run silently).
- Pushes only go in four directions, and there's no line of sight for ranged
  attacks.
