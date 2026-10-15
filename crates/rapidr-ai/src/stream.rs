//! A streamed answer: each format's server-sent events decoded into
//! normalized [`Event`]s (text as it comes, tool calls and their arguments,
//! usage, why it stopped), and gathered into a [`Reply`] — the assistant's
//! message for the conversation and the tool calls to run.

use std::collections::HashMap;

use serde_json::{json, Value};

use crate::chat::{Block, Message, Role, Usage};
use crate::provider::Format;
use crate::sse::SseParser;

#[derive(Clone, Debug, PartialEq)]
pub enum Event {
    /// More of the answer's text.
    Text(String),
    /// A tool call begins (`index` tells its arguments' pieces apart).
    ToolStart { index: usize, id: String, name: String },
    /// A piece of a tool call's arguments (JSON text).
    ToolArgs { index: usize, json: String },
    /// A block to send back as it came (Anthropic's thinking).
    Opaque(Value),
    /// Tokens so far (each field the largest seen counts).
    Usage(Usage),
    /// Why the answer ended: `end`, `tool_use`, `max_tokens`, `refusal`,
    /// `pause`, or the provider's own word.
    Stop(String),
    /// The stream reported an error.
    Error(String),
}

/// One format's decoder: feed it the body's bytes as they come.
pub struct Decoder {
    format: Format,
    sse: SseParser,
    /// (Anthropic) blocks being streamed that go back as they came.
    opaque: HashMap<usize, Value>,
    /// (OpenAI) tool calls seen, by index: whether ToolStart went out.
    started: HashMap<usize, bool>,
    /// (OpenAI) the stream said `[DONE]`.
    pub done: bool,
}

impl Decoder {
    pub fn new(format: Format) -> Decoder {
        Decoder { format, sse: SseParser::new(), opaque: HashMap::new(), started: HashMap::new(), done: false }
    }

    pub fn feed(&mut self, bytes: &[u8]) -> Vec<Event> {
        let events = self.sse.feed(bytes);
        self.decode(events)
    }

    pub fn finish(&mut self) -> Vec<Event> {
        let events = self.sse.finish();
        self.decode(events)
    }

    fn decode(&mut self, events: Vec<crate::sse::SseEvent>) -> Vec<Event> {
        let mut out = Vec::new();
        for e in events {
            if e.data.trim() == "[DONE]" {
                self.done = true;
                continue;
            }
            let Ok(v) = serde_json::from_str::<Value>(&e.data) else { continue };
            match self.format {
                Format::Anthropic => self.anthropic(&v, &mut out),
                Format::OpenAi => self.openai(&v, &mut out),
            }
        }
        out
    }

