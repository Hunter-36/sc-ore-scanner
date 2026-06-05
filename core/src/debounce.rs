//! Detection debouncing — a faithful port of the Python OCR engine's
//! `_update_debouncing` / `get_confirmed_numbers`. A number must be present in
//! the last N consecutive frames before it's "confirmed", which filters out
//! transient OCR misreads (the role v1's confidence gate also played; ocrs does
//! not expose per-line confidence, so debouncing carries it).

use std::collections::{HashMap, HashSet, VecDeque};

const HISTORY_LEN: usize = 10;

pub struct Debouncer {
    min_frames: usize,
    history: HashMap<i64, VecDeque<bool>>,
}

impl Debouncer {
    pub fn new(min_frames: u32) -> Self {
        Self {
            min_frames: (min_frames as usize).max(1),
            history: HashMap::new(),
        }
    }

    /// Record this frame's detected numbers (push true for present, false for
    /// absent), capping each history at HISTORY_LEN and dropping numbers that
    /// have been absent for a full window.
    pub fn update(&mut self, detected: &[i64]) {
        let present: HashSet<i64> = detected.iter().copied().collect();

        for &n in &present {
            let dq = self.history.entry(n).or_default();
            dq.push_back(true);
            while dq.len() > HISTORY_LEN {
                dq.pop_front();
            }
        }

        let keys: Vec<i64> = self.history.keys().copied().collect();
        for n in keys {
            if !present.contains(&n) {
                let dq = self.history.get_mut(&n).unwrap();
                dq.push_back(false);
                while dq.len() > HISTORY_LEN {
                    dq.pop_front();
                }
                if dq.len() >= HISTORY_LEN && dq.iter().all(|&b| !b) {
                    self.history.remove(&n);
                }
            }
        }
    }

    /// Numbers detected in the last `min_frames` consecutive frames.
    pub fn confirmed(&self) -> Vec<i64> {
        self.history
            .iter()
            .filter(|(_, dq)| {
                dq.len() >= self.min_frames && dq.iter().rev().take(self.min_frames).all(|&b| b)
            })
            .map(|(&n, _)| n)
            .collect()
    }

    pub fn reset(&mut self) {
        self.history.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::Debouncer;

    #[test]
    fn confirms_after_min_consecutive_frames() {
        let mut d = Debouncer::new(3);
        d.update(&[7080]);
        assert!(d.confirmed().is_empty(), "1 frame");
        d.update(&[7080]);
        assert!(d.confirmed().is_empty(), "2 frames");
        d.update(&[7080]);
        assert_eq!(d.confirmed(), vec![7080], "3 frames -> confirmed");
    }

    #[test]
    fn a_gap_resets_the_streak() {
        let mut d = Debouncer::new(3);
        d.update(&[7080]);
        d.update(&[7080]);
        d.update(&[]); // missed frame
        assert!(d.confirmed().is_empty());
        d.update(&[7080]);
        d.update(&[7080]);
        assert!(d.confirmed().is_empty(), "only 2 since the gap");
        d.update(&[7080]);
        assert_eq!(d.confirmed(), vec![7080]);
    }

    #[test]
    fn transient_misread_never_confirms() {
        let mut d = Debouncer::new(3);
        d.update(&[7080, 88000]); // 88000 is a one-frame misread
        d.update(&[7080]);
        d.update(&[7080]);
        assert_eq!(d.confirmed(), vec![7080]);
    }
}
