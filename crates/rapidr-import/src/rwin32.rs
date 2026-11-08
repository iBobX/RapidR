//! A program that includes a classic `WIN32API.INC` (any case, any path):
//! the import offers RWin32.inc, RapidR's standard Windows API declarations
//! (`rapidr_preprocessor::rwin32`), in its place, and lists what would
//! differ for the names the program uses — never a switch without the user
//! saying so (`rapidr import-rapidq --rwin32`, Studio's question), never a
//! difference unsaid. The copy is made as always first (proved to compile
//! as the original, with the old file); the switch edits the copy's
//! `$INCLUDE` lines afterwards and proves the switched copy compiles.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use crate::files::Files;
use crate::import::ImportReport;

/// The classic include's file name.
pub const OLD_NAME: &str = "WIN32API.INC";

/// Whether `written` (as an `$INCLUDE` names it) is a WIN32API.INC.
pub fn is_win32api(written: &str) -> bool {
    written.trim().rsplit(['/', '\\']).next().is_some_and(|n| n.trim().eq_ignore_ascii_case(OLD_NAME))
}

/// The offer: where the programs include WIN32API.INC, the file, what
/// would differ, and whether the copy was switched.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Offer {
    /// Each `$INCLUDE` of it: the copy's file, its line, the name as the
    /// copy's line writes it.
    pub lines: Vec<(PathBuf, usize, String)>,
    /// The old file the programs read (the original), if found.
    pub file: Option<PathBuf>,
    /// What would differ, a sentence each, for the names the programs use.
    pub differences: Vec<String>,
    /// The copy uses RWin32.inc now ([`switch_copy`]): `Ok(())` it
    /// compiles, `Err` why not.
    pub switched: Option<Result<(), String>>,
}

/// A literal number as an old include writes it (`16`, `-1`, `&H10`,
/// `&H80000000&`): its 32-bit value (`None`: an expression).
fn number(text: &str) -> Option<i64> {
    let t = text.trim().trim_end_matches(['&', '%', '!', '#']).trim();
    let (neg, t) = match t.strip_prefix('-') {
        Some(r) => (true, r.trim()),
        None => (false, t),
    };
    let v = if let Some(h) = t.strip_prefix("&H").or_else(|| t.strip_prefix("&h")) {
        i64::from_str_radix(h, 16).ok().map(|v| v as u32 as i32 as i64)?
    } else if !t.is_empty() && t.chars().all(|c| c.is_ascii_digit()) {
        t.parse::<i64>().ok()?
    } else {
        return None;
    };
    Some(if neg { -v } else { v })
}

/// A RWin32.inc constant's value (as RapidR's LONG holds it).
fn rwin32_number(line: &str) -> Option<i64> {
    let v = line.split_once(" = ")?.1.trim();
    if let Some(h) = v.strip_prefix("&H") {
        let n = i64::from_str_radix(h, 16).ok()?;
        return Some(if n <= u32::MAX as i64 { n as u32 as i32 as i64 } else { n });
    }
    v.parse().ok()
}

/// The parameters of a DECLARE line: how many.
fn param_count(line: &str) -> Option<usize> {
    let open = line.find('(')?;
    let close = line.rfind(')')?;
    let inner = line.get(open + 1..close)?.trim();
    if inner.is_empty() {
        return Some(0);
    }
    let mut depth = 0;
    let mut n = 1;
    for c in inner.chars() {
        match c {
            '(' => depth += 1,
            ')' => depth -= 1,
            ',' if depth == 0 => n += 1,
            _ => {}
        }
    }
    Some(n)
}

fn words(line: &str) -> Vec<String> {
    line.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_' || c == '"' || c == '$')).filter(|w| !w.is_empty()).map(str::to_string).collect()
}

/// A quoted string after `key` (`LIB "user32"`), lower case.
fn quoted_after(line: &str, key: &str) -> Option<String> {
    let up = line.to_ascii_uppercase();
    let at = up.find(key)? + key.len();
    let rest = &line[at..];
    let q = rest.find('"')? + 1;
    let end = rest[q..].find('"')? + q;
    Some(rest[q..end].to_string())
}

