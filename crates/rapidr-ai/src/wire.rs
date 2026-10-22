//! Each wire format's HTTP requests and answers: a [`ChatRequest`] as
//! Anthropic's Messages or OpenAI's Chat Completions, the models list, and
//! a failed request explained in plain words.

use serde_json::{json, Map, Value};

use crate::chat::{Block, ChatRequest, Role, Thinking};
use crate::provider::{Format, Provider};

/// Where a request goes and as whom.
#[derive(Clone, Debug)]
pub struct Endpoint {
    pub provider: &'static Provider,
    /// The API's base URL ("" — the provider's).
    pub base_url: String,
    /// The API key ("" — none: a local server).
    pub key: String,
    /// Anthropic: the workspace a key that isn't scoped to one works in
    /// (`anthropic-workspace-id`); "" — none.
    pub workspace: String,
    /// The request is made by a web page: Anthropic wants it said
    /// (`anthropic-dangerous-direct-browser-access`).
    pub browser: bool,
}

impl Endpoint {
    pub fn new(provider: &'static Provider) -> Endpoint {
        Endpoint { provider, base_url: String::new(), key: String::new(), workspace: String::new(), browser: cfg!(target_arch = "wasm32") }
    }

    /// The base URL requests go to, without a trailing `/`.
    pub fn base(&self) -> String {
        let b = self.base_url.trim().trim_end_matches('/');
        // (the provider's own address — named or not — is the tests' server
        // when one is set: crate::provider::test_server)
        let b = if b.is_empty() || b.eq_ignore_ascii_case(self.provider.base_url) { crate::provider::default_base(self.provider) } else { b.to_string() };
        b.trim_end_matches('/').to_string()
    }

    /// The host requests go to (`api.openai.com`, `localhost:11434`).
    pub fn host(&self) -> String {
        let b = self.base();
        let rest = b.split_once("://").map_or(b.as_str(), |(_, r)| r);
        rest.split('/').next().unwrap_or("").to_string()
    }

    fn headers(&self) -> Vec<(String, String)> {
        let mut h = vec![("content-type".to_string(), "application/json".to_string())];
        match self.provider.format {
            Format::Anthropic => {
                h.push(("x-api-key".into(), self.key.clone()));
                h.push(("anthropic-version".into(), "2023-06-01".into()));
                if !self.workspace.trim().is_empty() {
                    h.push(("anthropic-workspace-id".into(), self.workspace.trim().to_string()));
                }
                if self.browser {
                    h.push(("anthropic-dangerous-direct-browser-access".into(), "true".into()));
                }
            }
            Format::OpenAi => {
                if !self.key.is_empty() {
                    h.push(("authorization".into(), format!("Bearer {}", self.key)));
                }
            }
        }
        h
    }
}

/// An HTTP request, ready for the transport.
#[derive(Clone, Debug, PartialEq)]
pub struct HttpRequest {
    pub method: &'static str,
    pub url: String,
    pub headers: Vec<(String, String)>,
    pub body: Option<String>,
}

impl HttpRequest {
    /// "What was sent" (docs/ide-ai.md §1): the method, the URL, the headers
    /// with the key's value hidden, and the body — what the user may read.
    pub fn redacted(&self) -> String {
        let mut out = format!("{} {}\n", self.method, self.url);
        for (k, v) in &self.headers {
            let shown = if is_secret_header(k) { "(your API key, not shown)".to_string() } else { v.clone() };
            out.push_str(&format!("{k}: {shown}\n"));
        }
        if let Some(b) = &self.body {
            out.push('\n');
            out.push_str(&serde_json::from_str::<Value>(b).ok().and_then(|v| serde_json::to_string_pretty(&v).ok()).unwrap_or_else(|| b.clone()));
            out.push('\n');
        }
        out
    }
}

fn is_secret_header(name: &str) -> bool {
    matches!(name.to_ascii_lowercase().as_str(), "x-api-key" | "authorization")
}

// ---- the chat request --------------------------------------------------------------

/// A conversation as the provider's streaming chat request.
pub fn chat_request(ep: &Endpoint, req: &ChatRequest) -> HttpRequest {
    let (path, body) = match ep.provider.format {
        Format::Anthropic => ("/messages", anthropic_body(req)),
        Format::OpenAi => ("/chat/completions", openai_body(ep.provider, req)),
    };
    HttpRequest { method: "POST", url: format!("{}{path}", ep.base()), headers: ep.headers(), body: Some(body.to_string()) }
}

