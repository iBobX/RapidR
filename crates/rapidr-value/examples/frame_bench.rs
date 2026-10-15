//! RDATAFRAME's speed on big tables (docs/data-science-plan.md §9): a CSV
//! of `ROWS` rows (default 1,000,000; five columns: id, name, dept, salary,
//! years) loaded, then filtered, sorted, grouped, joined, described and
//! saved — each from a fresh copy of the loaded frame — and the pipeline
//! load → filter → sort → group-by on one frame; each time the median of
//! `--runs` runs, beside its phase-1 budget.
//!
//!   cargo run --release -p rapidr-value --example frame_bench [rows] [--runs N] [--threads N]
//!   cargo run --release -p rapidr-value --example frame_bench 1000000 --write-csv tests/results/bench
//!
//! `--threads 1` runs every kernel on one thread, as the web does (no
//! threads in a page yet).
//!
//! Budgets are for 1,000,000 rows on the reference Mac (M4 Max) and scale
//! with the rows; the pipeline must take under 1 s for a million rows (5 s
//! for ten million). The exit code is 1 when a time misses its budget by
//! more than 25 % (the noise margin), or the pipeline its limit.
//!
//! `--write-csv DIR` writes the same table (DIR/people_ROWS.csv) and the
//! join's second table (DIR/depts.csv) for tools/bench/'s pandas and Polars
//! scripts and bench_rapidr.rr, then stops. The digest line at the end is
//! the results' fingerprint: two engines that answer alike print the same.

use std::time::Instant;

use rapidr_value::datascience::{frame, Host};
use rapidr_value::{v_int, v_str, Value};

struct Quiet;

impl Host for Quiet {
    fn print(&self, _: &str) {}
    fn warn(&self, text: &str) {
        eprintln!("{text}");
    }
    fn to_grid(&self, _: &str, _: &[Vec<String>]) {}
    fn save_plot(&self, _: &str, _: &str, _: f64) {}
    fn show_plot(&self, _: &str) {}
}

/// The benchmark's table as CSV text (deterministic: xorshift64).
pub fn csv(rows: usize) -> String {
    const DEPTS: [&str; 5] = ["Engineering", "Sales", "Marketing", "Support", "Finance"];
    let mut s = String::with_capacity(rows * 40);
    s.push_str("id,name,dept,salary,years\n");
    let mut x: u64 = 0x2545_F491_4F6C_DD1D;
    for i in 0..rows {
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        let salary = 30_000 + (x % 90_000);
        let years = (x >> 20) % 40;
        s.push_str(&format!("{i},person{},{},{salary},{years}\n", x % 100_000, DEPTS[(x >> 8) as usize % 5]));
    }
    s
}

/// The join's other table: each department's floor.
const DEPTS_CSV: &str = "dept,floor\nEngineering,3\nSales,1\nMarketing,2\nSupport,1\nFinance,4\n";

/// FNV-1a of `text` (the results' fingerprint).
fn fnv(hash: &mut u64, text: &str) {
    for b in text.bytes() {
        *hash ^= b as u64;
        *hash = hash.wrapping_mul(0x0100_0000_01b3);
    }
}

fn median(mut v: Vec<f64>) -> f64 {
    v.sort_by(|a, b| a.partial_cmp(b).unwrap());
    v[v.len() / 2]
}

