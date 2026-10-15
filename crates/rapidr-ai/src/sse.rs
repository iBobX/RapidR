//! Server-sent events (the WHATWG format both providers stream in): bytes
//! as they come, in any pieces, into events — an event name (`""` when the
//! stream names none) and its data (several `data:` lines joined by `\n`).

#[derive(Clone, Debug, PartialEq)]
pub struct SseEvent {
    pub event: String,
    pub data: String,
}

#[derive(Default)]
pub struct SseParser {
    /// The bytes of a line not ended yet (a UTF-8 character may be cut
    /// between two pieces: decoded once the line is whole).
    pending: Vec<u8>,
    /// A CR ended the last line: an LF right after it belongs to it.
    skip_lf: bool,
    event: String,
    data: Vec<String>,
}

impl SseParser {
    pub fn new() -> SseParser {
        SseParser::default()
    }

    /// The events `bytes` completes.
    pub fn feed(&mut self, bytes: &[u8]) -> Vec<SseEvent> {
        let mut out = Vec::new();
        for &b in bytes {
            if std::mem::take(&mut self.skip_lf) && b == b'\n' {
                continue;
            }
            match b {
                b'\n' => self.end_line(&mut out),
                b'\r' => {
                    self.end_line(&mut out);
                    self.skip_lf = true;
                }
                _ => self.pending.push(b),
            }
        }
        out
    }

    fn end_line(&mut self, out: &mut Vec<SseEvent>) {
        let line = std::mem::take(&mut self.pending);
        self.line(&String::from_utf8_lossy(&line), out);
    }

    /// The stream ended: an event not closed by a blank line still counts.
    pub fn finish(&mut self) -> Vec<SseEvent> {
        let mut out = Vec::new();
        if !self.pending.is_empty() {
            self.end_line(&mut out);
        }
        self.dispatch(&mut out);
        out
    }

    fn line(&mut self, line: &str, out: &mut Vec<SseEvent>) {
        if line.is_empty() {
            self.dispatch(out);
            return;
        }
        if line.starts_with(':') {
            return;
        }
        let (field, value) = match line.split_once(':') {
            Some((f, v)) => (f, v.strip_prefix(' ').unwrap_or(v)),
            None => (line, ""),
        };
        match field {
            "event" => self.event = value.to_string(),
            "data" => self.data.push(value.to_string()),
            _ => {}
        }
    }

    fn dispatch(&mut self, out: &mut Vec<SseEvent>) {
        if !self.data.is_empty() {
            out.push(SseEvent { event: std::mem::take(&mut self.event), data: self.data.join("\n") });
        }
        self.data.clear();
        self.event.clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_in_any_pieces() {
        let text = "event: message_start\ndata: {\"a\":1}\n\n: a comment\r\nevent: ping\r\ndata: x\r\ndata: y\r\n\r\ndata: [DONE]\n\n";
        let whole = SseParser::new().feed(text.as_bytes());
        assert_eq!(whole.len(), 3);
        assert_eq!(whole[0], SseEvent { event: "message_start".into(), data: "{\"a\":1}".into() });
        assert_eq!(whole[1], SseEvent { event: "ping".into(), data: "x\ny".into() });
        assert_eq!(whole[2].data, "[DONE]");
        // (a byte at a time, the CR LFs cut between pieces: the same events)
        let mut p = SseParser::new();
        let mut bytes = Vec::new();
        for b in text.as_bytes() {
            bytes.extend(p.feed(&[*b]));
        }
        bytes.extend(p.finish());
        assert_eq!(bytes, whole);
        // (UTF-8 cut in two; an unterminated last event)
        let mut p = SseParser::new();
        let s = "data: caf\u{e9} \u{1f600}".as_bytes();
        let mut got = p.feed(&s[..9]);
        got.extend(p.feed(&s[9..]));
        got.extend(p.finish());
        assert_eq!(got, vec![SseEvent { event: String::new(), data: "caf\u{e9} \u{1f600}".into() }]);
    }
}
