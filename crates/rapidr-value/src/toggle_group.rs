//! QCOOLBTN / QOVALBTN groups (RapidQ manual, QCOOLBTN: GroupIndex, Down,
//! AllowAllUp). Buttons with the same parent and the same non-zero
//! GroupIndex work together: pressing one puts it down and releases the
//! others; pressing the one that's down releases it only when AllowAllUp.
//! Setting `Down = True` from the program releases the others too. With
//! GroupIndex 0 a button is an ordinary button and never stays down.
//!
//! The runtimes gather the buttons sharing the clicked one's parent and
//! apply the changes this returns to their widgets (desktop and web).

/// A toggle button next to the one pressed (same parent).
#[derive(Clone, Debug, PartialEq)]
pub struct Member {
    pub name: String,
    pub group: i64,
    pub down: bool,
}

/// The user pressed `name`: the Down values that change.
pub fn press(name: &str, allow_all_up: bool, members: &[Member]) -> Vec<(String, bool)> {
    let Some(me) = members.iter().find(|m| m.name.eq_ignore_ascii_case(name)) else { return Vec::new() };
    if me.group == 0 {
        return Vec::new();
    }
    if me.down {
        return if allow_all_up { vec![(me.name.clone(), false)] } else { Vec::new() };
    }
    let mut out = vec![(me.name.clone(), true)];
    out.extend(release_others(me, members));
    out
}

/// The program set `name`'s Down to `down`: the other buttons of its group
/// that must come up.
pub fn set_down(name: &str, down: bool, members: &[Member]) -> Vec<(String, bool)> {
    match members.iter().find(|m| m.name.eq_ignore_ascii_case(name)) {
        Some(me) if down && me.group != 0 => release_others(me, members),
        _ => Vec::new(),
    }
}

fn release_others(me: &Member, members: &[Member]) -> Vec<(String, bool)> {
    members
        .iter()
        .filter(|m| m.group == me.group && m.down && !m.name.eq_ignore_ascii_case(&me.name))
        .map(|m| (m.name.clone(), false))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn m(name: &str, group: i64, down: bool) -> Member {
        Member { name: name.into(), group, down }
    }

    #[test]
    fn a_group_keeps_one_down() {
        let members = [m("a", 1, true), m("b", 1, false), m("c", 2, true), m("d", 0, false)];
        assert_eq!(press("b", false, &members), vec![("b".into(), true), ("a".into(), false)]);
        assert_eq!(press("a", false, &members), vec![]);
        assert_eq!(press("a", true, &members), vec![("a".into(), false)]);
        assert_eq!(press("d", false, &members), vec![]);
        assert_eq!(set_down("b", true, &members), vec![("a".into(), false)]);
        assert_eq!(set_down("b", false, &members), vec![]);
    }
}
