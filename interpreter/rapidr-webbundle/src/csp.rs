//! The Content-Security-Policy every page a web build makes carries
//! (docs/security-audit.md SEC-15), derived from what the program uses: a
//! program that never runs JavaScript, shows no web page and talks to no
//! server gets a page that can't either, whatever data it shows through
//! RDOM's InnerHTML or an attribute. What the policy allows, by component:
//!
//! | The program uses | The policy adds |
//! |---|---|
//! | always | the page's own files (`'self'`), WebAssembly, inline styles, images / media / fonts from `data:` and `blob:` |
//! | RJAVASCRIPT | `script-src 'unsafe-eval'` (Eval runs the program's JavaScript) |
//! | RWEBVIEW | `frame-src 'self' https:` (its Url; its Html runs in rapidr-webview.html, a sandboxed frame of the page's own) |
//! | RHTTP / RSOCKET / RDOWNLOAD (Q or R) | `connect-src https: wss:` |
//! | RDOM / RWEBVIEW | `img-src https:` (a page's markup shows pictures from the web) |
//! | RWEBAUDIO / RWEBVIDEO / RVIDEO | `media-src https:` |
//! | an `http://` / `ws://` … URL written in the program | that origin, in `connect-src` (and `frame-src` / `media-src` / `img-src` when those are open) |
//!
//! Anything else is the author's opt-in: `rapidr build --web --csp
//! "connect-src https://api.example.com; script-src https://cdn.example.com"`
//! adds those sources (docs/security-audit.md, "What a web program can opt
//! into"); and the page's `index.html` is theirs to edit after a build.

use std::collections::{BTreeMap, BTreeSet};

/// What a program uses that its page's policy must allow.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct WebNeeds {
    /// RJAVASCRIPT: `Eval` (JavaScript built from strings).
    pub eval: bool,
    /// RWEBVIEW: frames (its Url, and the RWEBVIEW frame for its Html).
    pub frames: bool,
    /// RHTTP / RSOCKET / RDOWNLOAD: servers other than the page's.
    pub network: bool,
    /// RDOM / RWEBVIEW: markup that may show pictures from the web.
    pub markup: bool,
    /// RWEBAUDIO / RWEBVIDEO / RVIDEO: sound and video from the web.
    pub media: bool,
    /// Origins written in the program (`http://host:port`, `wss://host`).
    pub origins: BTreeSet<String>,
    /// The author's own additions, by directive (`--csp`).
    pub extra: BTreeMap<String, Vec<String>>,
}

/// A component type's name without RapidQ's `Q` or RapidR's `R`.
fn base(ty: &str) -> &str {
    ty.strip_prefix(['R', 'Q']).unwrap_or(ty)
}

impl WebNeeds {
    /// What `source` (a program, its `$INCLUDE`s expanded) uses: the
    /// component types it names and the URLs it spells out. Comments are
    /// skipped; a type named anywhere else counts (a policy a little wider
    /// than needed beats a program that silently can't work).
    pub fn scan(source: &str) -> WebNeeds {
        let mut needs = WebNeeds::default();
        for line in source.lines() {
            let mut chars = line.char_indices().peekable();
            while let Some((i, c)) = chars.next() {
                if c == '\'' {
                    break; // (a comment)
                }
                if c == '"' {
                    let start = i + 1;
                    let mut end = line.len();
                    for (j, d) in chars.by_ref() {
                        if d == '"' {
                            end = j;
                            break;
                        }
                    }
                    needs.literal(&line[start..end]);
                    continue;
                }
                if c.is_ascii_alphabetic() || c == '_' {
                    let mut end = line.len();
                    while let Some(&(j, d)) = chars.peek() {
                        if d.is_ascii_alphanumeric() || d == '_' || d == '$' {
                            chars.next();
                        } else {
                            end = j;
                            break;
                        }
                    }
                    let word = line[i..end].to_ascii_uppercase();
                    if word == "REM" {
                        break;
                    }
                    needs.word(&word);
                }
            }
        }
        needs
    }

    fn word(&mut self, word: &str) {
        if !(word.starts_with('R') || word.starts_with('Q')) {
            return;
        }
        match base(word) {
            "JAVASCRIPT" => self.eval = true,
            "WEBVIEW" => {
                self.frames = true;
                self.markup = true;
            }
            "DOM" => self.markup = true,
            "HTTP" | "SOCKET" | "DOWNLOAD" => self.network = true,
            "WEBAUDIO" | "WEBVIDEO" | "VIDEO" => self.media = true,
            _ => {}
        }
    }

