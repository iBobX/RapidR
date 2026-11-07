//! Where on the host a version's runners are: pure text, so the web Studio
//! builds the same addresses for the browser's fetch.
//!
//! ```text
//! <base>/<version>/runners-manifest.json
//! <base>/<version>/runners-manifest.json.minisig
//! <base>/<version>/<target>.zst
//! ```

/// The list of runners.
pub const MANIFEST_NAME: &str = "runners-manifest.json";

/// Its minisign signature.
pub const SIGNATURE_NAME: &str = "runners-manifest.json.minisig";

/// Where runners are downloaded from unless a setting or `RAPIDR_RUNNERS_URL`
/// says otherwise: RapidR's own host (decided 2026-11-07: a bucket behind a
/// CDN, apart from the company's web server), a folder per version below it
/// (`https://studio.rapidr.dev/runners/<version>/`). The web Studio is served
/// from the same host, so its `fetch` is same-origin and needs no CORS
/// headers. The one place the address is written.
pub const DEFAULT_BASE_URL: &str = "https://studio.rapidr.dev/runners";

pub fn manifest_url(base: &str, version: &str) -> String {
    format!("{}/{version}/{MANIFEST_NAME}", base.trim_end_matches('/'))
}

pub fn signature_url(base: &str, version: &str) -> String {
    format!("{}/{version}/{SIGNATURE_NAME}", base.trim_end_matches('/'))
}

pub fn file_url(base: &str, version: &str, file: &str) -> String {
    format!("{}/{version}/{file}", base.trim_end_matches('/'))
}

/// `base` as an address runners may come from: `https://…`, or `http://`
/// only for this computer (`localhost`, `127.0.0.1`, `[::1]`: a test server).
/// Anything else is why not. Gives it without a trailing `/`.
///
/// ```
/// use rapidr_runners::urls::check_base_url;
/// assert_eq!(check_base_url(" https://studio.rapidr.dev/runners/ ").unwrap(), "https://studio.rapidr.dev/runners");
/// assert!(check_base_url("http://127.0.0.1:8080/runners").is_ok());
/// assert!(check_base_url("http://example.com/runners").is_err());
/// assert!(check_base_url("ftp://example.com").is_err());
/// ```
pub fn check_base_url(base: &str) -> Result<String, String> {
    let base = base.trim();
    if base.is_empty() || base.chars().any(|c| c.is_whitespace() || c.is_control()) {
        return Err("the address is empty or has a space in it".into());
    }
    if base.contains('?') || base.contains('#') {
        return Err("the address must not have a ? or # in it".into());
    }
    let (secure, rest) = if let Some(rest) = base.strip_prefix("https://") {
        (true, rest)
    } else if let Some(rest) = base.strip_prefix("http://") {
        (false, rest)
    } else {
        return Err("the address must start with https://".into());
    };
    let host = rest.split('/').next().unwrap_or("");
    let host_name = if let Some(v6) = host.strip_prefix('[') { v6.split(']').next().unwrap_or("") } else { host.split(':').next().unwrap_or("") };
    if host_name.is_empty() || host.contains('@') {
        return Err("the address has no host name (or has a user name in it)".into());
    }
    if !secure && !matches!(host_name, "localhost" | "127.0.0.1" | "::1") {
        return Err("only https:// addresses are used (http:// is for this computer's own test server)".into());
    }
    Ok(base.trim_end_matches('/').to_string())
}

/// The host name in `base`, for messages ("studio.rapidr.dev").
pub fn host_of(base: &str) -> String {
    let rest = base.split_once("://").map(|(_, r)| r).unwrap_or(base);
    rest.split('/').next().unwrap_or(rest).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addresses() {
        assert_eq!(manifest_url("https://h/runners/", "2.123.1"), "https://h/runners/2.123.1/runners-manifest.json");
        assert_eq!(signature_url("https://h/runners", "2.123.1"), "https://h/runners/2.123.1/runners-manifest.json.minisig");
        assert_eq!(file_url("https://h", "2.123.1", "web.zst"), "https://h/2.123.1/web.zst");
    }

    #[test]
    fn only_https_or_this_computer() {
        for ok in ["https://studio.rapidr.dev/runners", "http://localhost:9/x", "http://[::1]:80/x", "http://127.0.0.1"] {
            assert!(check_base_url(ok).is_ok(), "{ok}");
        }
        for bad in ["", "studio.rapidr.dev", "http://studio.rapidr.dev/runners", "https://", "https://user@h/x", "https://h/x?a=b", "https://h/ x", "file:///tmp", "http://localhost.evil.com/x"] {
            assert!(check_base_url(bad).is_err(), "{bad}");
        }
    }

    #[test]
    fn the_built_in_address_is_a_good_one_and_gives_the_documented_urls() {
        assert_eq!(check_base_url(DEFAULT_BASE_URL).unwrap(), DEFAULT_BASE_URL);
        assert_eq!(manifest_url(DEFAULT_BASE_URL, "2.123.1"), "https://studio.rapidr.dev/runners/2.123.1/runners-manifest.json");
        assert_eq!(file_url(DEFAULT_BASE_URL, "2.123.1", "windows-x86_64.zst"), "https://studio.rapidr.dev/runners/2.123.1/windows-x86_64.zst");
        assert_eq!(host_of(DEFAULT_BASE_URL), "studio.rapidr.dev");
    }

    #[test]
    fn the_host_for_messages() {
        assert_eq!(host_of("https://studio.rapidr.dev/runners"), "studio.rapidr.dev");
        assert_eq!(host_of("http://127.0.0.1:8080"), "127.0.0.1:8080");
    }
}
