//! What an app says about itself: its name, identifier, version and maker —
//! macOS' Info.plist, Windows' version information, Linux' `.desktop` entry.

/// An app's details.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppInfo {
    /// The name people see (the `.app`'s, the Dock's, Explorer's).
    pub name: String,
    /// The executable's file name, without `.exe`.
    pub exe: String,
    /// Reverse-DNS identifier (macOS' bundle identifier, the Linux desktop
    /// entry's name): `dev.rapidr.app.<name>` unless set.
    pub bundle_id: String,
    /// Up to four numbers separated by dots (`1.0`, `2.3.1`).
    pub version: String,
    /// Who makes it (Windows' CompanyName; macOS' copyright line).
    pub company: String,
    /// A copyright line; empty: "© <company>" when there's a company.
    pub copyright: String,
    /// One line about it (Windows' FileDescription, Linux' Comment).
    pub description: String,
    /// A console program (Linux: `Terminal=true`).
    pub console: bool,
}

/// The identifier an app gets unless it says: `dev.rapidr.app.` and its
/// name in lower-case letters, digits and hyphens.
pub fn default_bundle_id(name: &str) -> String {
    let mut part: String = name
        .chars()
        .map(|c| if c.is_ascii_alphanumeric() { c.to_ascii_lowercase() } else { '-' })
        .collect();
    while part.contains("--") {
        part = part.replace("--", "-");
    }
    let part = part.trim_matches('-');
    format!("dev.rapidr.app.{}", if part.is_empty() { "program" } else { part })
}

/// The executable's file name for a project called `name` (the `.rrproj`'s
/// `name`), the same on every system: its letters, digits, `-`, `_` and `.`;
/// a run of anything else — spaces, punctuation, what no file system allows
/// (`/ \ : * ? " < > |`) — becomes one `-`, and none stays at the ends. A
/// Windows device name (`con`, `nul`, `com1` …) gets `-app`, which Windows
/// would otherwise take for a device; at most 64 characters. `None` when
/// nothing usable is left (the build then names it after the main file).
pub fn exe_name_for(name: &str) -> Option<String> {
    let mut out = String::new();
    let mut pending = false;
    for c in name.trim().chars() {
        if c.is_alphanumeric() || matches!(c, '-' | '_' | '.') {
            if pending && !out.is_empty() {
                out.push('-');
            }
            pending = false;
            out.push(c);
        } else {
            pending = true;
        }
    }
    // (no leading or trailing dots or dashes: a hidden file, a name Windows trims)
    let trimmed = out.trim_matches(|c| c == '.' || c == '-');
    let mut out: String = trimmed.chars().take(64).collect();
    out = out.trim_end_matches(|c| c == '.' || c == '-').to_string();
    if out.is_empty() {
        return None;
    }
    let stem = out.split('.').next().unwrap_or("").to_ascii_lowercase();
    let device = matches!(stem.as_str(), "con" | "prn" | "aux" | "nul") || (stem.len() == 4 && (stem.starts_with("com") || stem.starts_with("lpt")) && stem.as_bytes()[3].is_ascii_digit() && stem.as_bytes()[3] != b'0');
    if device {
        out.push_str("-app");
    }
    Some(out)
}

impl AppInfo {
    /// `name`'s app whose executable is `exe`, version 1.0, everything else
    /// the defaults.
    pub fn new(name: &str, exe: &str) -> AppInfo {
        AppInfo {
            name: name.to_string(),
            exe: exe.to_string(),
            bundle_id: default_bundle_id(name),
            version: "1.0".into(),
            company: String::new(),
            copyright: String::new(),
            description: String::new(),
            console: false,
        }
    }

    /// The version's numbers, four of them (missing ones 0).
    pub fn version_numbers(&self) -> Result<[u16; 4], String> {
        let bad = || format!("version \"{}\": up to four numbers (0 to 65535) separated by dots, like 1.0 or 2.3.1", self.version);
        let parts: Vec<&str> = self.version.trim().split('.').collect();
        if parts.is_empty() || parts.len() > 4 {
            return Err(bad());
        }
        let mut out = [0u16; 4];
        for (i, p) in parts.iter().enumerate() {
            if p.is_empty() || !p.bytes().all(|b| b.is_ascii_digit()) {
                return Err(bad());
            }
            out[i] = p.parse().map_err(|_| bad())?;
        }
        Ok(out)
    }

    /// The version as macOS has it: one to three numbers.
    pub fn mac_version(&self) -> Result<String, String> {
        let parts: Vec<&str> = self.version.trim().split('.').collect();
        self.version_numbers()?;
        Ok(parts[..parts.len().min(3)].join("."))
    }