    /// The origins a string spells out (`"http://localhost:8080/api"`).
    fn literal(&mut self, s: &str) {
        let lower = s.to_ascii_lowercase();
        for scheme in ["https://", "http://", "wss://", "ws://"] {
            let mut from = 0;
            while let Some(at) = lower[from..].find(scheme) {
                let start = from + at;
                let rest = &s[start + scheme.len()..];
                let host_end = rest.find(|c: char| !(c.is_ascii_alphanumeric() || matches!(c, '.' | '-' | ':' | '[' | ']'))).unwrap_or(rest.len());
                let host = &rest[..host_end];
                if !host.is_empty() && host.len() <= 253 {
                    self.origins.insert(format!("{}{}", &lower[start..start + scheme.len()], host.to_ascii_lowercase()));
                }
                from = start + scheme.len();
            }
        }
    }

    /// The author's opt-in (`--csp "connect-src https://api.example.com;
    /// frame-src https://www.youtube.com"`): sources added to directives.
    /// A directive or a source that isn't one is an error (said, never
    /// silently dropped).
    pub fn add_extra(&mut self, spec: &str) -> Result<(), String> {
        for part in spec.split(';').map(str::trim).filter(|p| !p.is_empty()) {
            let mut words = part.split_ascii_whitespace();
            let directive = words.next().unwrap_or_default().to_ascii_lowercase();
            if !DIRECTIVES.contains(&directive.as_str()) {
                return Err(format!("--csp: '{directive}' isn't a directive this page's policy has ({})", DIRECTIVES.join(", ")));
            }
            for source in words {
                if !valid_source(source) {
                    return Err(format!("--csp: '{source}' isn't a source a policy takes"));
                }
                self.extra.entry(directive.clone()).or_default().push(source.to_string());
            }
        }
        Ok(())
    }
}

/// The directives the generated policy has (and `--csp` may add to).
pub const DIRECTIVES: &[&str] = &[
    "default-src", "script-src", "style-src", "img-src", "font-src", "media-src", "connect-src", "frame-src", "worker-src", "manifest-src", "object-src", "base-uri", "form-action",
];

/// A CSP source expression: a keyword in quotes, a scheme, or a host —
/// no `;`, `,`, quotes elsewhere or control characters (which could start
/// another directive or break the `content` attribute).
fn valid_source(s: &str) -> bool {
    if s.starts_with('\'') {
        return s.len() > 2 && s.ends_with('\'') && s[1..s.len() - 1].chars().all(|c| c.is_ascii_alphanumeric() || matches!(c, '-' | '+' | '/' | '='));
    }
    !s.is_empty() && s.chars().all(|c| c.is_ascii_graphic() && !matches!(c, ';' | ',' | '\'' | '"' | '<' | '>' | '\\'))
}

/// The policy for a program's page.
pub fn content_security_policy(needs: &WebNeeds) -> String {
    let http: Vec<String> = needs.origins.iter().filter(|o| o.starts_with("http")).cloned().collect();
    let all: Vec<String> = needs.origins.iter().cloned().collect();
    let mut d: Vec<(&str, Vec<String>)> = Vec::new();
    let s = |v: &[&str]| v.iter().map(|x| x.to_string()).collect::<Vec<_>>();
    d.push(("default-src", s(&["'self'"])));
    let mut script = s(&["'self'", "'wasm-unsafe-eval'"]);
    if needs.eval {
        script.push("'unsafe-eval'".into());
    }
    d.push(("script-src", script));
    // (the kernel host and RDOM.CssStyle set inline styles; a style can't
    // run code)
    d.push(("style-src", s(&["'self'", "'unsafe-inline'"])));
    let mut img = s(&["'self'", "data:", "blob:"]);
    if needs.markup {
        img.push("https:".into());
        img.extend(http.iter().cloned());
    }
    d.push(("img-src", img));
    d.push(("font-src", s(&["'self'", "data:", "blob:"])));
    let mut media = s(&["'self'", "data:", "blob:"]);
    if needs.media {
        media.push("https:".into());
        media.extend(http.iter().cloned());
    }
    d.push(("media-src", media));
    let mut connect = s(&["'self'", "data:", "blob:"]);
    if needs.network {
        connect.extend(s(&["https:", "wss:"]));
    }
    connect.extend(all.iter().cloned());
    d.push(("connect-src", connect));
    d.push(("frame-src", if needs.frames {
        let mut f = s(&["'self'", "https:"]);
        f.extend(http.iter().cloned());
        f
    } else {
        s(&["'none'"])
    }));
    d.push(("worker-src", s(&["'self'", "blob:"])));
    d.push(("manifest-src", s(&["'self'"])));
    d.push(("object-src", s(&["'none'"])));
    d.push(("base-uri", s(&["'none'"])));
    d.push(("form-action", s(&["'self'"])));
    for (directive, sources) in &needs.extra {
        if let Some((_, list)) = d.iter_mut().find(|(name, _)| name == directive) {
            list.retain(|x| x != "'none'");
            for src in sources {
                if !list.contains(src) {
                    list.push(src.clone());
                }
            }
        }
    }
    d.iter()
        .map(|(name, list)| {
            let mut seen = BTreeSet::new();
            let list: Vec<&String> = list.iter().filter(|x| seen.insert(x.as_str())).collect();
            format!("{name} {}", list.iter().map(|x| x.as_str()).collect::<Vec<_>>().join(" "))
        })
        .collect::<Vec<_>>()
        .join("; ")
}

