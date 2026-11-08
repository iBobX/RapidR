//! The download: one GET at a time, with progress and a size limit.
//! Native only (the browser has its own `fetch`). What it brings is
//! checked afterwards by verify.rs; nothing here decides whether to trust it.

use std::io::Read;
use std::time::Duration;

/// Why a GET gave nothing.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum FetchError {
    /// The host answered 404 (or 410): nothing is published there.
    NotFound,
    /// The host answered with another error status.
    Status(u16),
    /// No answer: no network, no such host, the connection dropped, a
    /// timeout. The text is the system's.
    Unreachable(String),
    /// More bytes than were allowed (the list says how big the file is).
    TooBig,
}

/// A way to GET an address. [`UreqFetch`] is the real one; the tests have
/// their own.
pub trait Fetch {
    /// The body at `url`, at most `max` bytes. `progress(done, total)` is
    /// called as bytes come (`total` is the host's `Content-Length`, else
    /// `max`).
    fn get(&self, url: &str, max: u64, progress: &mut dyn FnMut(u64, u64)) -> Result<Vec<u8>, FetchError>;
}

/// HTTPS with the system's TLS (the same ureq the AI providers and the
/// network components use).
pub struct UreqFetch {
    agent: ureq::Agent,
}

impl Default for UreqFetch {
    fn default() -> Self {
        Self::new()
    }
}

impl UreqFetch {
    pub fn new() -> UreqFetch {
        let agent = ureq::AgentBuilder::new()
            .timeout_connect(Duration::from_secs(10))
            // (between two reads, not for the whole file: a big runner on a slow line is fine)
            .timeout_read(Duration::from_secs(30))
            .redirects(3)
            .user_agent(&format!("RapidR/{}", env!("CARGO_PKG_VERSION")))
            .build();
        UreqFetch { agent }
    }
}

impl Fetch for UreqFetch {
    fn get(&self, url: &str, max: u64, progress: &mut dyn FnMut(u64, u64)) -> Result<Vec<u8>, FetchError> {
        let response = match self.agent.get(url).call() {
            Ok(r) => r,
            Err(ureq::Error::Status(404 | 410, _)) => return Err(FetchError::NotFound),
            Err(ureq::Error::Status(code, _)) => return Err(FetchError::Status(code)),
            Err(ureq::Error::Transport(t)) => return Err(FetchError::Unreachable(t.to_string())),
        };
        let announced = response.header("Content-Length").and_then(|v| v.trim().parse::<u64>().ok());
        if announced.is_some_and(|n| n > max) {
            return Err(FetchError::TooBig);
        }
        let total = announced.unwrap_or(max);
        let mut reader = response.into_reader();
        let mut body = Vec::with_capacity(announced.unwrap_or(0).min(max) as usize);
        let mut chunk = vec![0u8; 64 * 1024];
        let mut reported = 0u64;
        progress(0, total);
        loop {
            let n = reader.read(&mut chunk).map_err(|e| FetchError::Unreachable(e.to_string()))?;
            if n == 0 {
                break;
            }
            if body.len() as u64 + n as u64 > max {
                return Err(FetchError::TooBig);
            }
            body.extend_from_slice(&chunk[..n]);
            // (a report about every 256 KB, not for every 64 KB read)
            if body.len() as u64 - reported >= 256 * 1024 {
                reported = body.len() as u64;
                progress(reported, total);
            }
        }
        progress(body.len() as u64, body.len() as u64);
        Ok(body)
    }
}
