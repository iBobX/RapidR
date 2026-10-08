// Security regressions for the web bundle (docs/security-audit.md SEC-15,
// SEC-16, SEC-17). Compiled into interpreter/rapidr-webbundle's unit tests
// (src/lib.rs, `mod security_regressions`) and run by
// `tools/regress.sh security` (cargo test -p rapidr-webbundle security).
//
// SEC-16: a project's name and its files' names were written into the
// page's <title>, into JavaScript strings and into an inline <script> with
// only `"` escaped: a file named `</title><img src=x onerror=…>` or
// `x</script><script>…` was markup / script in the built bundle.
// SEC-15: the bundle's page had no Content-Security-Policy.
// SEC-17: the bundle's scripts were the web IDE's own (include_str! of
// web-ide/), so every program carried IDE code.

use super::*;
use std::collections::HashMap;
use std::io::Read;

const HOSTILE: &[&str] = &[
    "</title><img src=x onerror=alert(1)>",
    "x</script><script>alert(1)</script>",
    "a\"b'c\\d",
    "\");alert(1);//",
    "${alert(1)}`",
    "../../evil",
    "<!--",
    "line\u{2028}sep",
];

fn unzip(bytes: &[u8]) -> HashMap<String, Vec<u8>> {
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).expect("a zip");
    let mut out = HashMap::new();
    for i in 0..zip.len() {
        let mut f = zip.by_index(i).unwrap();
        let mut data = Vec::new();
        f.read_to_end(&mut data).unwrap();
        out.insert(f.name().to_string(), data);
    }
    out
}

fn bundle(name: &str, assets: &HashMap<String, String>, needs: &WebNeeds) -> HashMap<String, Vec<u8>> {
    let bytes = build_bundle(&BundleInputs {
        project_name: name,
        rrbc: b"RRBC",
        rapidrintr_wasm: b"\0asm",
        rapidrintr_js: "export default async function init(){}\n",
        title: None,
        assets: Some(assets),
        fonts: &[],
        notices: "NOTICES",
        needs,
    })
    .expect("bundle");
    unzip(&bytes)
}

fn text(files: &HashMap<String, Vec<u8>>, name: &str) -> String {
    String::from_utf8(files.get(name).unwrap_or_else(|| panic!("no {name} in {:?}", files.keys())).clone()).unwrap()
}

/// What HTML would parse as tags in `html`, outside the ones the page has.
fn stray_tags(html: &str) -> Vec<String> {
    const OURS: &[&str] = &["!doctype", "html", "/html", "head", "/head", "meta", "title", "/title", "!--", "link", "style", "/style", "script", "/script", "body", "/body", "div", "/div", "pre", "/pre"];
    let mut out = Vec::new();
    let mut rest = html;
    while let Some(at) = rest.find('<') {
        let tag: String = rest[at + 1..].chars().take_while(|c| !c.is_whitespace() && *c != '>').collect::<String>().to_ascii_lowercase();
        if !OURS.contains(&tag.as_str()) {
            out.push(tag);
        }
        rest = &rest[at + 1..];
    }
    out
}

#[test]
fn security_sec16_hostile_project_names_stay_text() {
    for name in HOSTILE {
        let files = bundle(name, &HashMap::new(), &WebNeeds::default());
        let html = text(&files, "index.html");
        assert!(stray_tags(&html).is_empty(), "name {name:?} made markup: {:?}\n{html}", stray_tags(&html));
        assert_eq!(html.matches("<script").count(), 1, "name {name:?}: one script, loader.js\n{html}");
        assert_eq!(html.matches("</title>").count(), 1, "name {name:?}\n{html}");
        // (the title is the name, as text)
        assert!(html.contains(&format!("<title>{}</title>", escape::html(name))), "{html}");
        // The bytecode's file stays in the bundle's root, its name a safe one.
        let rrbc: Vec<&String> = files.keys().filter(|k| k.ends_with(".rrbc")).collect();
        assert_eq!(rrbc.len(), 1, "{:?}", files.keys());
        assert!(!rrbc[0].contains('/') && !rrbc[0].contains('\\') && !rrbc[0].starts_with('.'), "{rrbc:?}");
        // loader.js is never generated: no name reaches a script.
        assert_eq!(text(&files, "loader.js"), LOADER_JS, "name {name:?} changed loader.js");
    }
}

