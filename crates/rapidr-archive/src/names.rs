//! Entry names made safe to write (the zip-slip guard): an entry is only
//! ever written below the folder it's extracted to.

/// Windows' device names: a file called `CON` or `nul.txt` is the device,
/// on Windows, not a file.
const DEVICES: &[&str] = &[
    "CON", "PRN", "AUX", "NUL", "COM1", "COM2", "COM3", "COM4", "COM5", "COM6", "COM7", "COM8", "COM9", "LPT1", "LPT2", "LPT3", "LPT4", "LPT5", "LPT6", "LPT7", "LPT8",
    "LPT9", "CONIN$", "CONOUT$",
];

/// The folders and the file an entry `name` is written as, below the
/// folder it's extracted to — or why it can't be, in plain words: an
/// absolute path (`/etc/passwd`, `\\server\share`), a drive (`C:`), `..`
/// anywhere, a `:` (a drive or an NTFS stream), a device (`CON`, `nul.txt`),
/// a control character, a part ending in a dot or a space (Windows drops
/// them: two names would be one file), or no name at all. `\` is a folder
/// separator as `/` is; `.` parts and empty ones (`a//b`) are dropped.
pub fn safe_path(name: &str) -> Result<Vec<String>, String> {
    let refuse = |why: &str| Err(format!("\"{}\" {why}: not extracted (it would be written outside the folder)", name.escape_debug()));
    let n = name.replace('\\', "/");
    if n.starts_with('/') {
        return refuse("is an absolute path");
    }
    let b = n.as_bytes();
    if b.len() >= 2 && b[0].is_ascii_alphabetic() && b[1] == b':' {
        return refuse("names a drive");
    }
    let mut parts = Vec::new();
    for part in n.split('/') {
        if part.is_empty() || part == "." {
            continue;
        }
        if part == ".." {
            return refuse("goes up a folder (..)");
        }
        if part.chars().any(|c| c.is_control()) {
            return refuse("has a control character");
        }
        if part.contains(':') {
            return refuse("has a ':' (a drive or a hidden stream)");
        }
        if part.ends_with('.') || part.ends_with(' ') {
            return refuse("has a part ending in a dot or a space");
        }
        let stem = part.split('.').next().unwrap_or(part).trim_end();
        if DEVICES.iter().any(|d| d.eq_ignore_ascii_case(stem)) {
            return refuse("is a device's name (CON, NUL, COM1 …)");
        }
        parts.push(part.to_string());
    }
    if parts.is_empty() {
        return refuse("has no name");
    }
    Ok(parts)
}

#[cfg(test)]
mod tests {
    use super::safe_path;

    #[test]
    fn safe_names_pass() {
        assert_eq!(safe_path("games/manic.z80").unwrap(), ["games", "manic.z80"]);
        assert_eq!(safe_path("./a//b/./c.txt").unwrap(), ["a", "b", "c.txt"]);
        assert_eq!(safe_path("dir\\sub\\f.bin").unwrap(), ["dir", "sub", "f.bin"]);
        assert_eq!(safe_path("folder/").unwrap(), ["folder"]);
        assert_eq!(safe_path("café ☕/naïve.txt").unwrap(), ["café ☕", "naïve.txt"]);
        assert_eq!(safe_path("a..b/..c").unwrap(), ["a..b", "..c"]);
        assert_eq!(safe_path("console.txt").unwrap(), ["console.txt"]);
    }

    #[test]
    fn escapes_are_refused() {
        for bad in [
            "../evil.txt",
            "a/../../evil.txt",
            "a/..",
            "/etc/passwd",
            "\\\\server\\share\\x",
            "\\windows\\x",
            "C:\\Windows\\x.dll",
            "c:evil",
            "a/b:stream",
            "nul.txt",
            "dir/CON",
            "com1.log",
            "trail.",
            "trail /x",
            "tab\there",
            "nul\0byte",
            "",
            ".",
            "./",
            "..\\..\\x",
        ] {
            assert!(safe_path(bad).is_err(), "{bad:?} should be refused");
        }
        let e = safe_path("../../etc/passwd").unwrap_err();
        assert!(e.contains("goes up a folder") && e.contains("outside the folder"), "{e}");
    }
}
