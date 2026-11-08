//! The web: a program as a folder of static files any web host serves —
//! today's bundle (rapidr-webbundle: the page, the loader, the runtime,
//! the program, its files, the fonts, the notices) plus what makes it an
//! installable web app: `manifest.webmanifest` (its name, description,
//! colour, icons) and `icons/` (32, 180, 192 and 512 px, and the SVG when
//! the icon is one).
//!
//! The web's "runner" is the web runtime as one file: a plain tar holding
//! `rapidrintr.js`, `rapidrintr_bg.wasm` and `fonts/…` ([`runtime_tar`],
//! [`Runtime::from_tar`]).

use std::collections::HashMap;

use rapidr_webbundle::{BundleInputs, PageMeta, WebNeeds};

use crate::files::Files;
use crate::info::{json_string, AppInfo};
use crate::Icon;

/// The web runtime's files.
pub struct Runtime {
    pub js: String,
    pub wasm: Vec<u8>,
    /// The fallback fonts (`index.json`, the chunks, `OFL.txt`), by name.
    pub fonts: Vec<(String, Vec<u8>)>,
}

pub const RUNTIME_JS: &str = "rapidrintr.js";
pub const RUNTIME_WASM: &str = "rapidrintr_bg.wasm";
pub const MANIFEST_FILE: &str = "manifest.webmanifest";
/// The page's background (rapidr-webbundle's PAGE_CSS): the manifest's
/// background and, unless the app says, its theme colour.
pub const PAGE_BACKGROUND: &str = "#e8e8e8";

/// The web runtime as one file (what `runner_bytes(web)` gives): a tar of
/// `rapidrintr.js`, `rapidrintr_bg.wasm` and `fonts/<name>`.
pub fn runtime_tar(js: &str, wasm: &[u8], fonts: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let mut w = rapidr_archive::Writer::new(rapidr_archive::Format::Tar, 0, 0)?;
    w.add(RUNTIME_JS, js.as_bytes().to_vec(), 0)?;
    w.add(RUNTIME_WASM, wasm.to_vec(), 0)?;
    for (name, data) in fonts {
        w.add(&format!("fonts/{name}"), data.clone(), 0)?;
    }
    w.finish()
}

impl Runtime {
    /// The runtime out of [`runtime_tar`]'s file.
    pub fn from_tar(bytes: &[u8]) -> Result<Runtime, String> {
        let bad = |why: String| format!("the web runtime: {why}");
        let limits = rapidr_archive::Limits { max_size: 512 << 20, max_ratio: 0 };
        let a = rapidr_archive::Archive::open(bytes.to_vec(), "web.tar", &limits).map_err(bad)?;
        let get = |name: &str| -> Result<Vec<u8>, String> {
            let i = a.find(name, true).ok_or_else(|| bad(format!("no {name} in it")))?;
            a.extract(i, &limits).map_err(bad)
        };
        let js = String::from_utf8(get(RUNTIME_JS)?).map_err(|_| bad(format!("{RUNTIME_JS} isn't text")))?;
        let wasm = get(RUNTIME_WASM)?;
        if !wasm.starts_with(b"\0asm") {
            return Err(bad(format!("{RUNTIME_WASM} isn't WebAssembly")));
        }
        let mut fonts = Vec::new();
        for (i, e) in a.entries().iter().enumerate() {
            if let Some(name) = e.name.strip_prefix("fonts/").filter(|n| !n.is_empty() && !n.contains('/')) {
                if e.kind == rapidr_archive::Kind::File {
                    fonts.push((name.to_string(), a.extract(i, &limits).map_err(bad)?));
                }
            }
        }
        fonts.sort_by(|x, y| x.0.cmp(&y.0));
        Ok(Runtime { js, wasm, fonts })
    }
}

