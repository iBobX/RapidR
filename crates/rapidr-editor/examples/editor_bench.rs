//! The editor model's benchmark (docs/ide-plan.md §6.2): a 200,000-line /
//! 10 MB BASIC file — opening it, typing in it, undo, search, colouring,
//! folding, many carets, memory.
//!
//!     cargo run --release -p rapidr-editor --example editor_bench [-- --quick]
//!
//! Prints a table and exits 1 when a target is missed. The targets are the
//! model's share of §6.2's (the view's layout and painting come on top, in
//! the kernel's own benchmark): open to first paint ≤ 300 ms, typing p99 ≤
//! 2 ms (of the 16 ms key-to-pixels budget), memory ≤ 3 × the file + 50 MB.

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use rapidr_editor::{Buffer, Direction, Document, Languages, SearchQuery, Selection, Selections};

/// Counts live heap bytes (and the peak).
struct Counting;

static LIVE: AtomicUsize = AtomicUsize::new(0);
static PEAK: AtomicUsize = AtomicUsize::new(0);

unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let p = unsafe { System.alloc(layout) };
        if !p.is_null() {
            let now = LIVE.fetch_add(layout.size(), Ordering::Relaxed) + layout.size();
            PEAK.fetch_max(now, Ordering::Relaxed);
        }
        p
    }
    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        unsafe { System.dealloc(ptr, layout) };
        LIVE.fetch_sub(layout.size(), Ordering::Relaxed);
    }
    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let p = unsafe { System.realloc(ptr, layout, new_size) };
        if !p.is_null() {
            if new_size > layout.size() {
                let now = LIVE.fetch_add(new_size - layout.size(), Ordering::Relaxed) + new_size - layout.size();
                PEAK.fetch_max(now, Ordering::Relaxed);
            } else {
                LIVE.fetch_sub(layout.size() - new_size, Ordering::Relaxed);
            }
        }
        p
    }
}

#[global_allocator]
static A: Counting = Counting;

/// A deterministic pseudo-random sequence (no dependencies).
struct Rng(u64);
impl Rng {
    fn next(&mut self) -> u64 {
        self.0 ^= self.0 << 13;
        self.0 ^= self.0 >> 7;
        self.0 ^= self.0 << 17;
        self.0
    }
    fn below(&mut self, n: usize) -> usize {
        (self.next() % n as u64) as usize
    }
}

/// `lines` lines of RapidQ BASIC (~50 bytes a line).
fn basic_source(lines: usize) -> String {
    let mut s = String::with_capacity(lines * 52);
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let mut n = 0;
    let mut sub = 0;
    while n < lines {
        sub += 1;
        s.push_str(&format!("SUB Handler{sub}(Sender AS QBUTTON, Value AS INTEGER)\r\n"));
        s.push_str("    DIM total AS DOUBLE, name$ AS STRING ' locals\r\n");
        n += 2;
        let body = 6 + rng.below(30);
        for i in 0..body {
            let line = match rng.below(9) {
                0 => format!("    IF Value > {i} THEN PRINT \"value is \"; Value; \" in {sub}\""),
                1 => format!("    total = total + SQR(Value * {i}.5) / (1 + ABS(Value - {i}))"),
                2 => "    ' a comment about what happens next, long enough to wrap".to_string(),
                3 => format!("    name$ = LEFT$(\"Handler number {sub}\", {i}) + MID$(name$, 2, 3)"),
                4 => "    FOR i = 1 TO 100 STEP 2: total = total + i: NEXT i".to_string(),
                5 => format!("    Form1.Caption = \"Total: \" + STR$(total) + \" ({i})\""),
                6 => "    SELECT CASE Value: CASE 1: PRINT 1: CASE ELSE: PRINT 0: END SELECT".to_string(),
                7 => format!("    CALL Handler{}(Sender, Value - 1)", sub.max(2) - 1),
                _ => format!("    WHILE total < {i}00: total = total * 1.5: WEND"),
            };
            s.push_str(&line);
            s.push_str("\r\n");
            n += 1;
        }
        s.push_str("END SUB\r\n\r\n");
        n += 2;
    }
    s
}

fn percentile(v: &mut [Duration], p: f64) -> Duration {
    v.sort();
    v[((v.len() as f64 - 1.0) * p).round() as usize]
}

fn ms(d: Duration) -> String {
    format!("{:.3} ms", d.as_secs_f64() * 1000.0)
}