/// The policy as an HTTP header for a host that sets them (`_headers`,
/// `.htaccess`): the page's, plus what only a header can say — who may put
/// the page in a frame.
pub fn header_policy(needs: &WebNeeds) -> String {
    format!("{}; frame-ancestors 'self'", content_security_policy(needs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_plain_program_gets_the_strict_policy() {
        let needs = WebNeeds::scan("CREATE F AS QFORM\n  Caption = \"hi\"\nEND CREATE\nF.ShowModal\n");
        let p = content_security_policy(&needs);
        assert!(p.contains("script-src 'self' 'wasm-unsafe-eval';"), "{p}");
        assert!(!p.contains("'unsafe-eval'"), "{p}");
        assert!(!p.contains("script-src 'self' 'wasm-unsafe-eval' 'unsafe-inline'"), "{p}");
        assert!(p.contains("frame-src 'none'"), "{p}");
        assert!(p.contains("object-src 'none'") && p.contains("base-uri 'none'"), "{p}");
        assert!(p.contains("connect-src 'self' data: blob:;"), "{p}");
    }

    #[test]
    fn components_open_what_they_need() {
        let n = WebNeeds::scan("DIM J AS RJavaScript\nCREATE W AS RWEBVIEW\nEND CREATE\nDIM H AS QHTTP\n");
        assert!(n.eval && n.frames && n.network && n.markup && !n.media);
        let p = content_security_policy(&n);
        assert!(p.contains("'unsafe-eval'") && p.contains("frame-src 'self' https:") && p.contains("wss:"), "{p}");
    }

    #[test]
    fn comments_and_strings_are_not_components() {
        let n = WebNeeds::scan("' DIM J AS RJAVASCRIPT\nREM RWEBVIEW\nPRINT \"RJAVASCRIPT\"\n");
        assert!(!n.eval && !n.frames, "{n:?}");
    }

    #[test]
    fn literal_origins_are_allowed_to_connect() {
        let n = WebNeeds::scan("H.Get(\"http://localhost:8080/api?x=1\")\nS.Host = \"wss://Chat.Example.com/socket\"\n");
        assert!(n.origins.contains("http://localhost:8080") && n.origins.contains("wss://chat.example.com"), "{n:?}");
        let p = content_security_policy(&n);
        assert!(p.contains("connect-src 'self' data: blob: http://localhost:8080 wss://chat.example.com"), "{p}");
    }

    #[test]
    fn the_authors_additions() {
        let mut n = WebNeeds::default();
        n.add_extra("connect-src https://api.example.com; frame-src https://www.youtube.com").unwrap();
        let p = content_security_policy(&n);
        assert!(p.contains("connect-src 'self' data: blob: https://api.example.com"), "{p}");
        assert!(p.contains("frame-src https://www.youtube.com"), "{p}");
        assert!(n.clone().add_extra("sandbox allow-scripts").is_err());
        assert!(n.clone().add_extra("script-src https://x\"><script>").is_err());
        assert!(n.clone().add_extra("script-src 'unsafe-inline'").is_ok());
    }
}
