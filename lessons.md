# Lessons

Long-term memory across days and models. Read this before designing today's
game. Each lesson is a **general** rule distilled from player feedback or from
a build that went wrong - never a fix for a specific old game (old games are
never modified). Add to it when feedback arrives; reword or merge, but don't
delete lessons that still hold.

## Feel & feedback
- Every player resource must be visible on screen. A game over must say *why*
  it happened. (day 1: no lives UI, unexplained red line on game over)
- Clear the full framebuffer every frame; ghosting reads as a bug. (day 1)
- Feedback must scale with importance: hitting or defeating a boss or key
  target needs heavier sound, shake and flash than a trash mob. (day 2: boss
  hits sounded too light)

- A core mechanic the player *discovers* (an "aha" moment) is memorable. Day
  3's lighting/shroud system was the highlight. Give each game one systemic
  idea the player can figure out and use cleverly.
- Music matters. Day 3's adaptive score was explicitly loved, so every game
  should have real music, not just sound effects.

## Abilities & challenge
- Movement or defensive abilities need a clear, readable defensive payoff
  (e.g. i-frames, passing through threats). If an ability exists, design the
  threats around it. (day 2: dash had no i-frames and still collided)
- No single tactic should trivialise the climax. Playtest that the boss or
  hardest challenge cannot be cheesed by one move. (day 2: boss avoidable by
  dashing)
- Most of the roster should be a real threat. If only 2 of 4 enemy types
  matter, the others are filler. (day 2)

## Movement & controls
- Movement must feel free and readable. On day 3 it felt "tricky, easy to get
  stuck in one place": cramped islands, height steps and blocking units made
  reachable tiles hard to predict. Be generous with movement range, keep maps
  open, and make it obvious *why* a tile can't be reached.
- Design for players outside the genre. Day 3 scored 7/10 from someone for
  whom tactics isn't their usual kind of game. Onboarding, previews and
  forgiving early levels carry a lot of weight.

## Longevity & progression
- Players get bored around 15-20 minutes without anything new. Plan for new
  content, mechanics or choices to keep appearing past that point, and assert
  it in the playtest. (day 2: nothing new after wave 21)
- Difficulty should ramp noticeably faster than feels safe; the first 3
  minutes may be gentle, after that push. (day 2)
- Meaningful in-session choices (upgrades, builds, routes) drive replay far
  more than raw score. (day 2: asked for roguelite-style upgrades)

## Scope & ambition
- "Bigger" was the explicit wish after day 1: animation, sprites, sound and
  more content. Plain shapes on black are below the bar.
- Day 3's art direction (lit isometric islands, glow, biome palettes) was
  loved. That is the visual bar now.
- Visuals were the weakest-rated axis on day 2 (4-6). Spend real effort on art
  direction: a coherent palette, lighting/glow, animated backgrounds, UI
  framing and typography.

## Process
- Balance with a headless bot before trusting your instincts. On day 3 it
  found endless stalemates (a lone healer could out-heal the damage) and one
  enemy type dealing half of all damage. Every session needs a hard stalemate
  breaker, and a test that every battle ends.
- AI that moves towards targets must use real path distance, not straight-line
  distance, or units get stuck behind water and walls. (day 3)
- Look at the screenshots at the true resolution. Dark biomes that look moody
  in your head can be unreadable on screen. (day 3: the boss island was nearly
  black)
- A generator is only as good as its oracle. The autopilot that plays every
  generated level (and plans by simulating ahead) found far more bugs than
  hand-checking: gaps that were crossable only by luck, vault rooms with no
  exit, hazards spawned inside rock. Build the bot first, then the content.
  (day 5)
- Sub-step collision loops that stop at a coarse step leave a gap above the
  floor; a ground probe then fails and the player cannot jump. Refine to
  contact, and treat the press frame of a button as held. (day 5)
- Rewinding invites exploits: claim pickups once and keep the resource outside
  the rewound state. (day 5)
- Test harnesses that replay a bot through the real UI must honour hit-stop
  and menus, or the bot's timing drifts and the failures look like game bugs.
  (day 5)
- Day 5 feedback (concept good, execution clunky). Distilled:
  - A bot that clears a level is not evidence the moment-to-moment feel is
    fun. Judge movement by how it feels to chain, not by solvability. Tests
    that only prove "can be completed" miss clunk.
  - Fast-movement games should reward flow: never make the player manage
    fiddly inputs (rope length, re-aiming) mid-motion. Automate the chores and
    let skill live in route, timing and momentum. Design for a speedrun.
  - Run the simulation at exactly one tick per display frame on a fixed-rate
    display (or interpolate renders). A free accumulator gives uneven ticks
    that look like hitching, especially in run animations and the camera.
  - Mechanics that are only a means to an end (wall jumps, patrolling foes)
    need their own payoff and tension. A passive patroller that only blocks the
    way is not fun; make foes react, rush and punish, and make every movement
    verb feel good to chain on its own.
  - Rewind or any undo must never remove all consequence. If wounds can be
    undone the game cannot be lost. Keep undo scarce and let it forgive
    positional mistakes, not damage.
  - Mix every sound to a loudness target by class (quiet, normal, heavy), then
    play by ear. Per-effect hand tuning left the volume all over the place.
- Day 5 art feedback: the visuals were almost identical to day 3. Do not hand-draw every sprite with the same primitives and palette again. Start each game by choosing a distinct art direction and sourcing assets for it (itch.io, Kenney, OpenGameArt), then build gameplay around what the pack supports. (day 5)
- Day 6 feedback (racing; graphics liked, AI uncatchable, HUD in the way,
  engine sound horrible, no working full screen). Distilled:
  - Tuning difficulty against your own autopilot overrates the player. A
    human is much slower than the bot (assume 10-15%), so the first events
    must be winnable by a newcomer, and contests must stay close. In races and
    chases, use rubber banding that is felt: leaders ease off, the pack never
    disappears out of reach.
  - Keep the play area clear. HUD panels belong in thin strips at the edges;
    show important state (boost, nitro, power-ups) on the player's own sprite
    and with screen effects, not only in a panel. Check how much of each
    screenshot the UI covers.
  - Continuous synthesised sounds (engines, drones) easily sound harsh. Use a
    sampled loop from an asset pack, or keep the synth soft, low and filtered,
    and mix it well under effects and music.
  - Full screen is expected, and players reach for the OS controls first.
    Leave the window resizable so the macOS green maximise / full-screen
    button works (scale the frame to the window, letterboxed), and also offer
    full screen as a visible Settings and pause-menu option. A shortcut listed
    only in a title-screen footer goes unnoticed.
- Day 6 art feedback: sourcing a coherent third-party pack (Kenney) and
  building the look around it worked - "graphics were nice". Keep doing it.
