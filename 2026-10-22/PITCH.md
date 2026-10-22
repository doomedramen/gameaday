# Lanternfall - pitch (day 3, 2026-10-22)

## Concept
A turn-based tactics roguelite on floating isometric islands. You command a
squad of three lantern-bearers - a Warden, a Ranger and a Lamplighter -
across seven procedurally generated islands swallowed by a creeping dark.
Light is the core tactical resource: enemies standing in darkness are
*Shrouded* (they take half damage and hit harder), so you fight with your
lanterns, kindle braziers, fire flares and shove shades off cliffs into the
void. Between islands you draft upgrades and choose your route, ending in a
fight against the Hollow King, whose aura swallows lantern light.

## Tags
| dimension | value |
| --- | --- |
| genre | tactics |
| core_verb | command |
| perspective | isometric |
| pacing | turn-based |
| input | keyboard+mouse (mouse-first) |
| structure | run-based-roguelite |
| win_condition | defeat-opponent |

## Closest previous game: Nebula Drift (day 2)
Both have enemies and a boss; that's where the overlap ends:
1. **Thinking, not reflexes** - turn-based moves on a height-mapped grid,
   with positioning, pushes, flanking and high ground deciding fights.
2. **Light as terrain** - you shape a living light map (lanterns, braziers,
   flares) that changes what enemies can survive; nothing like it in day 2.
3. **A run with choices** - route choice between islands and an upgrade
   draft after every win replace an endless wave counter.

## Bar capabilities
All 17 ship: core loop, win/lose, HUD (squad cards + HP pips), title screen,
best score (now persisted), 6 enemy types, Hollow King boss, escalating
islands + gloom + reinforcements, pickups (embers) + upgrades, kill-chain
scoring, particles, screen shake/flash/hit-stop, animated procedural sprites,
parallax sky layers, synth SFX, headless selftest, screenshot mode.
Not applicable: none.

## Firsts (new capabilities)
- procedural-level-generation - noise heightmap islands, connectivity-checked
- tactical-ai-opponent - enemies score every reachable tile x target
- persistent-save-file - best score, best island, runs, wins, tutorial flag
- in-run-upgrade-draft - pick 1 of 3 upgrades after every win
- route-choice - choose the next island between two options
- multiple-biomes - moss / ash / frost / crystal / hollow, each with palette,
  props and ambient particles
- dynamic-lighting - per-pixel lighting that is also a gameplay system
- adaptive-music - real-time procedural score whose intensity follows play
- tutorial-onboarding - contextual first-run hints + help overlay
- undo-move - take back a move before acting

## Lessons applied
- *Visible resources / say why you died*: squad cards, HP pips over every
  unit, defeat screen names the cause.
- *Feedback scales with importance*: boss hits get heavier SFX, shake and
  hit-stop than trash; chains escalate in pitch.
- *Defensive abilities need payoff*: Warden's Bulwark visibly reduces damage;
  undo lets players fix misclicks.
- *No single tactic trivialises the climax*: the Hollow King can't be pushed
  into the void, and its aura cancels lantern light - you need flares and
  braziers to strip its shroud.
- *Whole roster must threaten*: each enemy has a distinct job (rush, snipe,
  snuff light, shove, curse/summon).
- *New content past 15 minutes / faster ramp*: new enemy types arrive on
  islands 2, 3 and 4, elites from 4, biome changes every island, boss on 7;
  gloom shrinks lanterns and reinforcements punish turtling.
- *Choices drive replay*: upgrade draft + route choice each island.
- *Visuals were the weakest axis*: focus axis. Per-pixel warm lighting,
  floating islands with cliff faces over a parallax sky, five biomes.

## Focus axis
Visuals (day 2 scored 4-6), with replayability second.
