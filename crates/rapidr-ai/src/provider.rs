//! The providers, as data (docs/ide-ai.md §2.1): what differs between them
//! is a base URL, how the key is sent, where the models are listed and
//! where the prices are. No model is named here: models come and go, so
//! the list always comes from the provider (Fetch Models) and the user
//! picks one ([`crate::wire::model_hint`] marks the fast, low-cost ones by
//! the words in their names).

/// The wire format a provider speaks.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Format {
    /// Anthropic's Messages API (`POST /v1/messages`, `x-api-key`).
    Anthropic,
    /// OpenAI's Chat Completions (`POST /chat/completions`, `Authorization:
    /// Bearer`), which DeepSeek, Ollama, LM Studio, xAI … speak too.
    OpenAi,
}

#[derive(Debug)]
pub struct Provider {
    /// The id settings and the component use: `anthropic`, `openai`,
    /// `deepseek`, `compatible`.
    pub id: &'static str,
    /// As the settings list it.
    pub name: &'static str,
    /// In a narrow list (the AI panel's).
    pub short_name: &'static str,
    pub format: Format,
    /// Where its API is (no trailing `/`); `compatible`'s is the user's
    /// (Ollama's by default).
    pub base_url: &'static str,
    /// Whether it needs an API key (a local server needs none).
    pub needs_key: bool,
    /// The provider's own price list ("See costs").
    pub pricing_url: &'static str,
    /// Where the user makes a key.
    pub keys_url: &'static str,
    /// The environment variable a key may come from (read, never written).
    pub env_key: &'static str,
    /// The request's field for the reply's length cap: OpenAI's current
    /// models want `max_completion_tokens`, the others `max_tokens`.
    pub max_tokens_field: &'static str,
    /// Whether a streamed answer may be asked for its usage
    /// (`stream_options.include_usage`): not every compatible server knows
    /// the field.
    pub usage_in_stream: bool,
}

/// The providers RapidR Studio offers, in the order it lists them.
pub const PROVIDERS: &[Provider] = &[
    Provider {
        id: "anthropic",
        name: "Anthropic (Claude)",
        short_name: "Claude",
        format: Format::Anthropic,
        base_url: "https://api.anthropic.com/v1",
        needs_key: true,
        pricing_url: "https://www.anthropic.com/pricing",
        keys_url: "https://platform.claude.com/settings/keys",
        env_key: "ANTHROPIC_API_KEY",
        max_tokens_field: "max_tokens",
        usage_in_stream: true,
    },
    Provider {
        id: "openai",
        name: "OpenAI",
        short_name: "OpenAI",
        format: Format::OpenAi,
        base_url: "https://api.openai.com/v1",
        needs_key: true,
        pricing_url: "https://openai.com/api/pricing/",
        keys_url: "https://platform.openai.com/api-keys",
        env_key: "OPENAI_API_KEY",
        max_tokens_field: "max_completion_tokens",
        usage_in_stream: true,
    },
    Provider {
        id: "deepseek",
        name: "DeepSeek",
        short_name: "DeepSeek",
        format: Format::OpenAi,
        base_url: "https://api.deepseek.com",
        needs_key: true,
        pricing_url: "https://api-docs.deepseek.com/quick_start/pricing",
        keys_url: "https://platform.deepseek.com/api_keys",
        env_key: "DEEPSEEK_API_KEY",
        max_tokens_field: "max_tokens",
        usage_in_stream: true,
    },
    Provider {
        id: "compatible",
        name: "OpenAI-compatible (Ollama, LM Studio, xAI …)",
        short_name: "Local / other",
        format: Format::OpenAi,
        base_url: "http://localhost:11434/v1",
        needs_key: false,
        pricing_url: "",
        keys_url: "",
        env_key: "",
        max_tokens_field: "max_tokens",
        usage_in_stream: false,
    },
];

/// Base URLs the settings offer for an OpenAI-compatible server: (label,
/// URL). Local servers first: they keep the project on this computer.
pub const COMPATIBLE_PRESETS: &[(&str, &str)] = &[
    ("Ollama (this computer)", "http://localhost:11434/v1"),
    ("LM Studio (this computer)", "http://localhost:1234/v1"),
    ("xAI (Grok)", "https://api.x.ai/v1"),
    ("OpenRouter", "https://openrouter.ai/api/v1"),
];

/// A provider by id (any case).
pub fn provider(id: &str) -> Option<&'static Provider> {
    PROVIDERS.iter().find(|p| p.id.eq_ignore_ascii_case(id.trim()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn providers_are_whole() {
        for p in PROVIDERS {
            assert!(!p.base_url.ends_with('/'), "{}", p.id);
            assert!(p.needs_key == !p.env_key.is_empty(), "{}", p.id);
            assert!(p.pricing_url.is_empty() || p.pricing_url.starts_with("https://"), "{}", p.id);
        }
        assert_eq!(provider(" anthropic ").unwrap().format, Format::Anthropic);
        assert!(provider("gemini").is_none());
    }
}
