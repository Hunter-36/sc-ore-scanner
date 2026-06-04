"""Unit tests for the ore PriceCache (no network — uses _apply directly)."""

from src.prices import PriceCache


def test_apply_and_lookup():
    cache = PriceCache("http://example/prices.json")
    cache._apply({
        "updated_at": 123,
        "prices": {"beryl": {"name": "Beryl", "sell": 19745, "buy": 15390}},
    })
    assert cache.sell_price("beryl") == 19745
    assert cache.value_of("beryl", 3) == 59235
    assert cache.sell_price("unknown") is None
    assert cache.value_of("unknown", 5) is None

    summary = cache.summary()
    assert summary["count"] == 1
    assert summary["updated_at"] == 123
    assert summary["url"] == "http://example/prices.json"


def test_apply_empty_payload():
    cache = PriceCache("u")
    cache._apply({})
    assert cache.prices == {}
    assert cache.sell_price("beryl") is None
    assert cache.value_of("beryl", 2) is None
