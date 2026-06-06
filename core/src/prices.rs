//! Ore price cache — reads the published UEX feed (a small prices.json on GitHub
//! Pages) over HTTP and caches it. Mirrors the Python PriceCache: fetch on a
//! timer (the caller decides cadence), never per scan; keep last good values on
//! failure. Price data: UEX Corp (https://uexcorp.space).

use std::collections::HashMap;
use std::io::Read;
use std::time::Duration;

use serde::Deserialize;

pub const DEFAULT_FEED_URL: &str = "https://hunter-36.github.io/sc-ore-scanner/prices.json";

/// Default connect/read timeout for the feed fetch. It runs on the scan thread
/// (startup + hourly), so an unbounded request would stall the overlay; bound it.
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(10);

/// Hard cap on the response body. The real feed is a few KB; this guards against
/// a runaway or hostile response exhausting memory. An oversized body is read up
/// to the cap, then fails to parse — so the cache is preserved (see `refresh`).
const MAX_FEED_BYTES: u64 = 4 * 1024 * 1024;

#[derive(Deserialize, Clone, Default)]
pub struct PriceEntry {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub sell: i64,
    #[serde(default)]
    pub buy: i64,
}

#[derive(Deserialize, Default)]
struct Feed {
    #[serde(default)]
    prices: HashMap<String, PriceEntry>,
    #[serde(default)]
    updated_at: Option<i64>,
}

pub struct PriceCache {
    url: String,
    prices: HashMap<String, PriceEntry>,
    pub updated_at: Option<i64>,
    timeout: Duration,
}

impl PriceCache {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            prices: HashMap::new(),
            updated_at: None,
            timeout: DEFAULT_TIMEOUT,
        }
    }

    /// Override the network timeout (primarily for tests).
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Fetch the feed and replace the cache. On error — including a connect/read
    /// timeout or an over-cap response — the previous cache is kept untouched.
    pub fn refresh(&mut self) -> anyhow::Result<()> {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(self.timeout)
            .timeout_read(self.timeout)
            .build();
        let resp = agent.get(&self.url).call()?;

        // Cap the body: read at most MAX_FEED_BYTES. An oversized feed is
        // truncated here and then fails JSON parsing, so we never accept it and
        // the cache (assigned only after a successful parse) survives.
        let mut body = String::new();
        resp.into_reader()
            .take(MAX_FEED_BYTES)
            .read_to_string(&mut body)?;
        let feed: Feed = serde_json::from_str(&body)?;
        self.prices = feed.prices;
        self.updated_at = feed.updated_at;
        Ok(())
    }

    /// Sell price per SCU for an ore id, if known and > 0.
    pub fn sell_price(&self, ore_id: &str) -> Option<i64> {
        self.prices.get(ore_id).map(|p| p.sell).filter(|&s| s > 0)
    }

    pub fn len(&self) -> usize {
        self.prices.len()
    }

    pub fn is_empty(&self) -> bool {
        self.prices.is_empty()
    }
}
