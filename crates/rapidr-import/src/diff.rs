//! A preview of a conversion, as a unified diff (the conversion changes
//! names within lines, never the lines themselves).

/// `a` and `b` (the same text with some lines changed) as a unified diff
/// with `context` lines around each change.
pub fn unified(name: &str, a: &str, b: &str, context: usize) -> String {
    let al: Vec<&str> = a.split('\n').collect();
    let bl: Vec<&str> = b.split('\n').collect();
    let mut out = format!("--- {name}\n+++ {name} (RapidR names)\n");
    if al.len() != bl.len() {
        // (not a conversion's: every line)
        out.push_str(&format!("@@ -1,{} +1,{} @@\n", al.len(), bl.len()));
        out.extend(al.iter().map(|l| format!("-{}\n", l.trim_end_matches('\r'))));
        out.extend(bl.iter().map(|l| format!("+{}\n", l.trim_end_matches('\r'))));
        return out;
    }
    let changed: Vec<usize> = (0..al.len()).filter(|&i| al[i] != bl[i]).collect();
    let mut i = 0;
    while i < changed.len() {
        let start = changed[i].saturating_sub(context);
        let mut j = i;
        while j + 1 < changed.len() && changed[j + 1] <= changed[j] + 2 * context + 1 {
            j += 1;
        }
        let end = (changed[j] + context + 1).min(al.len());
        out.push_str(&format!("@@ -{0},{1} +{0},{1} @@\n", start + 1, end - start));
        for k in start..end {
            if al[k] == bl[k] {
                out.push_str(&format!(" {}\n", al[k].trim_end_matches('\r')));
            } else {
                out.push_str(&format!("-{}\n+{}\n", al[k].trim_end_matches('\r'), bl[k].trim_end_matches('\r')));
            }
        }
        i = j + 1;
    }
    out
}

#[cfg(test)]
mod tests {
    #[test]
    fn hunks() {
        let a = "1\n2\nDIM b AS QBUTTON\n4\n5\n6\n7\n8\nCREATE f AS QFORM\n";
        let b = "1\n2\nDIM b AS RButton\n4\n5\n6\n7\n8\nCREATE f AS RForm\n";
        let d = super::unified("x.bas", a, b, 1);
        assert!(d.contains("@@ -2,3 +2,3 @@\n 2\n-DIM b AS QBUTTON\n+DIM b AS RButton\n 4\n"), "{d}");
        assert!(d.contains("-CREATE f AS QFORM\n+CREATE f AS RForm\n"), "{d}");
        assert_eq!(super::unified("x", a, a, 2), "--- x\n+++ x (RapidR names)\n");
    }
}
