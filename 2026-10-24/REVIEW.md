# Cragswing: self-review (day 5)

Compared against the best previous game, **Lanternfall** (day 3). Player
ratings for it: overall 7/10, "very fun", art style and music loved, "movement
tricky, easy to get stuck". Night Signal (day 4) had no feedback yet.

| Axis | Lanternfall (player) | Cragswing (self) | Why |
| --- | --- | --- | --- |
| Fun | 7 | 8 | One idea carries the whole game: speed. Swinging builds it, spending it breaks glass doors, armour and the boss. Real-time movement with a skill ceiling, stomps, a combo, and a boss that cannot be cheesed. Unverified by a human. |
| Visuals | loved | 8 | Five lit biomes with parallax, weather and glow, a per-pixel lantern light map in the caves, a procedurally animated Wren with a fluttering scarf, telegraphs for every hazard. Wren is small at 720p. |
| Replayability | will replay | 8 | Procedural ascents, 15 boons, route choice, Workshop perks, Heat 1-3, medals with saved best times, a runnable boss. |
| Controls | tricky | 8 | Coyote time, jump buffer, variable jump, corner correction, forgiving wall jumps, sub-pixel collision, aim assist, a keyboard-only hook, rewind, assists, one-time hints. Never felt with human hands. |

Scope versus day 3: about 1.5x the code (15.5k lines including tests). New this
time: rope physics, rewind, a boss with scored attack choice, a generator that
records traversal beats for an autopilot that plans by simulating ahead,
workshop perks, medals and best times, assists, full-screen scaling.

## How it was checked
- `cargo test --release`: 45 tests (16 unit, 21 rules on hand-built levels,
  6 app flows, 2 shipped-binary tests).
- `--selftest` (about 18 s):
  - the autopilot cleared every sampled ascent (14/14 in each of the five
    biomes) and the Stormheart (4/4);
  - threat density per 100 tiles rises 0.5 / 1.3 / 0.5 / 2.3 / 3.4 and every
    ascent after the first introduces a new foe (Spitters and Wisps, gusts,
    Sawblades and Geysers, lightning);
  - at a typical (bronze) pace a run is about 14 minutes and the last new
    content arrives at about 11.5 minutes;
  - whole runs under normal rules: the bot (using rewind) wins about 15 of 24,
    13 of 24 at Heat 3; losses are common in Cinderworks and Stormcrown;
  - the real `App` driven through keys and mouse reaches victory, saves its
    bests, and shows every screen.
- A wider offline sweep (not part of the shipped tests): the invincible bot
  clears 195-196 of 200 generated ascents; the leftovers are timing combinations
  it cannot find, not provably impossible.
- I looked at all 15 screenshots and fixed what I saw: boss framing (the arena
  was too tall to show Wren and the boss), washed-out wind streaks, a clipped
  toggle label, an ugly medal glyph, missing prompts.
- Frame cost (software renderer): 4-10 ms per frame depending on biome after
  caching cloud textures and ridge gradients. The windowed binary was started
  and ran stably; one benign audio underrun is reported at launch.

## Bugs the tests and the bot found (and fixed)
- Collision stepped in 3 px chunks and could stop up to 3 px above the floor, so
  the ground probe failed and Wren could not jump right after landing on narrow
  ledges. Now it refines to sub-pixel contact.
- Cave stalactite generation filled the glass-door tunnels with rock.
- Wisps and bonus ledges spawned inside rock; a level end could clip a gate.
- Dodge macros wrecked precise crossings and made the bot dither.

## Known weaknesses
- No human has played it. Balance was tuned against a bot that is better at
  frame-perfect timing and worse at foresight than people, so Cinderworks and
  Stormcrown may feel harsher (or the bosses easier) than the numbers say.
- I could only check the audio numerically (every effect audible, layers add
  energy, important events louder, nothing clips). I never heard the mix.
- The Stormheart is the least battle-tested part: three hits, a pity timer, and
  the swing-through-the-anchor solution is explained on screen but is new to
  people. The fight is short for a skilled player.
- Wren and enemies are small relative to the 32 px tiles; the 8x8 bitmap font is
  readable but plain.
- Vaults and secret rooms were cut: sealed chambers either trapped the bot or
  needed awkward exits. Glass doors are mandatory set pieces instead.
- About 2% of generated ascents defeat the autopilot; the shipped tests only
  require 85% so a rare level may need a few retries to solve.
- Full screen relies on the window's own button (minifb has no toggle); the
  frame letterboxes and keeps 16:9.
- Saves are plain text with no versioning beyond ignoring unknown keys.