    fn anthropic(&mut self, v: &Value, out: &mut Vec<Event>) {
        let index = v["index"].as_u64().unwrap_or(0) as usize;
        match v["type"].as_str().unwrap_or("") {
            "message_start" => {
                let u = &v["message"]["usage"];
                let cached = u["cache_read_input_tokens"].as_u64().unwrap_or(0);
                let input = u["input_tokens"].as_u64().unwrap_or(0) + cached + u["cache_creation_input_tokens"].as_u64().unwrap_or(0);
                out.push(Event::Usage(Usage { input, output: u["output_tokens"].as_u64().unwrap_or(0), cached }));
            }
            "content_block_start" => {
                let b = &v["content_block"];
                match b["type"].as_str().unwrap_or("") {
                    "text" => {
                        if let Some(t) = b["text"].as_str().filter(|t| !t.is_empty()) {
                            out.push(Event::Text(t.to_string()));
                        }
                    }
                    "tool_use" => out.push(Event::ToolStart { index, id: b["id"].as_str().unwrap_or("").to_string(), name: b["name"].as_str().unwrap_or("").to_string() }),
                    // (thinking, redacted thinking: kept whole, sent back)
                    _ => {
                        self.opaque.insert(index, b.clone());
                    }
                }
            }
            "content_block_delta" => {
                let d = &v["delta"];
                match d["type"].as_str().unwrap_or("") {
                    "text_delta" => out.push(Event::Text(d["text"].as_str().unwrap_or("").to_string())),
                    "input_json_delta" => out.push(Event::ToolArgs { index, json: d["partial_json"].as_str().unwrap_or("").to_string() }),
                    kind => {
                        if let Some(b) = self.opaque.get_mut(&index) {
                            // (thinking_delta → "thinking", signature_delta → "signature")
                            let field = kind.strip_suffix("_delta").unwrap_or(kind);
                            if let Some(piece) = d[field].as_str() {
                                let was = b[field].as_str().unwrap_or("").to_string();
                                b[field] = json!(was + piece);
                            }
                        }
                    }
                }
            }
            "content_block_stop" => {
                if let Some(b) = self.opaque.remove(&index) {
                    out.push(Event::Opaque(b));
                }
            }
            "message_delta" => {
                if let Some(o) = v["usage"]["output_tokens"].as_u64() {
                    out.push(Event::Usage(Usage { output: o, ..Usage::default() }));
                }
                if let Some(r) = v["delta"]["stop_reason"].as_str() {
                    out.push(Event::Stop(normal_stop(r)));
                }
            }
            "error" => out.push(Event::Error(v["error"]["message"].as_str().unwrap_or("the stream failed").to_string())),
            _ => {}
        }
    }

    fn openai(&mut self, v: &Value, out: &mut Vec<Event>) {
        if let Some(e) = v.get("error") {
            out.push(Event::Error(e["message"].as_str().or_else(|| e.as_str()).unwrap_or("the stream failed").to_string()));
            return;
        }
        if let Some(choice) = v["choices"].as_array().and_then(|c| c.first()) {
            let d = &choice["delta"];
            if let Some(t) = d["content"].as_str().filter(|t| !t.is_empty()) {
                out.push(Event::Text(t.to_string()));
            }
            if let Some(calls) = d["tool_calls"].as_array() {
                for (k, c) in calls.iter().enumerate() {
                    let index = c["index"].as_u64().map_or(k, |i| i as usize);
                    let f = &c["function"];
                    if !self.started.get(&index).copied().unwrap_or(false) {
                        self.started.insert(index, true);
                        let id = c["id"].as_str().filter(|i| !i.is_empty()).map_or_else(|| format!("call_{index}"), str::to_string);
                        out.push(Event::ToolStart { index, id, name: f["name"].as_str().unwrap_or("").to_string() });
                    }
                    if let Some(a) = f["arguments"].as_str().filter(|a| !a.is_empty()) {
                        out.push(Event::ToolArgs { index, json: a.to_string() });
                    }
                }
            }
            if let Some(r) = choice["finish_reason"].as_str() {
                out.push(Event::Stop(normal_stop(r)));
            }
        }
        if let Some(u) = v.get("usage").filter(|u| u.is_object()) {
            let cached = u["prompt_tokens_details"]["cached_tokens"].as_u64().or_else(|| u["prompt_cache_hit_tokens"].as_u64()).unwrap_or(0);
            out.push(Event::Usage(Usage { input: u["prompt_tokens"].as_u64().unwrap_or(0), output: u["completion_tokens"].as_u64().unwrap_or(0), cached }));
        }
    }
}

/// A stop reason in RapidR's words.
fn normal_stop(r: &str) -> String {
    match r {
        "end_turn" | "stop" | "stop_sequence" => "end",
        "tool_use" | "tool_calls" | "function_call" => "tool_use",
        "max_tokens" | "length" => "max_tokens",
        "refusal" | "content_filter" => "refusal",
        "pause_turn" => "pause",
        other => other,
    }
    .to_string()
}

/// An answer gathered from its events.
#[derive(Clone, Debug, Default)]
pub struct Reply {
    pub blocks: Vec<Block>,
    pub stop: String,
    pub usage: Usage,
    pub error: Option<String>,
    /// Tool calls by index: their block's place and their arguments' text.
    tools: HashMap<usize, (usize, String)>,
}

