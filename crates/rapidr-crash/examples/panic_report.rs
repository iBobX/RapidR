//! A process that panics with crash reports on, to see the report a
//! release build writes (its backtrace keeps function names: release
//! builds strip debug info, not the symbols' names):
//!
//!   RAPIDR_CRASH_DIR=/tmp/crashes cargo run --release -p rapidr-crash --example panic_report
//!   cat /tmp/crashes/*.txt

#[inline(never)]
fn read_settings(n: usize) -> u32 {
    let v: Vec<u32> = (0..3).collect();
    // (an index past the end: a bug, as RapidR's own code could have)
    v[std::hint::black_box(n)]
}

fn main() {
    rapidr_crash::install(rapidr_crash::App::Tool);
    rapidr_crash::note("a log line of RapidR's own");
    rapidr_crash::call("tool: read_file");
    println!("{}", read_settings(7));
}
