//! RapidR build server.
//!
//! Provides HTTP endpoints used by the IDE to build, preview, and download
//! RapidR programs.
//!
//! Endpoints:
//!   POST /compile     body = source code (text/plain)
//!                     -> 200 application/json { "id": "<uuid>", "ok": true,
//!                                               "stderr": "...",
//!                                               "preview": "/preview/<uuid>/",
//!                                               "zip_source": "/zip/<uuid>/source",
//!                                               "zip_full":   "/zip/<uuid>/full" }
//!                     On compile failure: { "ok": false, "stderr": "..." }
//!   GET  /preview/<id>/       -> serves the compiled web bundle (index.html, *.js, *.wasm)
//!   GET  /zip/<id>/source     -> zip of just the .rr source
//!   GET  /zip/<id>/full       -> zip of source + binary (web bundle + native release binary)
//!   GET  /health              -> "ok"
//!
//! Security: this server compiles arbitrary source (including `RUSTSTART`
//! blocks built with cargo), so it must only ever be reachable from the local
//! machine. It binds to 127.0.0.1 by default, rejects requests whose `Host`
//! or `Origin` is not loopback (blocks cross-site requests and DNS rebinding),
//! and only answers CORS preflights from loopback origins.

use axum::{
    body::Body,
    extract::{Path, Request, State},
    http::{header, HeaderValue, Method, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    io::{Cursor, Write},
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Component, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};
use tower_http::{
    cors::{AllowOrigin, CorsLayer},
    services::ServeDir,
};

#[derive(Clone)]
struct Build {
    /// Workspace root (one tempdir per build).
    workdir: PathBuf,
    /// Path to the source `.rr` file.
    source_rr: PathBuf,
    /// Path to the generated web bundle directory (index.html lives here).
    web_dir: PathBuf,
    /// Path to the native release binary (may not exist if --no-binary).
    native_bin: Option<PathBuf>,
    /// Captured stderr from the compile step.
    stderr: String,
    /// Whether compilation succeeded.
    ok: bool,
}

#[derive(Clone)]
struct AppState {
    /// Path to the `rapidr` compiler binary.
    rapidr: PathBuf,
    /// Workspace root (where target/ for native builds lives — we reuse the
    /// repo root so cargo cache is shared).
    repo_root: PathBuf,
    /// In-memory build registry keyed by uuid.
    builds: Arc<Mutex<HashMap<String, Build>>>,
    /// Map from source-hash -> build id, so repeated /compile of unchanged
    /// source returns the cached artifact instead of recompiling.
    source_cache: Arc<Mutex<HashMap<u64, String>>>,
}

#[derive(Serialize)]
struct CompileResp {
    id: String,
    ok: bool,
    stderr: String,
    preview: String,
    zip_source: String,
    zip_full: String,
}

#[tokio::main]
async fn main() {
    let repo_root = std::env::var("RAPIDR_REPO_ROOT")
        .map(PathBuf::from)
        .unwrap_or_else(|_| std::env::current_dir().expect("cwd"));
    let rapidr = repo_root.join("rapidr");
    if !rapidr.exists() {
        eprintln!(
            "warning: rapidr binary not found at {:?} — set RAPIDR_REPO_ROOT or run from repo root",
            rapidr
        );
    }
    let state = AppState {
        rapidr,
        repo_root,
        builds: Arc::new(Mutex::new(HashMap::new())),
        source_cache: Arc::new(Mutex::new(HashMap::new())),
    };

    let app = Router::new()
        .route("/health", get(|| async { "ok" }))
        .route("/compile", post(compile))
        .route("/preview/:id/*path", get(preview))
        .route("/preview/:id/", get(preview_index))
        .route("/preview/:id", get(preview_index))
        .route("/zip/:id/source", get(zip_source))
        .route("/zip/:id/full", get(zip_full))
        .layer(middleware::from_fn(loopback_guard))
        .layer(
            CorsLayer::new()
                .allow_origin(AllowOrigin::predicate(|origin: &HeaderValue, _| {
                    origin.to_str().map(is_loopback_origin).unwrap_or(false)
                }))
                .allow_methods([Method::GET, Method::POST])
                .allow_headers([header::CONTENT_TYPE]),
        )
        .with_state(state);

    let port: u16 = std::env::var("RAPIDR_BUILDSERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8095);
    let ip: IpAddr = std::env::var("RAPIDR_BUILDSERVER_HOST")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(IpAddr::V4(Ipv4Addr::LOCALHOST));
    if !ip.is_loopback() {
        eprintln!(
            "WARNING: binding to non-loopback address {ip}. This server compiles and \
             runs arbitrary code; requests are still restricted to loopback Host/Origin, \
             but do not expose it to untrusted networks."
        );
    }
    let addr = SocketAddr::from((ip, port));
    println!("rapidr-buildserver listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

/// Rejects any request whose `Host` is not loopback, or whose `Origin` (when
/// present) is not a loopback http(s) origin. Browsers always send `Origin` on
/// cross-site POSTs, so this blocks drive-by compiles from arbitrary websites
/// even though a `text/plain` POST needs no CORS preflight.
async fn loopback_guard(req: Request, next: Next) -> Response {
    let host_ok = req
        .headers()
        .get(header::HOST)
        .and_then(|h| h.to_str().ok())
        .map(is_loopback_host)
        .unwrap_or(false);
    let origin_ok = match req.headers().get(header::ORIGIN) {
        None => true,
        Some(o) => o.to_str().map(is_loopback_origin).unwrap_or(false),
    };
    if host_ok && origin_ok {
        next.run(req).await
    } else {
        (StatusCode::FORBIDDEN, "rapidr-buildserver only accepts local requests").into_response()
    }
}

/// `host` may carry a port (`localhost:8095`, `[::1]:8095`).
fn is_loopback_host(host: &str) -> bool {
    let bare = match host.strip_prefix('[') {
        Some(rest) => rest.split(']').next().unwrap_or(""),
        None => host.split(':').next().unwrap_or(""),
    };
    bare.eq_ignore_ascii_case("localhost")
        || bare.parse::<IpAddr>().map(|ip| ip.is_loopback()).unwrap_or(false)
}

/// Accepts `http(s)://<loopback host>[:port]` only; `null` and anything with a
/// path or other scheme is rejected.
fn is_loopback_origin(origin: &str) -> bool {
    origin
        .strip_prefix("http://")
        .or_else(|| origin.strip_prefix("https://"))
        .map(|rest| !rest.is_empty() && !rest.contains('/') && is_loopback_host(rest))
        .unwrap_or(false)
}

/// A preview path is safe only if every component is a plain file/dir name
/// (no `..`, no root, no drive prefix).
fn is_safe_relative_path(rel: &str) -> bool {
    std::path::Path::new(rel)
        .components()
        .all(|c| matches!(c, Component::Normal(_)))
}

async fn compile(State(state): State<AppState>, body: String) -> Response {
    // Spawn the actual work on a blocking task — cargo + wasm-bindgen are slow.
    let res = tokio::task::spawn_blocking(move || do_compile(state, body))
        .await
        .unwrap_or_else(|e| {
            Err(format!("join error: {}", e))
        });
    match res {
        Ok(resp) => Json(resp).into_response(),
        Err(err) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({ "ok": false, "stderr": err })),
        )
            .into_response(),
    }
}

fn do_compile(state: AppState, source: String) -> Result<CompileResp, String> {
    // Cache hit? Same source -> reuse previous build.
    let src_hash = {
        use std::hash::{Hash, Hasher};
        let mut h = std::collections::hash_map::DefaultHasher::new();
        source.hash(&mut h);
        h.finish()
    };
    if let Some(existing_id) = state.source_cache.lock().unwrap().get(&src_hash).cloned() {
        if let Some(b) = state.builds.lock().unwrap().get(&existing_id) {
            if b.ok {
                return Ok(CompileResp {
                    id: existing_id.clone(),
                    ok: true,
                    stderr: format!("[cache hit]\n{}", b.stderr),
                    preview: format!("/preview/{}/", existing_id),
                    zip_source: format!("/zip/{}/source", existing_id),
                    zip_full: format!("/zip/{}/full", existing_id),
                });
            }
        }
    }
    let id = uuid::Uuid::new_v4().to_string();
    let workdir = std::env::temp_dir().join(format!("rapidr-build-{}", id));
    std::fs::create_dir_all(&workdir).map_err(|e| e.to_string())?;
    let source_rr = workdir.join("program.rr");
    std::fs::write(&source_rr, &source).map_err(|e| e.to_string())?;

    // Run the web compile pipeline.
    let out = Command::new(&state.rapidr)
        .args(["--web", source_rr.to_str().unwrap()])
        .current_dir(&state.repo_root)
        .output()
        .map_err(|e| format!("failed to spawn rapidr: {}", e))?;
    let stderr = format!(
        "{}{}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );

    // The compiler emits next to the source: <name>_rust/ and <name>_web/.
    let web_dir = workdir.join("program_web");
    let ok = out.status.success() && web_dir.exists();

    // Optional native binary build (best-effort; doesn't fail compile).
    let native_bin = if ok {
        let rust_dir = workdir.join("program_rust");
        if rust_dir.exists() {
            let st = Command::new("cargo")
                .args(["build", "--release"])
                .current_dir(&rust_dir)
                .output();
            if let Ok(o) = st {
                if o.status.success() {
                    let candidates = [
                        rust_dir.join("target/release/program"),
                        rust_dir.join("target/release/program.exe"),
                    ];
                    candidates.iter().find(|p| p.exists()).cloned()
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        }
    } else {
        None
    };

    let build = Build {
        workdir,
        source_rr,
        web_dir,
        native_bin,
        stderr: stderr.clone(),
        ok,
    };
    state
        .builds
        .lock()
        .unwrap()
        .insert(id.clone(), build);
    if ok {
        state.source_cache.lock().unwrap().insert(src_hash, id.clone());
    }

    Ok(CompileResp {
        id: id.clone(),
        ok,
        stderr,
        preview: format!("/preview/{}/", id),
        zip_source: format!("/zip/{}/source", id),
        zip_full: format!("/zip/{}/full", id),
    })
}

async fn preview_index(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    serve_preview_path(state, id, "index.html".into()).await
}

async fn preview(
    State(state): State<AppState>,
    Path((id, rest)): Path<(String, String)>,
) -> Response {
    let p = if rest.is_empty() { "index.html".into() } else { rest };
    serve_preview_path(state, id, p).await
}

async fn serve_preview_path(state: AppState, id: String, rel: String) -> Response {
    let dir = match state.builds.lock().unwrap().get(&id) {
        Some(b) if b.ok => b.web_dir.clone(),
        _ => return (StatusCode::NOT_FOUND, "build not found").into_response(),
    };
    if !is_safe_relative_path(&rel) {
        return (StatusCode::FORBIDDEN, "bad path").into_response();
    }
    let path = dir.join(&rel);
    match tokio::fs::read(&path).await {
        Ok(bytes) => {
            let mime = match path.extension().and_then(|s| s.to_str()) {
                Some("html") => "text/html; charset=utf-8",
                Some("js") => "application/javascript",
                Some("wasm") => "application/wasm",
                Some("css") => "text/css",
                Some("json") => "application/json",
                _ => "application/octet-stream",
            };
            Response::builder()
                .header(header::CONTENT_TYPE, mime)
                .body(Body::from(bytes))
                .unwrap()
        }
        Err(_) => (StatusCode::NOT_FOUND, "not found").into_response(),
    }
}

async fn zip_source(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let src = {
        let g = state.builds.lock().unwrap();
        match g.get(&id) {
            Some(b) => b.source_rr.clone(),
            None => return (StatusCode::NOT_FOUND, "build not found").into_response(),
        }
    };
    match build_zip(&[("program.rr".into(), std::fs::read(&src).unwrap_or_default())]) {
        Ok(bytes) => zip_response(bytes, "rapidr-source.zip"),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

async fn zip_full(State(state): State<AppState>, Path(id): Path<String>) -> Response {
    let build = {
        let g = state.builds.lock().unwrap();
        match g.get(&id) {
            Some(b) => b.clone(),
            None => return (StatusCode::NOT_FOUND, "build not found").into_response(),
        }
    };
    let mut entries: Vec<(String, Vec<u8>)> = Vec::new();
    if let Ok(b) = std::fs::read(&build.source_rr) {
        entries.push(("source/program.rr".into(), b));
    }
    if build.web_dir.exists() {
        for entry in walkdir::WalkDir::new(&build.web_dir).into_iter().flatten() {
            if entry.file_type().is_file() {
                if let Ok(rel) = entry.path().strip_prefix(&build.web_dir) {
                    if let Ok(b) = std::fs::read(entry.path()) {
                        entries.push((format!("web/{}", rel.display()), b));
                    }
                }
            }
        }
    }
    if let Some(bin) = build.native_bin.as_ref() {
        if let Ok(b) = std::fs::read(bin) {
            let name = bin
                .file_name()
                .map(|n| n.to_string_lossy().to_string())
                .unwrap_or_else(|| "program".into());
            entries.push((format!("bin/{}", name), b));
        }
    }
    // README explaining contents
    let readme = format!(
        "RapidR build bundle\n===================\n\n\
         build id: {}\n\
         web bundle: web/index.html\n\
         native binary: bin/ (if present)\n\
         source: source/program.rr\n",
        id
    );
    entries.push(("README.txt".into(), readme.into_bytes()));

    match build_zip(&entries) {
        Ok(bytes) => zip_response(bytes, "rapidr-bundle.zip"),
        Err(e) => (StatusCode::INTERNAL_SERVER_ERROR, e).into_response(),
    }
}

fn build_zip(entries: &[(String, Vec<u8>)]) -> Result<Vec<u8>, String> {
    let mut buf = Cursor::new(Vec::new());
    {
        let mut zw = zip::ZipWriter::new(&mut buf);
        let opts = zip::write::FileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        for (name, data) in entries {
            zw.start_file(name, opts).map_err(|e| e.to_string())?;
            zw.write_all(data).map_err(|e| e.to_string())?;
        }
        zw.finish().map_err(|e| e.to_string())?;
    }
    Ok(buf.into_inner())
}

fn zip_response(bytes: Vec<u8>, filename: &str) -> Response {
    Response::builder()
        .header(header::CONTENT_TYPE, "application/zip")
        .header(
            header::CONTENT_DISPOSITION,
            format!("attachment; filename=\"{}\"", filename),
        )
        .body(Body::from(bytes))
        .unwrap()
}

// Suppress "unused" for fields we keep for future use.
#[allow(dead_code)]
fn _keep(b: &Build) -> &PathBuf {
    &b.workdir
}
#[allow(dead_code)]
fn _keep2() -> ServeDir {
    ServeDir::new(".")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn loopback_hosts() {
        for h in ["localhost", "LOCALHOST:8095", "127.0.0.1", "127.0.0.1:8095", "[::1]:8095"] {
            assert!(is_loopback_host(h), "{h}");
        }
        for h in ["", "example.com", "evil.localhost.example.com", "192.168.1.5:8095", "0.0.0.0:8095"] {
            assert!(!is_loopback_host(h), "{h}");
        }
    }

    #[test]
    fn loopback_origins() {
        for o in ["http://localhost:8080", "http://127.0.0.1:8095", "https://[::1]:3000", "http://localhost"] {
            assert!(is_loopback_origin(o), "{o}");
        }
        for o in ["null", "http://evil.com", "http://localhost.evil.com", "file://", "http://localhost:8080/x", "ftp://localhost"] {
            assert!(!is_loopback_origin(o), "{o}");
        }
    }

    #[test]
    fn preview_paths() {
        assert!(is_safe_relative_path("index.html"));
        assert!(is_safe_relative_path("pkg/app_bg.wasm"));
        assert!(!is_safe_relative_path("../../etc/passwd"));
        assert!(!is_safe_relative_path("pkg/../../secret"));
        assert!(!is_safe_relative_path("/etc/passwd"));
    }
}
