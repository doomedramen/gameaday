# The Last Glasshouse — final review

26 October 2026. Player feedback supersedes the pre-play pitch. The game is
frozen after play; this review records the failure rather than changing it.

## Player feedback, verbatim

> The game is boring.
> the game is way too easy.
> seems silly for plants to require being clicked on to harvest if leaving them unharvested does nothing.

## Assessment against the best previous game

Lanternfall remains the strongest reference: the player reported 7/10,
enjoyed discovering its lighting mechanic and wanted to replay it. The Last
Glasshouse does not meet that bar. More cards, encounters and menus did not
produce more interesting decisions.

These are self-assessment scores, not invented numerical player ratings.

| Axis | Score | Assessment |
| --- | --- | --- |
| Fun | 3/10 | The player found it boring. The repeated plant–water–harvest cycle lacks enough tension or consequential alternatives. |
| Visuals | 7/10 | A fresh cream-paper botanical direction, licensed vector specimens and readable typography. The player supplied no visual rating; presentation cannot compensate for the weak loop. |
| Replayability | 3/10 | Deck drafts, seeds, relics and an alternate start exist, but easy encounters give too little reason to explore different builds. |
| Controls | 5/10 | Mouse targets and shortcuts work, but clicking to harvest adds repetitive work without a sufficiently valuable timing decision. Working input is not the same as satisfying input. |

## Why it falls short

- Difficulty rises numerically, but meaningful pressure does not follow. The
  competent bot won all 48 tested seeds. The reachable-loss test used an idle
  player; that proves loss is possible, not that normal play is challenging.
  Frequent healing and generous defensive tools further soften consequences.
- Mature adjacency grants a small harvest bonus in the implementation, but
  that is not a compelling enough reward for keeping a mature plant alive.
  The player's experience of harvesting as a needless click is the relevant
  result. There is no sustained production or strong accumulating benefit
  that makes waiting feel like an interesting alternative.
- Eighteen encounters expand the repetition. Four seasonal rules, card
  upgrades and keeper phases add content without fixing the central decision.
- The build overcorrected for earlier complaints about excessive difficulty.
  A forgiving opening should introduce a demanding game, not remove its stakes.

## Verification and limits

Release tests cover the shipped binary, 48 complete seeded campaigns,
representative PNG output, a complete campaign through actual UI hit regions,
boss intentions/phases, poison timing, harvest retargeting, deck edits,
settings, invalid actions, undo and persistent records. Ten representative
screenshots were inspected; the final vector-rendered exports are committed.
The native app was launched and the macOS fullscreen control was exposed.
The user's live session was left alone once play was apparent.

Automated checks establish operation and completion, not fun or suitable
balance. The simulated 15-minute gate is a declared per-decision pacing
estimate for an untimed game, not measured human retention. The field guide
records collection data but displays the whole card catalogue. There is no
unfinished-run save, and non-macOS fullscreen is not implemented.

## Outcome

Below the best-so-far bar. Preserve the played build and carry the lessons
forward: require a real reason to wait, test challenge against weak as well
as competent play, and prove the core loop is engaging before multiplying
content.
