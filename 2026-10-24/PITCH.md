# Cragswing - pitch (day 5, 2026-10-24)

## Concept
Wren, a courier with a grappling line, climbs the Cragspire to relight the
Beacon that the Stormheart smothered. A real-time side-view **platformer
roguelite** built around one systemic idea the player discovers: **momentum is
a weapon**. Swing on rope anchors, let go at the bottom of the arc, and a fast
Wren shatters glass doors, rams armoured guards and cracks the boss's shell; a
slow Wren just bounces off. A run is five procedurally generated ascents
(Mossbluff, Glimmerdeep, Frostspire, Cinderworks, Stormcrown) and the
Stormheart boss. Between ascents: pick one of three boons and choose a Steady
or Daring route. Hold a button to **rewind time** a few seconds when it goes
wrong.

Tags: genre `platformer`, core_verb `swing` (new), perspective `side-view`,
pacing `real-time`, input `keyboard+mouse`, structure `run-based-roguelite`,
win_condition `reach-goal`.

## Closest previous game and how this plays differently
Closest: **Nebula Drift** (real-time, keyboard+mouse, momentum) or
**Lanternfall** (roguelite run: draft, route, biomes, lighting).
1. Traversal, not combat: you cross hand-shaped terrain with jumps, wall jumps
   and a rope pendulum; there is no shooting at all. Foes are obstacles you
   stomp, ram or time.
2. Speed is the resource: the same body that crosses a chasm also breaks glass
   doors, armour and the boss's shell, so route and rhythm matter more than aim.
3. Time is rewindable: a held button scrubs the world back (player, enemies,
   broken blocks, score), unlike turn-based undo or a single-screen arena.

## bar.json capabilities
Shipped: core-loop, win-lose-states, health-lives-ui, title-screen,
session-best-score, multiple-enemy-or-piece-types, boss-or-climax,
difficulty-progression, pickups-or-powerups, combo-or-chain-scoring,
particles-vfx, screen-shake-juice, animated-sprites,
parallax-or-layered-background, synth-audio-sfx, headless-selftest,
screenshot-mode, procedural-level-generation, tactical-ai-opponent (the boss
scores the player's position and state to pick its next attack; spitters lead
their shots), persistent-save-file, in-run-upgrade-draft, route-choice,
multiple-biomes, dynamic-lighting (a lantern-lit cavern and lava glow),
adaptive-music, tutorial-onboarding, undo-move (rewind), threat-preview
(telegraphed bolts, saw rails, strikes, geysers, boss patterns),
autopilot-playtest, persistent-campaign-upgrades (glimmer banks into a
Workshop of permanent perks).

Not applicable: broadcast-programming, power-grid-allocation,
community-trust-network (Night Signal's radio-management systems have no
analogue in a platformer).

## New firsts (4)
- `rope-physics-traversal`: pendulum rope with reel, pumping and drag;
  brittle, rail and (boss) anchors. It is the movement, and the enabler of the
  momentum idea.
- `time-rewind`: hold-to-scrub rewind of the whole simulation with an Echo
  meter (orbs and checkpoints refill it; pickups cannot be farmed by rewinding).
- `accessibility-assists`: aim assist, game speed (100/85/70%), explorer
  (invincible) mode, a keyboard-only hook and a pause menu to change them.
- `time-trial-medals`: per-ascent par times, bronze/silver/gold medals and
  persisted best times.

## lessons.md applied
- *Discoverable systemic idea*: momentum smashes glass doors (mandatory in
  training and a recurring set piece from Ascent 2), rams armoured Bulwarks and
  is the only way to hurt the boss. Taught in training, hinted once in play.
- *Real music*: adaptive score per biome, layers follow speed and danger, boss
  theme, ambience beds.
- *Movement must feel free and readable* (the day-3 gripe): coyote time, jump
  buffering, corner correction, variable jump, wall slide, forgiving wall jump,
  sub-pixel collision, aim assist, a keyboard-only hook, glowing in-range anchors.
- *Players outside the genre*: guided training ascent, one-time hints for each
  hazard, rewind, assists.
- *Longevity*: new foes every ascent, a boss, 15 boons, Heat levels after a win,
  Workshop perks, medals and best times.
- *Visual bar*: lit scenery, per-biome palettes, parallax, glow, weather,
  procedural animated Wren with a fluttering scarf.
- *Balance with a bot*: a sim-lookahead autopilot plays every generated ascent;
  tests assert solvability, difficulty ramp, content pacing and no stuck states.
- *Boss cannot be cheesed*: the shell only cracks when stunned *and* hit at
  speed; slams land on moving spots; a pity timer guarantees openings.

## Focus axis
Controls (weakest in the latest rated feedback: "movement tricky, easy to get
stuck") plus visuals (weakest on day 2). Extra effort on game feel and polish.
