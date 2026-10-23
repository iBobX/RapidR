//! Security regressions (docs/security/audit-preview4.md) for what RAI
//! sends and keeps. Compiled into rapidr-ai's tests (`cargo test -p
//! rapidr-ai security_`); only 127.0.0.1, never the network.
//!
//! - SEC-27: a provider's answer that redirects is not followed, so the
//!   key's header (`x-api-key`) never reaches the address it points to.
//! - SEC-28: the kept chats and the audit log are only the user's (0600
//!   files, 0700 folders, on Unix).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use crate::http::{run_blocking, Options};
use crate::wire::HttpRequest;

/// Reads a request's head from a connection (up to the blank line).
fn head(stream: &mut std::net::TcpStream) -> String {
    stream.set_read_timeout(Some(Duration::from_secs(5))).unwrap();
    let mut buf = Vec::new();
    let mut byte = [0u8; 1];
    while !buf.ends_with(b"\r\n\r\n") && buf.len() < 64 * 1024 {
        match stream.read(&mut byte) {
            Ok(1) => buf.push(byte[0]),
            _ => break,
        }
    }
    String::from_utf8_lossy(&buf).into_owned()
}

#[test]
fn sec27_a_redirect_is_not_followed_with_the_key() {
    // (the elsewhere a redirect points to: what reaches it, if anything)
    let elsewhere = TcpListener::bind("127.0.0.1:0").unwrap();
    let elsewhere_port = elsewhere.local_addr().unwrap().port();
    elsewhere.set_nonblocking(true).unwrap();
    let thief = std::thread::spawn(move || {
        let until = Instant::now() + Duration::from_secs(4);
        while Instant::now() < until {
            if let Ok((mut s, _)) = elsewhere.accept() {
                s.set_nonblocking(false).unwrap();
                let h = head(&mut s);
                let _ = s.write_all(b"HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
                return Some(h);
            }
            std::thread::sleep(Duration::from_millis(20));
        }
        None
    });
    // (the provider's address, answering with a redirect there)
    let provider = TcpListener::bind("127.0.0.1:0").unwrap();
    let provider_port = provider.local_addr().unwrap().port();
    let server = std::thread::spawn(move || {
        let (mut s, _) = provider.accept().unwrap();
        let _ = head(&mut s);
        let answer = format!("HTTP/1.1 307 Temporary Redirect\r\nLocation: http://127.0.0.1:{elsewhere_port}/steal\r\nContent-Length: 0\r\nConnection: close\r\n\r\n");
        let _ = s.write_all(answer.as_bytes());
    });
    let key = format!("{}{}", "sk", "-ant-test-not-a-real-key-0000");
    let req = HttpRequest { method: "GET", url: format!("http://127.0.0.1:{provider_port}/v1/models"), headers: vec![("x-api-key".into(), key.clone()), ("anthropic-version".into(), "2023-06-01".into())], body: None };
    let got = run_blocking(&req, Options { connect_timeout: 5, read_timeout: 5, retries: 0 }, |_| {});
    server.join().unwrap();
    let reached = thief.join().unwrap();
    assert!(reached.is_none(), "the redirect was followed: {:?}", reached.map(|h| h.contains(&key)));
    assert_eq!(got.map(|(status, _)| status), Ok(307));
    // (said in plain words)
    let e = crate::wire::explain_error(crate::provider("anthropic").unwrap(), 307, "", None, "");
    assert!(e.message.contains("another address") && e.message.contains("doesn't follow"), "{}", e.message);
}

#[cfg(unix)]
#[test]
fn sec28_chats_and_the_audit_log_are_only_the_users() {
    use std::os::unix::fs::PermissionsExt;
    let mode = |p: &std::path::Path| std::fs::metadata(p).unwrap().permissions().mode() & 0o777;
    let base = std::env::temp_dir().join(format!("rapidr-sec28-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&base);
    std::fs::create_dir_all(&base).unwrap();
    // (the helpers chats and the audit log write through)
    let dir = base.join("chats").join("Calc-1234");
    crate::private_files::make_dir(&dir, true).unwrap();
    assert_eq!(mode(&dir), 0o700);
    assert_eq!(mode(&base.join("chats")), 0o700);
    let chat = dir.join("a.json");
    crate::private_files::write(&chat, "{}").unwrap();
    assert_eq!(mode(&chat), 0o600);
    // (a file made before, readable by others: made the user's)
    let old = base.join("old.jsonl");
    std::fs::write(&old, "x\n").unwrap();
    std::fs::set_permissions(&old, std::fs::Permissions::from_mode(0o644)).unwrap();
    drop(crate::private_files::append(&old).unwrap());
    assert_eq!(mode(&old), 0o600);
    // (the audit log itself)
    let log = base.join("logs").join("ai-audit.jsonl");
    std::env::set_var("RAPIDR_AI_AUDIT", &log);
    crate::audit::append("{\"op\":\"test\"}");
    std::env::remove_var("RAPIDR_AI_AUDIT");
    assert_eq!(mode(&log), 0o600);
    assert_eq!(mode(&base.join("logs")), 0o700);
    let _ = std::fs::remove_dir_all(&base);
}
