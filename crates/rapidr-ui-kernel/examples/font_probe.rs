//! Which font draws a character in RapidR's UI kernel on this computer: the
//! evidence for the CJK font check (docs/legal/i18n-zh-font.md). The kernel
//! lays text out in its built-in fonts (Latin, Greek, Cyrillic), then asks
//! the system for a font for what they lack; this prints, for sample text,
//! the family that drew each run and how many characters came out as the
//! empty box (glyph 0) — with no language given, and with `zh-Hans` given.
//!
//! `cargo run -p rapidr-ui-kernel --example font_probe [-- "text"]`

use parley::PositionedLayoutItem;
use rapidr_ui_kernel::text::styles;
use rapidr_ui_kernel::TextSystem;
use rapidr_value::objects::font::Font;

/// The font's family name, from its `name` table (a TrueType collection's
/// `index`-th font): ID 16 (typographic family) else 1, Windows' UTF-16.
fn family_of(data: &[u8], index: u32) -> String {
    fn u16_at(d: &[u8], o: usize) -> Option<usize> {
        Some(u16::from_be_bytes(d.get(o..o + 2)?.try_into().ok()?) as usize)
    }
    fn u32_at(d: &[u8], o: usize) -> Option<usize> {
        Some(u32::from_be_bytes(d.get(o..o + 4)?.try_into().ok()?) as usize)
    }
    let name = (|| {
        let base = if data.get(0..4)? == b"ttcf" { u32_at(data, 12 + 4 * index as usize)? } else { 0 };
        let tables = u16_at(data, base + 4)?;
        let mut name_table = None;
        for t in 0..tables {
            let rec = base + 12 + 16 * t;
            if data.get(rec..rec + 4)? == b"name" {
                name_table = Some(u32_at(data, rec + 8)?);
            }
        }
        let nt = name_table?;
        let (count, strings) = (u16_at(data, nt + 2)?, u16_at(data, nt + 4)?);
        let mut best: Option<(usize, String)> = None;
        for r in 0..count {
            let rec = nt + 6 + 12 * r;
            let (platform, id, len, off) = (u16_at(data, rec)?, u16_at(data, rec + 6)?, u16_at(data, rec + 8)?, u16_at(data, rec + 10)?);
            if platform != 3 || !(id == 1 || id == 16) {
                continue;
            }
            let bytes = data.get(nt + strings + off..nt + strings + off + len)?;
            let units: Vec<u16> = bytes.chunks(2).map(|c| u16::from_be_bytes([c[0], *c.get(1).unwrap_or(&0)])).collect();
            let text = String::from_utf16_lossy(&units);
            if best.as_ref().map_or(true, |(i, _)| id == 16 && *i != 16) {
                best = Some((id, text));
            }
        }
        best.map(|(_, t)| t)
    })();
    name.unwrap_or_else(|| "(unnamed)".to_string())
}

/// What `text` is drawn with: (family, characters, characters with no glyph), per run.
fn runs(ts: &mut TextSystem, text: &str) -> Vec<(String, String, usize)> {
    let font = Font::default();
    let mut builder = ts.layout_cx.ranged_builder(&mut ts.font_cx, text, 1.0, true);
    for p in styles(&font, 0) {
        builder.push_default(p);
    }
    let mut layout = builder.build(text);
    layout.break_all_lines(None);
    let mut out: Vec<(String, String, usize)> = Vec::new();
    for line in layout.lines() {
        for item in line.items() {
            let PositionedLayoutItem::GlyphRun(run) = item else { continue };
            let font = run.run().font();
            let family = format!("{}{}", family_of(font.data.data(), font.index), if font.index > 0 { format!(" #{}", font.index) } else { String::new() });
            for cluster in run.run().clusters() {
                let chars = text.get(cluster.text_range()).unwrap_or("").to_string();
                let missing = usize::from(cluster.glyphs().any(|g| g.id == 0));
                match out.last_mut() {
                    Some((f, s, m)) if *f == family => {
                        s.push_str(&chars);
                        *m += missing;
                    }
                    _ => out.push((family.clone(), chars, missing)),
                }
            }
        }
    }
    out
}

fn main() {
    let samples: Vec<String> = match std::env::args().nth(1) {
        Some(t) => vec![t],
        None => vec![
            "文件 编辑 视图 项目 运行 调试 格式 工具 窗口 帮助".to_string(),
            "你好，世界！保存 另存为... 全部接受(A) 学校锁定".to_string(),
            "直 骨 角 刃 门 户 说 化 海 关".to_string(),
            "Hello 世界 ✓ ▶ ↻ 0123".to_string(),
        ],
    };
    let mut ts = TextSystem::new();
    // (the families the system lists, by the names of the ones that draw Chinese)
    let all = ts.family_names();
    let named: Vec<&String> = all
        .iter()
        .filter(|n| {
            let l = n.to_lowercase();
            ["cjk", "droid", "wenquan", "noto sans sc", "yahei", "pingfang", "heiti", "simsun", "han sans", "songti"].iter().any(|k| l.contains(k))
        })
        .collect();
    println!("== {} font families in all; by name, Chinese-capable: {named:?}", all.len());
    // (the UI's language: English leaves the choice to the system; Chinese
    // puts the system's own Simplified Chinese fonts first)
    for language in ["en", "zh-Hans"] {
        rapidr_value::i18n::set_language(language);
        let mut with_none = 0;
        println!("== UI language: {language}");
        for s in &samples {
            println!("  {s}");
            for (family, chars, missing) in runs(&mut ts, s) {
                with_none += missing;
                println!("      {family:<28} {} char(s), {} with no glyph: {chars}", chars.chars().count(), missing);
            }
        }
        println!("== {language}: {with_none} character(s) with no glyph (drawn as a box)");
    }
}
