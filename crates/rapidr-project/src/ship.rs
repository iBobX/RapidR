//! What a program takes with it besides the files right beside it
//! (docs/program-languages-plan.md §2, lane T1b): the files in folders
//! below its own that its source names (`"art/logo.svg"`,
//! `"data\sample.csv"`), and its translations (`lang/*.po`) when it is
//! translated. These are the rules, without a disk: the desktop's builds
//! and copies apply them to the disk ([`crate::program_files::carried_of`]),
//! RapidR Studio on the web to the page's store.
//!
//! A name never leads out of the program's folder: `"../secret.txt"` and an
//! absolute path (`"/etc/hosts"`, `"C:\data\x.csv"`) are [`Place::Outside`],
//! which a build tells about and never copies.

/// The folder of a program's translations, beside it (`lang/es.po`).
pub const LANG_FOLDER: &str = "lang";

/// Where a file a program's source names is, from the program's folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Place {
    /// Right beside it (`"staff.csv"`): the files a build has always taken
    /// (`program_files::data_files_of`).
    Beside,
    /// In a folder below its own: its path from there, `/`-separated
    /// (`art/logo.svg` for `"art\logo.svg"` or `"./art/logo.svg"`).
    Below(String),
    /// Outside its folder (`..`, an absolute path): never taken.
    Outside,
}

/// Where name `name` (a string literal that reads as a file:
/// `named_paths`) is.
pub fn place(name: &str) -> Place {
    let n = name.trim().replace('\\', "/");
    if crate::implicit::is_absolute(&n) {
        return Place::Outside;
    }
    let parts: Vec<&str> = n.split('/').filter(|p| !p.is_empty() && *p != ".").collect();
    if parts.contains(&"..") {
        return Place::Outside;
    }
    match parts.len() {
        0 | 1 => Place::Beside,
        _ => Place::Below(parts.join("/")),
    }
}

/// Whether file name `name` is a translation catalog (`es.po`, `pt-BR.po`):
/// a `.po` file's plain name, not a hidden one.
pub fn is_catalog(name: &str) -> bool {
    name.len() > 3 && !name.starts_with('.') && !name.contains(['/', '\\']) && name.to_ascii_lowercase().ends_with(".po")
}

/// Whether source `text` translates: it uses `TR$`, `TRN$` or
/// `Application.Language` (any `.Language`), outside strings and comments.
pub fn uses_translations(text: &str) -> bool {
    for line in text.lines() {
        let t = line.trim_start();
        if t.starts_with('\'') || t.get(..4).is_some_and(|h| h.eq_ignore_ascii_case("rem ")) || t.eq_ignore_ascii_case("rem") {
            continue;
        }
        let b = line.as_bytes();
        let mut after_dot = false;
        let mut i = 0;
        while i < b.len() {
            let c = b[i];
            if c == b'"' {
                match line[i + 1..].find('"') {
                    Some(end) => {
                        i += end + 2;
                        after_dot = false;
                        continue;
                    }
                    None => break,
                }
            }
            if c == b'\'' {
                break;
            }
            if c.is_ascii_alphabetic() || c == b'_' {
                let start = i;
                while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') {
                    i += 1;
                }
                let mut word = line[start..i].to_ascii_lowercase();
                if b.get(i) == Some(&b'$') {
                    word.push('$');
                    i += 1;
                }
                if word == "tr$" || word == "trn$" || (after_dot && word == "language") {
                    return true;
                }
                if word == "rem" && !after_dot {
                    break;
                }
                after_dot = false;
                continue;
            }
            if c == b'.' {
                after_dot = true;
            } else if !c.is_ascii_whitespace() {
                after_dot = false;
            }
            i += 1;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn places() {
        assert_eq!(place("staff.csv"), Place::Beside);
        assert_eq!(place("./staff.csv"), Place::Beside);
        assert_eq!(place("art/logo.svg"), Place::Below("art/logo.svg".into()));
        assert_eq!(place("data\\sample.csv"), Place::Below("data/sample.csv".into()));
        assert_eq!(place("./art//icons/./a.png"), Place::Below("art/icons/a.png".into()));
        // (never out of the program's folder)
        for out in ["../secret.txt", "..\\secret.txt", "art/../../x.txt", "art/../logo.svg", "/etc/hosts", "\\x\\y.txt", "\\\\server\\share\\x.csv", "C:\\data\\x.csv", "c:x.csv", "C:/x/y.csv"] {
            assert_eq!(place(out), Place::Outside, "{out}");
        }
    }

    #[test]
    fn catalogs() {
        for yes in ["es.po", "pt-BR.po", "zh-Hans.PO", "es_AR.po"] {
            assert!(is_catalog(yes), "{yes}");
        }
        for no in [".po", ".hidden.po", "es.pot", "es.po.bak", "sub/es.po", "a\\b.po", "index.txt"] {
            assert!(!is_catalog(no), "{no}");
        }
    }

    #[test]
    fn translated_programs() {
        assert!(uses_translations("PRINT TR$(\"Hello\")\n"));
        assert!(uses_translations("x$ = trn$(\"{n} file\", \"{n} files\", n)"));
        assert!(uses_translations("Application.Language = \"es\"\n"));
        assert!(uses_translations("IF App . Language = \"pt\" THEN END"));
        assert!(uses_translations("PRINT 1: L.Caption = Tr$(\"Save\")"));
        // (in a string, a comment, another name: not)
        assert!(!uses_translations("PRINT \"TR$(x)\"\n' TR$(\"a\")\nREM Application.Language\nx = 1 ' TRN$(\nPRINT 2: REM TR$("));
        assert!(!uses_translations("Language = 1\nDIM TRX$ AS STRING\nSTR$(1)\nPRINT CenterTR$"));
        assert!(!uses_translations(""));
    }
}
