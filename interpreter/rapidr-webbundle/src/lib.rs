//! Build a downloadable web bundle (a single `.zip`) for a RapidR program,
//! and the pages of a native web build (`rapidr build --web`).
//!
//! The bundle is fully static and can be served from any HTTP host
//! (GitHub Pages, S3, plain nginx, `python3 -m http.server`, …). It
//! contains everything a browser needs to run a `.rrbc` program:
//!
//! ```text
//! <project>-web.zip
//!   index.html              <- the page: its Content-Security-Policy, the program's name
//!   loader.js               <- boots the runtime, runs the program (never generated)
//!   rapidrintr.js           <- wasm-bindgen JS shim for rapidr-vm-host-web
//!   rapidrintr_bg.wasm      <- bytecode VM compiled to wasm
//!   <project>.rrbc          <- the user's compiled bytecode
//!   rapidr-assets.js        <- the project's files (when it has some)
//!   rapidr-webview.html     <- the frame an RWEBVIEW's Html runs in
//!   bundle_console.js, ansi_screen.js <- the on-page console for PRINT
//!   _headers, .htaccess     <- the same policy and safe headers, for hosts that read them
//!   THIRD-PARTY-NOTICES.txt <- the open-source notices (index.html links it)
//!   fonts/                  <- the fallback fonts
//! ```
//!
//! Security (docs/security-audit.md SEC-15 / SEC-16 / SEC-17): every page
//! carries a Content-Security-Policy derived from what the program uses
//! ([`csp`]); no name a project gives (its own, its files') is ever written
//! into a page unescaped or into a script at all ([`escape`]); a bundle
//! carries nothing of an IDE — its scripts are this crate's own (`web/`).

pub mod csp;
pub mod escape;

use std::collections::HashMap;
use std::io::{Cursor, Write};

use zip::{write::FileOptions, CompressionMethod, ZipWriter};

pub use csp::{content_security_policy, header_policy, WebNeeds};

/// Inputs needed to build a bundle. All four byte/string slices are
/// embedded into the resulting ZIP.
pub struct BundleInputs<'a> {
    /// The project's name (e.g. `"hello_web"`): the page's title and, made
    /// a safe file name ([`escape::file_name`]), the embedded `.rrbc`'s.
    pub project_name: &'a str,
    /// Compiled bytecode produced by `rapidr-bcgen` (`module.to_bytes()`).
    pub rrbc: &'a [u8],
    /// Contents of `rapidrintr.wasm` — the `rapidr-vm-host-web` cdylib
    /// post-processed by `wasm-bindgen` (or raw, if loader is adjusted).
    pub rapidrintr_wasm: &'a [u8],
    /// Contents of `rapidrintr.js` — the wasm-bindgen-generated ES
    /// module that exports `default()` (init) and `rapidr_run_bc`.
    pub rapidrintr_js: &'a str,
    /// Optional page title; defaults to the project name.
    pub title: Option<&'a str>,
    /// Optional embedded assets map (filename -> data URL)
    pub assets: Option<&'a HashMap<String, String>>,
    /// The fallback fonts' files (`fonts/`: `index.json`, the chunks,
    /// `OFL.txt`; tools/fonts.py), by name — the page loads the ones its
    /// text needs. Empty: none (characters the built-in fonts lack show as
    /// boxes).
    pub fonts: &'a [(String, Vec<u8>)],
    /// The web runtime's `THIRD-PARTY-NOTICES.txt` (RapidR's licence and
    /// every open-source component's notices: `rapidr notices web`).
    pub notices: &'a str,
    /// What the program uses, for its page's policy ([`WebNeeds::scan`] of
    /// its source, plus the author's `--csp`).
    pub needs: &'a WebNeeds,
}

