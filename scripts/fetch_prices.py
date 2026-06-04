"""Fetch Star Citizen commodity prices from UEX Corp and write a small
prices.json for the overlay.

UEX's /commodities endpoint is public (no auth) and already includes per-SCU
buy/sell prices. This script filters it down to the ores we detect (matched by
name) so the published file stays tiny. Run by .github/workflows/prices.yml and
deployed to GitHub Pages; the scanner backend fetches it on a timer and caches it.

Data: UEX Corp (https://uexcorp.space).
"""

import json
import sys
import time
import urllib.request
from pathlib import Path

UEX_URL = "https://api.uexcorp.uk/2.0/commodities"
REPO_ROOT = Path(__file__).resolve().parent.parent
SIGNATURES = REPO_ROOT / "backend" / "data" / "signatures.json"
OUT = REPO_ROOT / "public" / "prices.json"


def load_ore_name_to_id() -> dict:
    """Map lowercase ore name -> our ore id, for real ores only.

    Skips asteroid types and salvage/debris, which aren't UEX commodities.
    """
    data = json.loads(SIGNATURES.read_text(encoding="utf-8"))
    mapping = {}
    for ore in data["ores"]:
        context = ore.get("context", [])
        if "asteroid" in context or "salvage" in context:
            continue
        mapping[ore["name"].lower()] = ore["id"]
    return mapping


def build_prices(commodities: list, ore_name_to_id: dict) -> dict:
    """Map UEX commodities to {ore_id: {name, sell, buy}} for our ores.

    If a name appears more than once (e.g. raw vs refined), keep the highest
    sell price.
    """
    prices = {}
    for c in commodities:
        ore_id = ore_name_to_id.get(str(c.get("name", "")).lower())
        if not ore_id:
            continue
        sell = int(c.get("price_sell") or 0)
        buy = int(c.get("price_buy") or 0)
        if ore_id not in prices or sell > prices[ore_id]["sell"]:
            prices[ore_id] = {"name": c.get("name"), "sell": sell, "buy": buy}
    return prices


def fetch_commodities() -> list:
    req = urllib.request.Request(UEX_URL, headers={"User-Agent": "sc-ore-scanner"})
    with urllib.request.urlopen(req, timeout=30) as resp:
        payload = json.load(resp)
    if payload.get("status") != "ok":
        raise RuntimeError(f"UEX returned status {payload.get('status')!r}")
    return payload.get("data", [])


def main() -> int:
    commodities = fetch_commodities()
    prices = build_prices(commodities, load_ore_name_to_id())
    output = {
        "updated_at": int(time.time()),
        "source": "UEX Corp (https://uexcorp.space)",
        "currency": "aUEC",
        "note": "Per-SCU sell price. Community-maintained data; may not match live servers.",
        "prices": prices,
    }
    OUT.parent.mkdir(parents=True, exist_ok=True)
    OUT.write_text(json.dumps(output, indent=2), encoding="utf-8")
    print(f"Wrote {len(prices)} ore prices to {OUT}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
