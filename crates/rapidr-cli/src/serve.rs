//! `rapidr serve <file.rr|.bas> [--open] [--port N]`: the program as its web
//! build, served on this machine only, and opened in the default browser —
//! RapidR Studio's Run ▸ Run in Browser (docs/studio-wow.md RUN-2).
//!
//! The bundle `rapidr build --web --interp` would zip is kept in memory and
//! served from it. The server:
//! - listens on 127.0.0.1 only, on a port the system picks (or `--port`);
//! - answers only under a random path (`/<token>/`), so another page or
//!   user on the machine can't guess the address;
//! - refuses a request whose `Host` isn't its own loopback address (a DNS
//!   rebinding site), and anything but GET / HEAD;
//! - sends the bundle's Content-Security-Policy and `nosniff`.
//!
//! It prints `Serving <url>` once it listens, and ends when its standard
//! input closes (the IDE that started it went away) or it's killed.

use std::collections::HashMap;
use std::hash::{BuildHasher, Hasher};
use std::io::{BufRead, BufReader, Read, Write};
use std::net::{TcpListener, TcpStream};
use std::process::ExitCode;
use std::sync::Arc;
use std::time::Duration;

/// A path segment nobody can guess: 128 bits from the system's random
/// hasher keys (no crypto crate: it names a page, it guards nothing else).
fn token() -> String {
    let mut out = String::new();
    for i in 0..2u64 {
        let mut h = std::collections::hash_map::RandomState::new().build_hasher();
        h.write_u64(i);
        h.write_u128(std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map_or(0, |d| d.as_nanos()));
        out.push_str(&format!("{:016x}", h.finish()));
    }
    out
}

fn content_type(name: &str) -> &'static str {
    match name.rsplit('.').next().unwrap_or("").to_ascii_lowercase().as_str() {
        "html" | "htm" => "text/html; charset=utf-8",
        "js" | "mjs" => "text/javascript; charset=utf-8",
        "wasm" => "application/wasm",
        "json" => "application/json",
        "css" => "text/css; charset=utf-8",
        "txt" => "text/plain; charset=utf-8",
        "svg" => "image/svg+xml",
        "png" => "image/png",
        "jpg" | "jpeg" => "image/jpeg",
        "gif" => "image/gif",
        "ico" => "image/x-icon",
        "otf" => "font/otf",
        "ttf" => "font/ttf",
        "woff2" => "font/woff2",
        _ => "application/octet-stream",
    }
}

struct Site {
    files: HashMap<String, Vec<u8>>,
    token: String,
    port: u16,
    csp: String,
}

/// One request: the file under `/<token>/`, or a refusal.
fn answer(site: &Site, mut stream: TcpStream) {
    let _ = stream.set_read_timeout(Some(Duration::from_secs(10)));
    let mut reader = BufReader::new(match stream.try_clone() {
        Ok(s) => s,
        Err(_) => return,
    });
    let mut first = String::new();
    if reader.read_line(&mut first).is_err() {
        return;
    }
    let mut host = String::new();
    loop {
        let mut line = String::new();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let line = line.trim_end();
        if line.is_empty() {
            break;
        }
        if let Some((k, v)) = line.split_once(':') {
            if k.trim().eq_ignore_ascii_case("host") {
                host = v.trim().to_ascii_lowercase();
            }
        }
    }
    let mut parts = first.split_whitespace();
    let (method, target) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    let ours = [format!("127.0.0.1:{}", site.port), format!("localhost:{}", site.port)];
    let reply = |stream: &mut TcpStream, status: &str, kind: &str, body: &[u8], head_only: bool| {
        let mut head = format!("HTTP/1.1 {status}\r\nContent-Type: {kind}\r\nContent-Length: {}\r\nCache-Control: no-store\r\nX-Content-Type-Options: nosniff\r\nReferrer-Policy: no-referrer\r\nConnection: close\r\n", body.len());
        if kind.starts_with("text/html") {
            head.push_str(&format!("Content-Security-Policy: {}\r\n", site.csp));
        }
        head.push_str("\r\n");
        let _ = stream.write_all(head.as_bytes());
        if !head_only {
            let _ = stream.write_all(body);
        }
    };
    if !ours.contains(&host) {
        reply(&mut stream, "421 Misdirected Request", "text/plain; charset=utf-8", b"not this server's address\n", false);
        return;
    }
    if method != "GET" && method != "HEAD" {
        reply(&mut stream, "405 Method Not Allowed", "text/plain; charset=utf-8", b"GET only\n", false);
        return;
    }
    let path = target.split(['?', '#']).next().unwrap_or("");
    let prefix = format!("/{}/", site.token);
    let Some(rest) = path.strip_prefix(&prefix) else {
        reply(&mut stream, "404 Not Found", "text/plain; charset=utf-8", b"not found\n", false);
        return;
    };
    let name = if rest.is_empty() { "index.html" } else { rest };
    match site.files.get(name) {
        Some(data) => reply(&mut stream, "200 OK", content_type(name), data, method == "HEAD"),
        None => reply(&mut stream, "404 Not Found", "text/plain; charset=utf-8", b"not found\n", false),
    }
}

