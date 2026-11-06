//! The code editor and RapidR's language service in the background
//! (docs/speed-audit-2026-11.md, "Studio: the edit pause"), with the real
//! service as RapidR Studio installs it:
//!
//! - a burst of changes applied to a program's files one after another —
//!   RAI's changes, then Save All —, the program broken in between: every
//!   editor ends with the colours, problems and lexical colours a freshly
//!   opened one has for the final texts (Robert, 2026-11-02: platform.rr
//!   showed plain after such a burst until it was reopened);
//! - typing never waits for the analysis: a key is handled while the
//!   program is being analysed, and the analysis that was under way for
//!   the older text is never shown for the newer one.

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rapidr_ui_kernel::{FormUi, MemClipboard, MemStore, Mods, TextSystem};
use rapidr_value::objects::{with_code, with_code_mut};
use rapidr_value::{v_int, v_str};

const MAIN: &str = "$APPTYPE CONSOLE\nSUB Helper\n    PRINT \"helping\"\nEND SUB\n$INCLUDE \"platform.rr\"\nGreet \"world\"\n";
const PLATFORM: &str = "SUB Greet(Who AS STRING)\n    Helper\n    PRINT \"Hello, \"; Who\nEND SUB\n";
/// The program mid-change: Helper taken out of main.rr (RAI rewriting it):
/// platform.rr's call of it is an error then.
const MAIN_BROKEN: &str = "$APPTYPE CONSOLE\n$INCLUDE \"platform.rr\"\nGreet \"world\"\n";
const PLATFORM_AFTER: &str = "SUB Greet(Who AS STRING)\n    Helper\n    PRINT \"Hello, \"; Who; \"!\"\nEND SUB\n\nSUB Wave(Times AS INTEGER)\n    DIM i AS INTEGER\n    FOR i = 1 TO Times\n        Greet \"again\"\n    NEXT\nEND SUB\n";
/// Helper back, written again.
const MAIN_AFTER: &str = "$APPTYPE CONSOLE\nSUB Helper\n    PRINT \"helping, again\"\nEND SUB\n$INCLUDE \"platform.rr\"\nGreet \"world\"\nWave 2\n";

