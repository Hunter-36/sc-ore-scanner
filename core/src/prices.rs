//! Ore price cache — reads the published UEX feed (a small prices.json on GitHub
//! Pages) over HTTP and caches it. Mirrors the Python PriceCache: fetch on a
//! timer (the caller decides cadence), never per scan; keep last good values on
//! failure. Price data: UEX Corp (https://uexcorp.space).

use std::collections::HashMap;

use serde::Deserialize;

pub const DEFAULT_FEED_URL: &str = "https://hunter-36.github.io/sc-ore-scanner/prices.json";

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
}

impl PriceCache {
    pub fn new(url: impl Into<String>) -> Self {
        Self {
            url: url.into(),
            prices: HashMap::new(),
            updated_at: None,
        }
    }

    /// Fetch the feed and replace the cache. On error, the previous cache is kept.
    pub fn refresh(&mut self) -> anyhow::Result<()> {
        let body = ureq::get(&self.url).call()?.into_string()?;
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
