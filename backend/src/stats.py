"""In-memory session statistics for detected ores.

A "session" is one backend run. The scanning loop records each cycle's detected
ores; the /stats endpoint exposes a summary and /stats/export.csv a CSV. Stats
reset when the backend restarts.
"""

import time
from dataclasses import asdict, dataclass
from typing import Dict


@dataclass
class OreStat:
    name: str
    tier: str
    first_seen: float
    last_seen: float
    times_seen: int = 0       # scan cycles this ore appeared in
    max_quantity: int = 0     # highest quantity seen in a single reading
    total_quantity: int = 0   # sum of quantities across sightings


class SessionStats:
    """Accumulates per-ore detection stats for the current backend session."""

    def __init__(self):
        self.start_time = time.time()
        self.ores: Dict[str, OreStat] = {}
        self.total_detections = 0  # ore-sightings summed across all scan cycles

    def record(self, aggregated: Dict[str, object]) -> None:
        """Record one scan cycle's aggregated ore matches (ore_id -> OreMatch)."""
        now = time.time()
        for ore_id, match in aggregated.items():
            self.total_detections += 1
            stat = self.ores.get(ore_id)
            if stat is None:
                self.ores[ore_id] = OreStat(
                    name=match.ore.name,
                    tier=match.ore.tier,
                    first_seen=now,
                    last_seen=now,
                    times_seen=1,
                    max_quantity=match.quantity,
                    total_quantity=match.quantity,
                )
            else:
                stat.times_seen += 1
                stat.max_quantity = max(stat.max_quantity, match.quantity)
                stat.total_quantity += match.quantity
                stat.last_seen = now

    def summary(self) -> dict:
        """Compact summary for the /stats endpoint and the live overlay footer."""
        return {
            "session_start": self.start_time,
            "elapsed_seconds": round(time.time() - self.start_time, 1),
            "distinct_ores": len(self.ores),
            "total_detections": self.total_detections,
            "ores": {ore_id: asdict(stat) for ore_id, stat in self.ores.items()},
        }

    def to_csv(self) -> str:
        """Session stats as CSV text."""
        rows = ["ore_id,name,tier,times_seen,max_quantity,total_quantity"]
        for ore_id, s in self.ores.items():
            rows.append(
                f"{ore_id},{s.name},{s.tier},{s.times_seen},{s.max_quantity},{s.total_quantity}"
            )
        return "\n".join(rows) + "\n"

    def reset(self) -> None:
        self.start_time = time.time()
        self.ores.clear()
        self.total_detections = 0
