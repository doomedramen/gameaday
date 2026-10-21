#!/usr/bin/env bash
# Pre-push: the full quality gate.
#
# For every game project that changed since the upstream:
#   1. cargo test --release   (logic + unit tests + the binary e2e selftest)
#   2. cargo build --release  (the game must actually build shippable)
#
# Also sanity-checks repo state: gameaday-state.json must be valid JSON and
# must list every game folder that exists.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lefthook-helpers.sh
source "$here/lefthook-helpers.sh"

root="$(repo_root)"
status=0

# ---- state file checks -----------------------------------------------------
state="$root/gameaday-state.json"
if [[ -f "$state" ]]; then
  if ! python3 -c "import json,sys; json.load(open('$state'))" 2>/dev/null; then
    echo "[pre-push] FAIL: gameaday-state.json is not valid JSON" >&2
    status=1
  else
    missing=""
    while IFS= read -r game; do
      [[ -z "$game" ]] && continue
      base="$(basename "$game")"
      if ! grep -q "\"$base\"" "$state"; then
        missing="$missing $base"
      fi
    done < <(all_games)
    if [[ -n "$missing" ]]; then
      echo "[pre-push] FAIL: gameaday-state.json does not mention game folder(s):$missing" >&2
      status=1
    fi
  fi
else
  echo "[pre-push] FAIL: gameaday-state.json is missing" >&2
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
