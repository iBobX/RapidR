//! What arrives on a server's connection, cut into the pieces a program
//! gets: RServerSocket's lines (its Mode "lines", the default), what came
//! as it came ("bytes"), or whole HTTP requests ("http", and RHttpServer's
//! connections) — the head, then the body by its Content-Length or its
//! chunks, under size limits. Nothing that arrived is lost: a line or a
//! body still coming waits in the buffer however slowly it comes (a read's
//! timeout used to drop it), and a last line without its end is given when
//! the connection closes.
//!
//! Only bytes in, pieces out: the threads reading the sockets
//! (crate::network, crate::httpserver) feed it.

/// How a connection's bytes are cut.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Framing {
    /// Each line (to its `\n`), trimmed; blank lines dropped.
    Lines,
    /// What arrived, as text (a UTF-8 character split between two reads
    /// waits for its end).
    Bytes,
    /// Whole HTTP/1.x requests.
    Http(Limits),
}

impl Framing {
    /// RServerSocket's Mode, by name ("lines" for any other).
    pub fn by_name(mode: &str, limits: Limits) -> Framing {
        match mode.trim().to_ascii_lowercase().as_str() {
            "bytes" | "raw" => Framing::Bytes,
            "http" => Framing::Http(limits),
            _ => Framing::Lines,
        }
    }
}

/// An HTTP request's limits: its head (request line and headers) and body,
/// in bytes.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Limits {
    pub max_head: usize,
    pub max_body: usize,
}

impl Default for Limits {
    fn default() -> Self {
        Limits { max_head: 64 * 1024, max_body: 4 * 1024 * 1024 }
    }
}

/// One HTTP request, whole.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Request {
    pub method: String,
    /// The request target as sent: the path and the query (`/mcp?x=1`).
    pub target: String,
    /// `HTTP/1.1`, `HTTP/1.0`.
    pub version: String,
    /// Each header as sent (`Content-Type`, `application/json`), in order.
    pub headers: Vec<(String, String)>,
    pub body: Vec<u8>,
}

impl Request {
    /// The first header named `name` (any case).
    pub fn header(&self, name: &str) -> Option<&str> {
        self.headers.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| v.as_str())
    }

    /// The path, without the query.
    pub fn path(&self) -> &str {
        self.target.split_once('?').map_or(self.target.as_str(), |(p, _)| p)
    }

    /// The query (after `?`; "" without one).
    pub fn query(&self) -> &str {
        self.target.split_once('?').map_or("", |(_, q)| q)
    }

    /// Whether the connection stays open after the answer (HTTP/1.1 unless
    /// `Connection: close`; HTTP/1.0 only with `Connection: keep-alive`).
    pub fn keep_alive(&self) -> bool {
        let conn = self.header("connection").unwrap_or("").to_ascii_lowercase();
        let has = |t: &str| conn.split(',').any(|c| c.trim() == t);
        if self.version.eq_ignore_ascii_case("HTTP/1.0") {
            has("keep-alive")
        } else {
            !has("close")
        }
    }

    /// The headers as text: one `Name: value` a line, CR LF between.
    pub fn header_text(&self) -> String {
        self.headers.iter().map(|(n, v)| format!("{n}: {v}")).collect::<Vec<_>>().join("\r\n")
    }

    /// The body as text (UTF-8; a byte that isn't, U+FFFD).
    pub fn body_text(&self) -> String {
        String::from_utf8_lossy(&self.body).into_owned()
    }

    /// The whole request as text, as RServerSocket's "http" mode gives it:
    /// the request line, the headers (a chunked body's Transfer-Encoding
    /// replaced by its Content-Length, the body being joined), a blank
    /// line, the body.
    pub fn whole_text(&self) -> String {
        let mut out = format!("{} {} {}\r\n", self.method, self.target, self.version);
        let mut length = false;
        for (n, v) in &self.headers {
            if n.eq_ignore_ascii_case("transfer-encoding") {
                continue;
            }
            if n.eq_ignore_ascii_case("content-length") {
                length = true;
            }
            out.push_str(&format!("{n}: {v}\r\n"));
        }
        if !length && !self.body.is_empty() {
            out.push_str(&format!("Content-Length: {}\r\n", self.body.len()));
        }
        out.push_str("\r\n");
        out.push_str(&self.body_text());
        out
    }
}

