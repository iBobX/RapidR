//! RDATAFRAME's speed on big tables (I7 / L-FRAME): a CSV of `ROWS` rows
//! (default 1,000,000; five columns: id, name, dept, salary, years) loaded,
//! then filtered, sorted, grouped and joined, each from a fresh copy of the
//! loaded frame; the times beside their targets.
//!
//!   cargo run --release -p rapidr-value --example frame_bench [rows]
//!
//! The web runs the same engine in wasm (tests/web_frame_bench.mjs
//! measures it in a browser).

use std::time::Instant;

use rapidr_value::datascience::{frame, Host};
use rapidr_value::{v_int, v_str};

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

/// The benchmark's table as CSV text.
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

fn main() {
    let rows: usize = std::env::args().nth(1).and_then(|a| a.parse().ok()).unwrap_or(1_000_000);
    let text = csv(rows);
    let depts = "dept,floor\nEngineering,3\nSales,1\nMarketing,2\nSupport,1\nFinance,4\n";
    let h = Quiet;
    println!("RDATAFRAME, {rows} rows ({:.1} MB of CSV)", text.len() as f64 / 1e6);
    let t = Instant::now();
    frame::dataframe_method("base", "loadfromcsv", &[v_str(&text)], &h);
    let load = t.elapsed();
    frame::dataframe_method("depts", "loadfromcsv", &[v_str(depts)], &h);
    let base = frame::frame("base");
    assert_eq!(base.height(), rows);
    let ops: [(&str, &str, Vec<rapidr_value::Value>, f64); 5] = [
        ("filter salary > 75000", "filter", vec![v_str("salary"), v_str(">"), v_int(75_000)], 60.0),
        ("sort by salary, highest first", "sort", vec![v_str("salary"), v_int(0)], 250.0),
        ("groupby dept, mean", "groupby", vec![v_str("dept"), v_str("mean")], 120.0),
        ("left join on dept", "merge", vec![v_str("depts"), v_str("dept"), v_str("left")], 120.0),
        ("describe", "describe", vec![], 150.0),
    ];
    let row = |what: &str, ms: f64, target: f64| println!("  {what:<32} {ms:>9.1} ms   target {target:>6.0} ms   {}", if ms <= target { "ok" } else { "SLOW" });
    row("load (CSV text)", load.as_secs_f64() * 1e3, 300.0);
    for (what, method, args, target) in ops {
        frame::set_frame("w", base.clone());
        let t = Instant::now();
        frame::dataframe_method("w", method, &args, &h);
        row(what, t.elapsed().as_secs_f64() * 1e3, target);
    }
    let t = Instant::now();
    let saved = frame::to_csv(&base);
    row("save (CSV text)", t.elapsed().as_secs_f64() * 1e3, 150.0);
    assert!(saved.len() > rows * 10);
}
