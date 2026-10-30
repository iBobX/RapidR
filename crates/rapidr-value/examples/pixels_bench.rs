//! How fast a frame of pixels is, the engine alone (tools/bench/README.md,
//! "Drawing many pixels fast"): one 256 x 192 screen, the case of an
//! emulator drawing its video memory, put into a bitmap and drawn twice as
//! large on a canvas — at the screen scales 1 (a plain screen), 2 (Retina) and
//! 3.
//!
//!   cargo run --release -p rapidr-value --example pixels_bench [-- --runs N] [--check]
//!
//! The times are the median of the runs, in microseconds, and include
//! everything the call does: reading the program's array, the bitmap's own
//! pixels and the finer ones a high-DPI screen shows. `Pset` is the
//! 49,152 calls the other ways replace, here without the interpreter's
//! cost for each (the program's loop is 1 to 3 µs a call on top).
//!
//! `--check` exits 1 when a frame costs 2 ms or more on a plain screen (the
//! budget of the pixel-block methods: well under a frame of 20 ms).

use rapidr_value::objects::bitmap::{set_display_scale, Bitmap};
use rapidr_value::objects::pixels::{get_pixels, set_pixels, set_pixels8};
use rapidr_value::{BasicArray, Value};
use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

const W: usize = 256;
const H: usize = 192;

fn array(n: usize, f: impl Fn(usize) -> i64) -> Value {
    let mut a = BasicArray::new(vec![(0, n as i64 - 1)], Value::Integer(0)).unwrap();
    for (i, slot) in a.data.iter_mut().enumerate() {
        *slot = Value::Integer(f(i));
    }
    Value::Array(Rc::new(RefCell::new(a)))
}

fn bitmap(w: usize, h: usize) -> Bitmap {
    let mut b = Bitmap::default();
    b.resize(w as i64, h as i64);
    b
}