fn anthropic_body(req: &ChatRequest) -> Value {
    let mut messages = Vec::new();
    for m in &req.messages {
        let content: Vec<Value> = m
            .content
            .iter()
            .map(|b| match b {
                Block::Text(t) => json!({"type": "text", "text": t}),
                Block::ToolUse { id, name, input } => json!({"type": "tool_use", "id": id, "name": name, "input": input}),
                Block::ToolResult { id, content, is_error } => {
                    let mut r = json!({"type": "tool_result", "tool_use_id": id, "content": content});
                    if *is_error {
                        r["is_error"] = json!(true);
                    }
                    r
                }
                Block::Opaque(v) => v.clone(),
            })
            // (an empty text block is refused)
            .filter(|v| !(v["type"] == "text" && v["text"].as_str().is_some_and(str::is_empty)))
            .collect();
        if content.is_empty() {
            continue;
        }
        messages.push(json!({"role": if m.role == Role::User { "user" } else { "assistant" }, "content": content}));
    }
    // (the conversation so far cached too: a turn's tool calls each send it
    // again, read from the cache)
    if let Some(last) = messages.last_mut().and_then(|m| m["content"].as_array_mut()).and_then(|c| c.last_mut()) {
        if matches!(last["type"].as_str(), Some("text" | "tool_result")) {
            last["cache_control"] = json!({"type": "ephemeral"});
        }
    }
    let mut body = Map::new();
    body.insert("model".into(), json!(req.model));
    body.insert("max_tokens".into(), json!(req.max_tokens.max(1)));
    body.insert("stream".into(), json!(true));
    // (an effort chosen: adaptive thinking with its summary shown, or an
    // older model's budget — max_tokens leaves room for it, RAI's job)
    match &req.thinking {
        Some(Thinking::Adaptive { effort, summarized }) => {
            body.insert("thinking".into(), if *summarized { json!({"type": "adaptive", "display": "summarized"}) } else { json!({"type": "adaptive"}) });
            body.insert("output_config".into(), json!({"effort": effort}));
        }
        Some(Thinking::Budget(n)) => {
            body.insert("thinking".into(), json!({"type": "enabled", "budget_tokens": n}));
        }
        None => {}
    }
    if !req.system.is_empty() || !req.system_tail.is_empty() {
        // (the stable instructions and the tools are the same every turn:
        // cached by the provider, read again at a tenth of the price; what
        // changes comes after them)
        let mut blocks = Vec::new();
        if !req.system.is_empty() {
            blocks.push(json!({"type": "text", "text": req.system, "cache_control": {"type": "ephemeral"}}));
        }
        if !req.system_tail.is_empty() {
            blocks.push(json!({"type": "text", "text": req.system_tail}));
        }
        body.insert("system".into(), Value::Array(blocks));
    }
    if !req.tools.is_empty() {
        let n = req.tools.len();
        let tools: Vec<Value> = req
            .tools
            .iter()
            .enumerate()
            .map(|(i, t)| {
                let mut v = json!({"name": t.name, "description": t.description, "input_schema": t.input_schema});
                if i + 1 == n {
                    v["cache_control"] = json!({"type": "ephemeral"});
                }
                v
            })
            .collect();
        body.insert("tools".into(), Value::Array(tools));
    }
    body.insert("messages".into(), Value::Array(messages));
    Value::Object(body)
}

fn openai_body(p: &Provider, req: &ChatRequest) -> Value {
    let mut messages = Vec::new();
    // (one system message, its stable part first: the providers' caches
    // match the request's prefix)
    let system = match (req.system.is_empty(), req.system_tail.is_empty()) {
        (false, false) => format!("{}\n\n{}", req.system, req.system_tail),
        (false, true) => req.system.clone(),
        (true, _) => req.system_tail.clone(),
    };
    if !system.is_empty() {
        messages.push(json!({"role": "system", "content": system}));
    }
    for m in &req.messages {
        match m.role {
            Role::User => {
                // (tool results are messages of their own, each after the
                // assistant's call; the user's text after them)
                for b in &m.content {
                    if let Block::ToolResult { id, content, is_error } = b {
                        let content = if *is_error { format!("Error: {content}") } else { content.clone() };
                        messages.push(json!({"role": "tool", "tool_call_id": id, "content": content}));
                    }
                }
                let text = m.text();
                if !text.is_empty() {
                    messages.push(json!({"role": "user", "content": text}));
                }
            }
            Role::Assistant => {
                let text = m.text();
                let calls: Vec<Value> = m
                    .content
                    .iter()
                    .filter_map(|b| match b {
                        Block::ToolUse { id, name, input } => Some(json!({"id": id, "type": "function", "function": {"name": name, "arguments": input.to_string()}})),
                        _ => None,
                    })
                    .collect();
                let mut msg = json!({"role": "assistant", "content": if text.is_empty() { Value::Null } else { json!(text) }});
                if !calls.is_empty() {
                    msg["tool_calls"] = Value::Array(calls);
                } else if text.is_empty() {
                    continue;
                }
                messages.push(msg);
            }
        }
    }
    let mut body = Map::new();
    body.insert("model".into(), json!(req.model));
    body.insert(p.max_tokens_field.into(), json!(req.max_tokens.max(1)));
    body.insert("stream".into(), json!(true));
    if p.usage_in_stream && !req.no_stream_usage {
        body.insert("stream_options".into(), json!({"include_usage": true}));
    }
    if let Some(r) = &req.reasoning_effort {
        body.insert("reasoning_effort".into(), json!(r));
    }
    if !req.tools.is_empty() {
        let tools: Vec<Value> = req.tools.iter().map(|t| json!({"type": "function", "function": {"name": t.name, "description": t.description, "parameters": t.input_schema}})).collect();
        body.insert("tools".into(), Value::Array(tools));
    }
    body.insert("messages".into(), Value::Array(messages));
    Value::Object(body)
}

// ---- the models list ----------------------------------------------------------------

/// A model the provider offers: its id and, when it has one, its name;
/// how many tokens it takes at once (its context window) and whether it
/// is loaded, when the server says (a local server's list does: mlx-serve,
/// LM Studio, vLLM, OpenRouter …).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
    /// Its context window in tokens (0: not said).
    pub context: u64,
    /// Loaded (`Some(true)`), not loaded (`Some(false)`: the window said
    /// may be the model's largest, not the one it will run with), or not said.
    pub loaded: Option<bool>,
    /// (Anthropic's list) How it thinks when asked to: "adaptive" (with an
    /// effort), "enabled" (a thinking budget), "none"; "" when not said.
    pub thinking: String,
}

