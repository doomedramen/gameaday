# Slipstream Cup - pitch (day 6, 2026-10-25)

## Concept
A top-down arcade **racing** championship. You drive the red car against seven
rivals with distinct personalities around procedurally generated circuits in
five venues (Greenvale, Dune Sea, Timberline, Neon Night and the Grand Prix
final against the reigning champion). Handling is the game: throttle, brake,
steer, and a **handbrake drift** that charges a mini-turbo (blue, orange,
purple sparks) released on exit. The systemic idea to discover is the
**slipstream**: tuck in behind any car and the air pocket pulls you along and
fills your NITRO meter, so the car in front is both an obstacle and fuel; the
best overtakes are "slingshots" out of a draft. Rivals draft too - including
you. Between races: pick one of three car parts (draft), spend prize money in
the garage, and choose which venue to race next. Finish in the qualifying
places or burn one of your two second chances; win the Grand Prix to take the
Cup. A Time Trial mode on five classic circuits records your best lap as a
**ghost** to chase, with medals calibrated against the AI's perfect lap.

Tags: genre `racing`, core_verb `steer`, perspective `top-down`, pacing
`real-time`, input `keyboard`, structure `campaign-meta`, win_condition
`defeat-opponent`.  (6 of 7 dimensions differ from Cragswing.)

## Closest previous game and how this plays differently
Closest: **Nebula Drift** (top-down, real-time, momentum ship).
1. You race *with* the field, not against waves: position, overtakes and lines
   matter, nothing is shot; contact is a tool (rivals ram, block and draft).
2. The track is the challenge: corners, surfaces (asphalt, sand, dirt, kerbs,
   oil, mud) and braking points reward a learned racing line, i.e. speedrun
   skill, rather than survival.
3. Opponents are other drivers with the same physics and rules as you, each
   with a style (Rocket, Drifter, Bully, Blocker, Wasp bike, Champion), and the
   slipstream turns them into a resource.

## bar.json capabilities
Shipped: core-loop, win-lose-states, health-lives-ui (damage, nitro, position,
lap, second chances), title-screen, session-best-score, multiple-enemy-or-piece
-types (5 rival personalities + champion), boss-or-climax (the Grand Prix vs
the Champion), difficulty-progression (rival pace and aggression rise every
race; qualifying gets stricter), pickups-or-powerups (nitro cans, repair
wrenches, car parts), combo-or-chain-scoring (style chain: drifts, near
misses, slingshots, cone carnage), particles-vfx, screen-shake-juice,
animated-sprites (spectators, flags, suspension bounce, flames),
parallax-or-layered-background (tree canopies and tribunes above the cars with
height parallax, drifting cloud shadows), synth-audio-sfx (synth engines and
skids alongside sampled impacts), headless-selftest, screenshot-mode,
procedural-level-generation (closed-loop circuits with separation, curvature
and length checks), tactical-ai-opponent (racing line, braking from curvature,
drafting, overtaking lanes, blocking, ramming), persistent-save-file,
in-run-upgrade-draft, route-choice, multiple-biomes, dynamic-lighting (night
venue: headlights, lamp posts, nitro glow), adaptive-music, tutorial-onboarding,
undo-move (undo garage purchases before the race), threat-preview (pace notes
for upcoming corners, rear-approach markers, Bully ram telegraph),
autopilot-playtest, persistent-campaign-upgrades (garage levels), 
accessibility-assists (steering assist, auto-throttle, game speed, pause menu),
time-trial-medals (Time Trial mode).

Not applicable: broadcast-programming, power-grid-allocation,
community-trust-network (radio-station management systems),
rope-physics-traversal (no rope; cars), time-rewind (lessons.md: rewind removed
all consequence on day 5; racing keeps every mistake meaningful).

## New firsts
- `third-party-art-assets`: the whole look is built from vendored CC0 packs
  (sprites, ground textures, fonts, sampled impacts), credited.
- `vehicle-drift-physics`: arcade car model with slip, grip per surface,
  handbrake drift and tiered mini-turbo.
- `slipstream-drafting`: aerodynamic tow behind any car, charging nitro.
- `ghost-replay`: best laps recorded and persisted, raced as a ghost.
- `minimap`: live circuit map with every car.
- `championship-standings`: points table across a cup.

## lessons.md applied
- *Fresh art*: Kenney Racing Pack (bright vector top-down), Kenney Future font,
  no hand-drawn pixel art, no dark palettes (except a deliberately lit night).
- *Clunk / speedrun feel*: no fiddly chores: one-button drift, auto-gears,
  coyote-style forgiveness on kerbs; skill lives in line, braking and drift
  timing. Variable-dt simulation that matches the real frame time (no hitch).
- *Foes must react and punish*: Bully telegraphs then rams; Blocker covers your
  line; everyone drafts you; champion uses nitro at the right moments.
- *Undo must not remove consequence*: no rewind; damage persists through a race.
- *Real music and mixed audio*: adaptive synth score per venue with a final-lap
  lift; every sound tagged quiet / normal / heavy and normalised.
- *Discoverable idea*: slipstream, hinted once; drift tiers visible as sparks.
- *15-20 minute boredom*: each venue adds a surface/hazard; parts, garage,
  route choice; Time Trial ghosts for replay.
- *Every resource visible, game over says why*: HUD shows everything; results
  say "Needed top 3, finished 5th".
- *Balance with a bot*: autopilot plays whole championships headlessly.

## Art style
Clean bright top-down vector (Kenney Racing Pack): textured grass/sand/dirt
ground, asphalt/sand/dirt road ribbons with red-white kerbs, Kenney cars,
bikes, tents, tribunes, trees. Typography: Kenney Future. Visibly different
from Lanternfall (dark lit isometric pixel art) and Cragswing (painted pixel
platformer).

## Focus axis
Fun/controls (day 5: "not fun, clunky"). The handling model gets the most
iteration, tuned by feel numbers and asserted by the bot (lap times, drift use,
no wall-grinding).
