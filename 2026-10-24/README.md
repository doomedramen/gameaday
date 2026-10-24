# Cragswing - GameADay 2026-10-24

A rope-swinging platformer roguelite. Wren climbs the Cragspire to relight the
Beacon: five procedurally generated ascents, a boon after each, and the
Stormheart at the summit.

**The idea: momentum is a weapon.** Swing low, let go at the bottom of the arc
and Wren shatters glass doors, rams armoured guards and cracks the Stormheart.
Slow, she bounces off.

## Run

```sh
cargo run --release
```

Software-rendered at 1280x720 with real-time synthesised audio (CoreAudio via
`cpal`). Progress (bests, Workshop perks, options, hints seen) is saved to
`~/.cragswing/save.txt` (override with `CRAGSWING_SAVE`).

## Controls

| Action | Input |
| --- | --- |
| Run | `A` `D` / arrows |
| Jump (hold for height) | `Space` / `K` / `Z` / `C` |
| Wall slide, wall jump | press into a wall, then jump |
| Hook | hold **left click** on a glowing anchor (aim assist helps) |
| Hook without a mouse | hold `Shift` / `X` / `J` (auto-aims up and forward) |
| Pump the swing | `A` `D` while hooked |
| Reel in / out | `W` / `S` while hooked |
| Let go | release the button, or `Space` to hop off |
| **Rewind time** | hold `R` / right click (spends Echo) |
| Pause, assists | `Esc` / `P` |
| Help, mute | `H`, `M` |
| Full screen | `F` or `F11` toggles a borderless full-screen window (16:9, letterboxed) |

## How it plays

- **Ascents**: each is generated from traversal pieces (runs, jump chains,
  stairs, wall-jump shafts, rope chasms with brittle and sliding anchors,
  crumbling bridges, drops, glass-door tunnels) and then seeded with hazards.
- **Foes**: Skitters (stomp), Bulwarks (armoured: ram or stomp twice),
  Spitters (red aim line), Sawblades (rails), Geysers (rumble first), Wisps,
  lightning and gusts. Every ascent adds something new.
- **Momentum**: above ~440 px/s Wren glows and trails. At that speed glass
  shatters and foes are rammed. Training teaches it with a mandatory glass door.
- **Flow**: crystals, hooks and kills build a combo (up to x6) that fades in a
  few seconds.
- **Rewind**: hold `R` to scrub back through hits, falls and even a fatal blow.
  Echo refills at checkpoints and from orbs and cannot be farmed.
- **The run**: after each ascent pick one of three boons (15 in all) and
  choose a Steady route (heal a heart) or a Daring one (harder, crystals x1.5,
  an extra boon). Chests on springbloom ledges grant extra boons.
- **Medals**: beat the par time for Gold. Best times are saved per ascent.
- **The Workshop**: crystals bank as glimmer for permanent perks.
- **Heat**: winning unlocks Heat 1-3: harder ascents, fewer hearts and Echo.
- **Assists**: aim assist, game speed (100/85/70%) and Explorer mode (no
  damage, results not recorded) are on the title and pause screens.

Biomes: Mossbluff, Glimmerdeep (dark, lantern-lit), Frostspire (ice, gusts),
Cinderworks (geysers, molten drop), Stormcrown (lightning, updrafts), and the
Beacon summit.

## Tech

All logic is engine-free in the library:

- `sim` (Wren's movement, rope, foes, hazards), `level` and `stagegen`
  (tile grid and the ascent generator, which also records traversal "beats"),
  `boss`, `game` (rewind), `run` and `save`;
- `bot`: the autopilot. It follows the beats, searches rope-swing timings by
  simulating ahead in cloned worlds, and dodges hazards by trying short
  macros. It is the solvability oracle for the generator;
- `gfx`: CPU renderer with a tile atlas, parallax backdrops per biome, a
  light-map pass for caves, procedural sprites, particles and UI;
- `audio`: synthesised effects and an adaptive per-biome score;
- `app`: screens, menus, input and the run flow; `selftest`.

`src/main.rs` is a thin `minifb` window and `cpal` audio adapter.

## Testing

```sh
cargo test --release
```

- `tests/game.rs`: rules on hand-built levels (jump tuning, coyote time, wall
  jumps, rope, glass, stomps and rams, spikes, pits, spitters, geysers, shield
  and phoenix, rewind, the boss's speed rule, generator invariants over 200
  levels).
- `tests/app.rs`: screens and menus through synthetic input.
- `tests/binary_e2e.rs`: runs the shipped binary: `--selftest` (autopilot
  clears every sampled ascent and the boss, difficulty and content pacing,
  whole runs under normal rules, the real app driven through keys and mouse to
  victory with a save file check, rewind) and `--screenshot`.

```sh
cargo run --release -- --selftest
cargo run --release -- --screenshot screenshots
```