/// How an Anthropic models list's entry says the model thinks
/// (`capabilities.thinking`): "adaptive", "enabled", "none", or "".
fn thinking_of(m: &Value) -> String {
    let t = &m["capabilities"]["thinking"];
    if t.is_null() {
        return String::new();
    }
    if t["types"]["adaptive"]["supported"].as_bool() == Some(true) {
        "adaptive"
    } else if t["types"]["enabled"]["supported"].as_bool() == Some(true) || t["supported"].as_bool() == Some(true) {
        "enabled"
    } else {
        "none"
    }
    .into()
}

/// The context window a models list's entry gives: the one it runs with
/// when loaded (LM Studio's `loaded_context_length`), else the model's
/// (`context_length`, vLLM's `max_model_len`, `context_window`, …); 0 when
/// none.
fn context_of(m: &Value) -> u64 {
    let num = |v: &Value| v.as_u64().or_else(|| v.as_f64().map(|f| f as u64)).or_else(|| v.as_str().and_then(|t| t.trim().parse().ok())).unwrap_or(0);
    for k in ["loaded_context_length", "context_length", "max_model_len", "context_window", "max_context_length", "max_input_tokens"] {
        let n = num(&m[k]);
        if n > 0 {
            return n;
        }
    }
    [&m["top_provider"]["context_length"], &m["meta"]["context_length"]].into_iter().map(num).find(|n| *n > 0).unwrap_or(0)
}

/// Whether a models list's entry says the model is loaded.
fn loaded_of(m: &Value) -> Option<bool> {
    if let Some(b) = m["loaded"].as_bool() {
        return Some(b);
    }
    match m["state"].as_str()?.to_ascii_lowercase().as_str() {
        "ready" | "loaded" | "running" => Some(true),
        "unloaded" | "not-loaded" | "not_loaded" | "loading" => Some(false),
        _ => None,
    }
}

/// The provider's list of models (Anthropic's `GET /v1/models`, OpenAI's
/// `GET /v1/models`, DeepSeek's `GET /models`, a compatible server's).
pub fn models_request(ep: &Endpoint) -> HttpRequest {
    let query = if ep.provider.format == Format::Anthropic { "?limit=1000" } else { "" };
    let mut headers = ep.headers();
    headers.retain(|(k, _)| k != "content-type");
    HttpRequest { method: "GET", url: format!("{}/models{query}", ep.base()), headers, body: None }
}

/// The models of a models list's answer, the ones a chat can use, newest
/// first where the provider says when they came (OpenAI lists embeddings,
/// voices, images … too: only chat models stay, and a dated snapshot only
/// when its undated name isn't there).
pub fn parse_models(p: &Provider, body: &str) -> Result<Vec<ModelInfo>, String> {
    let v: Value = serde_json::from_str(body).map_err(|e| format!("the models list isn't JSON: {e}"))?;
    // (Ollama's own /api/tags shape too: {"models": [{"name"}]}; a server
    // with no model yet — Ollama says `"data": null` — lists none)
    if v.get("data").is_some_and(Value::is_null) && v.get("models").is_none() {
        return Ok(Vec::new());
    }
    let list = v.get("data").or_else(|| v.get("models")).and_then(Value::as_array).ok_or("the answer has no models list")?;
    let mut items: Vec<(i64, ModelInfo)> = list
        .iter()
        .filter_map(|m| {
            let id = m.get("id").or_else(|| m.get("name")).and_then(Value::as_str)?.to_string();
            let name = m.get("display_name").or_else(|| m.get("name")).and_then(Value::as_str).filter(|n| *n != id).unwrap_or("").to_string();
            let created = m.get("created").and_then(Value::as_i64).unwrap_or(0);
            Some((created, ModelInfo { id, name, context: context_of(m), loaded: loaded_of(m), thinking: thinking_of(m) }))
        })
        .collect();
    if p.id == "openai" {
        let ids: Vec<String> = items.iter().map(|(_, m)| m.id.clone()).collect();
        items.retain(|(_, m)| openai_chat_model(&m.id) && !snapshot_of_listed(&m.id, &ids));
        items.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.id.cmp(&b.1.id)));
    }
    let mut out: Vec<ModelInfo> = items.into_iter().map(|(_, m)| m).collect();
    sort_models(&mut out);
    Ok(out)
}

/// Whether an OpenAI model id is one Chat Completions can talk with.
fn openai_chat_model(id: &str) -> bool {
    let id = id.to_ascii_lowercase();
    let family = id.starts_with("gpt-") || id.starts_with("o1") || id.starts_with("o3") || id.starts_with("o4") || id.starts_with("chatgpt-");
    const NOT: &[&str] = &["audio", "realtime", "transcribe", "tts", "image", "search", "embedding", "moderation", "whisper", "dall-e", "sora", "codex", "instruct", "deep-research", "computer-use", "-pro", "live", "translate", "diarize"];
    family && !NOT.iter().any(|n| id.contains(n))
}

/// `gpt-4o-2024-08-06` when `gpt-4o` is listed too.
fn snapshot_of_listed(id: &str, ids: &[String]) -> bool {
    let b = id.as_bytes();
    let dated = b.len() > 11 && b[b.len() - 11] == b'-' && id[b.len() - 10..].chars().enumerate().all(|(i, c)| if i == 4 || i == 7 { c == '-' } else { c.is_ascii_digit() });
    let short = b.len() > 5 && b[b.len() - 5] == b'-' && id[b.len() - 4..].chars().all(|c| c.is_ascii_digit());
    let base = if dated { &id[..b.len() - 11] } else if short { &id[..b.len() - 5] } else { return false };
    ids.iter().any(|i| i == base)
}

