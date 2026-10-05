//! QDOWNLOAD — Andreas Fink's `Qdownload.inc` (v2; the manual's Appendix
//! A): a file fetched over HTTP into a string (OutVar) or a file
//! (OutFile), its progress in State (percent) or in its own QGAUGE
//! (StateGauge) and QLABEL (SpeedLbl). The library did it with RapidQ's
//! QSOCKET, an `HTTP/1.0` GET, blocking; RapidR fetches with the runtime's
//! HTTP client (ureq on the desktop, `fetch` in the browser) while the
//! program waits as it waits for a dialog — its timers tick, its windows
//! paint, the gauge moves — and LeechFile returns when it's over.
//!
//! Here: the properties, the library's checks and messages (errors 1 … 11),
//! Percent / Speed / Check, and what a transfer does to them
//! ([`Download::begin`], [`Download::progress`], [`Download::finish`]); the
//! runtime does the transfer. docs/io-media-plan.md §3.

use crate::{v_int, v_str, Value};

/// The library's error texts, by LastError.
pub fn error_text(code: i64) -> &'static str {
    match code {
        1 => "No Server Specified",
        2 => "No Serverport Specified",
        3 => "No File Specified",
        4 => "No Outputfile Specified",
        5 => "No Connection to specified host could be established",
        6 => "You are not connected to the internet",
        7 => "Connection closed by peer",
        8 => "No idea what's wrong",
        9 => "Filesize couldn't be determined, or file is 0 bytes long",
        10 => "A transmission error has occured, retry",
        11 => "The Server doesn't know the file",
        _ => "",
    }
}

#[derive(Debug, Clone)]
pub struct Download {
    pub server: String,
    pub port: i64,
    pub file: String,
    pub outfile: String,
    pub outvar: String,
    pub outdevice: i64,
    pub statedevice: i64,
    pub state: i64,
    pub size: i64,
    pub last_error: i64,
    pub last_string_error: String,
    /// TIME$ when the transfer began (Speed's `before$`).
    started: String,
}

impl Default for Download {
    fn default() -> Self {
        Download {
            server: String::new(),
            port: 80,
            file: String::new(),
            outfile: String::new(),
            outvar: String::new(),
            outdevice: 1,
            statedevice: 1,
            state: 0,
            size: 0,
            last_error: 0,
            last_string_error: String::new(),
            started: String::new(),
        }
    }
}

/// What the runtime fetches.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// `http://server:port/file` (`https://` on port 443).
    pub url: String,
}

/// How a transfer ended.
#[derive(Debug, Clone, PartialEq)]
pub enum Outcome {
    /// The response: its status, Content-Length (when it had one), body.
    Response { status: u16, length: Option<u64>, body: Vec<u8> },
    /// No connection could be made.
    NoConnection,
    /// The connection broke.
    Closed,
}

/// The progress shown, for the runtime to put on the gauge and the label.
#[derive(Debug, Clone, PartialEq)]
pub enum Shown {
    /// State (StateDevice 1).
    State,
    /// StateGauge.Position and SpeedLbl.Caption (StateDevice 2).
    Gauge { position: i64, speed: String },
}

/// The library's `percent(now, complete)`: ROUND(now * 100 / complete).
pub fn percent(now: i64, complete: i64) -> i64 {
    if complete == 0 {
        return 0;
    }
    crate::numeric::round_to_int(now as f64 * 100.0 / complete as f64)
}

/// The seconds of a TIME$ ("HH:MM:SS").
fn seconds(t: &str) -> i64 {
    let field = |i: usize| t.split(':').nth(i).map_or(0.0, super::cgi::leading_number) as i64;
    field(2) + field(1) * 60 + field(0) * 3600
}

/// The library's `speed(now$, before$, complete)`: bytes a second between
/// two TIME$s (within the first second, the bytes so far).
pub fn speed(now: &str, before: &str, complete: i64) -> i64 {
    let sec = seconds(now) - seconds(before);
    if sec <= 0 {
        return complete;
    }
    crate::numeric::round_to_int(complete as f64 / sec as f64)
}

impl Download {
    pub fn get(&self, prop: &str) -> Option<Value> {
        Some(match prop {
            "server" => v_str(&self.server),
            "port" => v_int(self.port),
            "file" => v_str(&self.file),
            "outfile" => v_str(&self.outfile),
            "outvar" => v_str(&self.outvar),
            "outdevice" => v_int(self.outdevice),
            "statedevice" => v_int(self.statedevice),
            "state" => v_int(self.state),
            "size" => v_int(self.size),
            "lasterror" => v_int(self.last_error),
            "laststringerror" => v_str(&self.last_string_error),
            // (a function called without parentheses: there is a connection
            // — the library's dial-up flag no longer exists)
            "check" => v_int(1),
            _ => return None,
        })
    }

    pub fn set(&mut self, prop: &str, val: &Value) -> Option<()> {
        match prop {
            "server" => self.server = val.to_string_val(),
            "port" => self.port = val.to_i64(),
            "file" => self.file = val.to_string_val(),
            "outfile" => self.outfile = val.to_string_val(),
            "outvar" => self.outvar = val.to_string_val(),
            "outdevice" => self.outdevice = val.to_i64(),
            "statedevice" => self.statedevice = val.to_i64(),
            "state" => self.state = val.to_i64(),
            "size" => self.size = val.to_i64(),
            "lasterror" => self.last_error = val.to_i64(),
            "laststringerror" => self.last_string_error = val.to_string_val(),
            _ => return None,
        }
        Some(())
    }

