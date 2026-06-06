//! PriceCache network behaviour — the feed fetch must bound time and size and
//! never drop the last good cache on failure. Uses a tiny local TCP server so
//! the tests are hermetic (no real network).

use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::thread;
use std::time::{Duration, Instant};

use scanner_core::prices::PriceCache;

/// What a single connection should do.
enum Action {
    /// Send a normal `200 OK` with this body.
    Body(Vec<u8>),
    /// Accept the connection but never send a response (simulates a hung feed).
    Hang,
}

/// Spawn a one-shot-per-connection HTTP server on an ephemeral port. Each queued
/// `Action` handles exactly one connection, in order. Returns the feed URL.
fn spawn_server(actions: Vec<Action>) -> String {
    let listener = TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    thread::spawn(move || {
        for action in actions {
            let Ok((mut stream, _)) = listener.accept() else {
                break;
            };
            drain_request(&mut stream);
            match action {
                Action::Body(bytes) => {
                    let header = format!(
                        "HTTP/1.1 200 OK\r\nContent-Length: {}\r\nContent-Type: \
                         application/json\r\nConnection: close\r\n\r\n",
                        bytes.len()
                    );
                    let _ = stream.set_write_timeout(Some(Duration::from_secs(2)));
                    let _ = stream.write_all(header.as_bytes());
                    let _ = stream.write_all(&bytes);
                    let _ = stream.flush();
                }
                Action::Hang => {
                    // Hold the connection briefly, sending nothing, then drop it.
                    thread::sleep(Duration::from_secs(3));
                }
            }
        }
    });
    format!("http://{addr}/prices.json")
}

/// Read the request headers so the client's write completes before we respond.
fn drain_request(stream: &mut TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_millis(500)));
    let mut buf = [0u8; 1024];
    let mut data = Vec::new();
    loop {
        match stream.read(&mut buf) {
            Ok(0) => break,
            Ok(n) => {
                data.extend_from_slice(&buf[..n]);
                if data.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
            }
            Err(_) => break,
        }
    }
}

#[test]
fn refresh_populates_then_preserves_on_failure() {
    let good =
        br#"{"prices":{"QUANT":{"name":"Quantanium","sell":100,"buy":50}},"updated_at":123}"#
            .to_vec();
    let bad = b"this is not json".to_vec();
    let url = spawn_server(vec![Action::Body(good), Action::Body(bad)]);

    let mut cache = PriceCache::new(url);
    cache.refresh().expect("first refresh should succeed");
    assert_eq!(cache.len(), 1);
    assert_eq!(cache.sell_price("QUANT"), Some(100));
    assert_eq!(cache.updated_at, Some(123));

    // Second fetch returns garbage: refresh errors, but the good cache survives.
    assert!(
        cache.refresh().is_err(),
        "garbage body should fail to parse"
    );
    assert_eq!(cache.len(), 1, "cache preserved on failure");
    assert_eq!(cache.sell_price("QUANT"), Some(100));
}

#[test]
fn unresponsive_feed_times_out_without_hanging() {
    let url = spawn_server(vec![Action::Hang]);
    let mut cache = PriceCache::new(url).with_timeout(Duration::from_millis(300));

    let start = Instant::now();
    let result = cache.refresh();
    let elapsed = start.elapsed();

    assert!(result.is_err(), "a hung feed must surface as an error");
    assert!(
        elapsed < Duration::from_secs(2),
        "should time out quickly, took {elapsed:?}"
    );
    assert!(
        cache.is_empty(),
        "cache stays empty after a failed first fetch"
    );
}

#[test]
fn oversized_feed_is_rejected_and_cache_preserved() {
    // A valid JSON prefix followed by padding past the cap: the body is read up
    // to the cap, truncated mid-string, and then fails to parse.
    let mut body = Vec::from(&b"{\"prices\":{},\"pad\":\""[..]);
    body.resize(5 * 1024 * 1024, b'a');
    let url = spawn_server(vec![Action::Body(body)]);

    let mut cache = PriceCache::new(url);
    assert!(
        cache.refresh().is_err(),
        "an over-cap body must not be accepted"
    );
    assert!(cache.is_empty());
}
