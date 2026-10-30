//! Text kept where a crash can read it without a lock or an allocation: a
//! signal handler (a stack overflow) and the hang watchdog's thread read
//! what the program's thread wrote.
//!
//! A [`Slot`] is a fixed byte buffer of atomics: written with its length
//! set to 0 first and to the new length last, so a reader that interrupts
//! a write (a signal on the same thread) sees nothing rather than half a
//! line; a [`Ring`] keeps the last few lines.

use std::sync::atomic::{AtomicU8, AtomicUsize, Ordering};

/// Up to `N` bytes of UTF-8 text.
pub struct Slot<const N: usize> {
    len: AtomicUsize,
    bytes: [AtomicU8; N],
}

/// `text` cut to at most `max` bytes on a character's boundary.
pub fn cut(text: &str, max: usize) -> &str {
    if text.len() <= max {
        return text;
    }
    let mut end = max;
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    &text[..end]
}

impl<const N: usize> Slot<N> {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Slot { len: AtomicUsize::new(0), bytes: [const { AtomicU8::new(0) }; N] }
    }

    /// Keeps `parts` one after the other (cut to the slot's size on a
    /// character's boundary).
    pub fn set_parts(&self, parts: &[&str]) {
        self.len.store(0, Ordering::Release);
        let mut at = 0;
        for part in parts {
            let room = N - at;
            let p = cut(part, room);
            for (i, b) in p.bytes().enumerate() {
                self.bytes[at + i].store(b, Ordering::Relaxed);
            }
            at += p.len();
            if p.len() < part.len() {
                break;
            }
        }
        self.len.store(at, Ordering::Release);
    }

    pub fn set(&self, text: &str) {
        self.set_parts(&[text]);
    }

    pub fn clear(&self) {
        self.len.store(0, Ordering::Release);
    }

    /// Copies the text into `out` (no allocation): its length.
    pub fn read_into(&self, out: &mut [u8]) -> usize {
        let n = self.len.load(Ordering::Acquire).min(N).min(out.len());
        for (i, o) in out.iter_mut().enumerate().take(n) {
            *o = self.bytes[i].load(Ordering::Relaxed);
        }
        n
    }

    /// The text (an allocation: not in a signal handler).
    pub fn get(&self) -> String {
        let mut buf = vec![0u8; N];
        let n = self.read_into(&mut buf);
        buf.truncate(n);
        String::from_utf8_lossy(&buf).into_owned()
    }

    pub fn is_empty(&self) -> bool {
        self.len.load(Ordering::Acquire) == 0
    }
}

/// The last `SLOTS` lines of up to `N` bytes each.
pub struct Ring<const SLOTS: usize, const N: usize> {
    next: AtomicUsize,
    slots: [Slot<N>; SLOTS],
}

impl<const SLOTS: usize, const N: usize> Ring<SLOTS, N> {
    #[allow(clippy::new_without_default)]
    pub const fn new() -> Self {
        Ring { next: AtomicUsize::new(0), slots: [const { Slot::new() }; SLOTS] }
    }

    pub fn push(&self, line: &str) {
        let i = self.next.fetch_add(1, Ordering::AcqRel);
        self.slots[i % SLOTS].set(line);
    }

    /// Calls `f` with each line kept, the oldest first (no allocation).
    pub fn each(&self, mut f: impl FnMut(&[u8])) {
        let total = self.next.load(Ordering::Acquire);
        let start = total.saturating_sub(SLOTS);
        let mut buf = [0u8; N];
        for k in start..total {
            let n = self.slots[k % SLOTS].read_into(&mut buf);
            if n > 0 {
                f(&buf[..n]);
            }
        }
    }

    /// The lines kept, the oldest first.
    pub fn lines(&self) -> Vec<String> {
        let mut out = Vec::new();
        self.each(|b| out.push(String::from_utf8_lossy(b).into_owned()));
        out
    }

    /// How many lines were ever pushed.
    pub fn count(&self) -> usize {
        self.next.load(Ordering::Acquire)
    }

    pub fn clear(&self) {
        let total = self.next.load(Ordering::Acquire);
        for k in total.saturating_sub(SLOTS)..total {
            self.slots[k % SLOTS].clear();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slots_keep_whole_characters_and_rings_the_last_lines() {
        let s: Slot<8> = Slot::new();
        s.set("añoaño-and-more");
        // (8 bytes: "añoaño" is 8 bytes exactly)
        assert_eq!(s.get(), "añoaño");
        s.set_parts(&["On", "\u{1f}", "Clock"]);
        assert_eq!(s.get(), "On\u{1f}Clock");
        s.set("ñññññ");
        assert_eq!(s.get(), "ññññ");
        let r: Ring<3, 16> = Ring::new();
        for l in ["one", "two", "three", "four"] {
            r.push(l);
        }
        assert_eq!(r.lines(), ["two", "three", "four"]);
        assert_eq!(r.count(), 4);
        r.clear();
        assert!(r.lines().is_empty());
    }
}