    /// Check, Percent, Speed (LeechFile is the runtime's: [`Self::begin`]).
    pub fn call(&mut self, method: &str, args: &[Value]) -> Option<Value> {
        let arg = |i: usize| args.get(i).cloned().unwrap_or(Value::Null);
        Some(match method {
            "check" => v_int(1),
            "percent" => v_int(percent(arg(0).to_i64(), arg(1).to_i64())),
            "speed" => v_int(speed(&arg(0).to_string_val(), &arg(1).to_string_val(), arg(2).to_i64())),
            _ => return None,
        })
    }

    fn fail(&mut self, code: i64) -> i64 {
        self.last_error = code;
        self.last_string_error = error_text(code).to_string();
        0
    }

    /// LeechFile's start: the library's checks (errors 1 … 4, LeechFile
    /// returning False), else what to fetch. `now` is TIME$.
    pub fn begin(&mut self, now: &str) -> Result<Request, i64> {
        if self.server.is_empty() {
            return Err(self.fail(1));
        }
        if self.port == 0 {
            return Err(self.fail(2));
        }
        if self.file.is_empty() {
            return Err(self.fail(3));
        }
        if self.outdevice == 2 && self.outfile.is_empty() {
            return Err(self.fail(4));
        }
        self.started = now.to_string();
        let scheme = if self.port == 443 { "https" } else { "http" };
        let default = if self.port == 443 { 443 } else { 80 };
        let host = if self.port == default { self.server.clone() } else { format!("{}:{}", self.server, self.port) };
        Ok(Request { url: format!("{scheme}://{host}/{}", self.file) })
    }

    /// Bytes `received` of a response of `length` (its Content-Length, if
    /// it said): Size and State, or what the gauge and the label show.
    pub fn progress(&mut self, received: u64, length: Option<u64>, now: &str) -> Shown {
        if let Some(l) = length {
            self.size = l as i64;
        }
        let complete = if length.is_some() { self.size } else { received as i64 };
        let p = percent(received as i64, complete);
        if self.statedevice == 2 {
            Shown::Gauge { position: p, speed: format!("{} B/s", speed(now, &self.started, received as i64)) }
        } else {
            self.state = p;
            Shown::State
        }
    }

    /// The transfer's end: LeechFile's result (1 True, 0 False) and, for
    /// OutDevice 2, the file to write.
    pub fn finish(&mut self, outcome: Outcome) -> (i64, Option<(String, Vec<u8>)>) {
        let (status, length, body) = match outcome {
            Outcome::NoConnection => return (self.fail(5), None),
            Outcome::Closed => return (self.fail(7), None),
            Outcome::Response { status, length, body } => (status, length, body),
        };
        if status == 404 {
            return (self.fail(11), None);
        }
        match length {
            Some(0) => return (self.fail(9), None),
            Some(l) => self.size = l as i64,
            // (no Content-Length: the library failed (10); RapidR takes
            // what came — docs/io-media-plan.md)
            None => self.size = body.len() as i64,
        }
        if self.size != body.len() as i64 {
            return (self.fail(10), None);
        }
        if self.outdevice == 2 {
            (1, Some((self.outfile.clone(), body)))
        } else {
            self.outvar = body.iter().map(|&b| char::from(b)).collect();
            (1, None)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn checks_and_results() {
        let mut d = Download::default();
        assert_eq!(d.begin("10:00:00"), Err(0));
        assert_eq!((d.last_error, d.last_string_error.as_str()), (1, "No Server Specified"));
        d.server = "example.com".into();
        d.port = 0;
        assert_eq!(d.begin("10:00:00"), Err(0));
        assert_eq!(d.last_error, 2);
        d.port = 8080;
        d.file = "a/b.txt".into();
        d.outdevice = 2;
        assert_eq!(d.begin("10:00:00"), Err(0));
        assert_eq!(d.last_error, 4);
        d.outdevice = 1;
        assert_eq!(d.begin("10:00:00").unwrap().url, "http://example.com:8080/a/b.txt");
        assert_eq!(d.progress(5, Some(10), "10:00:00"), Shown::State);
        assert_eq!(d.state, 50);
        assert_eq!(d.finish(Outcome::Response { status: 200, length: Some(3), body: b"abc".to_vec() }), (1, None));
        assert_eq!(d.outvar, "abc");
        assert_eq!(d.finish(Outcome::Response { status: 200, length: Some(4), body: b"abc".to_vec() }).0, 0);
        assert_eq!(d.last_error, 10);
        assert_eq!(d.finish(Outcome::Response { status: 404, length: Some(4), body: b"nope".to_vec() }).0, 0);
        assert_eq!(d.last_string_error, "The Server doesn't know the file");
        assert_eq!(d.finish(Outcome::NoConnection).0, 0);
        assert_eq!(d.last_error, 5);
        d.port = 443;
        assert_eq!(d.begin("10:00:00").unwrap().url, "https://example.com/a/b.txt");
        d.statedevice = 2;
        assert_eq!(d.progress(300, Some(1000), "10:00:02"), Shown::Gauge { position: 30, speed: "150 B/s".into() });
    }

    #[test]
    fn library_functions() {
        assert_eq!(percent(1, 3), 33);
        assert_eq!(percent(5, 0), 0);
        assert_eq!(speed("01:00:10", "01:00:00", 1000), 100);
        assert_eq!(speed("01:00:00", "01:00:00", 1000), 1000);
    }
}
