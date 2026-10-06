//! Chart colours: the CSS / SVG colour names, `#RGB` / `#RRGGBB`, RapidQ's
//! `&HBBGGRR` numbers — and the palette series take when they name none.

/// The series palette (Tableau 10's, 0xRRGGBB): distinct, readable on
/// white and on a dark background, kind to the colour-blind.
pub const PALETTE: [u32; 10] = [0x4E79A7, 0xF28E2B, 0xE15759, 0x76B7B2, 0x59A14F, 0xEDC948, 0xB07AA1, 0xFF9DA7, 0x9C755F, 0xBAB0AC];

/// Palette colour `i`.
pub fn auto(i: usize) -> u32 {
    PALETTE[i % PALETTE.len()]
}

/// The CSS colour names (CSS Color Module Level 4), 0xRRGGBB.
const NAMES: &[(&str, u32)] = &[
    ("aliceblue", 0xF0F8FF), ("antiquewhite", 0xFAEBD7), ("aqua", 0x00FFFF), ("aquamarine", 0x7FFFD4), ("azure", 0xF0FFFF),
    ("beige", 0xF5F5DC), ("bisque", 0xFFE4C4), ("black", 0x000000), ("blanchedalmond", 0xFFEBCD), ("blue", 0x0000FF),
    ("blueviolet", 0x8A2BE2), ("brown", 0xA52A2A), ("burlywood", 0xDEB887), ("cadetblue", 0x5F9EA0), ("chartreuse", 0x7FFF00),
    ("chocolate", 0xD2691E), ("coral", 0xFF7F50), ("cornflowerblue", 0x6495ED), ("cornsilk", 0xFFF8DC), ("crimson", 0xDC143C),
    ("cyan", 0x00FFFF), ("darkblue", 0x00008B), ("darkcyan", 0x008B8B), ("darkgoldenrod", 0xB8860B), ("darkgray", 0xA9A9A9),
    ("darkgreen", 0x006400), ("darkgrey", 0xA9A9A9), ("darkkhaki", 0xBDB76B), ("darkmagenta", 0x8B008B), ("darkolivegreen", 0x556B2F),
    ("darkorange", 0xFF8C00), ("darkorchid", 0x9932CC), ("darkred", 0x8B0000), ("darksalmon", 0xE9967A), ("darkseagreen", 0x8FBC8F),
    ("darkslateblue", 0x483D8B), ("darkslategray", 0x2F4F4F), ("darkslategrey", 0x2F4F4F), ("darkturquoise", 0x00CED1), ("darkviolet", 0x9400D3),
    ("deeppink", 0xFF1493), ("deepskyblue", 0x00BFFF), ("dimgray", 0x696969), ("dimgrey", 0x696969), ("dodgerblue", 0x1E90FF),
    ("firebrick", 0xB22222), ("floralwhite", 0xFFFAF0), ("forestgreen", 0x228B22), ("fuchsia", 0xFF00FF), ("gainsboro", 0xDCDCDC),
    ("ghostwhite", 0xF8F8FF), ("gold", 0xFFD700), ("goldenrod", 0xDAA520), ("gray", 0x808080), ("green", 0x008000),
    ("greenyellow", 0xADFF2F), ("grey", 0x808080), ("honeydew", 0xF0FFF0), ("hotpink", 0xFF69B4), ("indianred", 0xCD5C5C),
    ("indigo", 0x4B0082), ("ivory", 0xFFFFF0), ("khaki", 0xF0E68C), ("lavender", 0xE6E6FA), ("lavenderblush", 0xFFF0F5),
    ("lawngreen", 0x7CFC00), ("lemonchiffon", 0xFFFACD), ("lightblue", 0xADD8E6), ("lightcoral", 0xF08080), ("lightcyan", 0xE0FFFF),
    ("lightgoldenrodyellow", 0xFAFAD2), ("lightgray", 0xD3D3D3), ("lightgreen", 0x90EE90), ("lightgrey", 0xD3D3D3), ("lightpink", 0xFFB6C1),
    ("lightsalmon", 0xFFA07A), ("lightseagreen", 0x20B2AA), ("lightskyblue", 0x87CEFA), ("lightslategray", 0x778899), ("lightslategrey", 0x778899),
    ("lightsteelblue", 0xB0C4DE), ("lightyellow", 0xFFFFE0), ("lime", 0x00FF00), ("limegreen", 0x32CD32), ("linen", 0xFAF0E6),
    ("magenta", 0xFF00FF), ("maroon", 0x800000), ("mediumaquamarine", 0x66CDAA), ("mediumblue", 0x0000CD), ("mediumorchid", 0xBA55D3),
    ("mediumpurple", 0x9370DB), ("mediumseagreen", 0x3CB371), ("mediumslateblue", 0x7B68EE), ("mediumspringgreen", 0x00FA9A), ("mediumturquoise", 0x48D1CC),
    ("mediumvioletred", 0xC71585), ("midnightblue", 0x191970), ("mintcream", 0xF5FFFA), ("mistyrose", 0xFFE4E1), ("moccasin", 0xFFE4B5),
    ("navajowhite", 0xFFDEAD), ("navy", 0x000080), ("oldlace", 0xFDF5E6), ("olive", 0x808000), ("olivedrab", 0x6B8E23),
    ("orange", 0xFFA500), ("orangered", 0xFF4500), ("orchid", 0xDA70D6), ("palegoldenrod", 0xEEE8AA), ("palegreen", 0x98FB98),
    ("paleturquoise", 0xAFEEEE), ("palevioletred", 0xDB7093), ("papayawhip", 0xFFEFD5), ("peachpuff", 0xFFDAB9), ("peru", 0xCD853F),
    ("pink", 0xFFC0CB), ("plum", 0xDDA0DD), ("powderblue", 0xB0E0E6), ("purple", 0x800080), ("rebeccapurple", 0x663399),
    ("red", 0xFF0000), ("rosybrown", 0xBC8F8F), ("royalblue", 0x4169E1), ("saddlebrown", 0x8B4513), ("salmon", 0xFA8072),
    ("sandybrown", 0xF4A460), ("seagreen", 0x2E8B57), ("seashell", 0xFFF5EE), ("sienna", 0xA0522D), ("silver", 0xC0C0C0),
    ("skyblue", 0x87CEEB), ("slateblue", 0x6A5ACD), ("slategray", 0x708090), ("slategrey", 0x708090), ("snow", 0xFFFAFA),
    ("springgreen", 0x00FF7F), ("steelblue", 0x4682B4), ("tan", 0xD2B48C), ("teal", 0x008080), ("thistle", 0xD8BFD8),
    ("tomato", 0xFF6347), ("turquoise", 0x40E0D0), ("violet", 0xEE82EE), ("wheat", 0xF5DEB3), ("white", 0xFFFFFF),
    ("whitesmoke", 0xF5F5F5), ("yellow", 0xFFFF00), ("yellowgreen", 0x9ACD32),
];