/// What the words of a model's name say about it, for the pickers
/// (non-binding: nothing is chosen for the user, and no model is named
/// here): the small, fast ones — haiku, mini, nano, flash, lite, luna,
/// small — "fast · low cost"; the largest — opus, fable, mythos, pro —
/// "costs more"; else "".
pub fn model_hint(name: &str) -> &'static str {
    let words: Vec<String> = name.to_ascii_lowercase().split(|c: char| !c.is_ascii_alphanumeric()).filter(|w| !w.is_empty()).map(str::to_string).collect();
    let has = |list: &[&str]| words.iter().any(|w| list.contains(&w.as_str()));
    if has(&["haiku", "mini", "nano", "flash", "lite", "luna", "small"]) {
        "fast · low cost"
    } else if has(&["opus", "fable", "mythos", "pro"]) {
        "costs more"
    } else {
        ""
    }
}

/// The models in the order the pickers show them: the fast, low-cost ones
/// first, then the rest, each as the provider listed them (newest first
/// where it says when they came).
pub fn sort_models(models: &mut [ModelInfo]) {
    models.sort_by_key(|m| match model_hint(&format!("{} {}", m.id, m.name)) {
        "fast · low cost" => 0,
        "" => 1,
        _ => 2,
    });
}

/// (OpenAI-compatible) What `reasoning_effort` a request says. OpenAI's
/// newest models take function tools in Chat Completions only with `none`
/// ("use /v1/responses or set reasoning_effort to 'none'"), older ones and
/// many servers don't know the field. No model is named here: the first
/// request says the effort the user chose (RAI.Effort; nothing without
/// one), and the provider's answer ([`reasoning_refusal`]: `refused`, kept
/// per model) decides from then on — the field left out, or `none` with
/// tools. DeepSeek's models choose for themselves: never sent.
pub fn reasoning_for(p: &Provider, _model: &str, tools: bool, refused: Option<bool>, effort: &str) -> Option<String> {
    if p.format != Format::OpenAi || p.id == "deepseek" {
        return None;
    }
    match refused {
        Some(false) => None,
        Some(true) if tools => Some("none".into()),
        _ if !effort.trim().is_empty() => Some(effort.trim().to_ascii_lowercase()),
        _ => None,
    }
}

/// (Anthropic) An error answer about thinking or effort for a model asked
/// to think: whether it is about them (the request is then sent the next
/// simpler way: [`crate::rai`]'s steps down), and whether it names the
/// `display` field.
pub fn thinking_refusal(status: u16, body: &str) -> Option<bool> {
    let low = body.to_ascii_lowercase();
    if status != 400 || !["thinking", "effort", "output_config", "adaptive", "budget_tokens"].iter().any(|w| low.contains(w)) {
        return None;
    }
    Some(low.contains("display"))
}

/// (OpenAI) An error answer about `reasoning_effort`: `Some(true)` — the
/// model wants it `none` with tools; `Some(false)` — it doesn't take the
/// field; `None` — the error is about something else.
pub fn reasoning_refusal(status: u16, body: &str) -> Option<bool> {
    let low = body.to_ascii_lowercase();
    if status != 400 || !low.contains("reasoning_effort") {
        return None;
    }
    if low.contains("set reasoning_effort to 'none'") || low.contains("with reasoning_effort are not supported") {
        Some(true)
    } else {
        Some(false)
    }
}

/// (OpenAI-compatible) An error answer refusing `stream_options` (a
/// server that doesn't know the field): ask again without it.
pub fn stream_usage_refusal(status: u16, body: &str) -> bool {
    (400..500).contains(&status) && status != 401 && status != 429 && body.to_ascii_lowercase().contains("stream_options")
}

/// The context window an error answer about a too long prompt names, when
/// it does: mlx-serve's "13927 tokens requested, 12288 available", OpenAI's
/// "maximum context length is 8192 tokens", llama.cpp's "the available
/// context size (4096 tokens)".
pub fn context_from_error(body: &str) -> Option<u64> {
    let low = provider_message(body).to_ascii_lowercase();
    let number_before = |at: usize| -> Option<u64> {
        let head = low[..at].trim_end();
        let rev: String = head.chars().rev().take_while(|c| c.is_ascii_digit() || *c == ',').collect();
        rev.chars().rev().filter(char::is_ascii_digit).collect::<String>().parse().ok()
    };
    let number_after = |at: usize| -> Option<u64> {
        let tail = low[at..].trim_start_matches(|c: char| !c.is_ascii_digit());
        tail.chars().take_while(|c| c.is_ascii_digit() || *c == ',').filter(char::is_ascii_digit).collect::<String>().parse().ok()
    };
    if low.contains("context") || low.contains("prompt") {
        if let Some(i) = low.find(" available") {
            if let Some(n) = number_before(i).filter(|n| *n >= 256) {
                return Some(n);
            }
        }
    }
    for phrase in ["maximum context length is", "context size (", "context length of", "context window of", "n_ctx"] {
        if let Some(i) = low.find(phrase) {
            if let Some(n) = number_after(i + phrase.len()).filter(|n| *n >= 256) {
                return Some(n);
            }
        }
    }
    None
}

// ---- errors -------------------------------------------------------------------------

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ErrorKind {
    /// No answer: the network, the server down, a timeout.
    Connection,
    /// The key was refused (401), or none was given.
    Key,
    /// The account has no credit (402; OpenAI's `insufficient_quota`).
    Credit,
    /// The key may not do that (403).
    Permission,
    /// The model isn't there for this key (404).
    Model,
    /// The conversation is longer than the model takes.
    TooLong,
    /// Too many requests (429): wait.
    RateLimit,
    /// The provider is overloaded or failing (5xx, 529).
    Server,
    /// Anthropic: the key isn't scoped to a workspace.
    Workspace,
    /// Anything else the provider refused (400 …).
    Request,
}