/// What a connection's bytes made.
#[derive(Clone, Debug, PartialEq)]
pub enum Piece {
    /// A line, or what arrived (Lines, Bytes).
    Text(String),
    /// A whole request (Http).
    Request(Request),
    /// (Http) The client waits for `100 Continue` before sending the body
    /// it announced (`Expect: 100-continue`, curl's for bodies over 1 KB).
    /// The request so far (its head) comes with it: one the server refuses
    /// is refused before its body is invited.
    Continue(Request),
    /// (Http) What arrived isn't a request this server takes: the status to
    /// answer with, why, and the connection is closed.
    Refuse(u16, String),
}

#[derive(Clone, Debug)]
enum State {
    Head,
    /// So many more bytes of body.
    Body(Request, usize),
    /// A chunked body: its chunks so far; reading a size line, the data of
    /// one (`Some(left)`, then its CR LF), or the trailers (after size 0).
    Chunks(Request, Chunk),
    /// Refused: nothing more is read.
    Closed,
}

#[derive(Clone, Copy, Debug)]
enum Chunk {
    Size,
    Data(usize),
    DataEnd,
    Trailers,
}

/// A connection's bytes, cut as its [`Framing`] says.
pub struct Framer {
    framing: Framing,
    buf: Vec<u8>,
    state: State,
}

/// Whether `s` is an HTTP token (a method's, a header name's characters).
fn token(s: &str) -> bool {
    !s.is_empty() && s.bytes().all(|b| b.is_ascii_alphanumeric() || b"!#$%&'*+-.^_`|~".contains(&b))
}

fn find(hay: &[u8], needle: &[u8]) -> Option<usize> {
    hay.windows(needle.len()).position(|w| w == needle)
}

impl Framer {
    pub fn new(framing: Framing) -> Framer {
        Framer { framing, buf: Vec::new(), state: State::Head }
    }

    /// Whether part of something has arrived and waits for the rest (an
    /// HTTP request begun: the time it may take is limited).
    pub fn in_progress(&self) -> bool {
        match &self.state {
            State::Head => !self.buf.is_empty(),
            State::Body(..) | State::Chunks(..) => true,
            State::Closed => false,
        }
    }

