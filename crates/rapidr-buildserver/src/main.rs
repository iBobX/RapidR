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

use axum::{
    body::Body,
    extract::{Path, State},
    http::{header, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use std::{
    collections::HashMap,
    io::{Cursor, Write},
    net::SocketAddr,
    path::PathBuf,
    process::Command,
    sync::{Arc, Mutex},
};
use tower_http::{cors::CorsLayer, services::ServeDir};

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
        .layer(CorsLayer::permissive())
        .with_state(state);

    let port: u16 = std::env::var("RAPIDR_BUILDSERVER_PORT")
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or(8095);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    println!("rapidr-buildserver listening on http://{}", addr);
    let listener = tokio::net::TcpListener::bind(addr).await.unwrap();
    axum::serve(listener, app).await.unwrap();
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
    let path = dir.join(&rel);
    if !path.starts_with(&dir) {
        return (StatusCode::FORBIDDEN, "bad path").into_response();
    }
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