/// A colour as a program names it — a CSS name (`"steelblue"`), `#RGB`,
/// `#RRGGBB`, `C0` … `C9` (the palette's, as Matplotlib's), or RapidQ's
/// `&HBBGGRR` as a number — as 0xRRGGBB; `None` for anything else.
pub fn parse(text: &str) -> Option<u32> {
    let s = text.trim().to_ascii_lowercase();
    if s.is_empty() {
        return None;
    }
    if let Some(hex) = s.strip_prefix('#') {
        let v = u32::from_str_radix(hex, 16).ok()?;
        return match hex.len() {
            6 => Some(v),
            3 => {
                let (r, g, b) = ((v >> 8) & 0xF, (v >> 4) & 0xF, v & 0xF);
                Some((r * 17) << 16 | (g * 17) << 8 | (b * 17))
            }
            _ => None,
        };
    }
    if let Some(i) = s.strip_prefix('c').and_then(|d| d.parse::<usize>().ok()) {
        return (i < PALETTE.len()).then(|| PALETTE[i]);
    }
    if let Some(&(_, c)) = NAMES.iter().find(|(n, _)| *n == s) {
        return Some(c);
    }
    // (RapidQ's colours are numbers, &HBBGGRR: RGB(255, 0, 0) is red)
    let n = s.strip_prefix("&h").map(|h| i64::from_str_radix(h, 16).ok()).unwrap_or_else(|| s.parse::<f64>().ok().map(|f| f as i64))?;
    (0..=0xFF_FFFF).contains(&n).then(|| bgr_to_rgb(n as u32))
}

/// RapidQ's &HBBGGRR as 0xRRGGBB.
pub fn bgr_to_rgb(c: u32) -> u32 {
    (c & 0xFF) << 16 | (c & 0xFF00) | (c >> 16) & 0xFF
}

/// `a` mixed with `b`: `t` = 0 is `a`, 1 is `b`.
pub fn mix(a: u32, b: u32, t: f64) -> u32 {
    let ch = |s: u32| {
        let (x, y) = (f64::from((a >> s) & 0xFF), f64::from((b >> s) & 0xFF));
        ((x + (y - x) * t).round().clamp(0.0, 255.0) as u32) << s
    };
    ch(16) | ch(8) | ch(0)
}

/// How light a colour looks (0 … 1, WCAG's relative luminance).
pub fn luminance(c: u32) -> f64 {
    let lin = |s: u32| {
        let v = f64::from((c >> s) & 0xFF) / 255.0;
        if v <= 0.039_28 {
            v / 12.92
        } else {
            ((v + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * lin(16) + 0.7152 * lin(8) + 0.0722 * lin(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_hex_numbers() {
        assert_eq!(parse("SteelBlue"), Some(0x4682B4));
        assert_eq!(parse("royalblue"), Some(0x4169E1));
        assert_eq!(parse("#f00"), Some(0xFF0000));
        assert_eq!(parse("#4682b4"), Some(0x4682B4));
        assert_eq!(parse("C1"), Some(PALETTE[1]));
        // RGB(255, 0, 0) = 255 in RapidQ: red
        assert_eq!(parse("255"), Some(0xFF0000));
        assert_eq!(parse("&HFF0000"), Some(0x0000FF));
        assert_eq!(parse("nonsense"), None);
        assert_eq!(mix(0x000000, 0xFFFFFF, 0.5), 0x808080);
    }
}
