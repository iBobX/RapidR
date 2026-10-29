//! RUPDOWN's `Associate`: the edit (or label) beside the arrows that shows
//! their number, as RapidQ's QUpDown.Associate does (Windows' up-down
//! control with a "buddy" window). Setting Position, or the arrows moving
//! it, writes the number into the buddy; and the arrows count from the
//! number the buddy shows, so a value typed there is stepped from, not lost.
//!
//! Shared by the UI kernel (which reads the buddy's number when an arrow is
//! pressed) and the runtimes (which write the buddy when Position or
//! Associate is set: `rp_comp_set`, desktop and web).

use crate::Value;

/// The property that shows the number, by the buddy's type: an edit's
/// `Text`, a label's `Caption`; other components can't be a buddy.
pub fn buddy_prop(type_name: &str) -> Option<&'static str> {
    match type_name {
        "REDIT" => Some("text"),
        "RLABEL" => Some("caption"),
        _ => None,
    }
}

/// The whole number a buddy's text holds (blanks around it are fine), or
/// none: empty, or not a whole number.
pub fn parse(text: &str) -> Option<i64> {
    let t = text.trim();
    let digits = t.strip_prefix(['-', '+']).unwrap_or(t);
    if digits.is_empty() || !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    t.parse().ok()
}

/// Min, Max and Position (clamped) as the up-down keeps them.
pub fn range(get: &dyn Fn(&str, &str) -> Value, id: &str) -> (i64, i64, i64) {
    let int = |prop: &str, default: i64| match get(id, prop) {
        Value::Null => default,
        v => v.to_i64(),
    };
    let min = int("min", 0);
    let max = int("max", 100).max(min);
    (min, max, int("position", 0).clamp(min, max))
}

/// The buddy of up-down `id`: its id (lower case) and the property that
/// shows the number. `get` reads a property, `type_of` a type name
/// (`REDIT` …). None: no Associate, or one that can't show a number.
pub fn buddy(get: &dyn Fn(&str, &str) -> Value, type_of: &dyn Fn(&str) -> String, id: &str) -> Option<(String, &'static str)> {
    let name = match get(id, "associate") {
        Value::Null => return None,
        v => v.to_string_val().trim().to_lowercase(),
    };
    if name.is_empty() || name.eq_ignore_ascii_case(id) {
        return None;
    }
    buddy_prop(&type_of(&name).to_ascii_uppercase()).map(|prop| (name, prop))
}

/// What up-down `id`'s buddy should show now: (the buddy, its property, the
/// text), or none when there is no buddy or it shows that text already
/// (`get` reads the buddy's current text).
pub fn buddy_text(get: &dyn Fn(&str, &str) -> Value, type_of: &dyn Fn(&str) -> String, id: &str) -> Option<(String, &'static str, String)> {
    let (name, prop) = buddy(get, type_of, id)?;
    let text = range(get, id).2.to_string();
    (get(&name, prop).to_string_val() != text).then_some((name, prop, text))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{v_int, v_str};

    #[test]
    fn a_buddy_shows_a_whole_number() {
        assert_eq!(parse("7"), Some(7));
        assert_eq!(parse("  -12 "), Some(-12));
        assert_eq!(parse("+3"), Some(3));
        for no in ["", " ", "x", "1.5", "1 2", "--1", "+", "٣"] {
            assert_eq!(parse(no), None, "{no:?}");
        }
    }

    #[test]
    fn only_edits_and_labels_can_be_buddies() {
        assert_eq!(buddy_prop("REDIT"), Some("text"));
        assert_eq!(buddy_prop("RLABEL"), Some("caption"));
        assert_eq!(buddy_prop("RBUTTON"), None);
    }

    #[test]
    fn the_buddy_follows_position_clamped() {
        let props = |id: &str, prop: &str| match (id, prop) {
            ("ud", "associate") => v_str("Dose"),
            ("ud", "min") => v_int(0),
            ("ud", "max") => v_int(20),
            ("ud", "position") => v_int(35),
            ("dose", "text") => v_str("5"),
            _ => Value::Null,
        };
        let types = |id: &str| match id {
            "dose" => "REDIT".to_string(),
            _ => String::new(),
        };
        assert_eq!(buddy_text(&props, &types, "ud"), Some(("dose".to_string(), "text", "20".to_string())));
        // (it shows it already: nothing to do)
        let same = |id: &str, prop: &str| if (id, prop) == ("dose", "text") { v_str("20") } else { props(id, prop) };
        assert_eq!(buddy_text(&same, &types, "ud"), None);
        // (no Associate; one that is no edit; itself)
        let none = |id: &str, prop: &str| if prop == "associate" { Value::Null } else { props(id, prop) };
        assert_eq!(buddy_text(&none, &types, "ud"), None);
        let other = |id: &str, prop: &str| if prop == "associate" { v_str("ud") } else { props(id, prop) };
        assert_eq!(buddy_text(&other, &types, "ud"), None);
        assert_eq!(buddy_text(&props, &|_| "RBUTTON".to_string(), "ud"), None);
    }
}
