//! RNOTIFY's and RNOTIFYBAR's models: timing, stacking, merging, the
//! history, and where everything goes (docs/design-guidelines.md,
//! "Notifications").

use super::notify::*;
use super::notifybar::{self, by_letter, compact_h, parse_buttons, Bar, BUTTON_GAP, EDGE, TEXT_X};
use crate::objects::font::Font;
use crate::objects::text::text_size;

fn font() -> Font {
    Font { name: "Inter".into(), size: 10, color: 0, styles: 0 }
}

fn say(m: &mut Notify, text: &str, key: &str) -> i64 {
    m.say(text, Kind::Info, Vec::new(), None, key, "10:42").0
}

#[test]
fn a_toast_stays_as_long_as_its_words_need() {
    // (3 s and 80 ms a character, 2 s more with an action, 5 to 20 s)
    assert_eq!(stay_ms("Saved.", false, Kind::Info, -1, 100), Some(5000));
    let fifty_four = "5 files were reloaded: main.rr, form1.rr, utils.inc +2";
    assert_eq!(fifty_four.chars().count(), 54);
    assert_eq!(stay_ms(fifty_four, true, Kind::Info, -1, 100), Some(3000 + 80 * 54 + 2000));
    assert_eq!(stay_ms(&"x".repeat(500), false, Kind::Info, -1, 100), Some(20000));
    // (a warning one and a half times as long, an error until it is closed)
    assert_eq!(stay_ms("Saved.", false, Kind::Warning, -1, 100), Some(7500));
    assert_eq!(stay_ms("It failed", false, Kind::Error, -1, 100), None);
    // (the program's own time, the person's setting)
    assert_eq!(stay_ms("x", false, Kind::Info, 1500, 100), Some(1500));
    assert_eq!(stay_ms("x", false, Kind::Info, 0, 100), None);
    assert_eq!(stay_ms("x", false, Kind::Info, -1, 0), None);
    assert_eq!(stay_ms("Saved.", false, Kind::Info, -1, 200), Some(10000));
}

#[test]
fn the_same_key_is_one_toast_and_one_history_line() {
    let mut m = Notify::default();
    let a = say(&mut m, "5 files were reloaded", "reload");
    let b = say(&mut m, "7 files were reloaded", "reload");
    assert_eq!(a, b);
    assert_eq!(m.visible().count(), 1);
    assert_eq!(m.toast(a).unwrap().text, "7 files were reloaded");
    assert_eq!(m.history.len(), 1);
    assert_eq!(m.history[0].text, "7 files were reloaded");
    // (no key: every one is its own)
    say(&mut m, "Copied", "");
    say(&mut m, "Copied", "");
    assert_eq!(m.visible().count(), 3);
}

#[test]
fn three_at_most_the_oldest_goes_to_the_history() {
    let mut m = Notify::default();
    let ids: Vec<i64> = (0..3).map(|i| say(&mut m, &format!("One {i}"), "")).collect();
    let (id4, pushed) = m.say("Four", Kind::Info, Vec::new(), None, "", "10:42");
    assert_eq!(pushed, [Closed { id: ids[0], key: String::new(), reason: "pushed".into() }]);
    assert_eq!(m.visible().map(|t| t.id).collect::<Vec<_>>(), [ids[1], ids[2], id4]);
    assert_eq!(m.history.len(), 4, "the pushed one is still in the history");
    assert_eq!(m.unseen(), 4);
}