fn scratch(name: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rapidr-edit-pause-{}-{name}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// A form with a code editor per file, each loaded from its file.
fn editors(form: &str, files: &[(&str, &Path)]) -> (MemStore, FormUi) {
    let mut s = MemStore::new();
    s.add(form, "RFORM", None).set(form, "clientwidth", v_int(900)).set(form, "clientheight", v_int(600));
    for (i, (id, path)) in files.iter().enumerate() {
        s.add(id, "RCODEEDITOR", Some(form)).set(id, "left", v_int(0)).set(id, "top", v_int(i as i64 * 300)).set(id, "width", v_int(900)).set(id, "height", v_int(300));
        s.call(id, "loadfromfile", &[v_str(&path.to_string_lossy())]);
    }
    let f = FormUi::build(&s, form, false);
    (s, f)
}

/// The window loop's turns (the kernel's ticks, as the host makes them)
/// until every editor shows its analysis for the texts as they are, and a
/// moment more.
fn settle(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, ids: &[&str]) {
    settle_for(f, s, ts, ids, Duration::from_secs(60), true);
}

/// [`settle`], given up after `most` (`must`: a failure then).
fn settle_for(f: &mut FormUi, s: &MemStore, ts: &mut TextSystem, ids: &[&str], most: Duration, must: bool) {
    let until = Instant::now() + most;
    let mut quiet_since: Option<Instant> = None;
    loop {
        f.tick(s, ts, rapidr_ui_kernel::tick::now());
        let current = ids.iter().all(|id| {
            with_code(id, |c| c.service_gen.is_some_and(|g| rapidr_editor::service::with(|s| s.generation()) == Some(g))).unwrap_or(false)
        });
        if current {
            // (and stays so: nothing else is due)
            let since = *quiet_since.get_or_insert_with(Instant::now);
            if since.elapsed() > Duration::from_millis(500) {
                return;
            }
        } else {
            quiet_since = None;
        }
        if Instant::now() >= until {
            assert!(!must, "the editors never settled");
            return;
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// What an editor shows of the language: its semantic colours, its
/// problems, its lines' lexical colours.
fn shown(id: &str) -> (Vec<(usize, usize, &'static str)>, Vec<(usize, usize, String)>, Vec<String>) {
    with_code_mut(id, |c| {
        let semantic = c.semantic.iter().map(|t| (t.start, t.end, t.kind)).collect();
        let problems = c.diagnostics.iter().map(|d| (d.start, d.end, d.message.clone())).collect();
        let lines = c.doc.text().split('\n').count();
        let lexical = (0..lines).map(|l| format!("{:?}", c.doc.tokens(l))).collect();
        (semantic, problems, lexical)
    })
    .unwrap()
}

#[test]
fn a_burst_of_applied_changes_never_leaves_an_editor_without_its_colours() {
    rapidr_langsvc::editor::install();
    let dir = scratch("burst");
    let (main, platform) = (dir.join("main.rr"), dir.join("platform.rr"));
    std::fs::write(&main, MAIN).unwrap();
    std::fs::write(&platform, PLATFORM).unwrap();
    rapidr_editor::service::with(|s| s.set_project(&main.to_string_lossy(), "desktop"));
    let mut ts = TextSystem::new();
    let (mut s, mut f) = editors("bf", &[("bmain", &main), ("bplat", &platform)]);
    settle(&mut f, &s, &mut ts, &["bmain", "bplat"]);
    let (colours, problems, _) = shown("bplat");
    assert!(colours.iter().any(|t| t.2 == "function"), "platform.rr's SUBs coloured: {colours:?}");
    assert!(problems.is_empty(), "{problems:?}");

    // RAI's changes applied one file after another (each a text set from
    // outside, analysed at once): main.rr without Helper — platform.rr's
    // call of it an error then —, platform.rr, a moment for both to be
    // analysed so …
    s.set("bmain", "text", v_str(MAIN_BROKEN));
    f.tick(&s, &mut ts, rapidr_ui_kernel::tick::now());
    s.set("bplat", "text", v_str(PLATFORM_AFTER));
    settle(&mut f, &s, &mut ts, &["bmain", "bplat"]);
    assert!(shown("bplat").1.iter().any(|p| p.2.contains("Helper")), "mid-burst, platform.rr's call of Helper is an error: {:?}", shown("bplat").1);
    // … then main.rr with Helper written again — platform.rr itself not
    // touched again —, then Save All.
    s.set("bmain", "text", v_str(MAIN_AFTER));
    f.tick(&s, &mut ts, rapidr_ui_kernel::tick::now());
    s.call("bplat", "savetofile", &[v_str(&platform.to_string_lossy())]);
    s.call("bmain", "savetofile", &[v_str(&main.to_string_lossy())]);
    settle_for(&mut f, &s, &mut ts, &["bmain", "bplat"], Duration::from_secs(5), false);

    // What a freshly opened editor shows for the same files.
    let (fresh_s, mut fresh_f) = editors("ff", &[("fmain", &main), ("fplat", &platform)]);
    settle(&mut fresh_f, &fresh_s, &mut ts, &["fmain", "fplat"]);
    let after = shown("bplat");
    assert!(after.1.is_empty(), "no problem left from the broken moment: {:?}", after.1);
    assert!(after.0.iter().any(|t| t.2 == "function"), "platform.rr's SUBs coloured after the burst: {:?}", after.0);
    assert_eq!(after, shown("fplat"), "platform.rr after the burst shows what it shows when opened");
    assert_eq!(shown("bmain"), shown("fmain"), "main.rr too");
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn typing_never_waits_for_the_analysis() {
    rapidr_langsvc::editor::install();
    let dir = scratch("typing");
    // (a program big enough that its analysis takes a while)
    let mut big = String::from("$APPTYPE CONSOLE\nDIM Total AS INTEGER\n");
    for i in 0..1500 {
        big.push_str(&format!("SUB Step{i}(n AS INTEGER)\n    DIM k AS INTEGER\n    FOR k = 1 TO n\n        Total = Total + k * {i}\n    NEXT\nEND SUB\n"));
    }
    big.push_str("Step1 3\n");
    let main = dir.join("main.rr");
    std::fs::write(&main, &big).unwrap();
    let mut ts = TextSystem::new();
    let (mut s, mut f) = editors("tf", &[("tmain", &main)]);
    settle(&mut f, &s, &mut ts, &["tmain"]);
    f.focus_id(&s, "tmain");
    s.call("tmain", "gotolinecolumn", &[v_int(3), v_int(1)]);
    // Typing, a pause (the analysis starts), and typing again while it runs.
    let mut clip = MemClipboard::default();
    let mut worst = Duration::ZERO;
    let t = Instant::now();
    f.key_down(&s, &mut ts, 'X' as i64, "x", Mods::NONE, &mut clip);
    f.tick(&s, &mut ts, rapidr_ui_kernel::tick::now());
    worst = worst.max(t.elapsed());
    // (the pause: the diagnostics' timer comes, the analysis is asked for)
    let pause = Instant::now() + rapidr_ui_kernel::components::codeeditor::lang::DIAGNOSE_AFTER + Duration::from_millis(20);
    while Instant::now() < pause {
        let t = Instant::now();
        f.tick(&s, &mut ts, rapidr_ui_kernel::tick::now());
        worst = worst.max(t.elapsed());
        std::thread::sleep(Duration::from_millis(2));
    }
    // (typed while it runs: the work for "x" must never be shown for "")
    let t = Instant::now();
    f.key_down(&s, &mut ts, 8, "", Mods::NONE, &mut clip);
    f.tick(&s, &mut ts, rapidr_ui_kernel::tick::now());
    worst = worst.max(t.elapsed());
    for _ in 0..20 {
        let t = Instant::now();
        f.tick(&s, &mut ts, rapidr_ui_kernel::tick::now());
        worst = worst.max(t.elapsed());
        std::thread::sleep(Duration::from_millis(2));
    }
    settle(&mut f, &s, &mut ts, &["tmain"]);
    // the problems shown are the final text's: none (the "x" line had one)
    assert!(shown("tmain").1.is_empty(), "the stray x's problem is gone with it: {:?}", shown("tmain").1);
    // (debug builds are slow; the release build's budget is the benchmark's)
    if !cfg!(debug_assertions) {
        assert!(worst < Duration::from_millis(16), "a key or a turn took {worst:?}");
    }
    let _ = std::fs::remove_dir_all(&dir);
}
