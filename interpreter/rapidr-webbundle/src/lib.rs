//! Build a downloadable web bundle (a single `.zip`) for a RapidR program.
//!
//! The bundle is fully static and can be served from any HTTP host
//! (GitHub Pages, S3, plain nginx, `python3 -m http.server`, …). It
//! contains everything a browser needs to run a `.rrbc` program:
//!
//! ```text
//! <project>-web.zip
//!   index.html
//!   loader.js
//!   rapidrintr.js     <- wasm-bindgen JS shim for rapidr-vm-host-web
//!   rapidrintr.wasm   <- bytecode VM compiled to wasm
//!   <project>.rrbc    <- the user's compiled bytecode
//! ```
//!
//! At runtime `index.html` loads `loader.js` (an ES module), which in
//! turn imports `rapidrintr.js`, calls its `default()` init, fetches
//! `<project>.rrbc`, and finally invokes `rapidr_run_bc(bytes)`.

use std::io::{Cursor, Write};

use zip::{write::FileOptions, CompressionMethod, ZipWriter};

/// Inputs needed to build a bundle. All four byte/string slices are
/// embedded into the resulting ZIP.
pub struct BundleInputs<'a> {
    /// Slug used for the download filename and the embedded `.rrbc`
    /// (e.g. `"hello_web"` → file `hello_web.rrbc`).
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
}

/// Build the ZIP bytes. Never fails on well-formed inputs — the only
/// possible source of error is the in-memory `ZipWriter`.
pub fn build_bundle(inputs: &BundleInputs<'_>) -> Result<Vec<u8>, String> {
    let mut buf = Cursor::new(Vec::<u8>::new());
    {
        let mut zw = ZipWriter::new(&mut buf);
        let stored = FileOptions::default().compression_method(CompressionMethod::Stored);
        let deflated = FileOptions::default().compression_method(CompressionMethod::Deflated);

        let title = inputs.title.unwrap_or(inputs.project_name);
        let html = render_index_html(title, inputs.project_name);
        let loader = render_loader_js(inputs.project_name);

        write_file(&mut zw, "index.html", html.as_bytes(), deflated)?;
        write_file(&mut zw, "loader.js", loader.as_bytes(), deflated)?;
        write_file(&mut zw, "rapidrintr.js", inputs.rapidrintr_js.as_bytes(), deflated)?;
        // wasm-bindgen's generated `rapidrintr.js` expects to fetch
        // `rapidrintr_bg.wasm` (the conventional `_bg` suffix), so we
        // ship the binary under that name even though the build script
        // produces it as `rapidrintr.wasm`.
        write_file(&mut zw, "rapidrintr_bg.wasm", inputs.rapidrintr_wasm, stored)?;
        let rrbc_name = format!("{}.rrbc", inputs.project_name);
        write_file(&mut zw, &rrbc_name, inputs.rrbc, stored)?;

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

fn render_index_html(title: &str, _project_name: &str) -> String {
    format!(
        r#"<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <title>{title}</title>
  <style>
    html, body {{ margin: 0; padding: 0; font-family: system-ui, sans-serif; }}
    #rapidr-status {{ position: fixed; top: 8px; right: 12px; font-size: 12px;
                     color: #888; pointer-events: none; }}
  </style>
</head>
<body>
  <div id="rapidr-status">loading…</div>
  <script type="module" src="./loader.js"></script>
</body>
</html>
"#
    )
}

fn render_loader_js(project_name: &str) -> String {
    format!(
        r#"// RapidR web bundle loader.
// Boots `rapidrintr.wasm` (the bytecode VM) and runs `{project_name}.rrbc`.
import init, {{ rapidr_run_bc }} from "./rapidrintr.js";

const status = document.getElementById("rapidr-status");
function setStatus(msg) {{ if (status) status.textContent = msg; }}

(async function () {{
  try {{
    setStatus("init wasm…");
    await init();
    setStatus("fetch program…");
    const resp = await fetch("./{project_name}.rrbc");
    if (!resp.ok) throw new Error(`fetch {project_name}.rrbc: ${{resp.status}}`);
    const bytes = new Uint8Array(await resp.arrayBuffer());
    setStatus("running…");
    rapidr_run_bc(bytes);
    setStatus("");
  }} catch (e) {{
    console.error(e);
    setStatus("error: " + (e && e.message ? e.message : e));
  }}
}})();
"#
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundle_contains_expected_entries() {
        let bytes = build_bundle(&BundleInputs {
            project_name: "demo",
            rrbc: b"RRBC\x01\x00",
            rapidrintr_wasm: &[0x00, 0x61, 0x73, 0x6d],
            rapidrintr_js: "export default async function init(){};\nexport function rapidr_run_bc(){}\n",
            title: None,
        })
        .expect("bundle");
        // Smoke: zip starts with PK header and is non-trivial.
        assert!(bytes.len() > 200, "bundle suspiciously small: {}", bytes.len());
        assert_eq!(&bytes[0..2], b"PK");
        // Quick check that file names appear in the central dir.
        let s = String::from_utf8_lossy(&bytes);
        for name in ["index.html", "loader.js", "rapidrintr.js", "rapidrintr_bg.wasm", "demo.rrbc"] {
            assert!(s.contains(name), "missing {name} in bundle");
        }
    }
}
