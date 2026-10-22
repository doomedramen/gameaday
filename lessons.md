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
