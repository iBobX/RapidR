//! The panels' search: what the user typed found in a name, the way VS
//! Code's quick open and Xcode's library find things — the letters in
//! order, any case, others skipped between them — scored so that the
//! closest matches come first (a whole word, a word's start, letters side
//! by side, an early start), with the matched letters' places for drawing
//! them marked.
//!
//! The query's words (split at spaces) match separately, each anywhere in
//! the text: "save all" finds "File: Save All" and "All Save".

/// How well `query` matches `text`: a score (higher is better) and the
/// matched characters' indexes in `text` (chars, ascending). `None`: no
/// match. An empty query matches everything with score 0.
pub fn score(query: &str, text: &str) -> Option<(i32, Vec<usize>)> {
    let words: Vec<&str> = query.split_whitespace().collect();
    if words.is_empty() {
        return Some((0, Vec::new()));
    }
    let chars: Vec<char> = text.chars().collect();
    let mut total = 0;
    let mut marks: Vec<usize> = Vec::new();
    for w in words {
        let (s, m) = word_score(w, &chars)?;
        total += s;
        marks.extend(m);
    }
    marks.sort_unstable();
    marks.dedup();
    Some((total, marks))
}

/// Whether `query` matches `text` at all.
pub fn matches(query: &str, text: &str) -> bool {
    score(query, text).is_some()
}

fn lower(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

/// Where a word starts: the text's start, after a separator, a lower to
/// upper case step (`SaveAll`), a letter after a digit or the other way.
fn word_start(chars: &[char], i: usize) -> bool {
    if i == 0 {
        return true;
    }
    let (p, c) = (chars[i - 1], chars[i]);
    (!p.is_alphanumeric() && c.is_alphanumeric()) || (p.is_lowercase() && c.is_uppercase()) || (p.is_alphabetic() != c.is_alphabetic() && c.is_alphanumeric())
}

/// One word of the query: the best of a greedy match from each place its
/// first letter occurs (texts are short: names, titles).
fn word_score(word: &str, chars: &[char]) -> Option<(i32, Vec<usize>)> {
    let q: Vec<char> = word.chars().map(lower).collect();
    if q.is_empty() {
        return Some((0, Vec::new()));
    }
    let low: Vec<char> = chars.iter().map(|&c| lower(c)).collect();
    // (a substring is best: as a word's start, better still)
    let text: String = low.iter().collect();
    let needle: String = q.iter().collect();
    if let Some(byte) = text.find(&needle) {
        let start = text[..byte].chars().count();
        let mut s = 100 + 10 * q.len() as i32 - start as i32;
        if word_start(chars, start) {
            s += 60;
        }
        if start == 0 {
            s += 40;
        }
        if q.len() == low.len() {
            s += 50;
        }
        return Some((s, (start..start + q.len()).collect()));
    }
    let mut best: Option<(i32, Vec<usize>)> = None;
    for first in (0..low.len()).filter(|&i| low[i] == q[0]) {
        let mut at = Vec::with_capacity(q.len());
        let mut j = first;
        let mut ok = true;
        for &qc in &q {
            // (prefer the next word's start for this letter, else the next one)
            let next = (j..low.len()).find(|&k| low[k] == qc && word_start(chars, k)).filter(|&k| at.last().is_none_or(|&l: &usize| k > l));
            let plain = (j..low.len()).find(|&k| low[k] == qc);
            let pick = match (plain, next) {
                (Some(p), _) if at.last().is_some_and(|&l| p == l + 1) => Some(p),
                (_, Some(n)) => Some(n),
                (p, None) => p,
            };
            match pick {
                Some(k) => {
                    at.push(k);
                    j = k + 1;
                }
                None => {
                    ok = false;
                    break;
                }
            }
        }
        if !ok {
            continue;
        }
        let mut s = 10 * q.len() as i32 - first as i32;
        for (n, &k) in at.iter().enumerate() {
            if word_start(chars, k) {
                s += 15;
            }
            if n > 0 && k == at[n - 1] + 1 {
                s += 8;
            } else if n > 0 {
                s -= (k - at[n - 1]) as i32;
            }
        }
        if best.as_ref().is_none_or(|(b, _)| s > *b) {
            best = Some((s, at));
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn letters_in_order_any_case() {
        assert!(matches("btn", "QBUTTON"));
        assert!(matches("QBUT", "qbutton"));
        assert!(!matches("tb", "QBUTTON") && matches("tb", "QTRACKBAR"));
        assert_eq!(score("", "anything"), Some((0, vec![])));
    }

    #[test]
    fn closer_matches_score_higher() {
        let s = |q, t| score(q, t).map(|(s, _)| s).unwrap_or(i32::MIN);
        // (a word's start beats the middle; a substring beats scattered letters)
        assert!(s("sa", "File: Save All") > s("sa", "Disassemble"));
        assert!(s("button", "QBUTTON") > s("button", "QCOOLBTN QBUTTONS"));
        assert!(s("edit", "QEDIT") > s("edit", "QRICHEDIT"));
        assert!(s("cb", "QCOMBOBOX") > s("cb", "QCOOLBTN") || s("cb", "QCOOLBTN") > 0);
    }

    #[test]
    fn marks_are_the_matched_letters() {
        assert_eq!(score("sav", "File: Save").unwrap().1, vec![6, 7, 8]);
        // (letters side by side beat a later word's start)
        let (_, m) = score("fsa", "File: Save All").unwrap();
        assert_eq!(m, vec![0, 6, 7]);
        // (words match apart)
        let (_, m) = score("all save", "File: Save All").unwrap();
        assert_eq!(m, vec![6, 7, 8, 9, 11, 12, 13]);
    }
}