#[test]
fn security_sec16_hostile_asset_names_stay_strings() {
    let mut assets = HashMap::new();
    for (i, name) in HOSTILE.iter().enumerate() {
        assets.insert(format!("{name}.png"), format!("data:image/png;base64,AAAA{i}</script>"));
    }
    let files = bundle("demo", &assets, &WebNeeds::default());
    let html = text(&files, "index.html");
    // The names aren't in the page at all, only in rapidr-assets.js.
    for name in assets.keys() {
        assert!(!html.contains(name.as_str()), "{name:?} is in index.html");
    }
    assert!(stray_tags(&html).is_empty(), "{html}");
    let js = text(&files, ASSETS_FILE);
    for bad in ["</script", "<!--", "<script", "\u{2028}"] {
        assert!(!js.contains(bad), "{bad:?} in {ASSETS_FILE}:\n{js}");
    }
    // Every entry is one `"name": "value",` pair of string literals whose
    // decoded name is the asset's: nothing broke out of its string.
    let entries: Vec<&str> = js.lines().filter(|l| l.starts_with("  \"")).collect();
    assert_eq!(entries.len(), assets.len() * 2, "{js}");
    for line in entries {
        let (k, v) = parse_pair(line).unwrap_or_else(|| panic!("not a string pair: {line}"));
        let name = k.strip_prefix("assets/").unwrap_or(&k);
        assert!(assets.contains_key(name), "{k:?} isn't an asset's name ({line})");
        assert_eq!(&assets[name], &v);
    }
}

/// `  "k": "v",` with JSON string escapes → (k, v); None if anything else
/// is on the line.
fn parse_pair(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("  ")?;
    let (k, rest) = parse_str(rest)?;
    let rest = rest.strip_prefix(": ")?;
    let (v, rest) = parse_str(rest)?;
    (rest == ",").then_some((k, v))
}

fn parse_str(s: &str) -> Option<(String, &str)> {
    let mut chars = s.strip_prefix('"')?.char_indices();
    let mut out = String::new();
    while let Some((i, c)) = chars.next() {
        match c {
            '"' => return Some((out, &s[i + 2..])),
            '\\' => match chars.next()?.1 {
                'n' => out.push('\n'),
                'r' => out.push('\r'),
                't' => out.push('\t'),
                'u' => {
                    let hex: String = (0..4).filter_map(|_| chars.next().map(|x| x.1)).collect();
                    out.push(char::from_u32(u32::from_str_radix(&hex, 16).ok()?)?);
                }
                c => out.push(c),
            },
            c if (c as u32) < 0x20 => return None,
            c => out.push(c),
        }
    }
    None
}

#[test]
fn security_sec15_every_page_has_a_policy() {
    let files = bundle("demo", &HashMap::new(), &WebNeeds::scan("CREATE F AS QFORM\nEND CREATE\nCREATE D AS RDOM\nEND CREATE\n"));
    let html = text(&files, "index.html");
    let csp = html.split("http-equiv=\"Content-Security-Policy\" content=\"").nth(1).and_then(|r| r.split('"').next()).expect("a CSP meta");
    let csp = csp.replace("&#39;", "'");
    for must in ["default-src 'self'", "script-src 'self' 'wasm-unsafe-eval';", "object-src 'none'", "base-uri 'none'", "frame-src 'none'"] {
        assert!(csp.contains(must), "{must:?} not in {csp}");
    }
    assert!(!csp.contains("'unsafe-eval'") && !csp.contains("script-src 'self' 'wasm-unsafe-eval' 'unsafe-inline'"), "{csp}");
    // (RDOM: pictures from the web in its markup, never scripts)
    assert!(csp.contains("img-src 'self' data: blob: https:"), "{csp}");
    // The hosts' files say the same, with frame-ancestors, on the page only.
    let headers = text(&files, "_headers");
    assert!(headers.contains("/index.html\n  Content-Security-Policy: ") && headers.contains("frame-ancestors 'self'"), "{headers}");
    assert!(headers.contains("X-Content-Type-Options: nosniff"), "{headers}");
    let htaccess = text(&files, ".htaccess");
    assert!(htaccess.contains("<Files \"index.html\">") && !htaccess.contains("Access-Control-Allow-Origin"), "{htaccess}");
}