/// A failed request, in plain words.
#[derive(Clone, Debug, PartialEq)]
pub struct ApiError {
    pub kind: ErrorKind,
    pub status: u16,
    pub message: String,
}

impl ApiError {
    /// Worth trying again by itself (once the wait is over).
    pub fn retryable(&self) -> bool {
        matches!(self.kind, ErrorKind::RateLimit | ErrorKind::Server)
    }
}

/// The provider's own message in an error answer (Anthropic's and OpenAI's
/// `{"error": {"message": …}}`, a plain `{"message"}`, else the text).
pub fn provider_message(body: &str) -> String {
    let v: Option<Value> = serde_json::from_str(body).ok();
    let m = v.as_ref().and_then(|v| v.pointer("/error/message").or_else(|| v.get("message")).or_else(|| v.get("error")).and_then(Value::as_str).map(str::to_string));
    let m = m.unwrap_or_else(|| body.trim().chars().take(300).collect());
    m.trim().to_string()
}

/// An error answer (`status` and its body) explained: what happened and
/// what the user can do, in words a beginner follows.
pub fn explain_error(p: &Provider, status: u16, body: &str, retry_after: Option<u64>, model: &str) -> ApiError {
    let said = provider_message(body);
    let low = format!("{said} {body}").to_ascii_lowercase();
    let name = short_name(p);
    // (the provider's words, at most a line or two: the explanation shows
    // whole in a settings dialog or a program's status line)
    let short = if said.chars().count() > 160 {
        let cut: String = said.chars().take(160).collect();
        format!("{}…", cut.rsplit_once(' ').map_or(cut.as_str(), |(a, _)| a).trim_end_matches([',', ';', ':']))
    } else {
        said.trim_end_matches('.').to_string()
    };
    let quoted = if said.is_empty() { String::new() } else { format!(" ({name} says: \"{short}\")") };
    let (kind, message) = if low.contains("anthropic-workspace-id") || low.contains("not scoped to a workspace") {
        (ErrorKind::Workspace, "This Anthropic key isn't made inside a workspace, so Anthropic needs to know which workspace to use: fill in the Workspace ID (the Claude Console lists your workspaces), or make a key inside a workspace.".to_string())
    } else if status == 402 || low.contains("insufficient_quota") || low.contains("insufficient balance") || low.contains("credit balance is too low") || low.contains("exceeded your current quota") {
        (ErrorKind::Credit, format!("Your {name} account has no credit left for this key{quoted}. Add credit on {name}'s website; RapidR can't do it for you."))
    } else if status == 401 || low.contains("invalid_api_key") || low.contains("invalid x-api-key") || low.contains("incorrect api key") {
        (ErrorKind::Key, format!("{name} refused the API key (401){quoted}. Check the key: it may be mistyped, revoked, or a key of another provider."))
    } else if status == 403 {
        (ErrorKind::Permission, format!("{name} doesn't let this key do that (403){quoted}."))
    } else if low.contains("gpu memory") || low.contains("out of memory") || low.contains("not enough memory") {
        (ErrorKind::TooLong, format!("{name} is short of memory for this conversation{quoted}. Wait for the model to finish what it is doing, close other apps, start a new chat (it sends less), or choose a smaller model."))
    } else if low.contains("context_length") || low.contains("context length") || low.contains("prompt is too long") || low.contains("too many tokens") || status == 413 {
        (ErrorKind::TooLong, format!("The conversation is too long for this model{quoted}. Start a new chat, or pick a model that takes more."))
    } else if status == 404 || low.contains("model_not_found") || (low.contains("model") && low.contains("does not exist")) {
        let m = if model.is_empty() { "this model".to_string() } else { format!("The model \"{model}\"") };
        (ErrorKind::Model, format!("{m} isn't available to your key (404){quoted}. List the provider's models again and pick one of them."))
    } else if status == 429 {
        let wait = retry_after.map_or("a minute".to_string(), |s| format!("{s} seconds"));
        (ErrorKind::RateLimit, format!("{name} is limiting how often this key asks (429): wait {wait} and try again."))
    } else if status >= 500 {
        (ErrorKind::Server, format!("{name} is overloaded or having trouble ({status}){quoted}. Try again in a minute."))
    } else {
        (ErrorKind::Request, format!("{name} refused the request ({status}){quoted}."))
    };
    ApiError { kind, status, message }
}

/// A request that got no answer, explained (`what`: the transport's words).
pub fn explain_connection(ep: &Endpoint, what: &str) -> ApiError {
    let host = ep.host();
    let local = host.starts_with("localhost") || host.starts_with("127.0.0.1");
    let quiet = what.contains("stopped coming");
    if local && quiet {
        // (it answered, then went quiet: a local model writing a long change,
        // or short of memory — not a server that isn't there)
        return ApiError { kind: ErrorKind::Connection, status: 0, message: format!("The server at {host} began answering, then sent nothing more ({what}): a model on this computer writing a long change, or short of memory, can be that slow. Try again, ask for a smaller step, or choose a smaller model.") };
    }
    let hint = if local {
        " Is the local server running (Ollama, LM Studio)? On the web, Ollama also needs OLLAMA_ORIGINS to allow this page.".to_string()
    } else if what.to_ascii_lowercase().contains("timed out") || what.to_ascii_lowercase().contains("timeout") || what.contains(" seconds") {
        " Try again in a moment, and check the internet connection.".to_string()
    } else {
        " Check the internet connection.".to_string()
    };
    ApiError { kind: ErrorKind::Connection, status: 0, message: format!("Couldn't reach {host}: {what}.{hint}") }
}

