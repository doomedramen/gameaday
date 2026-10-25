# Slipstream Cup - self review (day 6)

Compared against the best-rated games so far: **Lanternfall** (7/10, art and
music loved) and **Nebula Drift** (fun 7-8, controls 7-8, visuals 4-6).

| Axis | Score | Why |
| --- | --- | --- |
| Fun | 7.5 | Racing in a pack is lively: slipstream tows, slingshots, rams, oil spins, rocket starts, drift turbos. Every rival kind does something to you. Not hand-played by a human, so the feel numbers are tuned by tests and reasoning, not by thumbs. |
| Visuals | 8 | A completely new look built on Kenney's CC0 racing art: bright vector cars, textured ground, red-white kerbs and barriers, grandstands with spectators, tree canopies with parallax, drifting cloud shadows, a genuinely lit night venue. Clean UI in Kenney Future. Clearly different from the last two pixel-art games. |
| Replayability | 7.5 | Procedural circuits, route choice, 14 parts, garage builds, a rising field, plus five Time Trial circuits with persistent ghosts and medals calibrated to the AI. |
| Controls | 7.5 | Keyboard-first: one button drifts (no fiddly chores), smoothed steering, auto-gears, honest grip; assists for newcomers. Pace notes call every bend. |

## Lessons honoured
- Fresh art from third-party packs, credited (the day-5 art complaint).
- Speedrun feel: line, braking, drift timing and slipstream are the skill;
  nothing to babysit mid-motion. Time Trial ghosts make it a race against
  yourself.
- Foes react: the Bully telegraphs and rams, the Blocker covers your line, the
  Champion defends and drafts, everybody overtakes and uses nitro.
- No undo of consequences: damage lasts the race; second chances cost
  something and are limited.
- Audio by loudness class; samples RMS-normalised at load.
- Bot first: an autopilot laps 60 generated circuits per selftest (it found
  oval fallbacks, props on the road and pile-ups on oil), plays whole Cups
  through the real app and proves win, loss, ramp and new content after 15
  minutes.

## Known weaknesses
- Feel is untested by a human hand. The car model passed unit checks
  (acceleration, braking, drift tiers, nitro) and the bot drives it cleanly,
  but a person may find the steering rate or drift angle off.
- Difficulty is calibrated against a strong bot. A newcomer may find rounds
  3-5 hard without garage upgrades; assists and second chances soften it.
- The AI drifts less often than a human would and rarely reaches tier 3.
- Circuits are flat loops without crossovers; venues differ by surface, props
  and lighting rather than shape language.
- Synth music is simpler than Lanternfall's score; engine sound is a filtered
  oscillator, not a sample.
- Keyboard only (no mouse in menus, no gamepad).

## Verdict
More systems than any previous game (car physics, procedural circuits, a
seven-personality AI field, campaign economy, ghosts, lighting) and the
strongest visual identity so far. It beats Lanternfall on scope and visuals;
fun has to be confirmed by playing.
