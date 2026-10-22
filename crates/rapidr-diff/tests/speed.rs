//! The diff's speed on a big source file (lane G2's acceptance: a
//! 4,500-line file diffs in under 50 ms). Timed in release builds
//! (`cargo test --release -p rapidr-diff --test speed -- --nocapture`);
//! debug builds only check the results.

use std::time::Instant;

use rapidr_diff::text::{changes, hunks, split_lines, word_marks, HunkLine};

/// A program of `n` lines that reads like RapidR code: SUBs of a dozen
/// lines, blank lines between them, repeated lines (END SUB, END IF …).
fn program(n: usize) -> Vec<String> {
    let mut out = Vec::with_capacity(n);
    let mut sub = 0;
    while out.len() < n {
        out.push(format!("SUB Handler{sub} (Sender AS RButton)"));
        out.push("    DIM i AS INTEGER".into());
        out.push(format!("    FOR i = 1 TO {}", sub % 17 + 3));
        out.push(format!("        Total = Total + i * {}", sub % 7 + 1));
        out.push(format!("        IF Total > {} THEN", 100 + sub));
        out.push(format!("            Label{}.Caption = \"Total: \" + STR$(Total)", sub % 9));
        out.push("        END IF".into());
        out.push("    NEXT i".into());
        out.push(format!("    PRINT \"Handler {sub} done\""));
        out.push("END SUB".into());
        out.push(String::new());
        sub += 1;
    }
    out.truncate(n);
    out
}

struct Rng(u64);
impl Rng {
    fn below(&mut self, n: usize) -> usize {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        (self.0 % n as u64) as usize
    }
}

/// `edits` scattered edits: changed, inserted and removed lines.
fn edited(lines: &[String], edits: usize, seed: u64) -> Vec<String> {
    let mut rng = Rng(seed);
    let mut out = lines.to_vec();
    for e in 0..edits {
        let at = rng.below(out.len());
        match e % 3 {
            0 => out[at] = format!("{} ' changed", out[at]),
            1 => out.insert(at, format!("    PRINT \"new line {e}\"")),
            _ => {
                out.remove(at);
            }
        }
    }
    out
}

/// The whole job a diff view does: the line diff, the hunks with three
/// lines of context, and the word marks of every changed line pair.
fn full_diff(old: &str, new: &str) -> (usize, f64) {
    let t = Instant::now();
    let (a, b) = (split_lines(old.as_bytes()), split_lines(new.as_bytes()));
    let ch = changes(&a, &b);
    let hs = hunks(&ch, a.len(), b.len(), 3);
    let mut marked = 0;
    for c in &ch {
        for k in 0..c.old.len().min(c.new.len()) {
            let (o, n) = (String::from_utf8_lossy(a[c.old.start + k]), String::from_utf8_lossy(b[c.new.start + k]));
            if word_marks(&o, &n).is_some() {
                marked += 1;
            }
        }
    }
    let ms = t.elapsed().as_secs_f64() * 1000.0;
    // (the hunks give the new text back)
    let mut rebuilt = String::new();
    let mut j = 0;
    for h in &hs {
        for l in &h.lines {
            if let HunkLine::Same { new, .. } | HunkLine::Added(new) = *l {
                if new >= j {
                    for line in &b[j..=new] {
                        rebuilt.push_str(std::str::from_utf8(line).unwrap());
                    }
                    j = new + 1;
                }
            }
        }
    }
    for line in &b[j..] {
        rebuilt.push_str(std::str::from_utf8(line).unwrap());
    }
    assert_eq!(rebuilt, new);
    let _ = marked;
    (hs.len(), ms)
}

#[test]
fn a_4500_line_file_diffs_in_under_50_ms() {
    let base = program(4500);
    let old = base.join("\n") + "\n";
    let cases = [
        ("10 scattered edits", edited(&base, 10, 1)),
        ("100 scattered edits", edited(&base, 100, 2)),
        ("900 scattered edits (a fifth of the lines)", edited(&base, 900, 3)),
        ("a block of 400 lines rewritten", {
            let mut v = base.clone();
            for line in &mut v[2000..2400] {
                *line = format!("    ' rewritten: {}", line.trim());
            }
            v
        }),
        ("every line different", base.iter().map(|l| format!("{l} '")).collect()),
        ("the file reversed", base.iter().rev().cloned().collect()),
    ];
    let mut worst: f64 = 0.0;
    for (what, lines) in cases {
        let new = lines.join("\n") + "\n";
        // (the best of three: the machine's other work isn't the diff's)
        let mut best = f64::MAX;
        let mut n = 0;
        for _ in 0..3 {
            let (h, ms) = full_diff(&old, &new);
            n = h;
            best = best.min(ms);
        }
        eprintln!("4,500 lines, {what}: {n} hunks in {best:.2} ms");
        worst = worst.max(best);
    }
    if !cfg!(debug_assertions) {
        assert!(worst < 50.0, "the slowest diff took {worst:.1} ms");
    }
}