/// The provider's name in a sentence ("Anthropic", "OpenAI", "the server").
pub fn short_name(p: &Provider) -> &'static str {
    match p.id {
        "anthropic" => "Anthropic",
        "openai" => "OpenAI",
        "deepseek" => "DeepSeek",
        _ => "The server",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::chat::{Message, Tool};
    use crate::provider::provider;

    fn convo() -> ChatRequest {
        ChatRequest {
            model: "m1".into(),
            system: "You help.".into(),
            system_tail: "The project: x.".into(),
            messages: vec![
                Message::user("Add a button"),
                Message {
                    role: Role::Assistant,
                    content: vec![
                        Block::Opaque(json!({"type": "thinking", "thinking": "", "signature": "sig"})),
                        Block::Text("Looking.".into()),
                        Block::ToolUse { id: "t1".into(), name: "read_file".into(), input: json!({"path": "main.rr"}) },
                    ],
                },
                Message { role: Role::User, content: vec![Block::ToolResult { id: "t1".into(), content: "PRINT 1".into(), is_error: false }] },
            ],
            tools: vec![Tool { name: "read_file".into(), description: "Reads a file".into(), input_schema: json!({"type": "object", "properties": {"path": {"type": "string"}}, "required": ["path"]}) }],
            max_tokens: 1000,
            reasoning_effort: None,
            no_stream_usage: false,
            thinking: None,
        }
    }

    #[test]
    fn anthropic_request() {
        let mut ep = Endpoint::new(provider("anthropic").unwrap());
        ep.key = "sk-ant-secret".into();
        ep.workspace = "wrkspc_1".into();
        ep.browser = false;
        let r = chat_request(&ep, &convo());
        assert_eq!(r.url, "https://api.anthropic.com/v1/messages");
        assert!(r.headers.contains(&("anthropic-workspace-id".into(), "wrkspc_1".into())));
        assert!(!r.headers.iter().any(|(k, _)| k == "anthropic-dangerous-direct-browser-access"));
        let b: Value = serde_json::from_str(r.body.as_deref().unwrap()).unwrap();
        assert_eq!(b["system"][0]["cache_control"]["type"], "ephemeral");
        assert_eq!(b["system"][1]["text"], "The project: x.");
        assert!(b["system"][1].get("cache_control").is_none());
        assert_eq!(b["messages"][2]["content"][0]["cache_control"]["type"], "ephemeral");
        assert_eq!(b["tools"][0]["cache_control"]["type"], "ephemeral");
        assert_eq!(b["messages"][1]["content"][0]["type"], "thinking");
        assert_eq!(b["messages"][1]["content"][2]["input"]["path"], "main.rr");
        assert_eq!(b["messages"][2]["content"][0]["tool_use_id"], "t1");
        assert!(b.get("temperature").is_none());
        // ("what was sent" never shows the key)
        let shown = r.redacted();
        assert!(!shown.contains("sk-ant-secret") && shown.contains("x-api-key: (your API key, not shown)"), "{shown}");
        ep.browser = true;
        assert!(chat_request(&ep, &convo()).headers.contains(&("anthropic-dangerous-direct-browser-access".into(), "true".into())));
        // (no effort: nothing said of thinking)
        assert!(b.get("thinking").is_none() && b.get("output_config").is_none());
        // (an effort: adaptive thinking with its summary, or an older model's budget)
        let mut req = convo();
        req.thinking = Some(Thinking::Adaptive { effort: "high".into(), summarized: true });
        let b: Value = serde_json::from_str(chat_request(&ep, &req).body.as_deref().unwrap()).unwrap();
        assert_eq!(b["thinking"], json!({"type": "adaptive", "display": "summarized"}));
        assert_eq!(b["output_config"]["effort"], "high");
        req.thinking = Some(Thinking::Budget(6144));
        let b: Value = serde_json::from_str(chat_request(&ep, &req).body.as_deref().unwrap()).unwrap();
        assert_eq!(b["thinking"], json!({"type": "enabled", "budget_tokens": 6144}));
        assert!(b.get("output_config").is_none());
        // (what the provider refuses of it: told apart)
        assert_eq!(thinking_refusal(400, r#"{"error":{"message":"thinking.display: Extra inputs are not permitted"}}"#), Some(true));
        assert_eq!(thinking_refusal(400, r#"{"error":{"message":"adaptive thinking is not supported on this model"}}"#), Some(false));
        assert_eq!(thinking_refusal(400, r#"{"error":{"message":"messages: text content blocks must be non-empty"}}"#), None);
        assert_eq!(thinking_refusal(401, "thinking"), None);
        // (an Anthropic models list says how each model thinks)
        let list = parse_models(provider("anthropic").unwrap(), r#"{"data":[{"id":"claude-a","max_input_tokens":1000000,"capabilities":{"thinking":{"supported":true,"types":{"enabled":{"supported":false},"adaptive":{"supported":true}}}}},{"id":"claude-b","capabilities":{"thinking":{"supported":true,"types":{"enabled":{"supported":true},"adaptive":{"supported":false}}}}},{"id":"claude-c"}]}"#).unwrap();
        let by = |id: &str| list.iter().find(|m| m.id == id).map(|m| (m.thinking.clone(), m.context)).unwrap();
        // (a server running with no model yet: an empty list, not an error)
        assert_eq!(parse_models(provider("compatible").unwrap(), r#"{"object":"list","data":null}"#).unwrap(), Vec::new());
        assert_eq!(by("claude-a"), ("adaptive".into(), 1_000_000));
        assert_eq!(by("claude-b").0, "enabled");
        assert_eq!(by("claude-c").0, "");
    }

    #[test]
    fn openai_request() {
        let mut ep = Endpoint::new(provider("openai").unwrap());
        ep.key = "sk-proj-secret".into();
        let r = chat_request(&ep, &convo());
        assert_eq!(r.url, "https://api.openai.com/v1/chat/completions");
        let b: Value = serde_json::from_str(r.body.as_deref().unwrap()).unwrap();
        assert_eq!(b["max_completion_tokens"], 1000);
        assert_eq!(b["stream_options"]["include_usage"], true);
        assert_eq!(b["messages"][0]["role"], "system");
        assert_eq!(b["messages"][0]["content"], "You help.\n\nThe project: x.");
        assert_eq!(b["messages"][2]["tool_calls"][0]["function"]["arguments"], "{\"path\":\"main.rr\"}");
        assert_eq!(b["messages"][3]["role"], "tool");
        assert_eq!(b["messages"].as_array().unwrap().len(), 4);
        assert_eq!(b["tools"][0]["function"]["parameters"]["required"][0], "path");
        assert!(!r.redacted().contains("sk-proj-secret"));
        // (DeepSeek: max_tokens; a compatible server: no stream_options, no key, its own URL)
        let ds = chat_request(&Endpoint::new(provider("deepseek").unwrap()), &convo());
        assert_eq!(ds.url, "https://api.deepseek.com/chat/completions");
        assert!(ds.body.as_deref().unwrap().contains("\"max_tokens\":1000"));
        let mut local = Endpoint::new(provider("compatible").unwrap());
        local.base_url = "http://127.0.0.1:1234/v1/".into();
        let r = chat_request(&local, &convo());
        assert_eq!(r.url, "http://127.0.0.1:1234/v1/chat/completions");
        assert!(!r.headers.iter().any(|(k, _)| k == "authorization"));
        // (a compatible server is asked for its usage too: the meter and
        // the context budget need it; one that refuses the field is asked
        // again without it)
        assert!(r.body.unwrap().contains("\"stream_options\":{\"include_usage\":true}"));
        let mut quiet = convo();
        quiet.no_stream_usage = true;
        assert!(!chat_request(&local, &quiet).body.unwrap().contains("stream_options"));
        assert!(stream_usage_refusal(400, r#"{"error":{"message":"Unrecognized request argument supplied: stream_options"}}"#));
        assert!(!stream_usage_refusal(400, r#"{"error":{"message":"bad model"}}"#));
        assert_eq!(local.host(), "127.0.0.1:1234");
    }

    #[test]
    fn models() {
        let openai = provider("openai").unwrap();
        let body = r#"{"data":[{"id":"gpt-4o-mini","created":10},{"id":"gpt-4o-mini-2024-07-18","created":9},{"id":"text-embedding-3-small","created":11},{"id":"gpt-5.4-mini","created":30},{"id":"gpt-5.4-mini-2026-03-17","created":29},{"id":"gpt-5-codex","created":31},{"id":"gpt-realtime","created":32},{"id":"gpt-5.5","created":40},{"id":"o4-mini","created":20},{"id":"whisper-1","created":1},{"id":"gpt-image-2","created":41},{"id":"gpt-4o-mini-tts","created":3}]}"#;
        let m = parse_models(openai, body).unwrap();
        let ids: Vec<&str> = m.iter().map(|m| m.id.as_str()).collect();
        // (chat models only; the fast, low-cost ones first, each newest first)
        assert_eq!(ids, ["gpt-5.4-mini", "o4-mini", "gpt-4o-mini", "gpt-5.5"]);
        let anthropic = provider("anthropic").unwrap();
        let m = parse_models(anthropic, r#"{"data":[{"type":"model","id":"x-opus-9","display_name":"X Opus 9"},{"type":"model","id":"x-sonnet-9","display_name":"X Sonnet 9"},{"type":"model","id":"x-haiku-9","display_name":"X Haiku 9"}],"has_more":false}"#).unwrap();
        assert_eq!(m.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["x-haiku-9", "x-sonnet-9", "x-opus-9"]);
        assert_eq!(m[0].name, "X Haiku 9");
        let ds = parse_models(provider("deepseek").unwrap(), r#"{"object":"list","data":[{"id":"deepseek-v4-pro","name":"DeepSeek-V4-Pro"},{"id":"deepseek-flash","name":"DeepSeek-V4.1-Flash"}]}"#).unwrap();
        assert_eq!(ds.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["deepseek-flash", "deepseek-v4-pro"]);
        assert!(parse_models(openai, "nope").is_err());
        let local = parse_models(provider("compatible").unwrap(), r#"{"models":[{"name":"llama3.2"}]}"#).unwrap();
        assert_eq!(local[0].id, "llama3.2");
        assert_eq!((local[0].context, local[0].loaded), (0, None));
        // (a local server's context windows: the loaded one when it says)
        let mlx = parse_models(provider("compatible").unwrap(), r#"{"object":"list","data":[{"id":"gemma-x","loaded":true,"state":"ready","context_length":12288,"max_model_len":12288},{"id":"qwen-y","loaded":false,"state":"unloaded","context_length":262144},{"id":"lms","max_context_length":32768,"loaded_context_length":8192,"state":"loaded"},{"id":"or","top_provider":{"context_length":"65536"}}]}"#).unwrap();
        let by = |id: &str| mlx.iter().find(|m| m.id == id).map(|m| (m.context, m.loaded)).unwrap();
        assert_eq!(by("gemma-x"), (12288, Some(true)));
        assert_eq!(by("qwen-y"), (262144, Some(false)));
        assert_eq!(by("lms"), (8192, Some(true)));
        assert_eq!(by("or"), (65536, None));
        assert_eq!(context_from_error(r#"{"error":{"message":"Prompt exceeds maximum context length: 13927 tokens requested, 12288 available","type":"invalid_request_error"}}"#), Some(12288));
        assert_eq!(context_from_error(r#"{"error":{"message":"This model's maximum context length is 8,192 tokens. However, your messages resulted in 9000 tokens."}}"#), Some(8192));
        assert_eq!(context_from_error(r#"{"error":{"message":"the request exceeds the available context size (4096 tokens), try increasing it"}}"#), Some(4096));
        assert_eq!(context_from_error(r#"{"error":{"message":"Rate limit reached"}}"#), None);
        let mut ep = Endpoint::new(anthropic);
        ep.key = "k".into();
        let r = models_request(&ep);
        assert_eq!((r.method, r.url.as_str()), ("GET", "https://api.anthropic.com/v1/models?limit=1000"));
        // (the hints come from the words of a name, never from an id)
        assert_eq!(model_hint("Claude Haiku 9"), "fast · low cost");
        assert_eq!(model_hint("gpt-9-luna"), "fast · low cost");
        assert_eq!(model_hint("deepseek-flash"), "fast · low cost");
        assert_eq!(model_hint("Claude Opus 9"), "costs more");
        assert_eq!(model_hint("deepseek-v9-pro"), "costs more");
        assert_eq!(model_hint("gpt-9"), "");
        assert_eq!(model_hint("minimal-model"), "");
    }

    #[test]
    fn openai_reasoning_with_tools() {
        let o = provider("openai").unwrap();
        assert_eq!(reasoning_for(o, "gpt-6-luna", true, None, ""), None);
        assert_eq!(reasoning_for(o, "gpt-6-luna", false, Some(true), ""), None);
        assert_eq!(reasoning_for(o, "gpt-4.1-mini", true, None, ""), None);
        assert_eq!(reasoning_for(o, "gpt-5.4-mini", true, Some(true), "").as_deref(), Some("none"));
        assert_eq!(reasoning_for(o, "gpt-6-sol", true, Some(false), ""), None);
        assert_eq!(reasoning_for(provider("deepseek").unwrap(), "gpt-6-luna", true, None, "high"), None);
        // (an effort chosen: said, until the model refuses the field)
        assert_eq!(reasoning_for(o, "o4-mini", true, None, "High").as_deref(), Some("high"));
        assert_eq!(reasoning_for(provider("compatible").unwrap(), "gpt-oss", true, None, "low").as_deref(), Some("low"));
        assert_eq!(reasoning_for(o, "gpt-6-sol", true, Some(true), "high").as_deref(), Some("none"));
        assert_eq!(reasoning_for(o, "gpt-6-sol", false, Some(true), "high").as_deref(), Some("high"));
        assert_eq!(reasoning_for(o, "gpt-4.1", true, Some(false), "high"), None);
        let said = r#"{"error":{"message":"Function tools with reasoning_effort are not supported for gpt-6-luna in /v1/chat/completions. To use function tools, use /v1/responses or set reasoning_effort to 'none'."}}"#;
        assert_eq!(reasoning_refusal(400, said), Some(true));
        assert_eq!(reasoning_refusal(400, r#"{"error":{"message":"Unrecognized request argument supplied: reasoning_effort"}}"#), Some(false));
        assert_eq!(reasoning_refusal(401, said), None);
        let mut req = convo();
        req.reasoning_effort = Some("none".into());
        let b: Value = serde_json::from_str(chat_request(&Endpoint::new(o), &req).body.as_deref().unwrap()).unwrap();
        assert_eq!(b["reasoning_effort"], "none");
    }

    #[test]
    fn errors_in_plain_words() {
        let a = provider("anthropic").unwrap();
        let o = provider("openai").unwrap();
        let ws = r#"{"type":"error","error":{"type":"invalid_request_error","message":"This API key is not scoped to a workspace, so this request must include the anthropic-workspace-id header with the ID of the workspace to use."}}"#;
        assert_eq!(explain_error(a, 400, ws, None, "").kind, ErrorKind::Workspace);
        let e = explain_error(o, 401, r#"{"error":{"message":"Incorrect API key provided: sk-proj-****abcd.","type":"invalid_request_error","code":"invalid_api_key"}}"#, None, "");
        assert_eq!(e.kind, ErrorKind::Key);
        assert!(e.message.starts_with("OpenAI refused the API key (401)"), "{}", e.message);
        assert_eq!(explain_error(o, 429, r#"{"error":{"message":"You exceeded your current quota","code":"insufficient_quota"}}"#, None, "").kind, ErrorKind::Credit);
        let r = explain_error(o, 429, r#"{"error":{"message":"Rate limit reached"}}"#, Some(7), "");
        assert!(r.retryable() && r.message.contains("wait 7 seconds"), "{}", r.message);
        assert_eq!(explain_error(provider("deepseek").unwrap(), 402, r#"{"error":{"message":"Insufficient Balance"}}"#, None, "").kind, ErrorKind::Credit);
        let m = explain_error(o, 404, r#"{"error":{"message":"The model `gpt-9` does not exist","code":"model_not_found"}}"#, None, "gpt-9");
        assert!(m.kind == ErrorKind::Model && m.message.contains("\"gpt-9\""), "{}", m.message);
        assert_eq!(explain_error(a, 529, r#"{"type":"error","error":{"type":"overloaded_error","message":"Overloaded"}}"#, None, "").kind, ErrorKind::Server);
        assert_eq!(explain_error(a, 400, r#"{"error":{"message":"prompt is too long: 250000 tokens > 200000 maximum"}}"#, None, "").kind, ErrorKind::TooLong);
        let mut ep = Endpoint::new(provider("compatible").unwrap());
        ep.base_url = "http://localhost:11434/v1".into();
        assert!(explain_connection(&ep, "connection refused").message.contains("Is the local server running"));
    }
}