/// What differs between the old file `old` and RWin32.inc for the names a
/// program uses (`used`, upper case): a CONST's value, a function's DLL
/// function or its number of arguments, a name RWin32.inc doesn't have,
/// and what the old file did besides declaring ($DEFINE, $OPTION).
pub fn differences(old: &str, used: &HashSet<String>) -> Vec<String> {
    let mut out = Vec::new();
    let mut seen: HashSet<String> = HashSet::new();
    for raw in old.lines() {
        let line = raw.split('\'').next().unwrap_or("").trim();
        if line.is_empty() {
            continue;
        }
        let up = line.to_ascii_uppercase();
        let w = words(line);
        let up_words: Vec<String> = w.iter().map(|s| s.to_ascii_uppercase()).collect();
        // what the old file did to the program beyond its declarations
        if up.starts_with("$DEFINE ") {
            if let (Some(name), Some(value)) = (w.get(1), line.split_whitespace().nth(2)) {
                if used.contains(&name.to_ascii_uppercase()) && seen.insert(format!("$DEFINE {name}")) {
                    out.push(format!("$DEFINE {name} {value}: your WIN32API.INC made {name} mean {value} in the whole program; RWin32.inc doesn't ({name} is RapidR's own again)"));
                }
            }
            continue;
        }
        if up.starts_with("$OPTION ") {
            if seen.insert(up.clone()) {
                out.push(format!("{line}: your WIN32API.INC set this for the program; RWin32.inc doesn't"));
            }
            continue;
        }
        let skip = usize::from(matches!(up_words.first().map(String::as_str), Some("PUBLIC" | "PRIVATE" | "GLOBAL")));
        match up_words.get(skip).map(String::as_str) {
            Some("CONST") => {
                let Some(name) = w.get(skip + 1) else { continue };
                let key = name.to_ascii_uppercase();
                if !used.contains(&key) || !seen.insert(key.clone()) {
                    continue;
                }
                let Some(value_text) = line.split_once('=').map(|(_, v)| v.trim()) else { continue };
                let theirs = number(value_text);
                if rapidr_preprocessor::rwin32::left_to_rapidq_inc(name) {
                    let ours = rapidr_preprocessor::RAPIDQ_INC_CONSTANTS.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| *v);
                    if let (Some(t), Some(o)) = (theirs, ours) {
                        if t != o {
                            out.push(format!("CONST {name}: {value_text} in your WIN32API.INC, {o} in RAPIDQ.INC (RWin32.inc leaves it to RAPIDQ.INC)"));
                        }
                    }
                    continue;
                }
                match rapidr_preprocessor::rwin32::declaration(name) {
                    Some(d) if d.starts_with("CONST ") => {
                        if let (Some(t), Some(o)) = (theirs, rwin32_number(d)) {
                            if t != o {
                                out.push(format!("CONST {name}: {value_text} in your WIN32API.INC, {} in RWin32.inc", d.split_once(" = ").map_or("", |x| x.1)));
                            }
                        }
                    }
                    Some(d) => out.push(format!("CONST {name}: RWin32.inc has {name} as something else ({})", d.split('(').next().unwrap_or(d).trim())),
                    None => out.push(format!("CONST {name} ({value_text}): not in RWin32.inc — keep it in your program (CONST {name} = {value_text})")),
                }
            }
            Some("DECLARE") => {
                let Some(name) = w.get(skip + 2).filter(|_| matches!(up_words.get(skip + 1).map(String::as_str), Some("FUNCTION" | "SUB"))) else { continue };
                let name = name.trim_matches('"');
                let key = name.to_ascii_uppercase();
                if !used.contains(&key) || !seen.insert(key.clone()) {
                    continue;
                }
                let lib = quoted_after(line, "LIB").unwrap_or_default();
                let alias = quoted_after(line, "ALIAS").unwrap_or_else(|| name.to_string());
                match rapidr_preprocessor::rwin32::declaration(name) {
                    Some(d) if d.starts_with("DECLARE ") => {
                        let our_alias = quoted_after(d, "ALIAS").unwrap_or_default();
                        let our_lib = quoted_after(d, "LIB").unwrap_or_default();
                        if !our_alias.eq_ignore_ascii_case(alias.trim()) {
                            out.push(format!("{name}: RWin32.inc calls {our_alias} in {our_lib}; your WIN32API.INC called {} in {lib}", alias.trim()));
                        }
                        if let (Some(a), Some(b)) = (param_count(line), param_count(d)) {
                            if a != b {
                                out.push(format!("{name}: {b} argument(s) in RWin32.inc, {a} in your WIN32API.INC — the calls need the new number"));
                            }
                        }
                    }
                    Some(_) => out.push(format!("{name}: RWin32.inc has {name} as something else")),
                    None => out.push(format!("{name} ({lib} {}): not in RWin32.inc — keep your DECLARE of it in your program", alias.trim())),
                }
            }
            Some("TYPE") if up_words.len() == skip + 2 => {
                let name = &w[skip + 1];
                let key = name.to_ascii_uppercase();
                if used.contains(&key) && seen.insert(key) && rapidr_preprocessor::rwin32::declaration(name).is_none() {
                    out.push(format!("TYPE {name}: not in RWin32.inc — keep your TYPE {name} in your program"));
                }
            }
            _ => {}
        }
    }
    out
}

