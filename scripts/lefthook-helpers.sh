#!/usr/bin/env bash
# Shared helpers for lefthook scripts in this repo (GameADay).
#
# Each day's game lives in its own YYYY-MM-DD/ folder as a self-contained
# Rust project. These helpers discover those projects so hooks can run
# per-game without hard-coding dates.

set -euo pipefail

repo_root() {
  git rev-parse --show-toplevel 2>/dev/null || pwd
}

# List game directories (YYYY-MM-DD) that contain a Cargo.toml.
# With an argument, list only games affected by the given git range/set.
all_games() {
  local root
  root="$(repo_root)"
  find "$root" -maxdepth 2 -name Cargo.toml -type f \
    | while read -r manifest; do
        dir="$(dirname "$manifest")"
        base="$(basename "$dir")"
        if [[ "$base" =~ ^[0-9]{4}-[0-9]{2}-[0-9]{2}$ ]]; then
          echo "$dir"
        fi
      done | sort
}

# Games touched by the staged changes (falls back to all games).
staged_games() {
  local root
  root="$(repo_root)"
  local dirs
  dirs="$(
    git diff --cached --name-only --diff-filter=ACMR \
      | awk -F/ '$1 ~ /^[0-9]{4}-[0-9]{2}-[0-9]{2}$/ {print $1}' \
      | sort -u
  )"
  if [[ -z "$dirs" ]]; then
    return 0
  fi
  for d in $dirs; do
    if [[ -f "$root/$d/Cargo.toml" ]]; then
      echo "$root/$d"
    fi
  done
}

# Games whose files differ between HEAD and the point we are pushing from.
# Prefers the branch upstream; falls back to origin/main or the first commit so
# legacy games that are not part of this push are never re-linted.
push_games() {
  local root
  root="$(repo_root)"

  local base=""
  local upstream
  upstream="$(git rev-parse --abbrev-ref --symbolic-full-name '@{upstream}' 2>/dev/null || true)"
  if [[ -n "$upstream" ]]; then
    base="$upstream"
  elif git rev-parse --verify -q origin/main >/dev/null; then
    base="origin/main"
  fi

  local dirs
  if [[ -n "$base" ]]; then
    dirs="$(
      git diff --name-only "$base"...HEAD 2>/dev/null \
        | awk -F/ '$1 ~ /^[0-9]{4}-[0-9]{2}-[0-9]{2}$/ {print $1}' \
        | sort -u
    )"
  else
    # No usable base: consider only game folders touched by recent commits.
    dirs="$(
      git log --name-only --pretty=format: -n 50 \
        | awk -F/ '$1 ~ /^[0-9]{4}-[0-9]{2}-[0-9]{2}$/ {print $1}' \
        | sort -u
    )"
  fi

  if [[ -z "$dirs" ]]; then
    return 0
  fi
  for d in $dirs; do
    if [[ -f "$root/$d/Cargo.toml" ]]; then
      echo "$root/$d"
    fi
  done
}
