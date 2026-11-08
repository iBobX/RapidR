//! `runners-manifest.json`: the list of runners one RapidR version has,
//! with each file's size and SHA-256. The release signs it with minisign
//! (`runners-manifest.json.minisig`), so everything in it, and so every
//! runner it lists, is as good as the signature.
//!
//! ```json
//! {
//!   "format": 1,
//!   "version": "2.123.1",
//!   "runners": [
//!     { "target": "windows-x86_64", "file": "windows-x86_64.zst",
//!       "size": 11234567, "sha256": "…64 hex…",
//!       "unpacked": 31234567, "files": ["THIRD-PARTY-NOTICES.txt", "rapidrintr-runner.exe", "rapidrintr-runnerw.exe"] }
//!   ]
//! }
//! ```
//!
//! `file` is a `zstd`-compressed `tar` (the bundle: bundle.rs); `size` and
//! `sha256` are of that file as served; `unpacked` is the tar's size (the
//! limit unpacking holds to) and `files` the names in it.
//!
//! Parsing is strict about what it uses and ignores the rest, so a later
//! RapidR can add fields. Parse it only after its signature was checked
//! (verify.rs does both, in that order).

use serde_json::Value;

/// The manifest's `format`.
pub const FORMAT: u64 = 1;

/// The most one runner file may be (compressed): a list that says more is
/// refused before anything is downloaded.
pub const MAX_FILE_SIZE: u64 = 300 << 20;

/// The most a bundle may unpack to.
pub const MAX_UNPACKED: u64 = 600 << 20;

/// A list of runners.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Manifest {
    /// The RapidR version these runners are for.
    pub version: String,
    pub runners: Vec<Entry>,
}

/// One runner in the list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Entry {
    pub target: String,
    /// The file's name next to the manifest (`windows-x86_64.zst`).
    pub file: String,
    /// Its size, in bytes, as served.
    pub size: u64,
    /// Its SHA-256 (lowercase hex).
    pub sha256: String,
    /// What it unpacks to, in bytes.
    pub unpacked: u64,
    /// The files in the bundle (relative names, `/` between folders).
    pub files: Vec<String>,
}

impl Manifest {
    pub fn parse(bytes: &[u8]) -> Result<Manifest, String> {
        let json: Value = serde_json::from_slice(bytes).map_err(|e| format!("not JSON ({e})"))?;
        let object = json.as_object().ok_or("not a JSON object")?;
        match object.get("format").and_then(Value::as_u64) {
            Some(FORMAT) => {}
            Some(other) => return Err(format!("format {other}, which this RapidR does not read (it reads {FORMAT})")),
            None => return Err("no \"format\"".into()),
        }
        let version = object.get("version").and_then(Value::as_str).ok_or("no \"version\"")?;
        if version.is_empty() || version.len() > 40 || !version.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '+')) {
            return Err(format!("\"version\" {version:?} is not a version"));
        }
        let list = object.get("runners").and_then(Value::as_array).ok_or("no \"runners\"")?;
        let mut runners: Vec<Entry> = Vec::new();
        for item in list {
            let entry = Entry::parse(item)?;
            if runners.iter().any(|e| e.target == entry.target) {
                return Err(format!("{} is listed twice", entry.target));
            }
            runners.push(entry);
        }
        Ok(Manifest { version: version.to_string(), runners })
    }

    /// The entry for `target` (a canonical name).
    pub fn entry(&self, target: &str) -> Option<&Entry> {
        self.runners.iter().find(|e| e.target == target)
    }
}

