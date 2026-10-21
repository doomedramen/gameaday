#!/usr/bin/env bash
# Pre-commit: fast checks on the game(s) touched by this commit.
#
# Checks (per affected 2026-XX-XX project):
#   1. cargo fmt --check   (formatting)
#   2. cargo clippy        (lints, warnings denied)
#   3. cargo test --lib    (fast unit tests only; the full e2e suite runs on push)
#
# The full test suite plus a release build runs in pre-push.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
# shellcheck source=lefthook-helpers.sh
source "$here/lefthook-helpers.sh"

games="$(staged_games)"
if [[ -z "$games" ]]; then
  echo "[pre-commit] no game projects staged - nothing to check"
  exit 0
fi

status=0

while IFS= read -r game; do
  [[ -z "$game" ]] && continue
  rel="${game#"$(repo_root)/"}"
  echo "[pre-commit] checking $rel"

  if ! (cd "$game" && cargo fmt --all -- --check); then
    echo "[pre-commit] FAIL: $rel is not rustfmt-clean. Run: (cd $rel && cargo fmt)" >&2
    status=1
  fi

  if ! (cd "$game" && cargo clippy --offline --all-targets -- -D warnings); then
    echo "[pre-commit] FAIL: $rel has clippy warnings" >&2
    status=1
  fi

  if ! (cd "$game" && cargo test --offline --lib); then
    echo "[pre-commit] FAIL: $rel unit tests failed" >&2
    status=1
  fi
done <<< "$games"

exit "$status"