    /// The copyright line, if any.
    pub fn copyright_line(&self) -> String {
        match (self.copyright.trim(), self.company.trim()) {
            ("", "") => String::new(),
            ("", company) => format!("© {company}"),
            (line, _) => line.to_string(),
        }
    }

    /// Everything an app can be made with: a name, an executable name that
    /// is a plain file name, an identifier of letters, digits, `-` and `.`,
    /// and a version of numbers.
    pub fn check(&self) -> Result<(), String> {
        if self.name.trim().is_empty() {
            return Err("the app needs a name".into());
        }
        if self.name.contains(['/', '\\', ':']) {
            return Err(format!("app name \"{}\": no / \\ or : in it (it names the app's folder)", self.name));
        }
        if self.exe.is_empty() || self.exe.contains(['/', '\\']) {
            return Err(format!("executable name \"{}\": a plain file name", self.exe));
        }
        let id = &self.bundle_id;
        let ok = !id.is_empty()
            && id.split('.').all(|p| !p.is_empty())
            && id.bytes().all(|b| b.is_ascii_alphanumeric() || b == b'-' || b == b'.');
        if !ok {
            return Err(format!("bundle ID \"{id}\": letters, digits, - and dots, like com.example.notepad"));
        }
        self.version_numbers().map(|_| ())
    }
}

/// `text` escaped for XML (Info.plist).
pub(crate) fn xml(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_projects_executable_name() {
        assert_eq!(exe_name_for("Notepad").as_deref(), Some("Notepad"));
        assert_eq!(exe_name_for("RapidR Database Studio").as_deref(), Some("RapidR-Database-Studio"));
        assert_eq!(exe_name_for("  My  Great App!  ").as_deref(), Some("My-Great-App"));
        assert_eq!(exe_name_for("a/b\\c:d*e?f\"g<h>i|j").as_deref(), Some("a-b-c-d-e-f-g-h-i-j"));
        assert_eq!(exe_name_for("Tool_v2.1").as_deref(), Some("Tool_v2.1"));
        assert_eq!(exe_name_for("Café Ñandú").as_deref(), Some("Café-Ñandú"));
        assert_eq!(exe_name_for("..hidden.").as_deref(), Some("hidden"));
        // (nothing usable: the main file names it)
        for none in ["", "   ", "***", "..", "-"] {
            assert_eq!(exe_name_for(none), None, "{none:?}");
        }
        // (a Windows device name, any case, with or without an extension)
        for device in ["con", "NUL", "Com1", "lpt9", "aux.txt"] {
            assert!(exe_name_for(device).unwrap().contains("-app"), "{device}");
        }
        assert_eq!(exe_name_for("com0").as_deref(), Some("com0"));
        assert_eq!(exe_name_for("console").as_deref(), Some("console"));
        // (long names are cut, never to a trailing dash)
        assert_eq!(exe_name_for(&"x".repeat(100)).unwrap().len(), 64);
        let cut = format!("{} y", "x".repeat(63));
        assert_eq!(exe_name_for(&cut).as_deref(), Some("x".repeat(63).as_str()));
        // (every result is a name an app accepts)
        for n in ["RapidR Database Studio", "a/b", "Café"] {
            assert!(AppInfo::new("n", &exe_name_for(n).unwrap()).check().is_ok());
        }
    }

    #[test]
    fn identifiers_versions_and_checks() {
        assert_eq!(default_bundle_id("Notepad"), "dev.rapidr.app.notepad");
        assert_eq!(default_bundle_id("My  Great App!"), "dev.rapidr.app.my-great-app");
        assert_eq!(default_bundle_id("日本"), "dev.rapidr.app.program");
        let mut a = AppInfo::new("Notepad", "notepad");
        assert_eq!(a.version_numbers().unwrap(), [1, 0, 0, 0]);
        assert!(a.check().is_ok());
        a.version = "2.3.1.7".into();
        assert_eq!(a.version_numbers().unwrap(), [2, 3, 1, 7]);
        assert_eq!(a.mac_version().unwrap(), "2.3.1");
        for bad in ["", "1.x", "1..2", "1.2.3.4.5", "70000", "v1"] {
            a.version = bad.into();
            assert!(a.check().unwrap_err().contains("version"), "{bad}");
        }
        a.version = "1.0".into();
        a.bundle_id = "com.example.my app".into();
        assert!(a.check().unwrap_err().contains("bundle ID"));
        a.bundle_id = "com..x".into();
        assert!(a.check().is_err());
        a.bundle_id = "com.example.notepad".into();
        a.name = "a/b".into();
        assert!(a.check().is_err());
        a.name = "Notepad".into();
        assert_eq!(a.copyright_line(), "");
        a.company = "Ruta".into();
        assert_eq!(a.copyright_line(), "© Ruta");
        assert_eq!(xml("a<b&\"c\""), "a&lt;b&amp;&quot;c&quot;");
    }
}
