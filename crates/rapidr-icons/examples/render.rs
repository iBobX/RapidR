//! Renders icons to PNG through the crate's own path (the review sheets,
//! design/icons/tools/sheets.py, and spot checks):
//!
//!     cargo run -p rapidr-icons --example render -- REQUEST
//!
//! REQUEST is a text file: line 1 the scale (1, 1.5, 2 …), line 2 the
//! logical sizes, line 3 the themes (space-separated), line 4 the output
//! directory, then one icon id per line ("*" for all). Writes
//! OUT/<category>__<name>_<size>_<theme>.png.

use rapidr_icons::{all, get, render, Style};

fn main() {
    let path = std::env::args().nth(1).expect("usage: render REQUEST");
    let text = std::fs::read_to_string(&path).expect("read the request");
    let mut lines = text.lines();
    let scale: f32 = lines.next().unwrap().trim().parse().unwrap();
    let sizes: Vec<u32> = lines.next().unwrap().split_whitespace().map(|s| s.parse().unwrap()).collect();
    let themes: Vec<String> = lines.next().unwrap().split_whitespace().map(String::from).collect();
    let out = lines.next().unwrap().trim().to_string();
    let ids: Vec<&str> = lines.map(str::trim).filter(|l| !l.is_empty()).collect();
    let icons: Vec<_> = if ids == ["*"] { all().iter().collect() } else { ids.iter().map(|i| get(i).unwrap_or_else(|| panic!("no icon {i}"))).collect() };
    std::fs::create_dir_all(&out).unwrap();
    for theme in &themes {
        let style = Style::new(theme);
        for icon in &icons {
            for &s in &sizes {
                let px = render(icon, s, scale, &style).unwrap();
                let file = format!("{out}/{}_{s}_{theme}.png", icon.id.replace('/', "__"));
                write_png(&file, &px);
            }
        }
    }
}

fn write_png(file: &str, px: &rapidr_icons::Rgba) {
    let mut pm = resvg::tiny_skia::Pixmap::new(px.width, px.height).unwrap();
    for (d, s) in pm.pixels_mut().iter_mut().zip(px.data.chunks(4)) {
        *d = resvg::tiny_skia::ColorU8::from_rgba(s[0], s[1], s[2], s[3]).premultiply();
    }
    pm.save_png(file).unwrap();
}
