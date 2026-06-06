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
import sys
import time
import urllib.request
from pathlib import Path

UEX_URL = "https://api.uexcorp.uk/2.0/commodities"
REPO_ROOT = Path(__file__).resolve().parent.parent
SIGNATURES = REPO_ROOT / "core" / "data" / "signatures.json"
OUT = REPO_ROOT / "public" / "prices.json"

# Brand icon (48px PNG) inlined as a data URI so the published Pages table is
# self-contained (no extra asset to deploy). Source: docs/assets/icon-master.png.
FAVICON_DATA_URI = (
    "data:image/png;base64,"
    "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"
)


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

    print(f"Wrote {len(prices)} ore prices to {OUT} (+ index.html)")
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
<link rel="icon" type="image/png" href="{FAVICON_DATA_URI}">
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
  <h1><img src="{FAVICON_DATA_URI}" alt="" width="30" height="30" style="vertical-align:-5px;margin-right:.45rem">SC Ore Scanner — Ore Prices</h1>
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