fn main() {
    let mut rows = 1_000_000usize;
    let mut runs = 3usize;
    let mut write_dir = None::<String>;
    let mut threads = std::thread::available_parallelism().map_or(1, |n| n.get()).min(16);
    let mut args = std::env::args().skip(1);
    while let Some(a) = args.next() {
        match a.as_str() {
            "--runs" => runs = args.next().and_then(|n| n.parse().ok()).unwrap_or(3).max(1),
            "--threads" => threads = args.next().and_then(|n| n.parse().ok()).unwrap_or(threads).max(1),
            "--write-csv" => write_dir = args.next(),
            n => rows = n.replace('_', "").parse().unwrap_or(rows),
        }
    }
    // (the engine's own sizes worth splitting; at most `threads` workers)
    frame::set_thresholds(50_000, 4096, 2 << 20, threads);
    let text = csv(rows);
    if let Some(dir) = write_dir {
        std::fs::create_dir_all(&dir).expect("can't make the directory");
        let people = format!("{dir}/people_{rows}.csv");
        std::fs::write(&people, &text).expect("can't write the table");
        std::fs::write(format!("{dir}/depts.csv"), DEPTS_CSV).expect("can't write depts.csv");
        println!("{people} ({:.1} MB) and {dir}/depts.csv written", text.len() as f64 / 1e6);
        return;
    }
    let h = Quiet;
    let cores = std::thread::available_parallelism().map_or(1, |n| n.get());
    println!("RDATAFRAME, {rows} rows ({:.1} MB of CSV), {threads} of {cores} cores, median of {runs} runs", text.len() as f64 / 1e6);
    let scale = rows as f64 / 1e6;
    let mut digest = 0xcbf2_9ce4_8422_2325u64;
    let mut missed = Vec::new();
    let mut machine = Vec::new();
    let mut report = |what: &str, ms: f64, budget: f64| {
        machine.push(format!("#{} {ms:.1}", what.split_whitespace().next().unwrap_or(what).trim_end_matches(':')));
        let budget = budget * scale;
        let verdict = if ms <= budget {
            "ok"
        } else if ms <= budget * 1.25 {
            "ok (within the 25 % margin)"
        } else {
            missed.push(what.to_string());
            "MISSED"
        };
        println!("  {what:<34} {ms:>9.1} ms   budget {budget:>7.0} ms   {verdict}");
    };

    // load (CSV text in memory → a frame)
    let mut times = Vec::new();
    let text_arg = [v_str(&text)];
    for _ in 0..runs {
        frame::remove_frame("base");
        let t = Instant::now();
        frame::dataframe_method("base", "loadfromcsv", &text_arg, &h);
        times.push(t.elapsed().as_secs_f64() * 1e3);
    }
    report("load (CSV text)", median(times), 120.0);
    frame::dataframe_method("depts", "loadfromcsv", &[v_str(DEPTS_CSV)], &h);
    let base = frame::frame("base");
    assert_eq!(base.height(), rows);

    let ops: [(&str, &str, Vec<Value>, f64); 5] = [
        ("filter salary > 75000", "filter", vec![v_str("salary"), v_str(">"), v_int(75_000)], 25.0),
        ("sort by salary, highest first", "sort", vec![v_str("salary"), v_int(0)], 120.0),
        ("group-by dept, mean", "groupby", vec![v_str("dept"), v_str("mean")], 60.0),
        ("join dept (left)", "merge", vec![v_str("depts"), v_str("dept"), v_str("left")], 60.0),
        ("describe", "describe", vec![], 60.0),
    ];
    for (what, method, args, budget) in ops {
        let mut times = Vec::new();
        for _ in 0..runs {
            frame::set_frame("w", base.clone());
            let t = Instant::now();
            frame::dataframe_method("w", method, &args, &h);
            times.push(t.elapsed().as_secs_f64() * 1e3);
        }
        report(what, median(times), budget);
        let out = frame::frame("w");
        fnv(&mut digest, &out.to_text());
        fnv(&mut digest, &format!("{}x{}", out.height(), out.width()));
        if out.height() <= 100 {
            fnv(&mut digest, &frame::to_csv(&out));
            fnv(&mut digest, &frame::to_json(&out));
        }
    }

    // save (the frame → CSV text in memory)
    let mut times = Vec::new();
    let mut saved = String::new();
    for _ in 0..runs {
        let t = Instant::now();
        saved = frame::to_csv(&base);
        times.push(t.elapsed().as_secs_f64() * 1e3);
    }
    report("save (CSV text)", median(times), 100.0);
    assert_eq!(saved, text, "the saved CSV differs from the loaded one");

    // the pipeline: load → filter → sort → group-by, one frame
    let mut times = Vec::new();
    for _ in 0..runs {
        frame::remove_frame("p");
        let t = Instant::now();
        frame::dataframe_method("p", "loadfromcsv", &text_arg, &h);
        frame::dataframe_method("p", "filter", &[v_str("salary"), v_str(">"), v_int(75_000)], &h);
        frame::dataframe_method("p", "sort", &[v_str("salary"), v_int(0)], &h);
        frame::dataframe_method("p", "groupby", &[v_str("dept"), v_str("mean")], &h);
        times.push(t.elapsed().as_secs_f64() * 1e3);
    }
    let pipeline = median(times);
    let limit = if rows <= 1_000_000 { 1000.0 } else { 500.0 * scale };
    machine.push(format!("#pipeline {pipeline:.1}"));
    let verdict = if pipeline <= 300.0 * scale.max(1.0) {
        "ok (under the 300 ms-a-million target)"
    } else if pipeline <= limit {
        "ok (under the hard limit)"
    } else {
        missed.push("pipeline".into());
        "MISSED"
    };
    println!("  {:<34} {pipeline:>9.1} ms   limit  {limit:>7.0} ms   {verdict}", "pipeline: load+filter+sort+group-by");
    fnv(&mut digest, &frame::frame("p").to_text());
    println!("{}", frame::frame("p").to_text());
    println!("digest {digest:016x}");
    for m in &machine {
        println!("{m}");
    }
    if !missed.is_empty() {
        println!("missed: {}", missed.join(", "));
        std::process::exit(1);
    }
}