    /// Bytes arrived: the pieces they complete.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Piece> {
        if matches!(self.state, State::Closed) {
            return Vec::new();
        }
        self.buf.extend_from_slice(bytes);
        match self.framing {
            Framing::Lines => self.lines(),
            Framing::Bytes => self.text(false),
            Framing::Http(limits) => self.http(limits),
        }
    }

    /// The connection closed: what is left (a last line without its end;
    /// the bytes' last piece). A request cut short is dropped.
    pub fn finish(&mut self) -> Vec<Piece> {
        match self.framing {
            Framing::Lines => {
                let rest = String::from_utf8_lossy(&std::mem::take(&mut self.buf)).trim().to_string();
                if rest.is_empty() { Vec::new() } else { vec![Piece::Text(rest)] }
            }
            Framing::Bytes => self.text(true),
            Framing::Http(_) => Vec::new(),
        }
    }

    fn lines(&mut self) -> Vec<Piece> {
        let mut out = Vec::new();
        while let Some(i) = self.buf.iter().position(|&b| b == b'\n') {
            let line: Vec<u8> = self.buf.drain(..=i).collect();
            let text = String::from_utf8_lossy(&line).trim().to_string();
            if !text.is_empty() {
                out.push(Piece::Text(text));
            }
        }
        out
    }

    /// The buffer as text, but a UTF-8 character cut at its end (unless
    /// `all`).
    fn text(&mut self, all: bool) -> Vec<Piece> {
        let keep = if all { 0 } else { incomplete_tail(&self.buf) };
        let take = self.buf.len() - keep;
        if take == 0 {
            return Vec::new();
        }
        let bytes: Vec<u8> = self.buf.drain(..take).collect();
        vec![Piece::Text(String::from_utf8_lossy(&bytes).into_owned())]
    }

    fn refuse(&mut self, status: u16, why: &str) -> Piece {
        self.state = State::Closed;
        self.buf.clear();
        Piece::Refuse(status, why.to_string())
    }

    fn http(&mut self, limits: Limits) -> Vec<Piece> {
        let mut out = Vec::new();
        loop {
            match std::mem::replace(&mut self.state, State::Closed) {
                State::Closed => return out,
                State::Head => {
                    // (blank lines before a request are skipped: RFC 9112 §2.2)
                    while self.buf.first().is_some_and(|b| *b == b'\r' || *b == b'\n') {
                        self.buf.remove(0);
                    }
                    let end = find(&self.buf, b"\r\n\r\n").map(|i| (i, 4)).or_else(|| find(&self.buf, b"\n\n").map(|i| (i, 2)));
                    let Some((at, sep)) = end else {
                        if self.buf.len() > limits.max_head {
                            out.push(self.refuse(431, "the request's headers are too large"));
                            return out;
                        }
                        self.state = State::Head;
                        return out;
                    };
                    if at > limits.max_head {
                        out.push(self.refuse(431, "the request's headers are too large"));
                        return out;
                    }
                    let head: Vec<u8> = self.buf.drain(..at + sep).collect();
                    let head = String::from_utf8_lossy(&head[..at]).into_owned();
                    match parse_head(&head) {
                        Err((status, why)) => {
                            out.push(self.refuse(status, why));
                            return out;
                        }
                        Ok((req, Body::None)) => {
                            out.push(Piece::Request(req));
                            self.state = State::Head;
                        }
                        Ok((req, Body::Length(n))) => {
                            if n > limits.max_body {
                                out.push(self.refuse(413, "the request's body is too large"));
                                return out;
                            }
                            if expects_continue(&req) {
                                out.push(Piece::Continue(req.clone()));
                            }
                            self.state = State::Body(req, n);
                        }
                        Ok((req, Body::Chunked)) => {
                            if expects_continue(&req) {
                                out.push(Piece::Continue(req.clone()));
                            }
                            self.state = State::Chunks(req, Chunk::Size);
                        }
                    }
                }
                State::Body(mut req, left) => {
                    let n = left.min(self.buf.len());
                    req.body.extend(self.buf.drain(..n));
                    if left - n > 0 {
                        self.state = State::Body(req, left - n);
                        return out;
                    }
                    out.push(Piece::Request(req));
                    self.state = State::Head;
                }
                State::Chunks(mut req, chunk) => match chunk {
                    Chunk::Size => {
                        let Some(i) = self.buf.iter().position(|&b| b == b'\n') else {
                            if self.buf.len() > 1024 {
                                out.push(self.refuse(400, "a chunk's size line is too long"));
                                return out;
                            }
                            self.state = State::Chunks(req, Chunk::Size);
                            return out;
                        };
                        let line: Vec<u8> = self.buf.drain(..=i).collect();
                        let line = String::from_utf8_lossy(&line).into_owned();
                        let size = line.trim().split(';').next().unwrap_or("").trim();
                        let Some(size) = usize::from_str_radix(size, 16).ok().filter(|_| !size.is_empty() && size.len() <= 16) else {
                            out.push(self.refuse(400, "a chunk's size isn't a number"));
                            return out;
                        };
                        if req.body.len().saturating_add(size) > limits.max_body {
                            out.push(self.refuse(413, "the request's body is too large"));
                            return out;
                        }
                        self.state = State::Chunks(req, if size == 0 { Chunk::Trailers } else { Chunk::Data(size) });
                    }
                    Chunk::Data(left) => {
                        let n = left.min(self.buf.len());
                        req.body.extend(self.buf.drain(..n));
                        self.state = State::Chunks(req, if left - n > 0 { Chunk::Data(left - n) } else { Chunk::DataEnd });
                        if left - n > 0 {
                            return out;
                        }
                    }
                    Chunk::DataEnd => {
                        if self.buf.len() < 2 && !self.buf.starts_with(b"\n") {
                            self.state = State::Chunks(req, Chunk::DataEnd);
                            return out;
                        }
                        let skip = if self.buf.starts_with(b"\r\n") {
                            2
                        } else if self.buf.starts_with(b"\n") {
                            1
                        } else {
                            out.push(self.refuse(400, "a chunk doesn't end where its size says"));
                            return out;
                        };
                        self.buf.drain(..skip);
                        self.state = State::Chunks(req, Chunk::Size);
                    }
                    Chunk::Trailers => {
                        let Some(i) = self.buf.iter().position(|&b| b == b'\n') else {
                            if self.buf.len() > limits.max_head {
                                out.push(self.refuse(431, "the request's trailers are too large"));
                                return out;
                            }
                            self.state = State::Chunks(req, Chunk::Trailers);
                            return out;
                        };
                        let line: Vec<u8> = self.buf.drain(..=i).collect();
                        if line.iter().all(|b| *b == b'\r' || *b == b'\n') {
                            out.push(Piece::Request(req));
                            self.state = State::Head;
                        } else {
                            self.state = State::Chunks(req, Chunk::Trailers);
                        }
                    }
                },
            }
        }
    }
}