fn main() {
    let quick = std::env::args().any(|a| a == "--quick");
    let lines = if quick { 20_000 } else { 200_000 };
    let base = LIVE.load(Ordering::Relaxed);
    let src = basic_source(lines);
    let file_bytes = src.len();
    let mut failed = Vec::new();
    let mut row = |what: &str, value: String, target: Option<(&str, bool)>| {
        let t = match target {
            Some((t, ok)) => {
                if !ok {
                    failed.push(what.to_string());
                }
                format!("{t:>14}  {}", if ok { "ok" } else { "MISSED" })
            }
            None => String::new(),
        };
        println!("{what:<46} {value:>16}  {t}");
    };
    println!("rapidr-editor benchmark: {} lines, {:.1} MB of RapidQ BASIC (CR LF)\n", src.matches('\n').count(), file_bytes as f64 / 1e6);

    // open to first paint: the document and the first screen's tokens (the
    // built-in languages load on first use, inside this)
    let t = Instant::now();
    let lang = Languages::builtin().get("rapidq-basic").expect("built in");
    let mut doc = Document::new(&src, lang);
    for l in 0..60 {
        let _ = doc.tokens(l);
    }
    let open = t.elapsed();
    row("open + first screen coloured", ms(open), Some(("≤ 300 ms", open <= Duration::from_millis(300))));
    drop(src);

    // the whole file coloured (idle time, after the first paint)
    let t = Instant::now();
    while !doc.highlight_idle(5_000) {}
    let full = t.elapsed();
    row("colour every line (idle work)", ms(full), None);
    let t = Instant::now();
    let _ = doc.tokens(doc.line_count() / 2);
    row("one line's tokens, middle of the file", ms(t.elapsed()), None);

    // memory: the document (rope + colouring states + the languages), the
    // file's text dropped
    let mem = LIVE.load(Ordering::Relaxed).saturating_sub(base);
    let budget = 3 * file_bytes + 50 * 1024 * 1024;
    row("memory: document", format!("{:.1} MB", mem as f64 / 1e6), Some(("≤ 3×file+50MB", mem <= budget)));

    // typing in the middle: each key, then the edited line's colours (what
    // the view asks for next)
    let mid = doc.buffer().line_start(doc.line_count() / 2 + 3);
    doc.set_selections(Selections::caret(mid));
    let mut times = Vec::new();
    let mut now = 1_000u64;
    let typed = "    total = total + 1 ' typed\n".repeat(if quick { 20 } else { 60 });
    for c in typed.chars() {
        now += 120;
        let t = Instant::now();
        if c == '\n' {
            doc.newline(now).expect("newline");
        } else {
            doc.type_text(&c.to_string(), now).expect("type");
        }
        let line = doc.buffer().line_of(doc.selections().primary().head);
        let _ = doc.tokens(line);
        let _ = doc.take_restyled();
        times.push(t.elapsed());
    }
    let (p50, p99) = (percentile(&mut times, 0.5), percentile(&mut times, 0.99));
    row(&format!("typing: {} keys, p50", times.len()), ms(p50), None);
    row("typing: p99 (model's share of 16 ms)", ms(p99), Some(("≤ 2 ms", p99 <= Duration::from_millis(2))));

    // a key that changes the colours below it (a string opened at a line
    // start: BASIC's end at the line's end, so it stops at once)
    let t = Instant::now();
    doc.type_text("\"", now + 1000).expect("type");
    let _ = doc.tokens(doc.buffer().line_of(doc.selections().primary().head) + 1);
    row("type a quote (re-colour stops at the line)", ms(t.elapsed()), None);

    // undo / redo everything typed
    let t = Instant::now();
    let mut steps = 0;
    while doc.undo() {
        steps += 1;
    }
    let undo = t.elapsed();
    row(&format!("undo {steps} steps"), ms(undo), None);
    let t = Instant::now();
    while doc.redo() {}
    row(&format!("redo {steps} steps"), ms(t.elapsed()), None);

    // search
    let t = Instant::now();
    let n = doc.find_all(&SearchQuery::literal("PRINT"), false).expect("search").len();
    row(&format!("find all \"PRINT\" ({n})"), ms(t.elapsed()), None);
    let t = Instant::now();
    let n = doc.find_all(&SearchQuery::regex(r"Handler(\d+)\("), false).expect("search").len();
    row(&format!("find all regex Handler(\\d+)\\( ({n})"), ms(t.elapsed()), None);
    let t = Instant::now();
    let found = doc.find_next(&SearchQuery::with_options("wend", "word"), mid, true).expect("search");
    row("find next whole word", ms(t.elapsed()), None);
    assert!(found.is_some());
    let t = Instant::now();
    let n = doc.replace_all(&SearchQuery::regex(r"Handler(\d+)"), "On$1", false, now + 2000).expect("replace");
    row(&format!("replace all with groups ({n}), one step"), ms(t.elapsed()), None);
    let t = Instant::now();
    doc.undo();
    row("undo that replace", ms(t.elapsed()), None);

    // (26,684 lines changed twice: their colours again, in idle time)
    let t = Instant::now();
    while !doc.highlight_idle(5_000) {}
    row("re-colour after replace all + undo (idle)", ms(t.elapsed()), None);

    // structure
    let t = Instant::now();
    let folds = doc.fold_ranges().len();
    row(&format!("fold ranges ({folds})"), ms(t.elapsed()), None);
    let open_paren = doc.text().find("SQR(").map(|i| i + 3).expect("a call");
    let t = Instant::now();
    let m = doc.matching_bracket(open_paren);
    row("matching bracket", ms(t.elapsed()), None);
    assert!(m.is_some());

    // many carets: one on each of 10,000 lines, one key
    let carets: Vec<Selection> = (0..10_000).map(|i| Selection::caret(doc.buffer().line_start(i * (doc.line_count() / 10_000)))).collect();
    doc.set_selections(Selections::new(carets, 0));
    let t = Instant::now();
    doc.type_text("x", now + 3000).expect("type");
    row("type at 10,000 carets", ms(t.elapsed()), None);
    let t = Instant::now();
    doc.move_word(Direction::Forward, true);
    row("move 10,000 carets a word", ms(t.elapsed()), None);

    let peak = PEAK.load(Ordering::Relaxed).saturating_sub(base);
    row("memory: peak (the file text too, undo, search)", format!("{:.1} MB", peak as f64 / 1e6), None);

    println!();
    if failed.is_empty() {
        println!("all targets met");
    } else {
        println!("MISSED: {}", failed.join(", "));
        std::process::exit(1);
    }
}
