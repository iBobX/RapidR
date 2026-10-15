//! Typed values (docs/ai-architecture.md §3.4): plain JSON where natural,
//! a small object where a value needs more than its number — the same on
//! both sides. What a number means comes from the registry (an enum's
//! constants, a set's bits, a colour's BGR).

use rapidr_lang::Property;
use serde_json::{json, Value};

/// A colour: `{"color":"#RRGGBB","bgr":n}` (with `const` when written as
/// one); a system colour (clBtnFace: negative in RapidQ) as
/// `{"const":…,"bgr":n,"system":true}`.
pub fn color(bgr: i64, constant: Option<&str>) -> Value {
    let mut m = serde_json::Map::new();
    if bgr < 0 {
        if let Some(c) = constant {
            m.insert("const".into(), json!(c));
        }
        m.insert("bgr".into(), json!(bgr));
        m.insert("system".into(), json!(true));
        return Value::Object(m);
    }
    let (r, g, b) = (bgr & 0xFF, (bgr >> 8) & 0xFF, (bgr >> 16) & 0xFF);
    m.insert("color".into(), json!(format!("#{r:02X}{g:02X}{b:02X}")));
    m.insert("bgr".into(), json!(bgr));
    if let Some(c) = constant {
        m.insert("const".into(), json!(c));
    }
    Value::Object(m)
}

/// An enum's value: `{"const":"alClient","value":5}` — the constant as
/// written when it's one of the property's, else the first of its values
/// that has this number.
pub fn enumerated(p: &Property, n: i64, written: Option<&str>) -> Value {
    let name = written
        .and_then(|w| p.values.iter().find(|v| v.eq_ignore_ascii_case(w.trim())))
        .or_else(|| p.values.iter().find(|v| rapidr_lang::constant(v).is_some_and(|(x, _)| x == n)));
    match name {
        Some(c) => json!({"const": c, "value": n}),
        None => json!({"value": n}),
    }
}

/// A set's value: `{"set":["akLeft","akTop"],"value":3}` — its values whose
/// bits are all in `n` (a value of 0 only when `n` is 0).
pub fn set(p: &Property, n: i64) -> Value {
    let mut names = Vec::new();
    let mut covered = 0i64;
    for v in p.values {
        let Some((bits, _)) = rapidr_lang::constant(v) else { continue };
        if (bits == 0 && n == 0) || (bits != 0 && n & bits == bits) {
            names.push(*v);
            covered |= bits;
        }
    }
    if covered == n {
        json!({"set": names, "value": n})
    } else {
        json!({"value": n})
    }
}

/// A number as JSON: an integer when it is one.
pub fn number(f: f64) -> Value {
    if f.fract() == 0.0 && f.abs() < 9.0e15 {
        json!(f as i64)
    } else {
        json!(f)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn colours_enums_and_sets() {
        assert_eq!(color(255, Some("clRed")), json!({"color": "#FF0000", "bgr": 255, "const": "clRed"}));
        assert_eq!(color(0xFF0000, None), json!({"color": "#0000FF", "bgr": 0xFF0000}));
        assert_eq!(color(-2147483633, Some("clBtnFace")), json!({"const": "clBtnFace", "bgr": -2147483633i64, "system": true}));
        let b = rapidr_lang::component("RBUTTON").unwrap();
        let align = b.property("Align").unwrap();
        assert_eq!(enumerated(align, 5, None), json!({"const": "alClient", "value": 5}));
        assert_eq!(enumerated(align, 5, Some("ALCLIENT")), json!({"const": "alClient", "value": 5}));
        assert_eq!(enumerated(align, 99, None), json!({"value": 99}));
        let anchors = b.property("Anchors").unwrap();
        assert_eq!(set(anchors, 7), json!({"set": ["akLeft", "akTop", "akRight"], "value": 7}));
        assert_eq!(number(10.0), json!(10));
        assert_eq!(number(1.5), json!(1.5));
    }
}