#[test]
fn time_runs_only_while_nothing_holds_the_toast() {
    let mut m = Notify::default();
    let id = say(&mut m, "Saved.", "");
    // (drawn for the first time: its 5 s start)
    assert!(m.advance(1000).is_empty());
    assert_eq!(m.toast(id).unwrap().left_ms, 5000);
    // (the mouse on it: the time stands still)
    m.hover = Some(Hit::Card(id));
    m.advance(4000);
    assert_eq!(m.toast(id).unwrap().left_ms, 5000);
    // (the mouse leaves: 2 s at least, here the 5 s it has)
    m.hover = None;
    m.released(id);
    m.advance(6000);
    assert_eq!(m.toast(id).unwrap().left_ms, 3000);
    // (a little time left, the mouse comes and goes: 2 s at least)
    m.advance(8500);
    assert_eq!(m.toast(id).unwrap().left_ms, 500);
    m.released(id);
    assert_eq!(m.toast(id).unwrap().left_ms, HELD_MIN_MS);
    // (out of time: it fades, then it is gone, reported once)
    m.advance(10_500);
    assert!(m.toast(id).unwrap().leaving.is_some());
    assert!(m.advance(10_500 + LEAVE_MS - 1).is_empty());
    let gone = m.advance(10_500 + LEAVE_MS);
    assert_eq!(gone, [Closed { id, key: String::new(), reason: "timeout".into() }]);
    assert!(m.toasts.is_empty());
    // (the keyboard on it holds it too)
    let id = say(&mut m, "Again", "");
    m.advance(20_000);
    m.focus = Some(Hit::Action(id, 0));
    m.advance(30_000);
    assert_eq!(m.toast(id).unwrap().left_ms, 5000);
}

#[test]
fn an_error_stays_until_it_is_closed() {
    let mut m = Notify::default();
    let id = m.say("It failed", Kind::Error, Vec::new(), None, "", "10:42").0;
    m.advance(0);
    assert!(m.advance(600_000).is_empty());
    m.leave(id, "user", 600_000);
    assert_eq!(m.advance(600_000 + LEAVE_MS), [Closed { id, key: String::new(), reason: "user".into() }]);
}

#[test]
fn the_newest_toast_is_nearest_the_corner() {
    let mut m = Notify::default();
    let a = say(&mut m, "Older", "");
    let b = say(&mut m, "Newer", "");
    let f = font();
    let cards = m.layout(1000, 600, &f, &f);
    let (ca, cb) = (cards.iter().find(|c| c.id == a).unwrap(), cards.iter().find(|c| c.id == b).unwrap());
    // (bottom right: 12 in; the newer one lowest, the older above it, 8 apart)
    assert_eq!(cb.rect.0 + cb.rect.2, 1000 - 12);
    assert_eq!(cb.rect.1 + cb.rect.3, 600 - 12);
    assert_eq!(ca.rect.1 + ca.rect.3 + GAP, cb.rect.1);
    assert_eq!(cb.rect.3, ONE_H);
    assert_eq!(cb.rect.2, CARD_W);
    // (top left: the newest at the top)
    m.position = Position::TopLeft;
    let cards = m.layout(1000, 600, &f, &f);
    let (ca, cb) = (cards.iter().find(|c| c.id == a).unwrap(), cards.iter().find(|c| c.id == b).unwrap());
    assert_eq!((cb.rect.0, cb.rect.1), (12, 12));
    assert_eq!(cb.rect.1 + cb.rect.3 + GAP, ca.rect.1);
}

#[test]
fn a_toast_sits_in_the_area_it_was_given() {
    let mut m = Notify::default();
    say(&mut m, "Saved", "");
    m.area = Some((200, 100, 600, 300));
    let f = font();
    let c = &m.layout(1000, 600, &f, &f)[0];
    // (12 in from the area's bottom right, not the component's)
    assert_eq!((c.rect.0 + c.rect.2, c.rect.1 + c.rect.3), (800 - 12, 400 - 12));
}

#[test]
fn long_text_is_two_lines_and_cut_in_its_middle() {
    let f = font();
    let two = wrap_two("5 files were reloaded: main.rr, form1.rr, utils.inc, invoice.rr, orders.rr, stock.rr", &f, 250);
    assert_eq!(two.len(), 2);
    assert!(two[0].starts_with("5 files were"));
    let long = "x".repeat(40) + " " + &"y".repeat(100) + ".rr";
    let cut = wrap_two(&long, &f, 250);
    assert_eq!(cut.len(), 2);
    assert!(cut[1].contains('…') && cut[1].ends_with(".rr"), "the extension stays: {cut:?}");
    // (two lines make a 52-px card)
    let mut m = Notify::default();
    m.say(&long, Kind::Info, Vec::new(), None, "", "10:42");
    assert_eq!(m.layout(1000, 600, &f, &f)[0].rect.3, TWO_H);
}

