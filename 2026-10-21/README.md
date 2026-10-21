# Nebula Drift — GameADay 2026-10-21

A top-down arena survival shooter. You pilot a lone ship in a drifting nebula,
fending off endless escalating waves of hostiles — including a **Warden boss**
every fifth wave.

## Run

```sh
cargo run --release
```

> Release mode is strongly recommended: the software renderer does per-pixel
> lighting and runs far smoother optimised.

## Controls

| Action        | Keys                          |
| ------------- | ----------------------------- |
| Thrust        | `WASD` / arrow keys           |
| Aim           | mouse                         |
| Fire          | left click / `Space`          |
| Dash          | `Shift` / right click         |
| Energy pulse  | `E` / middle click (full meter) |
| Start/restart | `Enter` / `Space`             |
| Quit          | `Esc`                         |

## How it plays

- Movement is **thrusty** — you carry momentum, and the ship bounces off the
  arena walls. Dashing is a burst you can use to escape or to close distance.
- Enemies drop **energy orbs**. Collect them to fill the pulse meter; a full
  meter lets you fire a **screen-clearing shockwave** that damages everything
  nearby.
- Killing enemies quickly builds a **combo multiplier** (up to a point), which
  multiplies all score gains.
- Bosses drop **power-ups**: `REPAIR` (heal), `RAPID` (double fire rate for a
  few seconds) and `CHARGE` (instant full pulse meter).
- Every fifth wave is a **Warden** fight — a huge, orbiting hex-hull that
  hovers at range and spits slow homing projectiles.

### Enemy roster

| Enemy     | Behaviour                                              |
| --------- | ------------------------------------------------------ |
| Drifter   | slow, steady chaser, 1 HP                              |
| Darter    | fast zig-zag weaver, 1 HP                              |
| Bloater   | slow tank, 3 HP, **splits into two Drifters** on death |
| Warden    | boss, 40+ HP, hover-and-spit, drops three power-ups    |

## Visuals & audio

- Everything is drawn on the CPU into an `u32` framebuffer: soft-edged
  circles, rings, rotated triangles, a hand-built 5×7 pixel font, a parallax
  starfield with drifting nebula dust, and an ambient vignette.
- Juice: screen shake, full-screen damage flashes, hit-stop-style enemy
  knockback, thruster trails, particle bursts on every kill, spawn-in scale
  animation, and pulsing/rotating sprite animation on every enemy type.
- Audio is **dependency-free**: short WAV clips are synthesised at startup
  (chirps, noise bursts, saw/square voices) and played through macOS
  `afplay`. If audio is unavailable the game silently runs without it.

## Testing

```sh
cargo test --release
```

Three layers:

- **`src/lib.rs` logic tests** — spawning, combat, wave progression, boss
  gating, power-ups, combos, damage, restart, determinism.
- **`src/render.rs` / `src/audio.rs` unit tests** — framebuffer coverage, view
  transforms, glyph geometry, WAV encoding, non-silent synth voices.
- **`tests/binary_e2e.rs`** — runs the real shipped binary with `--selftest`,
  which headlessly drives simulation *and* rendering for a scripted 400-second
  run, then verifies a boss wave, the boss bar, death screen and restart all
  render correctly and that no framebuffer pixels are left unwritten.

Extra diagnostic modes (not needed for normal play):

```sh
cargo run --release -- --selftest            # headless end-to-end check
cargo run --release -- --screenshot ./shots  # render representative frames
```
