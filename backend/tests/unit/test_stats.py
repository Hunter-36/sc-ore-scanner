"""Unit tests for SessionStats."""

from types import SimpleNamespace

from src.stats import SessionStats


def _match(name, tier, qty):
    """Minimal stand-in for an OreMatch (duck-typed)."""
    return SimpleNamespace(ore=SimpleNamespace(name=name, tier=tier), quantity=qty)


def test_record_new_and_repeat_ore():
    s = SessionStats()
    s.record({"beryl": _match("Beryl", "A", 3)})
    s.record({"beryl": _match("Beryl", "A", 2)})

    summary = s.summary()
    assert summary["distinct_ores"] == 1
    assert summary["total_detections"] == 2

    beryl = summary["ores"]["beryl"]
    assert beryl["name"] == "Beryl"
    assert beryl["times_seen"] == 2
    assert beryl["max_quantity"] == 3      # max of (3, 2)
    assert beryl["total_quantity"] == 5    # 3 + 2


def test_record_multiple_ores():
    s = SessionStats()
    s.record({"beryl": _match("Beryl", "A", 3), "iron": _match("Iron", "C", 1)})
    summary = s.summary()
    assert summary["distinct_ores"] == 2
    assert summary["total_detections"] == 2


def test_to_csv():
    s = SessionStats()
    s.record({"beryl": _match("Beryl", "A", 3)})
    csv = s.to_csv()
    assert csv.startswith("ore_id,name,tier,times_seen,max_quantity,total_quantity")
    assert "beryl,Beryl,A,1,3,3" in csv


def test_reset():
    s = SessionStats()
    s.record({"beryl": _match("Beryl", "A", 1)})
    s.reset()
    summary = s.summary()
    assert summary["distinct_ores"] == 0
    assert summary["total_detections"] == 0