#[test]
fn two_actions_make_a_wider_card_and_the_actions_sit_before_the_close_button() {
    let mut m = Notify::default();
    m.say("Done", Kind::Success, parse_actions("Show|&Undo"), None, "", "10:42");
    let f = font();
    let c = &m.layout(1000, 600, &f, &f)[0];
    assert_eq!(c.rect.2, CARD_W_TWO);
    assert_eq!(c.actions.len(), 2);
    assert!(c.actions[0].1 .0 < c.actions[1].1 .0 && c.actions[1].1 .0 + c.actions[1].1 .2 < c.close.0);
    assert_eq!(c.close.2, CLOSE);
    assert_eq!(plain(&c.actions[1].0), "Undo");
}

#[test]
fn a_retired_action_is_greyed_in_the_history_and_gone_from_the_toast() {
    let mut m = Notify::default();
    let id = m.say("Chat deleted", Kind::Info, parse_actions("Undo"), None, "chat-7", "10:42").0;
    assert!(m.entry(id).unwrap().works(0));
    assert_eq!(m.retire("chat-7", "Undo is no longer possible: the chat was deleted for good"), 1);
    assert!(!m.entry(id).unwrap().works(0));
    assert_eq!(m.entry(id).unwrap().retired[0].as_deref(), Some("Undo is no longer possible: the chat was deleted for good"));
    assert!(m.toast(id).unwrap().actions.is_empty());
}

#[test]
fn the_history_is_even_newest_first_and_marks_seen() {
    let mut m = Notify::default();
    say(&mut m, "First", "");
    say(&mut m, "Second thing that is quite a bit longer than the card is wide so it needs two lines or more", "");
    say(&mut m, "Third", "");
    assert_eq!(m.unseen(), 3);
    assert_eq!(m.history[0].text, "Third");
    let f = font();
    m.history_open = true;
    let g = m.history_layout(1000, 600, &f, &f);
    // (rows touch, none overlapping: evenly spaced, 8 above and below each)
    for pair in g.rows.windows(2) {
        assert_eq!(pair[0].rect.1 + pair[0].rect.3, pair[1].rect.1);
    }
    assert!(g.rows.iter().all(|r| r.rect.3 == 8 + 16 + r.lines.len() as i64 * LINE + 8));
    assert!(g.rows[1].lines.len() > 1);
    // (above the corner, 12 in)
    assert_eq!(g.rect.0 + g.rect.2, 1000 - 12);
    assert_eq!(g.rect.1 + g.rect.3, 600 - 12);
    m.mark_seen();
    assert_eq!(m.unseen(), 0);
}

#[test]
fn the_history_scrolls_past_eight_rows() {
    let mut m = Notify::default();
    for i in 0..30 {
        m.say(&format!("Message {i}"), Kind::Info, Vec::new(), None, "", "10:42");
    }
    let f = font();
    let g = m.history_layout(1000, 900, &f, &f);
    assert!(g.content > g.body.3);
    assert!(g.body.3 <= HIST_ROWS * 64);
    m.history_top = 10_000;
    let g = m.history_layout(1000, 900, &f, &f);
    let last = g.rows.last().unwrap();
    assert!(last.rect.1 + last.rect.3 <= g.body.1 + g.body.3, "scrolled to the end: the last row is in view");
}

#[test]
fn a_message_never_spells_a_secret_out_or_hides_its_end() {
    // (a credential the program holds is masked, as in the Output pane)
    crate::secrets::note("sk-notify-test-0123456789");
    let mut m = Notify::default();
    let (id, _) = m.say("The call with sk-notify-test-0123456789 failed", Kind::Error, Vec::new(), None, "", "10:42");
    let said = m.toast_mut(id).unwrap().text.clone();
    assert!(!said.contains("sk-notify-test"), "{said}");
    assert!(said.contains(crate::secrets::MASK), "{said}");
    assert!(!m.history[0].text.contains("sk-notify-test"));
    // (a line break is a space; a right-to-left override is gone)
    assert_eq!(clean("two\nlines\tand\u{202E}gnirts"), "two lines andgnirts");
}