/// Opens `url` in the system's default browser.
fn open_browser(url: &str) -> Result<(), String> {
    use std::process::{Command, Stdio};
    let mut cmd = if cfg!(target_os = "macos") {
        let mut c = Command::new("open");
        c.arg(url);
        c
    } else if cfg!(windows) {
        // (rundll32 takes the URL as one argument: no shell parses it)
        let mut c = Command::new("rundll32");
        c.arg("url.dll,FileProtocolHandler").arg(url);
        c
    } else {
        let mut c = Command::new("xdg-open");
        c.arg(url);
        c
    };
    cmd.stdin(Stdio::null()).stdout(Stdio::null()).stderr(Stdio::null());
    cmd.spawn().map(drop).map_err(|e| format!("can't open the browser: {e}"))
}

pub fn run(args: &[String]) -> ExitCode {
    let mut file = None;
    let mut open = false;
    let mut port: u16 = 0;
    let mut i = 0;
    while i < args.len() {
        match args[i].as_str() {
            "--open" => open = true,
            "--port" => {
                i += 1;
                port = args.get(i).and_then(|p| p.parse().ok()).unwrap_or(0);
            }
            a if !a.starts_with('-') && file.is_none() => file = Some(a.to_string()),
            a => {
                eprintln!("rapidr serve: unknown option {a}");
                return ExitCode::from(2);
            }
        }
        i += 1;
    }
    let Some(file) = file else {
        eprintln!("rapidr serve <file.rr|.bas> [--open] [--port N]");
        return ExitCode::from(2);
    };
    let started = std::time::Instant::now();
    let web = match crate::web_bundle(&file, None, None, None) {
        Ok(w) => w,
        Err((e, code)) => {
            eprintln!("{e}");
            return ExitCode::from(code);
        }
    };
    let files = match rapidr_webbundle::bundle_files(&web.inputs()) {
        Ok(f) => f,
        Err(e) => {
            eprintln!("bundle error: {e}");
            return ExitCode::from(1);
        }
    };
    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("rapidr serve: can't listen on 127.0.0.1: {e}");
            return ExitCode::from(1);
        }
    };
    let port = listener.local_addr().map_or(port, |a| a.port());
    let site = Arc::new(Site {
        files: files.into_iter().collect(),
        token: token(),
        port,
        csp: rapidr_webbundle::content_security_policy(&web.needs),
    });
    let url = format!("http://127.0.0.1:{port}/{}/", site.token);
    println!("Serving {url} (built in {} ms)", started.elapsed().as_millis());
    let _ = std::io::stdout().flush();
    // (the IDE that started this went away: so does the server)
    std::thread::spawn(|| {
        let mut sink = [0u8; 256];
        let mut stdin = std::io::stdin();
        while matches!(stdin.read(&mut sink), Ok(n) if n > 0) {}
        std::process::exit(0);
    });
    if open && std::env::var_os("RAPIDR_NO_BROWSER").is_none() {
        if let Err(e) = open_browser(&url) {
            eprintln!("{e}");
        }
    }
    for stream in listener.incoming().flatten() {
        let site = Arc::clone(&site);
        std::thread::spawn(move || answer(&site, stream));
    }
    ExitCode::SUCCESS
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tokens_differ_and_are_long() {
        let (a, b) = (token(), token());
        assert_eq!(a.len(), 32);
        assert_ne!(a, b);
    }

    #[test]
    fn types() {
        assert_eq!(content_type("rapidrintr_bg.wasm"), "application/wasm");
        assert!(content_type("index.html").starts_with("text/html"));
        assert_eq!(content_type("x.rrbc"), "application/octet-stream");
    }
}