/// How many bytes at the end of `buf` begin a UTF-8 character that isn't
/// whole yet.
fn incomplete_tail(buf: &[u8]) -> usize {
    for back in 1..=3.min(buf.len()) {
        let b = buf[buf.len() - back];
        if b & 0xC0 == 0x80 {
            continue;
        }
        let need = if b >= 0xF0 { 4 } else if b >= 0xE0 { 3 } else if b >= 0xC0 { 2 } else { 1 };
        return if need > back { back } else { 0 };
    }
    0
}

enum Body {
    None,
    Length(usize),
    Chunked,
}

fn expects_continue(req: &Request) -> bool {
    req.header("expect").is_some_and(|e| e.trim().eq_ignore_ascii_case("100-continue")) && !req.version.eq_ignore_ascii_case("HTTP/1.0")
}

/// A request's head (its request line and header lines, without the blank
/// line): the request and how its body comes, or the status to refuse it
/// with.
fn parse_head(head: &str) -> Result<(Request, Body), (u16, &'static str)> {
    let mut lines = head.split('\n').map(|l| l.strip_suffix('\r').unwrap_or(l));
    let first = lines.next().unwrap_or("");
    let mut parts = first.split(' ');
    let (method, target, version) = (parts.next().unwrap_or(""), parts.next().unwrap_or(""), parts.next().unwrap_or(""));
    if parts.next().is_some() || !token(method) || target.is_empty() || target.contains(|c: char| c.is_control()) {
        return Err((400, "not an HTTP request"));
    }
    if !version.starts_with("HTTP/1.") || version.len() != 8 {
        return Err((505, "only HTTP/1.0 and HTTP/1.1 are spoken here"));
    }
    let mut req = Request { method: method.to_string(), target: target.to_string(), version: version.to_string(), ..Default::default() };
    for line in lines {
        // (a header folded onto the next line: obsolete, refused — RFC 9112 §5.2)
        if line.starts_with(' ') || line.starts_with('\t') {
            return Err((400, "a header is folded over two lines"));
        }
        let Some((name, value)) = line.split_once(':') else { return Err((400, "a header line without a colon")) };
        if !token(name) {
            return Err((400, "a header's name isn't valid"));
        }
        req.headers.push((name.to_string(), value.trim().to_string()));
    }
    if req.version == "HTTP/1.1" && req.header("host").is_none() {
        return Err((400, "an HTTP/1.1 request without a Host header"));
    }
    let lengths: Vec<&str> = req.headers.iter().filter(|(n, _)| n.eq_ignore_ascii_case("content-length")).map(|(_, v)| v.as_str()).collect();
    if let Some(te) = req.header("transfer-encoding") {
        // (both: a request smuggled past a proxy, refused — RFC 9112 §6.1)
        if !lengths.is_empty() {
            return Err((400, "both Transfer-Encoding and Content-Length"));
        }
        let last = te.rsplit(',').next().unwrap_or("").trim();
        if !last.eq_ignore_ascii_case("chunked") || te.split(',').count() > 1 {
            return Err((501, "only chunked transfer is understood"));
        }
        return Ok((req, Body::Chunked));
    }
    match lengths.as_slice() {
        [] => Ok((req, Body::None)),
        [one] => match one.trim().parse::<usize>() {
            Ok(0) => Ok((req, Body::None)),
            Ok(n) if one.trim().bytes().all(|b| b.is_ascii_digit()) => Ok((req, Body::Length(n))),
            _ => Err((400, "Content-Length isn't a number")),
        },
        _ => {
            if lengths.iter().all(|l| l.trim() == lengths[0].trim()) {
                let n = lengths[0].trim().parse::<usize>().map_err(|_| (400, "Content-Length isn't a number"))?;
                Ok((req, if n == 0 { Body::None } else { Body::Length(n) }))
            } else {
                Err((400, "two different Content-Lengths"))
            }
        }
    }
}

/// HTTP's reason phrase for `status`.
pub fn reason(status: u16) -> &'static str {
    match status {
        100 => "Continue",
        200 => "OK",
        201 => "Created",
        202 => "Accepted",
        204 => "No Content",
        301 => "Moved Permanently",
        302 => "Found",
        304 => "Not Modified",
        400 => "Bad Request",
        401 => "Unauthorized",
        403 => "Forbidden",
        404 => "Not Found",
        405 => "Method Not Allowed",
        408 => "Request Timeout",
        409 => "Conflict",
        413 => "Content Too Large",
        415 => "Unsupported Media Type",
        429 => "Too Many Requests",
        431 => "Request Header Fields Too Large",
        500 => "Internal Server Error",
        501 => "Not Implemented",
        503 => "Service Unavailable",
        505 => "HTTP Version Not Supported",
        _ => "",
    }
}

