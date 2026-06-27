"""Fetch Star Citizen commodity prices from UEX Corp and write a small
prices.json for the overlay.

UEX's /commodities endpoint is public (no auth) and already includes per-SCU
buy/sell prices. This script filters it down to the ores we detect (matched by
name) so the published file stays tiny. Run by .github/workflows/prices.yml and
deployed to GitHub Pages; the app fetches it on a timer and caches it.

Data: UEX Corp (https://uexcorp.space).
"""

import html
import json
import shutil
import sys
import time
import urllib.request
from pathlib import Path

UEX_URL = "https://api.uexcorp.uk/2.0/commodities"
REPO_ROOT = Path(__file__).resolve().parent.parent
SIGNATURES = REPO_ROOT / "core" / "data" / "signatures.json"
OUT = REPO_ROOT / "public" / "prices.json"

# Brand icon for the published Pages table. The single source of truth for web
# assets is frontend/public/ (where icon-master.png lives); favicon.ico is the one
# multi-res favicon the React app uses too. We copy it into the Pages artifact
# (public/) and reference it relatively, the same way the table links to prices.json.
# GitHub Pages serves the whole public/ dir as the site, so this is not an "extra
# asset to deploy" — it ships in the same artifact and the browser caches it once,
# instead of inlining a base64 data URI twice into every render.
ICON_SRC = REPO_ROOT / "frontend" / "public" / "favicon.ico"
ICON_NAME = "favicon.ico"


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
    (OUT.parent / "index.html").write_text(render_index(output), encoding="utf-8")
    shutil.copyfile(ICON_SRC, OUT.parent / ICON_NAME)

    print(f"Wrote {len(prices)} ore prices to {OUT} (+ index.html, {ICON_NAME})")
    return 0


def render_index(output: dict) -> str:
    """Render a simple dark price table for the Pages root."""
    prices = output.get("prices", {})
    updated = time.strftime("%Y-%m-%d %H:%M UTC", time.gmtime(output.get("updated_at") or 0))
    # Escape the commodity name: it comes from the UEX feed and is interpolated
    # into the published Pages HTML, so an unescaped name with markup would be
    # stored XSS. Sell/buy are int()-coerced below, so they can't carry markup.
    rows = "\n".join(
        f"      <tr><td>{html.escape(str(p.get('name', oid)))}</td>"
        f"<td class='num'>{int(p.get('sell') or 0):,}</td>"
        f"<td class='num'>{int(p.get('buy') or 0):,}</td></tr>"
        for oid, p in sorted(prices.items(), key=lambda kv: kv[1].get("sell", 0), reverse=True)
    )
    return f"""<!doctype html>
<html lang="en"><head><meta charset="utf-8">
<title>SC Ore Scanner — Ore Prices</title>
<link rel="icon" type="image/x-icon" href="{ICON_NAME}">
<meta name="viewport" content="width=device-width, initial-scale=1">
<style>
  body{{font-family:system-ui,Segoe UI,sans-serif;background:#0b1418;color:#cfe6ee;
       max-width:680px;margin:2.5rem auto;padding:0 1rem;line-height:1.5}}
  h1{{color:#4fd6e6;margin-bottom:.2rem}}
  a{{color:#4fd6e6}} .meta{{color:#7e98a3;font-size:.9rem;margin-bottom:1.2rem}}
  table{{width:100%;border-collapse:collapse}}
  th,td{{padding:.45rem .6rem;border-bottom:1px solid #1c333d;text-align:left}}
  th{{color:#9fd;font-weight:600}} .num{{text-align:right;font-variant-numeric:tabular-nums}}
  tr:hover td{{background:#11222a}}
  footer{{margin-top:1.5rem;color:#7e98a3;font-size:.85rem}}
</style></head><body>
  <h1><img src="{ICON_NAME}" alt="" width="30" height="30" style="vertical-align:-5px;margin-right:.45rem">SC Ore Scanner — Ore Prices</h1>
  <div class="meta">{len(prices)} ores · updated {updated} · refreshed hourly ·
    raw <a href="prices.json">prices.json</a></div>
  <table>
    <thead><tr><th>Ore</th><th class="num">Sell (aUEC/SCU)</th><th class="num">Buy (aUEC/SCU)</th></tr></thead>
    <tbody>
{rows}
    </tbody>
  </table>
  <footer>
    Price data from <a href="https://uexcorp.space">UEX Corp</a> (community-maintained).
    Part of <a href="https://github.com/Hunter-36/sc-ore-scanner">SC Ore Scanner</a>.
  </footer>
</body></html>
"""


if __name__ == "__main__":
    sys.exit(main())
