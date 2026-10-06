//! The code editor view's benchmark (docs/ide-plan.md §6.2): RCODEEDITOR on
//! the UI kernel with a 200,000-line / 10 MB BASIC file — open to the first
//! frame, typing latency (key → display list → pixels), scrolling (a wheel
//! notch, a page, jumps), memory. The pixels are vello_cpu's (the web
//! host's renderer and the desktop's without a GPU), so key → pixels here
//! is the slowest path the hosts have.
//!
//!     cargo run --release -p rapidr-ui-render --example codeeditor_bench [-- --quick | --lines N | --scale S]
//!
//! Prints a table and exits 1 when a target is missed (desktop's targets:
//! open ≤ 300 ms, typing p50 ≤ 8 ms and p99 ≤ 16 ms, every scrolled frame
//! ≤ 16 ms, memory ≤ 3 × the file + 50 MB).

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::{Duration, Instant};

use rapidr_ui_kernel::{FormUi, MemClipboard, MemStore, Mods, TextSystem};
use rapidr_ui_render::cpu::CpuRenderer;
use rapidr_value::{v_int, v_str};

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

/// `lines` lines of RapidQ BASIC (~50 bytes a line), CR LF.
fn basic_source(lines: usize) -> String {
    let mut s = String::with_capacity(lines * 52);
    let mut rng = Rng(0x9E37_79B9_7F4A_7C15);
    let (mut n, mut sub) = (0, 0);
    while n < lines {
        sub += 1;
        s.push_str(&format!("SUB Handler{sub}(Sender AS QBUTTON, Value AS INTEGER)\r\n"));
        s.push_str("    DIM total AS DOUBLE, name$ AS STRING ' locals\r\n");
        n += 2;
        for i in 0..6 + rng.below(30) {
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

fn pct(v: &mut [Duration], p: f64) -> Duration {
    v.sort();
    v[((v.len() as f64 - 1.0) * p).round() as usize]
}

fn ms(d: Duration) -> String {
    format!("{:.2} ms", d.as_secs_f64() * 1000.0)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let arg = |name: &str| args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned();
    let quick = args.iter().any(|a| a == "--quick");
    let lines: usize = arg("--lines").and_then(|v| v.parse().ok()).unwrap_or(if quick { 20_000 } else { 200_000 });
    let scale: f64 = arg("--scale").and_then(|v| v.parse().ok()).unwrap_or(2.0);
    let (w, h) = (900i64, 600i64);
    let src = basic_source(lines);
    let file = std::env::temp_dir().join(format!("rapidr-codeeditor-bench-{}.bas", std::process::id()));
    std::fs::write(&file, &src).expect("temp file");
    let file_bytes = src.len();
    drop(src);
    let mut failed = Vec::new();
    let mut row = |what: &str, value: String, target: Option<(&str, bool)>| {
        let t = match target {
            Some((t, ok)) => {
                if !ok {
                    failed.push(what.to_string());
                }
                format!("{t:>12}  {}", if ok { "ok" } else { "MISSED" })
            }
            None => String::new(),
        };
        println!("{what:<52} {value:>14}  {t}");
    };
    println!("RCODEEDITOR view benchmark: {lines} lines, {:.1} MB of RapidQ BASIC (CR LF), a {w}×{h} window at {scale}×\n", file_bytes as f64 / 1e6);
    let base = LIVE.load(Ordering::Relaxed);
    let mut ts = TextSystem::new();
    // (the fonts and the built-in languages load once per process: outside)
    let _ = rapidr_value::objects::codeedit::basic();
    let _ = ts.measure("0", &rapidr_value::objects::font::Font::default());

    // ---- open: LoadFromFile, the first frame, its pixels ----
    let t = Instant::now();
    let mut s = MemStore::new();
    s.add("bf", "RFORM", None).set("bf", "clientwidth", v_int(w)).set("bf", "clientheight", v_int(h));
    s.add("be", "RCODEEDITOR", Some("bf")).set("be", "left", v_int(0)).set("be", "top", v_int(0)).set("be", "width", v_int(w)).set("be", "height", v_int(h));
    s.call("be", "loadfromfile", &[v_str(&file.to_string_lossy())]);
    let mut f = FormUi::build(&s, "bf", false);
    f.blinks = false;
    let list = f.paint(&s, &mut ts, scale);
    let open_list = t.elapsed();
    let (dw, dh) = rapidr_ui_render::canvas::device_size(&list);
    let mut r = CpuRenderer::new(dw, dh);
    r.render(dw, dh, &list, &mut ts, &f);
    let open = t.elapsed();
    let _ = std::fs::remove_file(&file);
    row("open: load + first frame (display list)", ms(open_list), None);
    row("open: load + first frame (pixels)", ms(open), Some(("≤ 300 ms", open <= Duration::from_millis(300))));

    // ---- typing in the middle ----
    f.focus_id(&s, "be");
    s.call("be", "gotolinecolumn", &[v_int(lines as i64 / 2), v_int(5)]);
    let list = f.paint(&s, &mut ts, scale);
    r.render(dw, dh, &list, &mut ts, &f);
    let mut clip = MemClipboard::default();
    let keys: Vec<(i64, &str)> = "x = total + 1".chars().map(|c| (if c == ' ' { 32 } else { c.to_ascii_uppercase() as i64 }, "")).collect();
    let text: Vec<String> = "x = total + 1".chars().map(|c| c.to_string()).collect();
    let (mut to_list, mut to_px) = (Vec::new(), Vec::new());
    for round in 0..80 {
        for (k, (vk, _)) in keys.iter().enumerate() {
            let t = Instant::now();
            f.key_down(&s, &mut ts, *vk, &text[k], Mods::NONE, &mut clip);
            let list = f.paint(&s, &mut ts, scale);
            to_list.push(t.elapsed());
            r.render(dw, dh, &list, &mut ts, &f);
            to_px.push(t.elapsed());
        }
        // (Enter, with its auto-indent; a Backspace now and then)
        let vk = if round % 3 == 0 { 8 } else { 13 };
        let t = Instant::now();
        f.key_down(&s, &mut ts, vk, if vk == 13 { "\r" } else { "" }, Mods::NONE, &mut clip);
        let list = f.paint(&s, &mut ts, scale);
        to_list.push(t.elapsed());
        r.render(dw, dh, &list, &mut ts, &f);
        to_px.push(t.elapsed());
        let _ = f.take_events();
    }
    let n = to_px.len();
    let (l50, l99) = (pct(&mut to_list, 0.5), pct(&mut to_list, 0.99));
    let (p50, p99) = (pct(&mut to_px, 0.5), pct(&mut to_px, 0.99));
    row(&format!("typing ({n} keys): key → display list p50 / p99"), format!("{} / {}", ms(l50), ms(l99)), None);
    row("typing: key → pixels p50", ms(p50), Some(("≤ 8 ms", p50 <= Duration::from_millis(8))));
    row("typing: key → pixels p99", ms(p99), Some(("≤ 16 ms", p99 <= Duration::from_millis(16))));

    // ---- scrolling: a wheel notch, a page, jumps anywhere ----
    let center = (w as f64 / 2.0, h as f64 / 2.0);
    let mut rng = Rng(42);
    for (label, count, kind) in [("a wheel notch", 300, 0), ("a page", 200, 1), ("jumps anywhere", 100, 2)] {
        let mut v = Vec::with_capacity(count);
        for _ in 0..count {
            let t = Instant::now();
            match kind {
                0 => f.mouse_wheel(&s, &mut ts, center, (0.0, 1.0), Mods::NONE),
                1 => f.key_down(&s, &mut ts, 34, "", Mods::NONE, &mut clip),
                _ => {
                    let line = rng.below(lines) as i64 + 1;
                    s.call("be", "gotolinecolumn", &[v_int(line), v_int(1)]);
                }
            }
            let list = f.paint(&s, &mut ts, scale);
            r.render(dw, dh, &list, &mut ts, &f);
            v.push(t.elapsed());
        }
        let worst = *v.iter().max().unwrap_or(&Duration::ZERO);
        let p50 = pct(&mut v, 0.5);
        row(&format!("scroll, {label} ({count} frames): p50 / worst"), format!("{} / {}", ms(p50), ms(worst)), Some(("≤ 16 ms", worst <= Duration::from_millis(16))));
    }

    // ---- memory ----
    let live = LIVE.load(Ordering::Relaxed).saturating_sub(base);
    let budget = 3 * file_bytes + (50 << 20);
    row("memory: everything live (text, view, caches)", format!("{:.1} MB", live as f64 / 1e6), Some(("≤ 3×+50 MB", live <= budget)));
    row("memory: peak", format!("{:.1} MB", PEAK.load(Ordering::Relaxed).saturating_sub(base) as f64 / 1e6), None);
    if failed.is_empty() {
        println!("\nall targets met");
    } else {
        println!("\nmissed: {}", failed.join("; "));
        std::process::exit(1);
    }
}
