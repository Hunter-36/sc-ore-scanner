//! Detection debouncing — descended from the Python OCR engine's
//! `_update_debouncing` / `get_confirmed_numbers`. A number must be present in
//! `min_frames` of the last `2*min_frames` frames before it's "confirmed", which
//! filters out transient OCR misreads (the role v1's confidence gate also played;
//! ocrs does not expose per-line confidence, so debouncing carries it).
//!
//! The window (rather than v1's *strictly consecutive* run) tolerates OCR jitter:
//! a sig whose last digit wobbles frame-to-frame (e.g. 14,160 vs 14,150) splits
//! into two raw reads that each only appear ~half the frames, so a consecutive-run
//! rule never confirmed either and the card flickered. Counting presence in a
//! window keeps the dominant read confirmed through the jitter.

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

    /// The look-back window for confirmation: twice the confirm threshold (capped
    /// at the stored history), so a number needs to be present in at least *half*
    /// the recent frames — tolerant of every-other-frame OCR jitter.
    fn window(&self) -> usize {
        (self.min_frames * 2).min(HISTORY_LEN)
    }

    /// Numbers present in at least `min_frames` of the last `window()` frames.
    pub fn confirmed(&self) -> Vec<i64> {
        let window = self.window();
        self.history
            .iter()
            .filter(|(_, dq)| {
                dq.iter().rev().take(window).filter(|&&b| b).count() >= self.min_frames
            })
            .map(|(&n, _)| n)
            .collect()
    }

    pub fn reset(&mut self) {
        self.history.clear();
    }

    /// Change the confirmation threshold in place, preserving detection history —
    /// so live-tuning the frame count (settings UI) doesn't drop the ore that's
    /// currently shown. `confirmed()` keys off `min_frames`, so lowering it
    /// confirms immediately from existing history.
    pub fn set_min_frames(&mut self, min_frames: u32) {
        self.min_frames = (min_frames as usize).max(1);
    }
}

#[cfg(test)]
mod tests {
    use super::Debouncer;

    #[test]
    fn confirms_after_min_frames() {
        let mut d = Debouncer::new(3);
        d.update(&[7080]);
        assert!(d.confirmed().is_empty(), "1 frame");
        d.update(&[7080]);
        assert!(d.confirmed().is_empty(), "2 frames");
        d.update(&[7080]);
        assert_eq!(d.confirmed(), vec![7080], "3 frames -> confirmed");
    }

    #[test]
    fn tolerates_intermittent_misses() {
        // The flicker fix: a number read in most-but-not-all recent frames (OCR
        // jitter, e.g. 14,160 alternating with a 14,150 misread) must still
        // confirm — >=min_frames of the last 2*min_frames frames, not a strict
        // consecutive run. Here 14160 is present on 3 of the last 5 frames.
        let mut d = Debouncer::new(3);
        d.update(&[14160]);
        d.update(&[14150]); // jitter — 14160 absent
        d.update(&[14160]);
        d.update(&[14150]); // jitter again
        d.update(&[14160]);
        assert!(
            d.confirmed().contains(&14160),
            "dominant read confirms through jitter"
        );
        // The minority read (2 of 5) stays below the bar on its own.
        assert!(
            !d.confirmed().contains(&14150),
            "minority read not yet confirmed"
        );
    }

    #[test]
    fn a_single_dropped_frame_no_longer_blanks() {
        // Previously one missed frame broke the streak and blanked the card; now a
        // lone miss inside the window is tolerated.
        let mut d = Debouncer::new(3);
        d.update(&[7080]);
        d.update(&[7080]);
        d.update(&[]); // one missed frame
        d.update(&[7080]);
        assert_eq!(
            d.confirmed(),
            vec![7080],
            "3 of last 4 frames -> still confirmed"
        );
    }

    #[test]
    fn transient_misread_never_confirms() {
        let mut d = Debouncer::new(3);
        d.update(&[7080, 88000]); // 88000 is a one-frame misread
        d.update(&[7080]);
        d.update(&[7080]);
        assert_eq!(d.confirmed(), vec![7080]);
    }

    #[test]
    fn reset_clears_history() {
        // Mirrors the scan loop resetting on recalibration: a number confirmed
        // in the old region must not carry over and falsely confirm in the new.
        let mut d = Debouncer::new(3);
        d.update(&[7080]);
        d.update(&[7080]);
        d.update(&[7080]);
        assert_eq!(d.confirmed(), vec![7080]);

        d.reset();
        assert!(d.confirmed().is_empty(), "reset clears confirmed history");

        // After reset the streak starts from zero, not from the old count.
        d.update(&[7080]);
        d.update(&[7080]);
        assert!(d.confirmed().is_empty(), "only 2 frames since reset");
        d.update(&[7080]);
        assert_eq!(d.confirmed(), vec![7080]);
    }

    #[test]
    fn set_min_frames_preserves_history() {
        let mut d = Debouncer::new(3);
        d.update(&[7080]);
        d.update(&[7080]);
        assert!(d.confirmed().is_empty(), "2 frames, threshold 3");
        // Lowering the threshold confirms immediately from existing history —
        // no rebuild, no flicker.
        d.set_min_frames(2);
        assert_eq!(d.confirmed(), vec![7080]);
    }
}
