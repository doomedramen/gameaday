#!/usr/bin/env bash
# Pre-push: the full quality gate.
#
# For every game project that changed since the upstream:
#   1. cargo test --release   (logic + unit tests + the binary e2e selftest)
#   2. cargo build --release  (the game must actually build shippable)
#
# Also validates repo state via check-state.py (state schema, novelty rules,
# capability ratchet, PITCH/REVIEW/screenshots deliverables).

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lefthook-helpers.sh
source "$here/lefthook-helpers.sh"

root="$(repo_root)"
status=0

# ---- state file checks -----------------------------------------------------
# State, novelty cooldowns, the capability ratchet and per-game deliverables.
if ! python3 "$here/check-state.py" "$root"; then
  status=1
fi

# ---- per-game build + test -------------------------------------------------
games="$(push_games)"
if [[ -z "$games" ]]; then
  echo "[pre-push] no changed game projects - skipping per-game checks"
else
  while IFS= read -r game; do
    [[ -z "$game" ]] && continue
    rel="${game#"$root/"}"
    echo "[pre-push] testing $rel (release)"

    if ! (cd "$game" && cargo test --offline --release); then
      echo "[pre-push] FAIL: $rel tests failed" >&2
      status=1
    fi

    if ! (cd "$game" && cargo build --offline --release); then
      echo "[pre-push] FAIL: $rel release build failed" >&2
      status=1
    fi
  done <<< "$games"
fi

exit "$status"