impl Reply {
    pub fn apply(&mut self, e: &Event) {
        match e {
            Event::Text(t) => match self.blocks.last_mut() {
                Some(Block::Text(s)) => s.push_str(t),
                _ => self.blocks.push(Block::Text(t.clone())),
            },
            Event::ToolStart { index, id, name } => {
                self.tools.insert(*index, (self.blocks.len(), String::new()));
                self.blocks.push(Block::ToolUse { id: id.clone(), name: name.clone(), input: json!({}) });
            }
            Event::ToolArgs { index, json } => {
                if let Some((_, args)) = self.tools.get_mut(index) {
                    args.push_str(json);
                }
            }
            Event::Opaque(v) => self.blocks.push(Block::Opaque(v.clone())),
            Event::Usage(u) => {
                self.usage.input = self.usage.input.max(u.input);
                self.usage.output = self.usage.output.max(u.output);
                self.usage.cached = self.usage.cached.max(u.cached);
            }
            Event::Stop(r) => self.stop = r.clone(),
            Event::Error(m) => self.error = Some(m.clone()),
        }
    }

    /// The tool calls' arguments parsed (an argument text that isn't a JSON
    /// object becomes `{"_invalid_json": text}`: the tool says so).
    pub fn finish(&mut self) {
        for (pos, args) in self.tools.values() {
            if let Some(Block::ToolUse { input, .. }) = self.blocks.get_mut(*pos) {
                let t = args.trim();
                *input = if t.is_empty() {
                    json!({})
                } else {
                    match serde_json::from_str::<Value>(t) {
                        Ok(v) if v.is_object() => v,
                        _ => json!({"_invalid_json": t}),
                    }
                };
            }
        }
        self.tools.clear();
    }

    /// The answer's text.
    pub fn text(&self) -> String {
        self.message().text()
    }

    /// The answer as the conversation's assistant message.
    pub fn message(&self) -> Message {
        Message { role: Role::Assistant, content: self.blocks.clone() }
    }

