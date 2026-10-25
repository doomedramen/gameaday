# Slipstream Cup

A top-down arcade racing championship (day 6 of GameADay, 2026-10-25).

Drive the orange car against six rivals with real personalities around
procedurally generated circuits. Drift through bends to charge mini-turbos,
tuck in behind anyone to ride their **slipstream** and fill your nitro, then
slingshot past. Qualify through four venues you choose between, upgrade the
car between rounds, and beat the reigning Champion at the Grand Prix.

## Run

```
cargo run --release
```

Headless checks: `cargo run --release -- --selftest`, screenshots:
`cargo run --release -- --screenshot screenshots`.

## Controls

| Key | Action |
| --- | --- |
| UP / W | accelerate (press as the lights go out for a rocket start) |
| DOWN / S | brake, reverse |
| LEFT RIGHT / A D | steer |
| SPACE (hold) | drift - sparks go blue, orange, purple; release for a mini-turbo |
| SHIFT (hold) | nitro |
| ESC / P | pause (resume, restart, settings, quit) |
| ENTER | confirm, skip the cool-down lap |
| R | restart a Time Trial |
| H | help, M mute, F fullscreen |

## Features

- Arcade car physics: grip-limited cornering, handbrake drifts with three
  mini-turbo tiers, nitro, slipstream tow, surfaces (asphalt, kerbs, grass,
  sand, mud, oil), damage that costs top speed, rocket starts and stalls.
- Procedural closed-loop circuits with separation, curvature, length and
  start-straight checks; a minimum-curvature racing line; pace notes.
- Five venues: Greenvale, Dune Sea, Timberline, Neon Night (headlights and
  lamp posts light the track) and the Grand Prix.
- Rivals on the same physics: Rocket, Drifter, Bully (telegraphs then rams),
  Blocker, Wasp (bike), Ace and the Champion; they draft, overtake, defend,
  dodge hazards and use nitro.
- The Cup: five rounds, route choice, a pick-one-of-three parts draft (14
  parts), a garage with undo, prize money, points table, two second chances.
- Style chains (drifts, slingshots, close calls, overtakes, cones) with a
  multiplier that pays cash and fills nitro.
- Time Trial on five classic circuits with persisted ghost laps and medals
  calibrated against the AI's own lap.
- Adaptive synth score per venue with a final-lap lift, synthesised engines
  with gear changes, sampled impacts and UI sounds, loudness classes.
- Assists: steering help, auto-throttle, game speed 70/85/100%, screen shake.
- Save file at `~/.slipstream-cup/save.txt` (override with `SLIPSTREAM_SAVE`).

## Credits

Art, fonts and sampled sounds: Kenney (CC0) - see `assets/CREDITS.md`.
Code, music and engine sounds: generated for this game.
