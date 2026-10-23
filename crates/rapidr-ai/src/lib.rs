//! RapidR's AI provider layer (docs/ide-ai.md §2, docs/ai-architecture.md
//! §4.4): one normalized request, stream and tool-call model over the two
//! wire formats that cover the providers RapidR Studio offers —
//!
//! | Format | Providers |
//! |---|---|
//! | Anthropic Messages | Anthropic (Claude) |
//! | OpenAI-compatible Chat Completions | OpenAI, DeepSeek, and any compatible server: Ollama, LM Studio, xAI, OpenRouter, vLLM, llama.cpp's server, a company proxy |
//!
//! * [`provider`] — the providers as data: base URL, auth, where the
//!   models are listed, the default model, the pricing page.
//! * [`chat`] — the normalized conversation: messages of text, tool calls
//!   and tool results; tools with JSON Schemas; usage.
//! * [`wire`] — a [`chat::ChatRequest`] as each format's HTTP request, the
//!   models list request and its answer, errors explained in plain words,
//!   "what was sent" (the key never shown).
//! * [`sse`] / [`stream`] — the streamed answer: server-sent events decoded
//!   into normalized [`stream::Event`]s and gathered into a [`stream::Reply`].
//! * [`http`] — the transport, the platform's: a thread and ureq (the
//!   system's TLS) on the desktop, `fetch` in a browser; requests run in
//!   the background and their news are taken by the caller's loop.
//! * `rai` (feature `component`) — **RAI**, the component: a conversation
//!   with a provider in a program or in RapidR Studio, its tools attached.
//! * [`audit`] — the local audit log of what an assistant asked and did.
//! * [`chats`] — RapidR Studio's kept chats: per project, in Studio's data
//!   folder (never the project's), keys and tokens redacted.
//! * [`claims`] — whether an answer says it changed something (a small
//!   model that says so without calling a tool is asked once to do it).
//! * [`lang`] — the language a message is written in, so the answer is in
//!   the user's language whatever the conversation's summary or files are
//!   written in.
//! * [`keys`] — API keys: the OS keychain on the desktop (macOS Keychain,
//!   Windows Credential Manager, the Secret Service), the tab's session on
//!   the web (remembered on the device only when the user asks); the
//!   providers' environment variables are read, never written.
//! * [`secrets`] — RSecrets: the same keychain for programs' own secrets,
//!   each program under its own services (`rapidr-app:<program>:…`); none
//!   on the web.
//!
//! No vendor SDKs (plain HTTPS and JSON), permissive dependencies only.
//! Nothing here sends anything by itself: every request is one the caller
//! (RapidR Studio's assistant, the `rapidr ai` command) starts on the
//! user's action.

pub mod audit;
pub mod chat;
pub mod chats;
pub mod claims;
pub mod http;
pub mod keys;
pub mod lang;
#[cfg(not(target_arch = "wasm32"))]
pub(crate) mod private_files;
pub mod provider;
#[cfg(feature = "component")]
pub mod rai;
pub mod secrets;
pub mod sse;
pub mod stream;
pub mod wire;

pub use chat::{Block, ChatRequest, Message, Role, Thinking, Tool, Usage};
pub use provider::{provider, Format, Provider, PROVIDERS};
pub use stream::{Decoder, Event, Reply};
pub use wire::{ApiError, Endpoint, HttpRequest, ModelInfo};

/// The security regressions (tests/security/, `tools/regress.sh security`):
/// whose keys and secrets a program reaches (SEC-21, SEC-22).
#[cfg(test)]
#[path = "../../../tests/security/keychain_scopes.rs"]
mod security_regressions;

/// The security regressions (tests/security/): what RAI sends and keeps —
/// no redirect with the key (SEC-27), files only the user's (SEC-28).
#[cfg(all(test, not(target_arch = "wasm32")))]
#[path = "../../../tests/security/ai_transport.rs"]
mod security_regressions_transport;