/// The page around a program (a bundle's, a `rapidr build --web` site's):
/// the UI kernel draws the windows on it, so the page only sets its
/// background and the native build's console (`#rr-console`).
pub const PAGE_CSS: &str = r#"
* { box-sizing: border-box; }
html, body { margin: 0; padding: 0; }
body { min-height: 100vh; background: #e8e8e8; font-family: system-ui, sans-serif; font-size: 13px; overflow: auto; }
#rr-console { position: fixed; bottom: 0; left: 0; width: 100%; max-height: 200px; overflow-y: auto; background: #1e1e1e; color: #d4d4d4; font-family: ui-monospace, Menlo, Consolas, monospace; font-size: 13px; padding: 8px; display: none; z-index: 10000; border-top: 2px solid #333; }
"#;

/// The notices' name in the bundle's root (the CLI's notices.rs).
pub const NOTICES_FILE: &str = "THIRD-PARTY-NOTICES.txt";
/// The on-page console for PRINT output: loader.js installs it, rendering
/// with ansi_screen.js (this crate's own copies: a bundle carries nothing of
/// an IDE — docs/security-audit.md SEC-17).
pub const BUNDLE_CONSOLE_JS: &str = include_str!("../web/bundle_console.js");
pub const ANSI_SCREEN_JS: &str = include_str!("../web/ansi_screen.js");
/// The bundle's loader: a fixed file, the program named by the page.
pub const LOADER_JS: &str = include_str!("../web/loader.js");
/// A native web build's start: a fixed file, the module named by the page.
pub const START_JS: &str = include_str!("../web/start.js");
/// The frame an RWEBVIEW's Html runs in (the web runtime's overlay_web.rs
/// loads it by this name, beside the page).
pub const WEBVIEW_FRAME_FILE: &str = "rapidr-webview.html";
pub const WEBVIEW_FRAME_HTML: &str = include_str!("../web/rapidr-webview.html");
/// The project's files, for the runtime (`window.__rapidr_assets`).
pub const ASSETS_FILE: &str = "rapidr-assets.js";

/// Build the ZIP bytes. Never fails on well-formed inputs — the only
/// possible source of error is the in-memory `ZipWriter`.
pub fn build_bundle(inputs: &BundleInputs<'_>) -> Result<Vec<u8>, String> {
    let mut buf = Cursor::new(Vec::<u8>::new());
    {
        let mut zw = ZipWriter::new(&mut buf);
        let stored = FileOptions::default().compression_method(CompressionMethod::Stored);
        let deflated = FileOptions::default().compression_method(CompressionMethod::Deflated);

        let title = inputs.title.unwrap_or(inputs.project_name);
        let rrbc_name = format!("{}.rrbc", escape::file_name(inputs.project_name));
        let assets = inputs.assets.filter(|a| !a.is_empty());
        let html = render_index_html(title, &rrbc_name, assets.is_some(), inputs.needs);

        write_file(&mut zw, "index.html", html.as_bytes(), deflated)?;
        write_file(&mut zw, "loader.js", LOADER_JS.as_bytes(), deflated)?;
        write_file(&mut zw, "rapidrintr.js", inputs.rapidrintr_js.as_bytes(), deflated)?;
        // wasm-bindgen's generated `rapidrintr.js` expects to fetch
        // `rapidrintr_bg.wasm` (the conventional `_bg` suffix), so we
        // ship the binary under that name even though the build script
        // produces it as `rapidrintr.wasm`.
        write_file(&mut zw, "rapidrintr_bg.wasm", inputs.rapidrintr_wasm, stored)?;
        write_file(&mut zw, &rrbc_name, inputs.rrbc, stored)?;
        if let Some(assets) = assets {
            write_file(&mut zw, ASSETS_FILE, render_assets_js(assets).as_bytes(), deflated)?;
        }
        // The bundle redistributes RapidR's runtime: its licence and the
        // open-source notices travel with it.
        if inputs.notices.trim().is_empty() {
            return Err(format!("{NOTICES_FILE} is empty: a bundle isn't shipped without its notices"));
        }
        write_file(&mut zw, NOTICES_FILE, inputs.notices.as_bytes(), deflated)?;
        write_file(&mut zw, "bundle_console.js", BUNDLE_CONSOLE_JS.as_bytes(), deflated)?;
        write_file(&mut zw, "ansi_screen.js", ANSI_SCREEN_JS.as_bytes(), deflated)?;
        write_file(&mut zw, WEBVIEW_FRAME_FILE, WEBVIEW_FRAME_HTML.as_bytes(), deflated)?;
        for (name, text) in host_config_files(inputs.needs) {
            write_file(&mut zw, name, text.as_bytes(), deflated)?;
        }
        for (name, data) in inputs.fonts {
            write_file(&mut zw, &format!("fonts/{}", escape::file_name(name)), data, stored)?;
        }

        zw.finish().map_err(|e| format!("zip finish: {e}"))?;
    }
    Ok(buf.into_inner())
}

fn write_file<W: Write + std::io::Seek>(
    zw: &mut ZipWriter<W>,
    name: &str,
    data: &[u8],
    options: FileOptions,
) -> Result<(), String> {
    zw.start_file(name, options)
        .map_err(|e| format!("zip start_file {name}: {e}"))?;
    zw.write_all(data)
        .map_err(|e| format!("zip write {name}: {e}"))?;
    Ok(())
}

/// `<meta http-equiv="Content-Security-Policy">` for a page.
fn csp_meta(needs: &WebNeeds) -> String {
    format!(r#"<meta http-equiv="Content-Security-Policy" content="{}">"#, escape::html(&content_security_policy(needs)))
}

/// `rapidr-assets.js`: the project's files by name, as data URLs — a
/// classic script the page loads before the runtime; every name and value
/// a JavaScript string literal ([`escape::js_string`]).
pub fn render_assets_js(assets: &HashMap<String, String>) -> String {
    let mut names: Vec<&String> = assets.keys().collect();
    names.sort();
    let mut out = String::from("// The project's files (rapidr build --web), for the web runtime.\nwindow.__rapidr_assets = {\n");
    for name in names {
        let value = escape::js_string(&assets[name]);
        out.push_str(&format!("  {}: {value},\n", escape::js_string(name)));
        out.push_str(&format!("  {}: {value},\n", escape::js_string(&format!("assets/{name}"))));
    }
    out.push_str("};\n");
    out
}

/// The bundle's `index.html`: `title` as text, the program's file name as
/// an attribute (read by loader.js), its policy as a meta tag.
pub fn render_index_html(title: &str, rrbc_name: &str, has_assets: bool, needs: &WebNeeds) -> String {
    let css = PAGE_CSS;
    let title = escape::html(title);
    let program = escape::html(rrbc_name);
    let csp = csp_meta(needs);
    let assets = if has_assets { format!("  <script src=\"./{ASSETS_FILE}\"></script>\n") } else { String::new() };
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  {csp}
  <meta name="referrer" content="strict-origin-when-cross-origin">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta name="rapidr-program" content="{program}">
  <title>{title}</title>
  <!-- Built with RapidR (MIT). Third-party software notices and licences: {NOTICES_FILE} -->
  <link rel="license" href="{NOTICES_FILE}">
  <style>{css}
#rapidr-status {{ position: fixed; top: 8px; right: 12px; font-size: 12px; color: #888; pointer-events: none; }}
</style>
{assets}</head>
<body>
  <div id="rapidr-status">loading…</div>
  <script type="module" src="./loader.js"></script>
</body>
</html>
"#
    )
}

/// The files of a native web build's page (`rapidr build --web`) beside
/// wasm-bindgen's output: `index.html` (the title as text, the program's
/// module name as an attribute, its policy), `start.js`, the project's files
/// (`rapidr-assets.js`), the RWEBVIEW frame and the hosts' configuration.
/// `notices_head` is the notices' `<link>` lines (the CLI's).
pub fn native_site_files(title: &str, wasm_module: &str, assets: &HashMap<String, String>, needs: &WebNeeds, notices_head: &str) -> Vec<(String, String)> {
    let css = PAGE_CSS;
    let title = escape::html(title);
    let module = escape::html(wasm_module);
    let csp = csp_meta(needs);
    let assets_tag = if assets.is_empty() { String::new() } else { format!("  <script src=\"./{ASSETS_FILE}\"></script>\n") };
    let html = format!(
        r#"<!DOCTYPE html>
<html lang="en">
<head>
  <meta charset="UTF-8">
  {csp}
  <meta name="referrer" content="strict-origin-when-cross-origin">
  <meta name="viewport" content="width=device-width, initial-scale=1.0">
  <meta name="rapidr-module" content="{module}">
  <title>{title}</title>
{notices_head}  <style>{css}</style>
{assets_tag}</head>
<body>
  <pre id="rr-console"></pre>
  <script type="module" src="./start.js"></script>
</body>
</html>
"#
    );
    let mut files = vec![("index.html".to_string(), html), ("start.js".to_string(), START_JS.to_string())];
    if !assets.is_empty() {
        files.push((ASSETS_FILE.to_string(), render_assets_js(assets)));
    }
    files.push((WEBVIEW_FRAME_FILE.to_string(), WEBVIEW_FRAME_HTML.to_string()));
    for (name, text) in host_config_files(needs) {
        files.push((name.to_string(), text));
    }
    files
}

/// Headers every file of a program's site gets from a host that sets them.
/// No cross-origin isolation (COEP): the runtime uses no SharedArrayBuffer,
/// and it would stop a program showing pictures and media from the web.
const COMMON_HEADERS: &[(&str, &str)] = &[
    ("X-Content-Type-Options", "nosniff"),
    ("Referrer-Policy", "strict-origin-when-cross-origin"),
    ("Cross-Origin-Opener-Policy", "same-origin"),
    ("Cross-Origin-Resource-Policy", "same-origin"),
];

/// `_headers` (Netlify, Cloudflare Pages) and `.htaccess` (Apache): the
/// page's policy as a header too — with `frame-ancestors`, which only a
/// header can say — on `index.html` only (the RWEBVIEW frame keeps its own
/// rules), and the common headers on everything. A host that reads neither
/// still has the page's meta policy.
pub fn host_config_files(needs: &WebNeeds) -> Vec<(&'static str, String)> {
    let policy = header_policy(needs);
    let mut headers = String::from(
        "# Response headers for hosts that read this file (Netlify, Cloudflare Pages).\n\
         # Made by rapidr build --web (docs/security-audit.md SEC-15). The policy is\n\
         # index.html's own, plus frame-ancestors; rapidr-webview.html, the frame an\n\
         # RWEBVIEW's Html runs in, must not get it.\n/*\n",
    );
    for (h, v) in COMMON_HEADERS {
        headers.push_str(&format!("  {h}: {v}\n"));
    }
    for path in ["/", "/index.html"] {
        headers.push_str(&format!("{path}\n  Content-Security-Policy: {policy}\n"));
    }
    let mut htaccess = String::from(
        "# Apache configuration for a RapidR web program (rapidr build --web;\n\
         # docs/security-audit.md SEC-15). The policy is index.html's own, plus\n\
         # frame-ancestors; rapidr-webview.html, the frame an RWEBVIEW's Html runs\n\
         # in, must not get it. No CORS: the program's files are its own page's.\n\
         Options -Indexes\n\
         <IfModule mod_mime.c>\n  AddType application/wasm .wasm\n  AddType text/javascript .js\n  AddType application/octet-stream .rrbc\n</IfModule>\n\
         <IfModule mod_headers.c>\n",
    );
    for (h, v) in COMMON_HEADERS {
        htaccess.push_str(&format!("  Header always set {h} \"{v}\"\n"));
    }
    htaccess.push_str(&format!("  <Files \"index.html\">\n    Header always set Content-Security-Policy \"{policy}\"\n  </Files>\n</IfModule>\n"));
    vec![("_headers", headers), (".htaccess", htaccess)]
}

#[cfg(test)]
mod tests {
    use super::*;

    fn inputs<'a>(name: &'a str, assets: Option<&'a HashMap<String, String>>, needs: &'a WebNeeds) -> BundleInputs<'a> {
        BundleInputs {
            project_name: name,
            rrbc: b"RRBC\x01\x00",
            rapidrintr_wasm: &[0x00, 0x61, 0x73, 0x6d],
            rapidrintr_js: "export default async function init(){};\nexport function rapidr_run_bc(){}\n",
            title: None,
            assets,
            fonts: &[],
            notices: "THIRD-PARTY SOFTWARE NOTICES AND LICENCES",
            needs,
        }
    }

    #[test]
    fn bundle_contains_expected_entries() {
        let needs = WebNeeds::default();
        let bytes = build_bundle(&inputs("demo", None, &needs)).expect("bundle");
        // Smoke: zip starts with PK header and is non-trivial.
        assert!(bytes.len() > 200, "bundle suspiciously small: {}", bytes.len());
        assert_eq!(&bytes[0..2], b"PK");
        // Quick check that file names appear in the central dir.
        let s = String::from_utf8_lossy(&bytes);
        for name in ["index.html", "loader.js", "rapidrintr.js", "rapidrintr_bg.wasm", "demo.rrbc", "THIRD-PARTY-NOTICES.txt", "bundle_console.js", "ansi_screen.js", "rapidr-webview.html", "_headers", ".htaccess"] {
            assert!(s.contains(name), "missing {name} in bundle");
        }
    }

    #[test]
    fn the_page_has_its_policy_and_no_inline_script() {
        let html = render_index_html("demo", "demo.rrbc", false, &WebNeeds::default());
        assert!(html.contains(r#"<meta http-equiv="Content-Security-Policy" content="default-src &#39;self&#39;; script-src &#39;self&#39; &#39;wasm-unsafe-eval&#39;;"#), "{html}");
        assert!(!html.contains("<script>"), "an inline script would need 'unsafe-inline': {html}");
    }
}

/// The security regressions (docs/security-audit.md SEC-15 / SEC-16 /
/// SEC-17), kept with the other security tests in tests/security/ and run by
/// `tools/regress.sh security`.
#[cfg(test)]
#[path = "../../../tests/security/web_bundle_injection.rs"]
mod security_regressions;
