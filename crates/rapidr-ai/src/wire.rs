//! Each wire format's HTTP requests and answers: a [`ChatRequest`] as
//! Anthropic's Messages or OpenAI's Chat Completions, the models list, and
//! a failed request explained in plain words.

use serde_json::{json, Map, Value};

use crate::chat::{Block, ChatRequest, Role};
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
        let b = self.base_url.trim();
        let b = if b.is_empty() { self.provider.base_url } else { b };
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
    let mut body = Map::new();
    body.insert("model".into(), json!(req.model));
    body.insert("max_tokens".into(), json!(req.max_tokens.max(1)));
    body.insert("stream".into(), json!(true));
    if !req.system.is_empty() {
        // (the system prompt and the tools are the same every turn: cached
        // by the provider, read again at a tenth of the price)
        body.insert("system".into(), json!([{"type": "text", "text": req.system, "cache_control": {"type": "ephemeral"}}]));
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
    if !req.system.is_empty() {
        messages.push(json!({"role": "system", "content": req.system}));
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
    if p.usage_in_stream {
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

/// A model the provider offers: its id and, when it has one, its name.
#[derive(Clone, Debug, PartialEq)]
pub struct ModelInfo {
    pub id: String,
    pub name: String,
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
    // (Ollama's own /api/tags shape too: {"models": [{"name"}]})
    let list = v.get("data").or_else(|| v.get("models")).and_then(Value::as_array).ok_or("the answer has no models list")?;
    let mut items: Vec<(i64, ModelInfo)> = list
        .iter()
        .filter_map(|m| {
            let id = m.get("id").or_else(|| m.get("name")).and_then(Value::as_str)?.to_string();
            let name = m.get("display_name").or_else(|| m.get("name")).and_then(Value::as_str).filter(|n| *n != id).unwrap_or("").to_string();
            let created = m.get("created").and_then(Value::as_i64).unwrap_or(0);
            Some((created, ModelInfo { id, name }))
        })
        .collect();
    if p.id == "openai" {
        let ids: Vec<String> = items.iter().map(|(_, m)| m.id.clone()).collect();
        items.retain(|(_, m)| openai_chat_model(&m.id) && !snapshot_of_listed(&m.id, &ids));
        items.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.id.cmp(&b.1.id)));
    }
    Ok(items.into_iter().map(|(_, m)| m).collect())
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

/// A model's cost against the provider's default, in a few words, for the
/// model pickers ("" — about as cheap): nothing is hidden, the costlier
/// ones are marked. From the providers' price lists (2026-10): Claude's
/// Sonnet ~20× Haiku's price, Opus ~40×, Fable and Mythos ~100×;
/// DeepSeek's Pro a few times Flash's.
pub fn cost_hint(p: &Provider, id: &str) -> &'static str {
    let id = id.to_ascii_lowercase();
    match p.id {
        "anthropic" if ["fable", "mythos", "opus"].iter().any(|k| id.contains(k)) => "$$$: costs much more",
        "anthropic" if id.contains("sonnet") => "$$: costs more",
        "deepseek" if id.contains("pro") => "$$: costs more",
        "openai" if id.ends_with("-pro") || id.starts_with("o1") || id.starts_with("o3") && !id.contains("mini") => "$$$: costs much more",
        _ => "",
    }
}

/// (OpenAI) What `reasoning_effort` a request with tools says for `model`:
/// OpenAI's GPT-6 models refuse function tools in Chat Completions unless
/// it is `none` ("use /v1/responses or set reasoning_effort to 'none'");
/// older ones don't know the field. `refused`: what the provider answered
/// last time ([`reasoning_refusal`]), which wins.
pub fn reasoning_for(p: &Provider, model: &str, tools: bool, refused: Option<bool>) -> Option<String> {
    if p.id != "openai" || !tools {
        return None;
    }
    let wants = refused.unwrap_or_else(|| model.to_ascii_lowercase().starts_with("gpt-6"));
    wants.then(|| "none".to_string())
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

/// The family a model belongs to, for a default that disappeared:
/// `claude-haiku` (Anthropic's tiers), `gpt`, `deepseek`.
fn family(id: &str) -> String {
    let parts: Vec<&str> = id.split('-').collect();
    if parts.first() == Some(&"claude") && parts.len() > 1 { format!("claude-{}", parts[1]) } else { parts.first().copied().unwrap_or("").to_string() }
}

/// The model to start with when the user has picked none: the provider's
/// default while the fetched list still has it (nothing fetched yet: the
/// default); else the first listed model of the default's family (the
/// list is newest first where the provider says so); else the first
/// listed.
pub fn suggested_model(p: &Provider, models: &[ModelInfo]) -> String {
    if models.is_empty() || models.iter().any(|m| m.id == p.default_model) {
        return p.default_model.to_string();
    }
    let fam = family(p.default_model);
    let same = models.iter().find(|m| !fam.is_empty() && (m.id == fam || m.id.starts_with(&format!("{fam}-"))));
    same.or_else(|| models.first()).map(|m| m.id.clone()).unwrap_or_default()
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
    let quoted = if said.is_empty() { String::new() } else { format!(" ({name} says: \"{}\")", said.trim_end_matches('.')) };
    let (kind, message) = if low.contains("anthropic-workspace-id") || low.contains("not scoped to a workspace") {
        (ErrorKind::Workspace, "This Anthropic key isn't made inside a workspace, so Anthropic needs to know which workspace to use: fill in Workspace ID in Tools > Settings > AI (the Claude Console lists your workspaces), or make a key inside a workspace.".to_string())
    } else if status == 402 || low.contains("insufficient_quota") || low.contains("insufficient balance") || low.contains("credit balance is too low") || low.contains("exceeded your current quota") {
        (ErrorKind::Credit, format!("Your {name} account has no credit left for this key{quoted}. Add credit on {name}'s website; RapidR can't do it for you."))
    } else if status == 401 || low.contains("invalid_api_key") || low.contains("invalid x-api-key") || low.contains("incorrect api key") {
        (ErrorKind::Key, format!("{name} refused the API key (401){quoted}. Check it in Tools > Settings > AI: it may be mistyped, revoked, or a key of another provider."))
    } else if status == 403 {
        (ErrorKind::Permission, format!("{name} doesn't let this key do that (403){quoted}."))
    } else if low.contains("context_length") || low.contains("context length") || low.contains("prompt is too long") || low.contains("too many tokens") || status == 413 {
        (ErrorKind::TooLong, format!("The conversation is too long for this model{quoted}. Start a new chat, or pick a model that takes more."))
    } else if status == 404 || low.contains("model_not_found") || (low.contains("model") && low.contains("does not exist")) {
        let m = if model.is_empty() { "this model".to_string() } else { format!("The model \"{model}\"") };
        (ErrorKind::Model, format!("{m} isn't available to your key (404){quoted}. Press Fetch Models in Tools > Settings > AI and pick one from the list."))
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
    let hint = if local {
        " Is the local server running (Ollama, LM Studio)? On the web, Ollama also needs OLLAMA_ORIGINS to allow this page.".to_string()
    } else if what.to_ascii_lowercase().contains("timed out") || what.to_ascii_lowercase().contains("timeout") {
        String::new()
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
        assert!(!r.body.unwrap().contains("stream_options"));
        assert_eq!(local.host(), "127.0.0.1:1234");
    }

    #[test]
    fn models() {
        let openai = provider("openai").unwrap();
        let body = r#"{"data":[{"id":"gpt-4o-mini","created":10},{"id":"gpt-4o-mini-2024-07-18","created":9},{"id":"text-embedding-3-small","created":11},{"id":"gpt-5.4-mini","created":30},{"id":"gpt-5.4-mini-2026-03-17","created":29},{"id":"gpt-5-codex","created":31},{"id":"gpt-realtime","created":32},{"id":"gpt-5.5","created":40},{"id":"o4-mini","created":20},{"id":"whisper-1","created":1}]}"#;
        let m = parse_models(openai, body).unwrap();
        let ids: Vec<&str> = m.iter().map(|m| m.id.as_str()).collect();
        assert_eq!(ids, ["gpt-5.5", "gpt-5.4-mini", "o4-mini", "gpt-4o-mini"]);
        // (the default gone from the list: the first of its family)
        assert_eq!(suggested_model(openai, &m), "gpt-5.5");
        let now = parse_models(openai, r#"{"data":[{"id":"gpt-6-sol","created":50},{"id":"gpt-6-luna","created":49},{"id":"gpt-image-2","created":51},{"id":"gpt-realtime-2.1-mini","created":52},{"id":"gpt-4o-mini-tts","created":3},{"id":"gpt-6-astra","created":48},{"id":"o3","created":5}]}"#).unwrap();
        assert_eq!(now.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["gpt-6-sol", "gpt-6-luna", "gpt-6-astra", "o3"]);
        assert_eq!(suggested_model(openai, &now), "gpt-6-luna");
        assert_eq!(suggested_model(openai, &[]), "gpt-6-luna");
        let anthropic = provider("anthropic").unwrap();
        let m = parse_models(anthropic, r#"{"data":[{"type":"model","id":"claude-haiku-5-5","display_name":"Claude Haiku 5.5"}],"has_more":false}"#).unwrap();
        assert_eq!(m, vec![ModelInfo { id: "claude-haiku-5-5".into(), name: "Claude Haiku 5.5".into() }]);
        assert_eq!(suggested_model(anthropic, &m), "claude-haiku-5-5");
        // (DeepSeek: its default listed; gone, the first deepseek-*)
        let deepseek = provider("deepseek").unwrap();
        let ds = parse_models(deepseek, r#"{"object":"list","data":[{"id":"deepseek-flash","name":"DeepSeek-V4.1-Flash"},{"id":"deepseek-v4-pro"}]}"#).unwrap();
        assert_eq!(ds.iter().map(|m| m.id.as_str()).collect::<Vec<_>>(), ["deepseek-flash", "deepseek-v4-pro"]);
        assert_eq!(suggested_model(deepseek, &ds), "deepseek-flash");
        assert_eq!(suggested_model(deepseek, &ds[1..]), "deepseek-v4-pro");
        assert_eq!(cost_hint(anthropic, "claude-fable-5-1"), "$$$: costs much more");
        assert_eq!(cost_hint(anthropic, "claude-opus-5-5"), "$$$: costs much more");
        assert_eq!(cost_hint(anthropic, "claude-sonnet-5-5"), "$$: costs more");
        assert_eq!(cost_hint(anthropic, "claude-haiku-5-5"), "");
        assert_eq!(cost_hint(deepseek, "deepseek-v4-pro"), "$$: costs more");
        assert_eq!(suggested_model(anthropic, &[ModelInfo { id: "claude-sonnet-5-5".into(), name: String::new() }, ModelInfo { id: "claude-haiku-6".into(), name: String::new() }]), "claude-haiku-6");
        assert!(parse_models(openai, "nope").is_err());
        let local = parse_models(provider("compatible").unwrap(), r#"{"models":[{"name":"llama3.2"}]}"#).unwrap();
        assert_eq!(local[0].id, "llama3.2");
        let mut ep = Endpoint::new(anthropic);
        ep.key = "k".into();
        let r = models_request(&ep);
        assert_eq!((r.method, r.url.as_str()), ("GET", "https://api.anthropic.com/v1/models?limit=1000"));
    }

    #[test]
    fn openai_reasoning_with_tools() {
        let o = provider("openai").unwrap();
        assert_eq!(reasoning_for(o, "gpt-6-luna", true, None).as_deref(), Some("none"));
        assert_eq!(reasoning_for(o, "gpt-6-luna", false, None), None);
        assert_eq!(reasoning_for(o, "gpt-4.1-mini", true, None), None);
        assert_eq!(reasoning_for(o, "gpt-5.4-mini", true, Some(true)).as_deref(), Some("none"));
        assert_eq!(reasoning_for(o, "gpt-6-sol", true, Some(false)), None);
        assert_eq!(reasoning_for(provider("deepseek").unwrap(), "gpt-6-luna", true, None), None);
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
