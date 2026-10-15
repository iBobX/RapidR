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

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChatRequest {
    pub model: String,
    pub system: String,
    pub messages: Vec<Message>,
    pub tools: Vec<Tool>,
    /// The reply's length cap (cost safety): the provider stops there.
    pub max_tokens: u32,
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
