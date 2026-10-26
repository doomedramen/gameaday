# The Last Glasshouse — pitch, 26 October 2026

An original botanical deckbuilder about bringing a greenhouse through an impossible winter. Plant cards occupy three beds, water grows them, and harvesting mature plants turns growth into attacks, shelter and draws. Keeping a mature plant alive improves adjacent harvests, so the strongest move is sometimes waiting. Any unwanted card can be composted once per turn for an extra action. Eighteen seeded encounters cross four seasons, with card drafts, branching risk/rest routes, nine relics, evolving enemies and three distinct keepers. A 20–35 minute sitting, untimed, entirely mouse playable with keyboard shortcuts.

Tags: **deckbuilder / grow / card-table / turn-based / mouse-only / run-based-roguelite / clear-board**.

## Distance from previous games
Closest is Lanternfall, only because both have a sequence of turn-based encounters. Play differs in three central ways: (1) a shuffled, changing deck controls available actions rather than movement on tiles; (2) delayed plant growth and adjacency pollination create an engine to construct and cash in, rather than positional combat; (3) unwanted cards are a second action economy through compost, rather than fixed per-unit moves. No movement, racing, aiming, ropes or squad commands.

## Art direction
A cream-paper botanical field journal: deep evergreen ink, vermilion annotations, brass rules, arch-shaped glass panes and colourful flat plant/insect specimens. Twemoji's CC BY 4.0 botanical/insect artwork is vendored, with Fraunces display type and Lato body type (SIL OFL). These are fresh sources, visually distinct from the previous two games' pixel cliff sprites and Kenney vector race circuits. Credit author, source, licence and any transformations in assets/CREDITS.md. All rendering authored in Rust.

## Ratchet
Ships: core loop, explicit win/loss, visible health/actions/shield/deck/resources, title, best score, 8 enemy types, 3 bosses, rising difficulty, relics, pollination chains, particles and impacts, animated specimen sprites, layered greenhouse, synthesised event sounds and adaptive music, headless binary e2e, screenshots, persistent bests/unlocks, three-card reward draft, branching routes, four seasonal environments, state-based illumination, guided onboarding + help, undo before committing a turn, enemy intention previews, full-run bot, accessibility settings and third-party art. Enemy intention selection evaluates bed maturity and player defence.

Not applicable: broadcast programming, station power grids and community trust have no station/neighborhood analogue; rope physics, vehicle physics, drafting and lap ghosts have no movement analogue; time-trial medals and championship standings do not fit untimed solo deckbuilding; a minimap is replaced by the visible encounter itinerary, since no spatial map exists; persistent campaign purchases are avoided because permanent numerical power would hide balance problems, with cosmetic records and an alternate starting deck instead; time rewind is not a continuous simulation, while turn undo stops at committed enemy consequences; procedural level geometry is absent (encounter rosters/drafts are seeded, but the three-bed table is fixed).

Firsts: deck construction with draw/discard/reshuffle and card removal/upgrading; persistent plants with growth, harvesting and adjacency pollination; composting hand cards into actions. A complete card field guide also ships.

## Lessons and focus
Focus axes: **fun and controls**, weak in the last two games. No fiddly movement; click card then bed, right click cancels; compost is explicit; invalid actions explain why; selectable large text help; on-screen shortcuts; native resizable window and visible fullscreen button in title/pause/settings. Tutorial introduces planting, watering, mature harvesting, defence and end turn in context. Early enemies telegraph small numbers, no bot-speed pressure; assists reduce incoming damage. Health loss stays after committed turns; undo only covers decisions in the current turn. Audio has soft instrumental plucks and chimes with event-specific envelopes, no harsh continuous drone. Important hits have deeper, louder sounds. Four seasonal rules and new card families/enemies appear late, with a new final boss phase after encounter 15. HUD stays at edges; central table is the game. Automated play proves completion, loss, ramp and post-15-minute content; human fun remains an honest limitation.

## Build phases
1. Engine-free core and full-run bot; validate actions, growth, combat and progression.
2. Content: 28 cards, seasonal rules, enemy roster, relics, routes, field guide and saves.
3. Dedicated visual/audio/interaction polish; inspect every exported screenshot; release tests and state validator.
