/* Minimal ZIP writer (PKZIP, STORED method only).
 *
 * Mirrors the layout produced by `interpreter/rapidr-webbundle::build_bundle`
 * so .zip files exported from the in-browser IDE are byte-format-compatible
 * with `rapidr bundle-bc` output. STORED-only keeps this self-contained
 * (no DEFLATE / no CDN dependency).
 *
 * Inputs (`buildBundleZip`):
 *   - projectName, rrbc (Uint8Array), rapidrintrJs (string), rapidrintrWasm (Uint8Array), title?
 * Returns: { bytes: Uint8Array, files: { name → Uint8Array } }
 */

const enc = new TextEncoder();

// Encode current local time as a DOS date/time pair (used in PKZIP headers).
function dosDateTime(d = new Date()) {
  const yr = d.getFullYear();
  const dosDate = (((Math.max(1980, yr) - 1980) & 0x7f) << 9) | (((d.getMonth()+1) & 0x0f) << 5) | (d.getDate() & 0x1f);
  const dosTime = ((d.getHours() & 0x1f) << 11) | ((d.getMinutes() & 0x3f) << 5) | ((d.getSeconds() >>> 1) & 0x1f);
  return { dosDate, dosTime };
}
const { dosDate: NOW_DATE, dosTime: NOW_TIME } = dosDateTime();
const CRC_TABLE = (() => {
  const t = new Uint32Array(256);
  for (let n = 0; n < 256; n++) {
    let c = n;
    for (let k = 0; k < 8; k++) c = (c & 1) ? (0xEDB88320 ^ (c >>> 1)) : (c >>> 1);
    t[n] = c >>> 0;
  }
  return t;
})();
function crc32(bytes) {
  let c = 0xFFFFFFFF;
  for (let i = 0; i < bytes.length; i++) c = CRC_TABLE[(c ^ bytes[i]) & 0xFF] ^ (c >>> 8);
  return (c ^ 0xFFFFFFFF) >>> 0;
}

function dataUrlToBytes(dataUrl) {
  const m = /^data:[^;,]*;base64,(.*)$/.exec(dataUrl);
  if (m) {
    const bin = atob(m[1]);
    const out = new Uint8Array(bin.length);
    for (let i = 0; i < bin.length; i++) out[i] = bin.charCodeAt(i);
    return out;
  }
  // Plain text data URL ("data:text/csv,foo,bar")
  const c = /^data:[^,]*,(.*)$/.exec(dataUrl);
  return enc.encode(c ? decodeURIComponent(c[1]) : "");
}

function u8concat(arrs) {
  let len = 0;
  for (const a of arrs) len += a.length;
  const out = new Uint8Array(len);
  let off = 0;
  for (const a of arrs) { out.set(a, off); off += a.length; }
  return out;
}

function writeEntry(name, data) {
  const nameBytes = enc.encode(name);
  const crc = crc32(data);
  const size = data.length;

  // Local file header (30 bytes + name)
  const lfh = new Uint8Array(30 + nameBytes.length);
  const dv = new DataView(lfh.buffer);
  dv.setUint32(0, 0x04034b50, true);   // signature
  dv.setUint16(4, 20, true);            // version needed
  dv.setUint16(6, 0, true);             // flags
  dv.setUint16(8, 0, true);             // STORED
  dv.setUint16(10, NOW_TIME, true);     // mod time
  dv.setUint16(12, NOW_DATE, true);     // mod date  (current local time)
  dv.setUint32(14, crc, true);
  dv.setUint32(18, size, true);
  dv.setUint32(22, size, true);
  dv.setUint16(26, nameBytes.length, true);
  dv.setUint16(28, 0, true);
  lfh.set(nameBytes, 30);
  return { lfh, data, crc, size, nameBytes };
}

function indexHtml(title, version) {
  // Mirrors render_index_html from rapidr-webbundle (sans CSS — the
  // runtime injects RR_BASE_CSS itself when components are created).
  const ver = version ? `<meta name="generator" content="RapidR IDE v${version}">` : "";
  // Strict-ish CSP: same-origin only; allow inline <style> + the small
  // bootstrapping JSON status updates done by the runtime; allow data:
  // images so embedded RImage assets work; explicitly disallow remote
  // scripts and frames. wasm-unsafe-eval is required for WebAssembly init.
  const csp = "default-src 'self'; " +
    "script-src 'self' 'wasm-unsafe-eval'; " +
    "style-src 'self' 'unsafe-inline'; " +
    "img-src 'self' data: blob:; " +
    "font-src 'self' data:; " +
    "connect-src 'self'; " +
    "frame-src 'none'; object-src 'none'; base-uri 'self';";
  return `<!doctype html>
<html lang="en">
<head>
  <meta charset="utf-8">
  <meta name="viewport" content="width=device-width, initial-scale=1">
  <meta http-equiv="Content-Security-Policy" content="${csp}">
  ${ver}
  <title>${title}</title>
  <style>#rapidr-status { position: fixed; top: 8px; right: 12px; font-size: 12px; color: #888; pointer-events: none; }</style>
</head>
<body>
  <div id="rapidr-status">loading…</div>
  <script type="module" src="./loader.js"></script>
</body>
</html>
`;
}

function loaderJs(name) {
  return `// RapidR web bundle loader (in-browser IDE export).
import init, { rapidr_run_bc } from "./rapidrintr.js";
const status = document.getElementById("rapidr-status");
const set = (m) => { if (status) status.textContent = m; };
(async () => {
  try {
    set("init wasm…");
    await init();
    set("fetch program…");
    const r = await fetch("./${name}.rrbc");
    if (!r.ok) throw new Error("fetch ${name}.rrbc: " + r.status);
    const bytes = new Uint8Array(await r.arrayBuffer());
    set("running…");
    rapidr_run_bc(bytes);
    set("");
  } catch (e) {
    console.error(e);
    set("error: " + (e && e.message ? e.message : e));
  }
})();
`;
}