fn median(runs: usize, mut f: impl FnMut()) -> f64 {
    let mut t: Vec<f64> = (0..runs)
        .map(|_| {
            let start = Instant::now();
            f();
            start.elapsed().as_secs_f64() * 1e6
        })
        .collect();
    t.sort_by(|a, b| a.partial_cmp(b).unwrap());
    t[t.len() / 2]
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let runs = args.iter().position(|a| a == "--runs").and_then(|i| args.get(i + 1)).and_then(|n| n.parse().ok()).unwrap_or(31);
    let check = args.iter().any(|a| a == "--check");
    let n = |v: i64| Value::Integer(v);
    let colours = array(W * H, |i| (i as i64 * 2654435761) & 0xFFFFFF);
    let indexes = array(W * H, |i| ((i + i / W) % 16) as i64);
    let palette = array(16, |i| (i as i64 * 0x111111) & 0xFFFFFF);
    let readback = array(W * H, |_| 0);
    let mut worst_plain = 0.0f64;

    println!("a {W} x {H} frame, microseconds (median of {runs} runs)");
    println!("{:<44}{:>10}{:>10}{:>10}", "", "scale 1", "scale 2", "scale 3");
    let row = |name: &str, f: &dyn Fn(usize) -> f64| {
        let t = [f(1), f(2), f(3)];
        println!("{name:<44}{:>10.0}{:>10.0}{:>10.0}", t[0], t[1], t[2]);
        t
    };

    row("Pset x 49,152 (the calls it replaces)", &|s| {
        set_display_scale(s as f64);
        let mut b = bitmap(W, H);
        let _ = b.display_rgba();
        median(runs.min(5), || {
            for y in 0..H as i64 {
                for x in 0..W as i64 {
                    b.pset(x, y, (x * 7 + y * 3) as u32 & 0xFFFFFF);
                }
            }
        })
    });
    let t = row("SetPixels (colours)", &|s| {
        set_display_scale(s as f64);
        let mut b = bitmap(W, H);
        let _ = b.display_rgba();
        median(runs, || set_pixels(&mut b, &[colours.clone(), n(W as i64), n(H as i64)]).unwrap())
    });
    worst_plain = worst_plain.max(t[0]);
    let t = row("SetPixels8 (palette indexes)", &|s| {
        set_display_scale(s as f64);
        let mut b = bitmap(W, H);
        let _ = b.display_rgba();
        median(runs, || set_pixels8(&mut b, &[indexes.clone(), palette.clone(), n(W as i64), n(H as i64)]).unwrap())
    });
    worst_plain = worst_plain.max(t[0]);
    row("GetPixels", &|s| {
        set_display_scale(s as f64);
        let b = bitmap(W, H);
        median(runs, || get_pixels(&b, &[readback.clone(), n(0), n(0), n(W as i64), n(H as i64)]).unwrap())
    });
    row("Draw 1 to 1 (256 x 192)", &|s| {
        set_display_scale(s as f64);
        let (mut dst, mut src) = (bitmap(W, H), bitmap(W, H));
        src.fill_rect(0, 0, W as i64, H as i64, 0x123456);
        let _ = (dst.display_rgba(), src.display_rgba());
        median(runs, || dst.draw(0, 0, &src))
    });
    let t = row("StretchDraw 2x, nearest (to 512 x 384)", &|s| {
        set_display_scale(s as f64);
        let (mut dst, mut src) = (bitmap(2 * W, 2 * H), bitmap(W, H));
        src.fill_rect(0, 0, W as i64, H as i64, 0x123456);
        let _ = (dst.display_rgba(), src.display_rgba());
        median(runs, || dst.copy_rect((0, 0, 2 * W as i64, 2 * H as i64), &src, (0, 0, W as i64, H as i64)))
    });
    worst_plain = worst_plain.max(t[0]);
    row("StretchDraw 2x, Smooth", &|s| {
        set_display_scale(s as f64);
        let (mut dst, mut src) = (bitmap(2 * W, 2 * H), bitmap(W, H));
        dst.smooth = true;
        src.fill_rect(0, 0, W as i64, H as i64, 0x123456);
        let _ = (dst.display_rgba(), src.display_rgba());
        median(runs, || dst.copy_rect((0, 0, 2 * W as i64, 2 * H as i64), &src, (0, 0, W as i64, H as i64)))
    });
    row("what the screen is given (display_rgba)", &|s| {
        set_display_scale(s as f64);
        let mut b = bitmap(2 * W, 2 * H);
        b.fill_rect(0, 0, 2 * W as i64, 2 * H as i64, 0x123456);
        median(runs, || {
            b.touch();
            let _ = b.display_rgba();
        })
    });
    let t = row("a whole frame: SetPixels8 + StretchDraw 2x", &|s| {
        set_display_scale(s as f64);
        let (mut dst, mut src) = (bitmap(2 * W, 2 * H), bitmap(W, H));
        let _ = (dst.display_rgba(), src.display_rgba());
        median(runs, || {
            set_pixels8(&mut src, &[indexes.clone(), palette.clone(), n(W as i64), n(H as i64)]).unwrap();
            dst.copy_rect((0, 0, 2 * W as i64, 2 * H as i64), &src, (0, 0, W as i64, H as i64));
        })
    });
    worst_plain = worst_plain.max(t[0]);
    row("… and handed to the screen (display_rgba)", &|s| {
        set_display_scale(s as f64);
        let (mut dst, mut src) = (bitmap(2 * W, 2 * H), bitmap(W, H));
        let _ = (dst.display_rgba(), src.display_rgba());
        median(runs, || {
            set_pixels8(&mut src, &[indexes.clone(), palette.clone(), n(W as i64), n(H as i64)]).unwrap();
            dst.copy_rect((0, 0, 2 * W as i64, 2 * H as i64), &src, (0, 0, W as i64, H as i64));
            let _ = dst.display_rgba();
        })
    });
    set_display_scale(1.0);
    if check && worst_plain >= 2000.0 {
        eprintln!("pixels_bench: a frame costs {worst_plain:.0} µs on a plain screen (budget: under 2000)");
        std::process::exit(1);
    }
}