    /// The tool calls: (id, name, arguments).
    pub fn tool_calls(&self) -> Vec<(String, String, Value)> {
        self.blocks
            .iter()
            .filter_map(|b| match b {
                Block::ToolUse { id, name, input } => Some((id.clone(), name.clone(), input.clone())),
                _ => None,
            })
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn gather(format: Format, body: &str, piece: usize) -> Reply {
        let mut d = Decoder::new(format);
        let mut r = Reply::default();
        for chunk in body.as_bytes().chunks(piece) {
            for e in d.feed(chunk) {
                r.apply(&e);
            }
        }
        for e in d.finish() {
            r.apply(&e);
        }
        r.finish();
        r
    }

    const ANTHROPIC: &str = concat!(
        "event: message_start\ndata: {\"type\":\"message_start\",\"message\":{\"id\":\"msg_1\",\"usage\":{\"input_tokens\":20,\"cache_read_input_tokens\":100,\"output_tokens\":1}}}\n\n",
        "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":0,\"content_block\":{\"type\":\"thinking\",\"thinking\":\"\"}}\n\n",
        "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":0,\"delta\":{\"type\":\"signature_delta\",\"signature\":\"abc\"}}\n\n",
        "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":0}\n\n",
        "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":1,\"content_block\":{\"type\":\"text\",\"text\":\"\"}}\n\n",
        "event: ping\ndata: {\"type\":\"ping\"}\n\n",
        "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"I'll read \"}}\n\n",
        "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":1,\"delta\":{\"type\":\"text_delta\",\"text\":\"it.\"}}\n\n",
        "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":1}\n\n",
        "event: content_block_start\ndata: {\"type\":\"content_block_start\",\"index\":2,\"content_block\":{\"type\":\"tool_use\",\"id\":\"toolu_1\",\"name\":\"read_file\",\"input\":{}}}\n\n",
        "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":2,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"{\\\"path\\\": \\\"ma\"}}\n\n",
        "event: content_block_delta\ndata: {\"type\":\"content_block_delta\",\"index\":2,\"delta\":{\"type\":\"input_json_delta\",\"partial_json\":\"in.rr\\\"}\"}}\n\n",
        "event: content_block_stop\ndata: {\"type\":\"content_block_stop\",\"index\":2}\n\n",
        "event: message_delta\ndata: {\"type\":\"message_delta\",\"delta\":{\"stop_reason\":\"tool_use\"},\"usage\":{\"output_tokens\":42}}\n\n",
        "event: message_stop\ndata: {\"type\":\"message_stop\"}\n\n",
    );

    #[test]
    fn anthropic_stream() {
        for piece in [1, 7, 4096] {
            let r = gather(Format::Anthropic, ANTHROPIC, piece);
            assert_eq!(r.text(), "I'll read it.");
            assert_eq!(r.stop, "tool_use");
            assert_eq!(r.usage, Usage { input: 120, output: 42, cached: 100 });
            assert_eq!(r.tool_calls(), vec![("toolu_1".to_string(), "read_file".to_string(), json!({"path": "main.rr"}))]);
            // (the thinking block kept, with its signature, before the text)
            assert_eq!(r.blocks[0], Block::Opaque(json!({"type": "thinking", "thinking": "", "signature": "abc"})));
        }
        let e = gather(Format::Anthropic, "event: error\ndata: {\"type\":\"error\",\"error\":{\"type\":\"overloaded_error\",\"message\":\"Overloaded\"}}\n\n", 64);
        assert_eq!(e.error.as_deref(), Some("Overloaded"));
    }

    #[test]
    fn openai_stream() {
        let body = concat!(
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"}}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Here \"}}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"content\":\"goes.\",\"tool_calls\":[{\"index\":0,\"id\":\"call_a\",\"type\":\"function\",\"function\":{\"name\":\"check_code\",\"arguments\":\"\"}}]}}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"{\\\"files\\\":\"}}]}}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{\"tool_calls\":[{\"index\":0,\"function\":{\"arguments\":\"[]}\"}},{\"index\":1,\"id\":\"call_b\",\"function\":{\"name\":\"list_files\",\"arguments\":\"{}\"}}]}}]}\n\n",
            "data: {\"choices\":[{\"index\":0,\"delta\":{},\"finish_reason\":\"tool_calls\"}]}\n\n",
            "data: {\"choices\":[],\"usage\":{\"prompt_tokens\":300,\"completion_tokens\":12,\"prompt_tokens_details\":{\"cached_tokens\":256}}}\n\n",
            "data: [DONE]\n\n",
        );
        for piece in [1, 5, 4096] {
            let r = gather(Format::OpenAi, body, piece);
            assert_eq!(r.text(), "Here goes.");
            assert_eq!(r.stop, "tool_use");
            assert_eq!(r.usage, Usage { input: 300, output: 12, cached: 256 });
            let calls = r.tool_calls();
            assert_eq!(calls.len(), 2);
            assert_eq!(calls[0], ("call_a".to_string(), "check_code".to_string(), json!({"files": []})));
            assert_eq!(calls[1].1, "list_files");
        }
        // (arguments that aren't JSON: kept for the tool to refuse)
        let bad = gather(Format::OpenAi, "data: {\"choices\":[{\"delta\":{\"tool_calls\":[{\"index\":0,\"id\":\"c\",\"function\":{\"name\":\"x\",\"arguments\":\"{oops\"}}]},\"finish_reason\":\"tool_calls\"}]}\n\n", 64);
        assert_eq!(bad.tool_calls()[0].2, json!({"_invalid_json": "{oops"}));
        let err = gather(Format::OpenAi, "data: {\"error\":{\"message\":\"boom\"}}\n\n", 64);
        assert_eq!(err.error.as_deref(), Some("boom"));
    }
}