#[test]
fn words_are_kept_whole_for_a_screen_reader() {
    assert_eq!(spoken("Done.", &[]), "Done.");
    assert_eq!(spoken("Chat deleted", &["&Undo".into()]), "Chat deleted (Undo)");
    assert_eq!(mnemonic("&Undo"), Some('u'));
    assert_eq!(plain("Use && Keep"), "Use & Keep");
}

// ---- RNOTIFYBAR ----------------------------------------------------------------

fn bar_fonts() -> (Font, Font) {
    let f = font();
    (Font { styles: 1, ..f.clone() }, f)
}

fn bar() -> Bar {
    Bar { text: "main.rr changed on disk while you were editing it.".into(), buttons: parse_buttons("&Compare,&Use Disk Version,&Keep Mine"), ..Bar::default() }
}

#[test]
fn the_strip_is_one_slim_line_when_there_is_room() {
    let (bold, f) = bar_fonts();
    let g = bar().layout(757, &bold, &f);
    assert!(!g.two_lines);
    assert_eq!(g.height, compact_h() + 12);
    // (the buttons at the right end, 12 from the edge, 8 apart, all on one centre line)
    let last = g.buttons.last().unwrap();
    assert_eq!(last.0 + last.2, 757 - EDGE);
    assert!(g.buttons.windows(2).all(|p| p[0].0 + p[0].2 + BUTTON_GAP == p[1].0 && p[0].1 == p[1].1));
    assert!(g.buttons.iter().all(|r| r.3 == compact_h()));
    // (the question whole, before the first button)
    assert_eq!(g.shown, "main.rr changed on disk while you were editing it.");
    assert!(g.text.0 + text_size(&g.shown, &bold).0 <= g.buttons[0].0);
}

#[test]
fn a_narrow_strip_puts_the_buttons_under_the_question() {
    let (bold, f) = bar_fonts();
    let g = bar().layout(480, &bold, &f);
    assert!(g.two_lines);
    assert!(g.height > bar().layout(757, &bold, &f).height);
    assert!(g.buttons.iter().all(|r| r.1 > g.text.1 + g.text.3 - 1));
    assert_eq!(g.buttons[0].0, TEXT_X);
    // (in between: the question cut with an ellipsis, still one line)
    let mid = bar().layout(650, &bold, &f);
    assert!(!mid.two_lines && mid.shown.ends_with('…'), "{mid:?}");
}

#[test]
fn a_very_narrow_strip_wraps_its_buttons_onto_more_rows_and_none_passes_the_edge() {
    let (bold, f) = bar_fonts();
    for w in [200, 280, 320, 480] {
        let g = bar().layout(w, &bold, &f);
        assert!(g.two_lines, "{w}");
        assert!(g.buttons.iter().all(|r| r.0 >= 0 && r.0 + r.2 <= w), "{w}: a button passes the right edge: {:?}", g.buttons);
        assert!(g.buttons.iter().all(|r| r.1 + r.3 <= g.height), "{w}: a button below the strip: {:?} in {}", g.buttons, g.height);
    }
    // (a row more makes it taller)
    assert!(bar().layout(280, &bold, &f).height > bar().layout(480, &bold, &f).height);
}

#[test]
fn a_strip_button_is_pressed_by_its_letter() {
    let b = parse_buttons("&Compare, &Use Disk Version ,&Keep Mine,,");
    assert_eq!(b, ["&Compare", "&Use Disk Version", "&Keep Mine"]);
    assert_eq!(by_letter(&b, 'k'), Some(2));
    assert_eq!(by_letter(&b, 'U'), Some(1));
    assert_eq!(by_letter(&b, 'z'), None);
    assert_eq!(notifybar::spoken(&bar()), "main.rr changed on disk while you were editing it. Compare, Use Disk Version, Keep Mine");
}