// Apache .htaccess shipped inside every bundle so static-hosting users
// (cPanel, Plesk, OVH, etc.) don't get 403 / wrong-MIME errors.
function htaccessText() {
  return `# RapidR static-site config\n` +
`<IfModule !mod_authz_core.c>\n  Order allow,deny\n  Allow from all\n</IfModule>\n` +
`<IfModule mod_authz_core.c>\n  Require all granted\n</IfModule>\n` +
`Options -Indexes +FollowSymLinks\n` +
`<IfModule mod_mime.c>\n` +
`  AddType application/wasm           .wasm\n` +
`  AddType application/javascript     .js .mjs\n` +
`  AddType application/json           .json\n` +
`  AddType application/octet-stream   .rrbc\n` +
`</IfModule>\n` +
`<IfModule mod_headers.c>\n` +
`  <FilesMatch "\\.(js|mjs|wasm|json|css)$">\n` +
`    Header set Access-Control-Allow-Origin "*"\n` +
`  </FilesMatch>\n` +
`</IfModule>\n`;
}

export function buildBundleZip({ projectName, rrbc, rapidrintrJs, rapidrintrWasm, title, assets, version }) {
  const t = title || projectName;
  // Accept rapidrintrJs as string OR as raw bytes (Uint8Array/ArrayBuffer)
  // — string is preferred (UTF-8 ESM source), bytes are pass-through.
  const jsBytes =
    typeof rapidrintrJs === "string"      ? enc.encode(rapidrintrJs) :
    rapidrintrJs instanceof Uint8Array    ? rapidrintrJs :
    rapidrintrJs instanceof ArrayBuffer   ? new Uint8Array(rapidrintrJs) :
    enc.encode(String(rapidrintrJs));
  const manifest = JSON.stringify({
    rapidr_bundle: 1,
    project: projectName,
    title: t,
    ide_version: version || null,
    built_at: new Date().toISOString(),
    asset_count: Array.isArray(assets) ? assets.length : 0,
  }, null, 2);
  const files = {
    "index.html":         enc.encode(indexHtml(t, version)),
    "loader.js":          enc.encode(loaderJs(projectName)),
    "manifest.json":      enc.encode(manifest),
    ".htaccess":          enc.encode(htaccessText()),
    "rapidrintr.js":      jsBytes,
    "rapidrintr_bg.wasm": rapidrintrWasm,
    [`${projectName}.rrbc`]: rrbc instanceof Uint8Array ? rrbc : new Uint8Array(rrbc),
  };

  // Optional project-bundled assets — { name, bytes } or { name, dataUrl }.
  // Each is written under "assets/<name>" so the runtime can fetch it via
  // the same relative URL the IDE uses ("assets/foo.png").
  if (Array.isArray(assets)) {
    for (const a of assets) {
      if (!a || !a.name) continue;
      let data;
      if (a.bytes instanceof Uint8Array) data = a.bytes;
      else if (typeof a.dataUrl === "string") data = dataUrlToBytes(a.dataUrl);
      else continue;
      files[`assets/${a.name}`] = data;
    }
  }

  const entries = [];
  const localChunks = [];
  let offset = 0;
  for (const [name, data] of Object.entries(files)) {
    const e = writeEntry(name, data);
    e.offset = offset;
    entries.push(e);
    localChunks.push(e.lfh, e.data);
    offset += e.lfh.length + e.data.length;
  }

  // Central directory
  const cdChunks = [];
  let cdSize = 0;
  for (const e of entries) {
    const cdh = new Uint8Array(46 + e.nameBytes.length);
    const dv = new DataView(cdh.buffer);
    dv.setUint32(0, 0x02014b50, true);  // central dir signature
    dv.setUint16(4, 20, true);           // version made by
    dv.setUint16(6, 20, true);           // version needed
    dv.setUint16(8, 0, true);            // flags
    dv.setUint16(10, 0, true);           // STORED
    dv.setUint16(12, NOW_TIME, true);    // mod time
    dv.setUint16(14, NOW_DATE, true);    // mod date
    dv.setUint32(16, e.crc, true);
    dv.setUint32(20, e.size, true);
    dv.setUint32(24, e.size, true);
    dv.setUint16(28, e.nameBytes.length, true);
    dv.setUint16(30, 0, true);
    dv.setUint16(32, 0, true);
    dv.setUint16(34, 0, true);
    dv.setUint16(36, 0, true);
    dv.setUint32(38, 0, true);
    dv.setUint32(42, e.offset, true);
    cdh.set(e.nameBytes, 46);
    cdChunks.push(cdh);
    cdSize += cdh.length;
  }
  const cdOffset = offset;

  // EOCD
  const eocd = new Uint8Array(22);
  const dv = new DataView(eocd.buffer);
  dv.setUint32(0, 0x06054b50, true);
  dv.setUint16(4, 0, true);
  dv.setUint16(6, 0, true);
  dv.setUint16(8, entries.length, true);
  dv.setUint16(10, entries.length, true);
  dv.setUint32(12, cdSize, true);
  dv.setUint32(16, cdOffset, true);
  dv.setUint16(20, 0, true);

  const bytes = u8concat([...localChunks, ...cdChunks, eocd]);
  return { bytes, files };
}
