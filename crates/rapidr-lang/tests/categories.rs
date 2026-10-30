//! The inspector's categories say what a property is for, never where it
//! comes from (INSPECTOR-CATEGORIES, Preview 5).
//!
//! RapidR is its own product: a property of RapidR's own on a component older
//! programs also know (`Anchors` on a button) sits in Layout with the other
//! properties of its purpose, not in a group of "RapidR's additions". The
//! origin stays in the registry (`origin`: the importer and a project kept
//! classic-compatible need it); it just isn't a category.
//!
//! - every property the inspector can show (`design`, one value, not
//!   `missing`) is in exactly one of [`PROPERTY_CATEGORIES`] — none left
//!   uncategorised, none in the "Misc" bin;
//! - no property, in any component, global or item, is in a category named
//!   after an origin (RapidR, RapidQ, extension, classic …), and any
//!   category any of them has is one the inspector knows (the list, or
//!   "Misc" for the read-only system values the inspector never shows);
//! - the list itself holds no origin word and no category twice.

use rapidr_lang::{Component, PROPERTY_CATEGORIES, COMPONENTS, GLOBALS, ITEMS};

/// Words (as part of a word) that name where a member comes from.
const ORIGIN_PARTS: &[&str] = &["rapidr", "rapidq", "extension", "classic", "legacy", "compat"];
/// Whole words that do the same.
const ORIGIN_WORDS: &[&str] = &["new", "added", "origin", "own", "extra", "extras", "additions"];

fn named_after_an_origin(category: &str) -> bool {
    let c = category.to_ascii_lowercase();
    ORIGIN_PARTS.iter().any(|w| c.contains(w)) || c.split(|ch: char| !ch.is_ascii_alphanumeric()).any(|word| ORIGIN_WORDS.contains(&word))
}

fn all() -> impl Iterator<Item = &'static Component> {
    COMPONENTS.iter().chain(GLOBALS.iter()).chain(ITEMS.iter())
}

#[test]
fn the_list_of_categories_is_purposes_only() {
    // (the check itself catches what it is for)
    for bad in ["RapidR extensions", "RapidR's own", "Classic members", "Added by RapidR", "New", "Extras", "RapidQ"] {
        assert!(named_after_an_origin(bad), "{bad:?} should count as an origin");
    }
    for good in PROPERTY_CATEGORIES.iter().chain(["Misc"].iter()) {
        assert!(!named_after_an_origin(good), "{good:?} should not count as an origin");
    }
    let mut seen = std::collections::BTreeSet::new();
    for c in PROPERTY_CATEGORIES {
        assert!(!named_after_an_origin(c), "category {c:?} is named after an origin");
        assert!(seen.insert(c.to_ascii_lowercase()), "category {c:?} twice");
        assert!(!c.is_empty() && c.chars().next().unwrap().is_ascii_uppercase(), "category {c:?}: a capitalised word");
    }
    // (the ones a student meets first are there)
    for c in ["Layout", "Appearance", "Font", "Behavior", "Help", "Accessibility", "Window"] {
        assert!(PROPERTY_CATEGORIES.contains(&c), "{c} is missing from the categories");
    }
}

#[test]
fn every_property_the_inspector_shows_has_a_purpose_category() {
    let mut bad = Vec::new();
    let mut shown = 0;
    for c in all() {
        for p in c.properties.iter().filter(|p| p.design && p.indexed == 0 && !p.missing) {
            shown += 1;
            if p.category.is_empty() {
                bad.push(format!("{}.{}: no category", c.name, p.name));
            } else if !PROPERTY_CATEGORIES.contains(&p.category) {
                bad.push(format!("{}.{}: category {:?} is not one of the purposes ({})", c.name, p.name, p.category, PROPERTY_CATEGORIES.join(", ")));
            }
        }
    }
    assert!(shown > 1500, "{shown}: the registry's properties were not read");
    assert!(bad.is_empty(), "{} properties the inspector shows have no purpose category:\n{}", bad.len(), bad.join("\n"));
}

#[test]
fn no_category_is_named_after_where_a_member_comes_from() {
    let mut bad = Vec::new();
    for c in all() {
        for p in c.properties {
            if p.category.is_empty() {
                continue;
            }
            if named_after_an_origin(p.category) {
                bad.push(format!("{}.{}: category {:?} names an origin", c.name, p.name, p.category));
            } else if p.category != "Misc" && !PROPERTY_CATEGORIES.contains(&p.category) {
                bad.push(format!("{}.{}: category {:?} is not a known purpose", c.name, p.name, p.category));
            }
        }
    }
    assert!(bad.is_empty(), "{}:\n{}", bad.len(), bad.join("\n"));
}

/// The members of RapidR's own that sit on a classic component (`Anchors`
/// on a button) have the category of their purpose: the same one the
/// classic members of that purpose have.
#[test]
fn a_member_of_rapidrs_own_sits_with_the_members_of_its_purpose() {
    let button = rapidr_lang::component("RButton").expect("RButton");
    let category = |name: &str| button.properties.iter().find(|p| p.name.eq_ignore_ascii_case(name)).unwrap_or_else(|| panic!("RButton.{name}")).category;
    for (name, expect) in [
        ("Anchors", "Layout"),
        ("MinWidth", "Layout"),
        ("MaxHeight", "Layout"),
        ("Width", "Layout"),
        ("FontColor", "Font"),
        ("FontSize", "Font"),
        ("Font", "Font"),
        ("AccessibleName", "Accessibility"),
        ("AccessibleDescription", "Accessibility"),
        ("AccessibleHidden", "Accessibility"),
        ("Hint", "Help"),
    ] {
        assert_eq!(category(name), expect, "RButton.{name}");
    }
    let form = rapidr_lang::component("RForm").expect("RForm");
    let category = |name: &str| form.properties.iter().find(|p| p.name.eq_ignore_ascii_case(name)).unwrap_or_else(|| panic!("RForm.{name}")).category;
    for (name, expect) in [("AutoSize", "Layout"), ("HelpFile", "Help"), ("BrushColor", "Drawing"), ("PenColor", "Drawing"), ("FontName", "Font")] {
        assert_eq!(category(name), expect, "RForm.{name}");
    }
}