impl Entry {
    fn parse(item: &Value) -> Result<Entry, String> {
        let o = item.as_object().ok_or("a runner is not an object")?;
        let text = |key: &str| o.get(key).and_then(Value::as_str).ok_or_else(|| format!("a runner has no \"{key}\""));
        let number = |key: &str| o.get(key).and_then(Value::as_u64).ok_or_else(|| format!("a runner has no \"{key}\" number"));
        let target = text("target")?;
        let file = text("file")?;
        if !is_plain_file_name(file) {
            return Err(format!("{target}: \"file\" {file:?} is not a plain file name"));
        }
        let sha256 = text("sha256")?;
        if sha256.len() != 64 || !sha256.bytes().all(|b| matches!(b, b'0'..=b'9' | b'a'..=b'f')) {
            return Err(format!("{target}: \"sha256\" is not 64 lowercase hex digits"));
        }
        let size = number("size")?;
        if size == 0 || size > MAX_FILE_SIZE {
            return Err(format!("{target}: \"size\" {size} is outside what a runner may be"));
        }
        let unpacked = number("unpacked")?;
        if unpacked == 0 || unpacked > MAX_UNPACKED {
            return Err(format!("{target}: \"unpacked\" {unpacked} is outside what a runner may be"));
        }
        let files = o.get("files").and_then(Value::as_array).ok_or_else(|| format!("{target}: no \"files\""))?;
        let mut names = Vec::new();
        for f in files {
            let name = f.as_str().ok_or_else(|| format!("{target}: a file name is not text"))?;
            let parts = rapidr_archive::safe_path(name).map_err(|e| format!("{target}: {e}"))?;
            if parts.join("/") != name {
                return Err(format!("{target}: file name {name:?} is not in its plain form"));
            }
            names.push(name.to_string());
        }
        if names.is_empty() {
            return Err(format!("{target}: \"files\" is empty"));
        }
        Ok(Entry { target: target.to_string(), file: file.to_string(), size, sha256: sha256.to_string(), unpacked, files: names })
    }
}

/// A name with no folder in it, nothing hidden, nothing a shell or a URL
/// would read differently: letters, digits, `.`, `-`, `_`; ends in `.zst`.
fn is_plain_file_name(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 100
        && !name.starts_with('.')
        && name.ends_with(".zst")
        && name.chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | '_'))
        && !name.contains("..")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample(extra: &str) -> String {
        format!(
            r#"{{"format":1,"version":"2.123.1","runners":[{{"target":"linux-x86_64","file":"linux-x86_64.zst","size":1000,"sha256":"{}","unpacked":3000,"files":["THIRD-PARTY-NOTICES.txt","rapidrintr-runner"]{extra}}}]}}"#,
            "ab".repeat(32)
        )
    }

    #[test]
    fn a_good_list_parses() {
        let m = Manifest::parse(sample("").as_bytes()).unwrap();
        assert_eq!(m.version, "2.123.1");
        let e = m.entry("linux-x86_64").unwrap();
        assert_eq!((e.file.as_str(), e.size, e.unpacked), ("linux-x86_64.zst", 1000, 3000));
        assert_eq!(e.files, ["THIRD-PARTY-NOTICES.txt", "rapidrintr-runner"]);
        assert!(m.entry("web").is_none());
    }

    #[test]
    fn fields_a_later_version_adds_are_ignored() {
        assert!(Manifest::parse(sample(r#","built":"2026-11-07","licence":"MIT""#).as_bytes()).is_ok());
    }

    #[test]
    fn bad_lists_are_refused_with_a_reason() {
        let good = sample("");
        for (broken, why) in [
            (good.replace("\"format\":1", "\"format\":2"), "format 2"),
            (good.replace("\"format\":1,", ""), "no \"format\""),
            (good.replace("2.123.1", "2.123 1"), "not a version"),
            (good.replace("linux-x86_64.zst", "../linux-x86_64.zst"), "plain file name"),
            (good.replace("linux-x86_64.zst", "linux-x86_64.exe"), "plain file name"),
            (good.replace(&"ab".repeat(32), &"AB".repeat(32)), "lowercase hex"),
            (good.replace("\"size\":1000", "\"size\":0"), "\"size\" 0"),
            (good.replace("\"size\":1000", "\"size\":9999999999"), "outside"),
            (good.replace("\"unpacked\":3000", "\"unpacked\":99999999999"), "outside"),
            (good.replace("\"rapidrintr-runner\"", "\"../rapidrintr-runner\""), "outside the folder"),
            (good.replace("\"rapidrintr-runner\"", "\"a//b\""), "plain form"),
            ("[]".to_string(), "not a JSON object"),
            ("{".to_string(), "not JSON"),
        ] {
            let err = Manifest::parse(broken.as_bytes()).unwrap_err();
            assert!(err.contains(why), "{err:?} should say {why:?}");
        }
    }

    #[test]
    fn a_target_listed_twice_is_refused() {
        let one = sample("");
        let entry = one.split_once("\"runners\":[").unwrap().1.strip_suffix("]}").unwrap();
        let twice = one.replace(entry, &format!("{entry},{entry}"));
        assert!(Manifest::parse(twice.as_bytes()).unwrap_err().contains("twice"));
    }
}