/// `manifest.webmanifest`: what a browser installs the app as.
pub fn manifest(info: &AppInfo, icon: &Icon) -> String {
    let theme = if info.theme_color.trim().is_empty() { PAGE_BACKGROUND } else { info.theme_color.trim() };
    let mut icons = vec![
        r#"{"src": "icons/icon-192.png", "sizes": "192x192", "type": "image/png", "purpose": "any"}"#.to_string(),
        r#"{"src": "icons/icon-512.png", "sizes": "512x512", "type": "image/png", "purpose": "any"}"#.to_string(),
    ];
    if icon.svg_text().is_some() {
        icons.push(r#"{"src": "icons/icon.svg", "sizes": "any", "type": "image/svg+xml", "purpose": "any"}"#.to_string());
    }
    let mut out = String::from("{\n");
    let mut field = |k: &str, v: String| out.push_str(&format!("  \"{k}\": {v},\n"));
    field("name", json_string(info.name.trim()));
    field("short_name", json_string(info.name.trim()));
    if !info.description.trim().is_empty() {
        field("description", json_string(info.description.trim()));
    }
    field("id", json_string(&format!("./?app={}", info.bundle_id)));
    field("start_url", json_string("./"));
    field("scope", json_string("./"));
    field("display", json_string("standalone"));
    field("background_color", json_string(PAGE_BACKGROUND));
    field("theme_color", json_string(theme));
    out.push_str(&format!("  \"icons\": [\n    {}\n  ]\n}}\n", icons.join(",\n    ")));
    out
}

/// A data URL's media type and the bytes, for the page's files.
fn data_url(name: &str, bytes: &[u8]) -> String {
    format!("data:{};base64,{}", rapidr_webbundle::mime_for(name), base64(bytes))
}

/// Standard base64 (RFC 4648), padded.
fn base64(bytes: &[u8]) -> String {
    const T: &[u8; 64] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut out = String::with_capacity(bytes.len().div_ceil(3) * 4);
    for c in bytes.chunks(3) {
        let n = (u32::from(c[0]) << 16) | (u32::from(*c.get(1).unwrap_or(&0)) << 8) | u32::from(*c.get(2).unwrap_or(&0));
        out.push(T[(n >> 18) as usize & 63] as char);
        out.push(T[(n >> 12) as usize & 63] as char);
        out.push(if c.len() > 1 { T[(n >> 6) as usize & 63] as char } else { '=' });
        out.push(if c.len() > 2 { T[n as usize & 63] as char } else { '=' });
    }
    out
}

/// The web app's files: the bundle (its page with the app's head lines),
/// the manifest and the icons. `data` are the program's files by path;
/// those under `lang/` ending `.po` are its translations.
pub fn web_files(info: &AppInfo, icon: &Icon, bytecode: &[u8], data: &[(String, Vec<u8>)], runtime: &Runtime, notices: &str, needs: &WebNeeds) -> Result<Files, String> {
    info.check()?;
    let mut assets = HashMap::new();
    let mut catalogs = Vec::new();
    for (path, bytes) in data {
        match path.strip_prefix("lang/").filter(|n| n.to_ascii_lowercase().ends_with(".po") && !n.contains('/')) {
            Some(po) => catalogs.push((po.to_string(), bytes.clone())),
            None => {
                assets.insert(path.clone(), data_url(path, bytes));
            }
        }
    }
    let meta = PageMeta {
        description: info.description.trim().to_string(),
        theme_color: if info.theme_color.trim().is_empty() { PAGE_BACKGROUND.into() } else { info.theme_color.trim().into() },
        manifest: MANIFEST_FILE.into(),
        icon: "icons/icon-32.png".into(),
        touch_icon: "icons/icon-180.png".into(),
    };
    let has_catalogs = data.iter().any(|(p, _)| p.starts_with("lang/"));
    let inputs = BundleInputs {
        project_name: &info.exe,
        rrbc: bytecode,
        rapidrintr_wasm: &runtime.wasm,
        rapidrintr_js: &runtime.js,
        title: Some(info.name.trim()),
        assets: Some(&assets),
        catalogs: has_catalogs.then_some(catalogs.as_slice()),
        fonts: &runtime.fonts,
        notices,
        needs,
        meta: Some(&meta),
    };
    let mut f = Files::new();
    for (name, bytes) in rapidr_webbundle::bundle_files(&inputs)? {
        f.add(&name, bytes)?;
    }
    f.add(MANIFEST_FILE, manifest(info, icon).into_bytes())?;
    for px in [32, 180, 192, 512] {
        f.add(&format!("icons/icon-{px}.png"), icon.png(px))?;
    }
    if let Some(svg) = icon.svg_text() {
        f.add("icons/icon.svg", svg.into_bytes())?;
    }
    Ok(f)
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn tiny_runtime() -> Vec<u8> {
        runtime_tar("export default async function init(){}\n", b"\0asm\x01\0\0\0", &[("index.json".into(), b"{}".to_vec()), ("OFL.txt".into(), b"OFL".to_vec())]).unwrap()
    }

    #[test]
    fn the_runtime_travels_as_one_tar() {
        let rt = Runtime::from_tar(&tiny_runtime()).unwrap();
        assert!(rt.js.starts_with("export default"));
        assert_eq!(rt.wasm, b"\0asm\x01\0\0\0");
        assert_eq!(rt.fonts.iter().map(|(n, _)| n.as_str()).collect::<Vec<_>>(), ["OFL.txt", "index.json"]);
        let no_wasm = runtime_tar("x", b"not wasm", &[]).unwrap();
        assert!(Runtime::from_tar(&no_wasm).err().unwrap().contains("isn't WebAssembly"));
        assert!(Runtime::from_tar(b"garbage").is_err());
    }

    #[test]
    fn the_manifest_is_json_with_every_value_escaped() {
        let mut info = AppInfo::new("Tips \"Pro\" </script>", "tips");
        info.description = "Line one\nline two".into();
        info.theme_color = "#1E88E5".into();
        let m = manifest(&info, &Icon::rapidr_default());
        let escaped_lt = format!("{}u003c/script>", '\\');
        assert!(m.contains(&format!("\"name\": \"Tips \\\"Pro\\\" {escaped_lt}\"")) && !m.contains("</script>"), "{m}");
        assert!(m.contains(r#""description": "Line one\nline two""#), "{m}");
        assert!(m.contains(r##""theme_color": "#1E88E5""##), "{m}");
        assert!(m.contains(r#""display": "standalone""#) && m.contains("icons/icon-512.png") && m.contains("icons/icon.svg"), "{m}");
        assert!(m.trim_end().ends_with('}'));
        // (a PNG icon: no SVG entry; no colour: the page's)
        let png = Icon::from_bytes(&crate::picture::encode_png(&crate::picture::blank(64, 64)), "p.png").unwrap();
        let m = manifest(&AppInfo::new("A", "a"), &png);
        assert!(!m.contains("icon.svg") && m.contains(PAGE_BACKGROUND), "{m}");
    }

    #[test]
    fn base64_is_standard() {
        assert_eq!(base64(b""), "");
        assert_eq!(base64(b"f"), "Zg==");
        assert_eq!(base64(b"fo"), "Zm8=");
        assert_eq!(base64(b"foo"), "Zm9v");
        assert_eq!(base64(&[0xFF, 0xFE, 0xFD, 0xFC]), "//79/A==");
    }
}