#[test]
fn security_sec15_policy_follows_the_components() {
    let js = csp::content_security_policy(&WebNeeds::scan("DIM J AS RJAVASCRIPT\n"));
    assert!(js.contains("script-src 'self' 'wasm-unsafe-eval' 'unsafe-eval';") && js.contains("frame-src 'none'"), "{js}");
    let web = csp::content_security_policy(&WebNeeds::scan("CREATE W AS RWEBVIEW\nEND CREATE\n"));
    assert!(web.contains("frame-src 'self' https:") && !web.contains("'unsafe-eval'"), "{web}");
    // An author's --csp can't smuggle in another directive or markup.
    let mut n = WebNeeds::default();
    assert!(n.add_extra("connect-src https://a.example; script-src 'unsafe-inline'").is_ok());
    assert!(n.add_extra("connect-src x\"><script>alert(1)</script>").is_err());
    assert!(n.add_extra("connect-src a,b").is_err());
    let html = render_index_html("t", "t.rrbc", false, &n);
    assert!(stray_tags(&html).is_empty(), "{html}");
}

#[test]
fn security_sec17_bundles_carry_nothing_of_the_ide() {
    let files = bundle("demo", &HashMap::new(), &WebNeeds::default());
    let allowed = ["index.html", "loader.js", "rapidrintr.js", "rapidrintr_bg.wasm", "demo.rrbc", "THIRD-PARTY-NOTICES.txt", "bundle_console.js", "ansi_screen.js", "rapidr-webview.html", "_headers", ".htaccess"];
    for name in files.keys() {
        assert!(allowed.contains(&name.as_str()), "unexpected file in a program's bundle: {name}");
    }
    // The crate's sources don't reach into the IDE's folders.
    let src = include_str!("../../interpreter/rapidr-webbundle/src/lib.rs");
    let ide = ["web-ide", "/"].concat();
    assert!(!src.contains(&format!("include_str!(\"../../../{ide}")), "rapidr-webbundle embeds web-ide/ files");
    for (name, body) in [("bundle_console.js", BUNDLE_CONSOLE_JS), ("ansi_screen.js", ANSI_SCREEN_JS)] {
        assert!(!body.contains("innerHTML"), "{name} writes markup");
    }
}

#[test]
fn security_sec15_native_site_pages_too() {
    let mut assets = HashMap::new();
    assets.insert("x</script><b>.png".to_string(), "data:,1".to_string());
    let files: HashMap<String, String> = native_site_files("</title><i>", "my_app", &assets, &WebNeeds::default(), "").into_iter().collect();
    let html = &files["index.html"];
    assert!(html.contains("http-equiv=\"Content-Security-Policy\""), "{html}");
    assert!(stray_tags(html).is_empty(), "{html}");
    assert!(!html.contains("<script>") && !html.contains("<script type=\"module\">"), "inline script: {html}");
    assert_eq!(files["start.js"], START_JS);
    assert!(!files[ASSETS_FILE].contains("</script"), "{}", files[ASSETS_FILE]);
    assert!(files.contains_key(WEBVIEW_FRAME_FILE) && files.contains_key("_headers"));
}