/// The offer for an import: `None` when no program includes WIN32API.INC.
/// `texts`: the programs' files (path, text) as read; the old file is one
/// of them.
pub(crate) fn offer(report: &ImportReport, texts: &[(PathBuf, String)], same: &dyn Fn(&Path, &Path) -> bool) -> Option<Offer> {
    let found: Vec<_> = report.includes.iter().filter(|i| is_win32api(&i.written)).collect();
    if found.is_empty() {
        return None;
    }
    let file = found.iter().find_map(|i| i.file.clone());
    let mut lines = Vec::new();
    for i in &found {
        let dest = report.files.iter().find(|f| same(&f.source, &i.from)).map(|f| f.dest.clone());
        if let Some(dest) = dest {
            lines.push((dest, i.line, i.rewritten.clone().unwrap_or_else(|| i.written.clone())));
        }
    }
    let mut used: HashSet<String> = HashSet::new();
    for (p, t) in texts {
        if file.as_ref().is_some_and(|f| same(p, f)) {
            continue;
        }
        used.extend(rapidr_preprocessor::rwin32::used_words(t));
    }
    let old = file.as_ref().and_then(|f| texts.iter().find(|(p, _)| same(p, f))).map(|(_, t)| t.as_str()).unwrap_or("");
    Some(Offer { lines, file, differences: differences(old, &used), switched: None })
}

/// Switches the copy to RWin32.inc: each `$INCLUDE` of WIN32API.INC in the
/// copy names RWin32.inc now; the copy of the old file stays (unused).
/// Then each program's copy is compiled ([`crate::verify`]): the result is
/// in the offer.
pub fn switch_copy(fs: &dyn Files, report: &mut ImportReport) -> Result<usize, String> {
    let Some(offer) = report.rwin32.as_mut() else { return Ok(0) };
    let mut n = 0;
    for (dest, line, written) in &offer.lines {
        let bytes = fs.read(dest)?;
        let (text, encoding) = rapidr_preprocessor::decode_source(&bytes);
        let mut out = String::with_capacity(text.len());
        for (i, l) in text.split_inclusive('\n').enumerate() {
            if i + 1 == *line && l.contains(written.as_str()) {
                out.push_str(&l.replacen(written.as_str(), rapidr_preprocessor::rwin32::FILE_NAME, 1));
                n += 1;
            } else {
                out.push_str(l);
            }
        }
        fs.write(dest, &rapidr_preprocessor::encode_source(&out, encoding))?;
    }
    // the proof: every program's copy compiles with RWin32.inc
    let mut result = Ok(());
    for p in &report.programs {
        if let Err(e) = crate::verify::bytecode_with(fs, &p.copy, &report.include_dirs, None, &[]) {
            result = Err(format!("{}: {}", p.copy.display(), e.lines().next().unwrap_or("")));
            break;
        }
    }
    offer.switched = Some(result);
    Ok(n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_differences_are_listed_for_what_is_used() {
        let old = "$OPTION VBDLL ON\n$DEFINE INTEGER SHORT\nPublic Const WM_CLOSE = &H10\nPublic Const MB_ICONERROR = 17\nPublic Const MY_OWN = 5\nConst NOT_USED = 1\nDeclare Function GetCurrentDirectory Lib \"kernel32\" Alias \"GetCurrentDirectory\" (ByVal n As Long, ByVal s As String) As Long\nDeclare Function PtInRect Lib \"user32\" (lpRect As RECT, ByVal x As Long, ByVal y As Long) As Long\nDeclare Function GetTickCount Lib \"kernel32\" () As Long\nType POINTAPI\n x As Long\nEnd Type\nType MYTYPE\n a As Long\nEnd Type\n";
        let used: HashSet<String> = ["WM_CLOSE", "MB_ICONERROR", "MY_OWN", "GETCURRENTDIRECTORY", "PTINRECT", "GETTICKCOUNT", "POINTAPI", "MYTYPE", "INTEGER"].iter().map(|s| s.to_string()).collect();
        let d = differences(old, &used);
        let all = d.join("\n");
        assert!(all.contains("$OPTION VBDLL ON"), "{all}");
        assert!(all.contains("$DEFINE INTEGER SHORT"), "{all}");
        assert!(!all.contains("WM_CLOSE"), "the same value: {all}");
        assert!(all.contains("CONST MB_ICONERROR: 17 in your WIN32API.INC, 16 in RAPIDQ.INC"), "{all}");
        assert!(all.contains("CONST MY_OWN (5): not in RWin32.inc"), "{all}");
        assert!(!all.contains("NOT_USED"), "{all}");
        assert!(all.contains("GetCurrentDirectory: RWin32.inc calls GetCurrentDirectoryA in kernel32"), "{all}");
        assert!(all.contains("PtInRect: 2 argument(s) in RWin32.inc, 3 in your WIN32API.INC"), "{all}");
        assert!(!all.contains("GetTickCount"), "{all}");
        assert!(!all.contains("POINTAPI") && all.contains("TYPE MYTYPE: not in RWin32.inc"), "{all}");
    }

    #[test]
    fn names() {
        assert!(is_win32api("WIN32API.INC") && is_win32api("c:\\rapidq\\include\\win32api.inc") && is_win32api("inc/Win32Api.Inc"));
        assert!(!is_win32api("RWin32.inc") && !is_win32api("windows.inc"));
        assert_eq!(number("&H80000000&"), Some(-2147483648));
        assert_eq!(number("-1"), Some(-1));
        assert_eq!(number("(WS_A Or WS_B)"), None);
    }
}