/// The header lines a program wrote (`Name: value` lines, CR LF or LF
/// between) as pairs; lines without a colon, and a name or value with a
/// line break in it, dropped (a response can't be split by one).
pub fn header_lines(text: &str) -> Vec<(String, String)> {
    text.split('\n')
        .filter_map(|l| {
            let l = l.strip_suffix('\r').unwrap_or(l);
            let (n, v) = l.split_once(':')?;
            let (n, v) = (n.trim(), v.trim());
            (token(n) && !v.contains(['\r', '\n'])).then(|| (n.to_string(), v.to_string()))
        })
        .collect()
}

/// A response's head: the status line, the program's headers (those the
/// server sets itself — the body's length and framing, Connection — left
/// out), then the server's.
pub fn response_head(status: u16, headers: &[(String, String)], length: Option<usize>, chunked: bool, close: bool) -> String {
    let mut out = format!("HTTP/1.1 {status} {}\r\n", reason(status));
    let own = |n: &str| ["content-length", "transfer-encoding", "connection"].iter().any(|h| n.eq_ignore_ascii_case(h));
    for (n, v) in headers.iter().filter(|(n, _)| !own(n)) {
        out.push_str(&format!("{n}: {v}\r\n"));
    }
    if let Some(n) = length {
        out.push_str(&format!("Content-Length: {n}\r\n"));
    }
    if chunked {
        out.push_str("Transfer-Encoding: chunked\r\n");
    }
    if close {
        out.push_str("Connection: close\r\n");
    }
    out.push_str("\r\n");
    out
}

/// A Server-Sent Event: `event:` (when named), a `data:` line for each of
/// the data's lines, a blank line.
pub fn sse_event(data: &str, event: &str) -> String {
    let mut out = String::new();
    let event = event.trim();
    if !event.is_empty() && !event.contains(['\r', '\n']) {
        out.push_str(&format!("event: {event}\n"));
    }
    for line in data.split('\n') {
        out.push_str("data: ");
        out.push_str(line.strip_suffix('\r').unwrap_or(line));
        out.push('\n');
    }
    out.push('\n');
    out
}

