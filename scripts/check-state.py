#!/usr/bin/env python3
"""Validate GameADay repo state: gameaday-state.json, novelty.json, bar.json.

Checks every game:
  - history is well-formed, date-ordered, and day == len(history)
  - every YYYY-MM-DD/ game folder is in history and vice versa
  - tags use the vocabulary in novelty.json
  - capabilities / firsts exist in bar.json

Checks games dated on or after novelty.json "enforced_from" (the stricter
rules introduced after day 2):
  - novelty: genre and core_verb cooldowns, and enough dimensions changed vs
    the previous game
  - ratchet: every earlier capability is shipped or explained, and at least
    bar.json rules.min_new_firsts new capabilities debut in this game
  - deliverables: PITCH.md, REVIEW.md and screenshots/ (>= 4 PNGs)

Usage: scripts/check-state.py [repo_root]
Exit status is non-zero if any check fails.
"""

import json
import re
import sys
from datetime import date
from pathlib import Path

DIMENSIONS = ["genre", "core_verb", "perspective", "pacing", "input", "structure", "win_condition"]
REQUIRED_KEYS = ["date", "folder", "title", "summary", "model", "tags", "capabilities", "not_applicable", "firsts"]
DATE_RE = re.compile(r"^\d{4}-\d{2}-\d{2}$")
MIN_SCREENSHOTS = 4

errors = []


def fail(msg):
    errors.append(msg)


def load(path):
    try:
        return json.loads(path.read_text())
    except FileNotFoundError:
        fail(f"{path.name} is missing")
    except json.JSONDecodeError as e:
        fail(f"{path.name} is not valid JSON: {e}")
    return None


def days_between(a, b):
    return abs((date.fromisoformat(a) - date.fromisoformat(b)).days)


def check_entry_shape(entry, vocab, caps):
    d = entry.get("date", "?")
    for key in REQUIRED_KEYS:
        if key not in entry:
            fail(f"{d}: history entry missing '{key}'")
    if not str(entry.get("model", "")).strip():
        fail(f"{d}: 'model' must be filled in")
    if entry.get("folder") != d:
        fail(f"{d}: folder '{entry.get('folder')}' must equal the date")

    tags = entry.get("tags", {})
    for dim in DIMENSIONS:
        value = tags.get(dim)
        if value is None:
            fail(f"{d}: tags.{dim} missing")
        elif value not in vocab.get(dim, {}):
            fail(f"{d}: tags.{dim} = '{value}' is not in novelty.json (add it there with a description)")

    for cap in entry.get("capabilities", []):
        if cap not in caps:
            fail(f"{d}: capability '{cap}' is not defined in bar.json")
    for cap in entry.get("firsts", []):
        if cap not in caps:
            fail(f"{d}: first '{cap}' is not defined in bar.json")
        elif caps[cap].get("first_date") != d:
            fail(f"{d}: '{cap}' is listed as a first but bar.json says it debuted {caps[cap].get('first_date')}")
        if cap not in entry.get("capabilities", []):
            fail(f"{d}: first '{cap}' must also be listed in capabilities")


def check_novelty(entry, earlier, novelty):
    d = entry["date"]
    rules = novelty["rules"]
    tags = entry["tags"]
    for prev in earlier:
        gap = days_between(d, prev["date"])
        if prev["tags"].get("genre") == tags["genre"] and gap < rules["genre_cooldown_days"]:
            fail(f"{d}: genre '{tags['genre']}' was used {gap} day(s) ago on {prev['date']} "
                 f"(cooldown {rules['genre_cooldown_days']} days)")
        if prev["tags"].get("core_verb") == tags["core_verb"] and gap < rules["core_verb_cooldown_days"]:
            fail(f"{d}: core_verb '{tags['core_verb']}' was used {gap} day(s) ago on {prev['date']} "
                 f"(cooldown {rules['core_verb_cooldown_days']} days)")
    if earlier:
        prev = earlier[-1]
        changed = [dim for dim in DIMENSIONS if prev["tags"].get(dim) != tags[dim]]
        need = rules["min_dimensions_changed_vs_previous"]
        if len(changed) < need:
            fail(f"{d}: only {len(changed)} tag dimension(s) differ from {prev['date']} ({', '.join(changed) or 'none'}); "
                 f"need at least {need}")


def check_ratchet(entry, caps, bar_rules):
    d = entry["date"]
    shipped = set(entry["capabilities"])
    na = entry["not_applicable"]
    for cap, info in caps.items():
        if info.get("first_date", "") >= d:
            continue
        if cap in shipped:
            continue
        reason = str(na.get(cap, "")).strip()
        if not reason:
            fail(f"{d}: bar capability '{cap}' is neither shipped nor explained in not_applicable")
    need = bar_rules["min_new_firsts"]
    if len(entry["firsts"]) < need:
        fail(f"{d}: only {len(entry['firsts'])} new capability(ies) in firsts; need at least {need}")


def check_deliverables(entry, root):
    d = entry["date"]
    folder = root / entry["folder"]
    for name in ["PITCH.md", "REVIEW.md", "Cargo.toml"]:
        if not (folder / name).is_file():
            fail(f"{d}: {entry['folder']}/{name} is missing")
    shots = list((folder / "screenshots").glob("*.png")) if (folder / "screenshots").is_dir() else []
    if len(shots) < MIN_SCREENSHOTS:
        fail(f"{d}: {entry['folder']}/screenshots/ has {len(shots)} PNG(s); need at least {MIN_SCREENSHOTS}")


def main():
    root = Path(sys.argv[1] if len(sys.argv) > 1 else Path(__file__).resolve().parent.parent)
    state = load(root / "gameaday-state.json")
    novelty = load(root / "novelty.json")
    bar = load(root / "bar.json")
    if errors:
        return report()

    history = state.get("history", [])
    vocab = novelty["dimensions"]
    caps = bar["capabilities"]

    if state.get("day") != len(history):
        fail(f"day is {state.get('day')} but history has {len(history)} entries")

    dates = [e.get("date", "") for e in history]
    if any(not DATE_RE.match(x) for x in dates):
        fail("every history entry needs a YYYY-MM-DD date")
        return report()
    if dates != sorted(dates) or len(set(dates)) != len(dates):
        fail("history must be in strictly ascending date order with no duplicates")

    folders = {p.name for p in root.iterdir() if p.is_dir() and DATE_RE.match(p.name) and (p / "Cargo.toml").is_file()}
    for missing in sorted(folders - set(dates)):
        fail(f"game folder {missing}/ is not recorded in history")
    for missing in sorted(set(dates) - folders):
        fail(f"history entry {missing} has no {missing}/Cargo.toml")

    for cap, info in caps.items():
        if info.get("first_date") not in dates:
            fail(f"bar.json '{cap}' has first_date {info.get('first_date')} which matches no game")

    for entry in history:
        check_entry_shape(entry, vocab, caps)
    if errors:
        return report()

    for i, entry in enumerate(history):
        if entry["date"] < novelty["enforced_from"]:
            continue
        check_novelty(entry, history[:i], novelty)
        check_ratchet(entry, caps, bar["rules"])
        check_deliverables(entry, root)

    return report()


def report():
    if errors:
        for e in errors:
            print(f"[check-state] FAIL: {e}", file=sys.stderr)
        return 1
    print("[check-state] ok")
    return 0


if __name__ == "__main__":
    sys.exit(main())
