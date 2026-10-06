//! Prints an SVG's coverage at its size (dev: hinting checks).
//!
//!     cargo run -p rapidr-icons --example probe -- FILE.svg [SIZE]

fn main() {
    let path = std::env::args().nth(1).expect("usage: probe FILE.svg [SIZE]");
    let svg = std::fs::read_to_string(path).unwrap();
    let size: u32 = std::env::args().nth(2).and_then(|s| s.parse().ok()).unwrap_or(16);
    let px = rapidr_icons::render_svg(&svg, size, 1.0).unwrap();
    for y in 0..size {
        let row: String = (0..size)
            .map(|x| {
                let a = px.data[((y * size + x) * 4 + 3) as usize];
                if a >= 250 { '#' } else if a > 8 { '+' } else { '.' }
            })
            .collect();
        println!("{row}");
    }
}
