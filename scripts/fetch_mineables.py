"""Build mineables.json: enrich the curated ore signatures with per-location harvest
data + spawn probability from the Star Citizen Wiki API.

`core/data/signatures.json` stays the hand-curated source of base_rs / tier / volatile.
This script only ATTACHES each ore's `locations[]` (body, system, type, spawn probability
%, quality band) by fetching the ore's Wiki API commodity — mapped via the `api_slug` table
in `core/data/mineables-curation.json`. Ores without a slug (asteroid types, salvage panel)
pass through with empty `locations`.

Outputs (identical content):
  core/data/mineables.json  — embedded in the binary at build time (offline fallback)
  public/mineables.json     — published to GitHub Pages; the app fetches this at startup

Run: `uv run scripts/fetch_mineables.py`  (stdlib only; no third-party deps).
Wired into .github/workflows/mineables.yml (scheduled + manual). Data: Star Citizen Wiki
API (https://api.star-citizen.wiki). The per-body `relative_probability_percent` is the
chance a deposit of that mining context at that body is this ore (sums to ~100% per body).
"""

import json
import sys
import time
import urllib.error
import urllib.request
from pathlib import Path

API = "https://api.star-citizen.wiki/api/commodities"
HEADERS = {"User-Agent": "sc-ore-scanner/1.0", "Accept": "application/json"}
REPO_ROOT = Path(__file__).resolve().parent.parent
SIGNATURES = REPO_ROOT / "core" / "data" / "signatures.json"
CURATION = REPO_ROOT / "core" / "data" / "mineables-curation.json"
OUT_EMBED = REPO_ROOT / "core" / "data" / "mineables.json"
OUT_FEED = REPO_ROOT / "public" / "mineables.json"


def fetch(url: str) -> dict:
    req = urllib.request.Request(url, headers=HEADERS)
    with urllib.request.urlopen(req, timeout=40) as resp:
        return json.load(resp)


def extract_locations(detail: dict):
    """Return (sorted per-body rows for this ore, set of resource signatures seen).

    A commodity IS one ore, so a location's `relative_probability_percent` is that ore's
    spawn chance at that body. Deduped by body, sorted by probability desc.
    """
    rows: dict[str, dict] = {}
    sigs: set[int] = set()
    for loc in detail.get("locations") or []:
        body = loc.get("name")
        prob = loc.get("relative_probability_percent")
        if not body or prob is None:
            continue
        rows[body] = {
            "body": body,
            "system": (loc.get("system") or "").replace(" System", "").strip(),
            "type": loc.get("type"),
            "probability": round(float(prob), 2),
            "quality_min": loc.get("quality_min"),
            "quality_max": loc.get("quality_max"),
        }
        for res in loc.get("resources") or []:
            if res.get("signature"):
                sigs.add(int(res["signature"]))
    ordered = sorted(rows.values(), key=lambda r: (-r["probability"], r["body"]))
    return ordered, sigs


def main() -> int:
    ores = json.loads(SIGNATURES.read_text(encoding="utf-8"))["ores"]
    slug_map = json.loads(CURATION.read_text(encoding="utf-8"))["api_slug"]
    warnings: list[str] = []
    wiki_processed_at = None
    game_version = None

    for ore in ores:
        slug = slug_map.get(ore["id"])
        if not slug:
            # asteroid types / salvage: not Wiki commodities — no API location data.
            ore["locations"] = ore.get("locations", [])
            continue
        try:
            payload = fetch(f"{API}/{slug}")
        except (urllib.error.HTTPError, urllib.error.URLError) as exc:
            warnings.append(f"{ore['id']} ({slug}): fetch failed ({exc}) — kept, no locations")
            ore["locations"] = []
            continue
        detail = payload.get("data", {})
        wiki_processed_at = wiki_processed_at or (payload.get("meta") or {}).get("processed_at")
        locations, sigs = extract_locations(detail)
        ore["locations"] = locations
        # Drift check: our curated base_rs should appear among the API's resource signatures.
        if sigs and ore["base_rs"] not in sigs:
            warnings.append(
                f"{ore['id']}: curated base_rs {ore['base_rs']} not in API signatures {sorted(sigs)}"
            )
        if game_version is None:
            for price in (detail.get("uex_prices") or {}).get("purchase") or []:
                if price.get("game_version"):
                    game_version = price["game_version"]
                    break

    # Sanity check: for the ambiguous contexts, a body's ore probabilities sum to ~100%.
    sums: dict[tuple, float] = {}
    for ore in ores:
        ctx = ore["context"][0] if ore["context"] else "?"
        for loc in ore["locations"]:
            sums[(loc["body"], ctx)] = sums.get((loc["body"], ctx), 0.0) + loc["probability"]
    for (body, ctx), total in sorted(sums.items()):
        if ctx in ("fps", "vehicle") and abs(total - 100.0) > 1.0:
            warnings.append(f"{ctx} probabilities at {body} sum to {total:.1f}% (expected ~100%)")

    out = {
        "generated_at": int(time.time()),
        "wiki_processed_at": wiki_processed_at,
        "game_version": game_version,
        "source": "Star Citizen Wiki API (api.star-citizen.wiki); base_rs/tier curated in signatures.json",
        "ore_count": len(ores),
        "location_count": sum(len(o["locations"]) for o in ores),
        "ores": ores,
    }
    text = json.dumps(out, indent=2, ensure_ascii=False) + "\n"
    OUT_EMBED.write_text(text, encoding="utf-8")
    OUT_FEED.parent.mkdir(parents=True, exist_ok=True)
    OUT_FEED.write_text(text, encoding="utf-8")

    print(
        f"Wrote {OUT_EMBED.relative_to(REPO_ROOT)} and {OUT_FEED.relative_to(REPO_ROOT)}: "
        f"{out['ore_count']} ores, {out['location_count']} location rows "
        f"(wiki {wiki_processed_at}, game {game_version})."
    )
    if warnings:
        print(f"\n{len(warnings)} warning(s):", file=sys.stderr)
        for w in warnings:
            print(f"  ! {w}", file=sys.stderr)
    return 0


if __name__ == "__main__":
    sys.exit(main())