/// `bytes` as one chunk of a chunked body (none for nothing: that would
/// end it).
pub fn chunk(bytes: &[u8]) -> Vec<u8> {
    if bytes.is_empty() {
        return Vec::new();
    }
    let mut out = format!("{:x}\r\n", bytes.len()).into_bytes();
    out.extend_from_slice(bytes);
    out.extend_from_slice(b"\r\n");
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn http() -> Framer {
        Framer::new(Framing::Http(Limits { max_head: 1024, max_body: 2 * 1024 * 1024 }))
    }

    fn requests(pieces: Vec<Piece>) -> Vec<Request> {
        pieces.into_iter().filter_map(|p| if let Piece::Request(r) = p { Some(r) } else { None }).collect()
    }

    #[test]
    fn lines_keep_a_line_that_comes_slowly_and_the_last_one_at_the_end() {
        let mut f = Framer::new(Framing::Lines);
        assert_eq!(f.feed(b"hel"), vec![]);
        assert_eq!(f.feed(b"lo\r\n\r\n  \nwor"), vec![Piece::Text("hello".into())], "trimmed, blank lines dropped");
        assert_eq!(f.feed(b"ld"), vec![]);
        assert_eq!(f.finish(), vec![Piece::Text("world".into())], "the last line, without its end, at the close");
    }

    #[test]
    fn bytes_come_as_they_are_a_character_split_waits() {
        let mut f = Framer::new(Framing::Bytes);
        let e = "é".as_bytes();
        assert_eq!(f.feed(&[b' ', b'a', e[0]]), vec![Piece::Text(" a".into())]);
        assert_eq!(f.feed(&[e[1], b'\n']), vec![Piece::Text("é\n".into())]);
        assert_eq!(f.feed(b""), vec![]);
    }

    #[test]
    fn a_post_body_by_its_length_arrives_whole_however_it_is_cut() {
        let body = "x".repeat(100_000);
        let wire = format!("POST /echo?a=1 HTTP/1.1\r\nHost: 127.0.0.1:9\r\nContent-Length: {}\r\n\r\n{body}GET / HTTP/1.1\r\nHost: h\r\n\r\n", body.len());
        let mut f = http();
        let mut got = Vec::new();
        for part in wire.as_bytes().chunks(777) {
            got.extend(requests(f.feed(part)));
        }
        assert_eq!(got.len(), 2, "two requests, the second pipelined");
        assert_eq!(got[0].method, "POST");
        assert_eq!(got[0].path(), "/echo");
        assert_eq!(got[0].query(), "a=1");
        assert_eq!(got[0].body.len(), 100_000);
        assert_eq!(got[0].header("content-length"), Some("100000"));
        assert_eq!(got[1].method, "GET");
        assert!(!f.in_progress());
    }

    #[test]
    fn a_chunked_body_is_joined() {
        let wire = b"POST /mcp HTTP/1.1\r\nHost: h\r\nTransfer-Encoding: chunked\r\nExpect: 100-continue\r\n\r\n5;ext=1\r\nhello\r\n7\r\n, world\r\n0\r\nX-Trailer: 1\r\n\r\n";
        let mut f = http();
        let mut pieces = Vec::new();
        for b in wire.iter() {
            pieces.extend(f.feed(&[*b]));
        }
        assert!(matches!(&pieces[0], Piece::Continue(h) if h.path() == "/mcp"), "the client waits for 100 Continue");
        let r = requests(pieces);
        assert_eq!(r.len(), 1);
        assert_eq!(r[0].body_text(), "hello, world");
        let whole = r[0].whole_text();
        assert!(whole.contains("Content-Length: 12\r\n") && !whole.to_ascii_lowercase().contains("transfer-encoding"), "{whole}");
        assert!(whole.ends_with("\r\n\r\nhello, world"));
    }

    #[test]
    fn what_is_too_big_or_malformed_is_refused() {
        let refusal = |wire: &[u8]| http().feed(wire).into_iter().find_map(|p| if let Piece::Refuse(s, _) = p { Some(s) } else { None });
        assert_eq!(refusal(b"POST / HTTP/1.1\r\nHost: h\r\nContent-Length: 99999999\r\n\r\n"), Some(413));
        assert_eq!(refusal(&[b'a'; 2000]), Some(431));
        assert_eq!(refusal(b"GET / HTTP/1.1\r\n\r\n"), Some(400), "no Host");
        assert_eq!(refusal(b"GET / HTTP/2.0\r\nHost: h\r\n\r\n"), Some(505));
        assert_eq!(refusal(b"POST / HTTP/1.1\r\nHost: h\r\nContent-Length: 5\r\nTransfer-Encoding: chunked\r\n\r\n"), Some(400));
        assert_eq!(refusal(b"POST / HTTP/1.1\r\nHost: h\r\nContent-Length: 5\r\nContent-Length: 6\r\n\r\n"), Some(400));
        assert_eq!(refusal(b"POST / HTTP/1.1\r\nHost: h\r\nTransfer-Encoding: gzip\r\n\r\n"), Some(501));
        assert_eq!(refusal(b"POST / HTTP/1.1\r\nHost: h\r\nTransfer-Encoding: chunked\r\n\r\nzz\r\n"), Some(400));
        assert_eq!(refusal(b"GET / HTTP/1.1\r\nHost: h\r\n folded\r\n\r\n"), Some(400));
    }

    #[test]
    fn responses_and_events_are_framed() {
        let h = header_lines("Content-Type: application/json\nX-Bad\r\nContent-Length: 1\nMcp-Session-Id: abc");
        assert_eq!(h.len(), 3);
        let head = response_head(200, &h, Some(5), false, true);
        assert!(head.starts_with("HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nMcp-Session-Id: abc\r\nContent-Length: 5\r\nConnection: close\r\n\r\n"), "{head}");
        assert_eq!(sse_event("{\"a\":1}\nsecond", "message"), "event: message\ndata: {\"a\":1}\ndata: second\n\n");
        assert_eq!(chunk(b"hi"), b"2\r\nhi\r\n");
        assert!(chunk(b"").is_empty());
        let r = Request { version: "HTTP/1.0".into(), ..Default::default() };
        assert!(!r.keep_alive());
        let r = Request { version: "HTTP/1.1".into(), headers: vec![("Connection".into(), "Close".into())], ..Default::default() };
        assert!(!r.keep_alive());
    }
}
