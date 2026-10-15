//! The audit log (docs/ai-architecture.md §5.7): what an assistant asked a
//! provider and did — each request, each tool call, each change it
//! proposed and what the user decided — as JSON lines, local only: in the
//! user's own settings folder on the desktop (never in the project, so it
//! isn't committed or shipped), in the browser's storage for the page on
//! the web. Never keys, never file contents: what a tool touched and a
//! digest of its arguments.
//!
//! The desktop's file: `RAPIDR_AI_AUDIT` when set; else `ai-audit.jsonl`
//! beside the settings store (`RAPIDR_REGISTRY`'s folder, else RapidR's
//! folder of the user's settings). The last 10,000 lines are kept.

/// The lines kept.
pub const KEEP: usize = 10_000;

/// Now, in UTC, as ISO 8601 (`2026-10-15T14:03:09Z`).
pub fn now_iso() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        let s: String = js_sys::Date::new_0().to_iso_string().into();
        // (without the milliseconds, as the desktop writes it)
        match s.split_once('.') {
            Some((head, _)) => format!("{head}Z"),
            None => s,
        }
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let secs = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_secs()) as i64;
        iso_of(secs)
    }
}

/// Unix seconds as an ISO 8601 UTC time (Howard Hinnant's civil-from-days).
pub fn iso_of(secs: i64) -> String {
    let days = secs.div_euclid(86_400);
    let rem = secs.rem_euclid(86_400);
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let doe = z - era * 146_097;
    let yoe = (doe - doe / 1460 + doe / 36_524 - doe / 146_096) / 365;
    let doy = doe - (365 * yoe + yoe / 4 - yoe / 100);
    let mp = (5 * doy + 2) / 153;
    let d = doy - (153 * mp + 2) / 5 + 1;
    let m = if mp < 10 { mp + 3 } else { mp - 9 };
    let y = yoe + era * 400 + i64::from(m <= 2);
    format!("{y:04}-{m:02}-{d:02}T{:02}:{:02}:{:02}Z", rem / 3600, rem / 60 % 60, rem % 60)
}

/// Where the log is, in words (the settings show it).
pub fn location() -> String {
    #[cfg(target_arch = "wasm32")]
    {
        "this browser's storage for the page".into()
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        path().map_or_else(|| "nowhere (no settings folder)".into(), |p| p.display().to_string())
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn path() -> Option<std::path::PathBuf> {
    use std::path::PathBuf;
    if let Some(p) = std::env::var_os("RAPIDR_AI_AUDIT").filter(|p| !p.is_empty()) {
        return Some(PathBuf::from(p));
    }
    if let Some(r) = std::env::var_os("RAPIDR_REGISTRY").filter(|p| !p.is_empty()) {
        let r = PathBuf::from(r);
        return Some(r.parent().map(|d| d.join("ai-audit.jsonl")).unwrap_or_else(|| PathBuf::from("ai-audit.jsonl")));
    }
    let dir = if cfg!(windows) {
        PathBuf::from(std::env::var_os("APPDATA")?).join("RapidR")
    } else if cfg!(target_os = "macos") {
        PathBuf::from(std::env::var_os("HOME")?).join("Library").join("Application Support").join("RapidR")
    } else {
        let home = std::env::var_os("HOME").map(PathBuf::from);
        std::env::var_os("XDG_CONFIG_HOME").filter(|p| !p.is_empty()).map(PathBuf::from).or_else(|| home.map(|h| h.join(".config")))?.join("rapidr")
    };
    Some(dir.join("ai-audit.jsonl"))
}

/// Adds a line (a JSON object), the oldest dropped past [`KEEP`].
pub fn append(line: &str) {
    let line = line.replace(['\n', '\r'], " ");
    #[cfg(not(target_arch = "wasm32"))]
    {
        use std::io::Write;
        let Some(p) = path() else { return };
        if let Some(dir) = p.parent() {
            let _ = std::fs::create_dir_all(dir);
        }
        // (trimmed now and then: when it is a tenth over)
        if std::fs::metadata(&p).is_ok_and(|m| m.len() > 4_000_000) {
            if let Ok(text) = std::fs::read_to_string(&p) {
                let lines: Vec<&str> = text.lines().collect();
                if lines.len() > KEEP {
                    let _ = std::fs::write(&p, lines[lines.len() - KEEP..].join("\n") + "\n");
                }
            }
        }
        if let Ok(mut f) = std::fs::OpenOptions::new().create(true).append(true).open(&p) {
            let _ = writeln!(f, "{line}");
        }
    }
    #[cfg(target_arch = "wasm32")]
    {
        let Some(s) = web_sys::window().and_then(|w| w.local_storage().ok().flatten()) else { return };
        const ITEM: &str = "dev.rapidr.ai.audit";
        let mut text = s.get_item(ITEM).ok().flatten().unwrap_or_default();
        text.push_str(&line);
        text.push('\n');
        // (the page's storage is small: its last 500 lines)
        let lines: Vec<&str> = text.lines().collect();
        if lines.len() > 500 {
            text = lines[lines.len() - 500..].join("\n") + "\n";
        }
        let _ = s.set_item(ITEM, &text);
    }
}

/// The log's last `n` lines, oldest first.
pub fn last(n: usize) -> Vec<String> {
    #[cfg(not(target_arch = "wasm32"))]
    let text = path().and_then(|p| std::fs::read_to_string(p).ok()).unwrap_or_default();
    #[cfg(target_arch = "wasm32")]
    let text = web_sys::window().and_then(|w| w.local_storage().ok().flatten()).and_then(|s| s.get_item("dev.rapidr.ai.audit").ok().flatten()).unwrap_or_default();
    let lines: Vec<String> = text.lines().map(str::to_string).collect();
    lines[lines.len().saturating_sub(n)..].to_vec()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn times() {
        assert_eq!(iso_of(0), "1970-01-01T00:00:00Z");
        assert_eq!(iso_of(1_792_090_712), "2026-10-15T18:58:32Z");
        assert_eq!(iso_of(951_825_600), "2000-02-29T12:00:00Z");
        assert!(now_iso().starts_with("20"));
    }
}
