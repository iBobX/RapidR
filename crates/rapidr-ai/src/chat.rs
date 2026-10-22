//! The normalized conversation (docs/ide-ai.md §2.1): what every provider's
//! request is made from and every answer is gathered into.

use serde_json::Value;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Role {
    User,
    Assistant,
}

#[derive(Clone, Debug, PartialEq)]
pub enum Block {
    Text(String),
    /// The model asks for a tool: its call's id, the tool's name, its
    /// arguments (a JSON object).
    ToolUse { id: String, name: String, input: Value },
    /// What a tool gave back, for the call `id`.
    ToolResult { id: String, content: String, is_error: bool },
    /// A provider's own block that must go back as it came (Anthropic's
    /// thinking blocks and their signatures, in a turn with tool calls);
    /// other formats leave it out.
    Opaque(Value),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Message {
    pub role: Role,
    pub content: Vec<Block>,
}

impl Message {
    pub fn user(text: &str) -> Message {
        Message { role: Role::User, content: vec![Block::Text(text.to_string())] }
    }

    /// The message's text, its text blocks joined.
    pub fn text(&self) -> String {
        self.content
            .iter()
            .filter_map(|b| match b {
                Block::Text(t) => Some(t.as_str()),
                _ => None,
            })
            .collect::<Vec<_>>()
            .join("")
    }
}

/// A tool the model may call: its name, what it does, and its arguments'
/// JSON Schema (the common subset every provider accepts: objects,
/// strings, numbers, Booleans, arrays, enums — no `$ref`).
#[derive(Clone, Debug, PartialEq)]
pub struct Tool {
    pub name: String,
    pub description: String,
    pub input_schema: Value,
}

/// (Anthropic) How the model thinks before it answers (RAI.Effort,
/// docs/preview4-plan.md R5).
#[derive(Clone, Debug, PartialEq)]
pub enum Thinking {
    /// Adaptive thinking at an effort ("low", "medium", "high"), with a
    /// readable summary of the reasoning when `summarized` (current models).
    Adaptive { effort: String, summarized: bool },
    /// A thinking budget, in tokens (older models, Haiku 4.5's kind).
    Budget(u32),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChatRequest {
    pub model: String,
    /// The instructions that don't change while the user works: first, so
    /// the providers' prompt caches keep them (Anthropic's cache_control;
    /// OpenAI's and DeepSeek's caches match the prefix).
    pub system: String,
    /// The instructions that change (the open project's files): after.
    pub system_tail: String,
    pub messages: Vec<Message>,
    pub tools: Vec<Tool>,
    /// The reply's length cap (cost safety): the provider stops there.
    pub max_tokens: u32,
    /// (OpenAI) `reasoning_effort`, when the model needs it said: its
    /// current models take function tools in Chat Completions only with
    /// `none` ([`crate::wire::reasoning_for`]).
    pub reasoning_effort: Option<String>,
    /// (OpenAI-compatible) Don't ask a streamed answer for its usage: the
    /// server refused `stream_options` once.
    pub no_stream_usage: bool,
    /// (Anthropic) Thinking before the answer, when the user chose an
    /// effort.
    pub thinking: Option<Thinking>,
}

/// Tokens a request used, as the provider reports them.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct Usage {
    pub input: u64,
    pub output: u64,
    /// Of `input`, how many were read from the provider's prompt cache.
    pub cached: u64,
}

impl Usage {
    pub fn add(&mut self, u: Usage) {
        self.input += u.input;
        self.output += u.output;
        self.cached += u.cached;
    }
}
