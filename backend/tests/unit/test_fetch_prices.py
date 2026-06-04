"""Unit test for the price-feed mapping in scripts/fetch_prices.py."""

import sys
from pathlib import Path

REPO_ROOT = Path(__file__).resolve().parents[3]
if str(REPO_ROOT) not in sys.path:
    sys.path.insert(0, str(REPO_ROOT))

from scripts.fetch_prices import build_prices  # noqa: E402


def test_build_prices_maps_by_name_and_keeps_highest_sell():
    ore_name_to_id = {"beryl": "beryl", "gold": "gold"}
    commodities = [
        {"name": "Beryl", "price_sell": 19745, "price_buy": 15390},
        {"name": "Beryl (Raw)", "price_sell": 9999, "price_buy": 0},  # different name -> ignored
        {"name": "Gold", "price_sell": 6000, "price_buy": 5000},
        {"name": "Gold", "price_sell": 7000, "price_buy": 0},          # dup -> keep higher sell
        {"name": "Quantanium", "price_sell": 88000, "price_buy": 0},   # not in map -> skip
    ]
    prices = build_prices(commodities, ore_name_to_id)
    assert set(prices) == {"beryl", "gold"}
    assert prices["beryl"]["sell"] == 19745
    assert prices["gold"]["sell"] == 7000


def test_build_prices_handles_missing_price_fields():
    prices = build_prices([{"name": "Beryl"}], {"beryl": "beryl"})
    assert prices["beryl"]["sell"] == 0
    assert prices["beryl"]["buy"] == 0
