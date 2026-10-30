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
    /// Whether a streamed answer is asked for its usage
    /// (`stream_options.include_usage`).
    pub usage_in_stream: bool,
    /// The longest answer RAI asks this provider's models for (tokens,
    /// thinking aside): writing a whole program needs room, and a model
    /// that allows less says so and is asked again with its own limit.
    pub max_reply: u32,
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
        max_reply: 32_000,
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
        max_reply: 32_000,
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
        max_reply: 8_192,
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
        // (asked for: Ollama, LM Studio, mlx-serve, vLLM and llama.cpp's
        // server report it; one that refuses the field is asked again
        // without it — wire::stream_usage_refusal)
        usage_in_stream: true,
        max_reply: 8_192,
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

/// Anthropic's models and their context windows (tokens), as the
/// provider's model list says them: what RAI keeps a conversation within
/// when the list hasn't been fetched in this session (a model chosen from
/// the kept settings, or typed). The list's own `max_input_tokens` wins
/// when it has come. A dated name (`claude-haiku-4-5-20251001`) is its
/// model's.
pub const ANTHROPIC_WINDOWS: &[(&str, u64)] = &[
    ("claude-fable-5-1", 1_000_000),
    ("claude-fable-5", 1_000_000),
    ("claude-opus-5-5", 1_000_000),
    ("claude-opus-5", 1_000_000),
    ("claude-opus-4-8", 1_000_000),
    ("claude-opus-4-7", 1_000_000),
    ("claude-opus-4-6", 1_000_000),
    ("claude-sonnet-5-5", 1_000_000),
    ("claude-sonnet-5", 1_000_000),
    ("claude-sonnet-4-6", 1_000_000),
    ("claude-haiku-5-5", 1_000_000),
    ("claude-haiku-4-5", 200_000),
];

/// The smallest window of any Claude model the provider still serves: a
/// `claude-` model this table doesn't know yet is kept within it.
pub const ANTHROPIC_LEAST_WINDOW: u64 = 200_000;

/// The context window of `provider`'s `model` when RapidR knows it
/// without asking ([`ANTHROPIC_WINDOWS`]); `None` for the others (their
/// models list says it).
pub fn known_window(provider: &str, model: &str) -> Option<u64> {
    if !provider.trim().eq_ignore_ascii_case("anthropic") {
        return None;
    }
    let model = model.trim().to_ascii_lowercase();
    // (the name itself, or the name and a date or `latest`: a dated
    // snapshot is its model's — `claude-opus-5-7` is not `claude-opus-5`'s)
    let of = |id: &str| match model.strip_prefix(id) {
        Some("") => true,
        Some(rest) => rest.strip_prefix('-').is_some_and(|tag| tag == "latest" || (tag.len() == 8 && tag.bytes().all(|b| b.is_ascii_digit()))),
        None => false,
    };
    let known = ANTHROPIC_WINDOWS.iter().find(|(id, _)| of(id)).map(|(_, w)| *w);
    known.or_else(|| model.starts_with("claude-").then_some(ANTHROPIC_LEAST_WINDOW))
}

// ---- the tests' server ---------------------------------------------------------------
//
// Tests (Studio's flows, the examples' runs) point the cloud providers at a
// mock server on this computer (tests/ai_mock_server.mjs) with
// `RAPIDR_AI_TEST_SERVER=http://127.0.0.1:PORT` — on the web, the page's
// test environment (rapidr-runtime-web's ai_web.rs passes it on). Each
// provider's requests then go to `<server>/<id><its path>`
// (`/anthropic/v1/models`, `/openai/v1/chat/completions`,
// `/deepseek/models`), so the mock knows who it plays. Only a server on
// this computer is accepted: 127.0.0.1, [::1] or localhost, nothing else —
// a key is never sent anywhere else by this switch.

use std::cell::RefCell;

thread_local! {
    // (None: not looked at yet; Some(None): none; requests are made on the
    // thread of the program's loop — a test of one thread can't change
    // another's)
    static TEST_SERVER: RefCell<Option<Option<String>>> = const { RefCell::new(None) };
}

/// `url` if it is a server on this computer (`http(s)://127.0.0.1:PORT`,
/// `[::1]`, `localhost`, an optional port and path), without a trailing `/`;
/// why not otherwise.
pub fn loopback_url(url: &str) -> Result<String, String> {
    let url = url.trim().trim_end_matches('/');
    let rest = url.strip_prefix("http://").or_else(|| url.strip_prefix("https://")).ok_or("not an http:// or https:// address")?;
    let authority = rest.split(['/', '?', '#']).next().unwrap_or("");
    if authority.contains('@') {
        return Err("an address with a user name is refused".into());
    }
    let (host, port) = if let Some(v6) = authority.strip_prefix('[') {
        let (h, after) = v6.split_once(']').ok_or("a bad IPv6 address")?;
        (h, after.strip_prefix(':'))
    } else {
        match authority.split_once(':') {
            Some((h, p)) => (h, Some(p)),
            None => (authority, None),
        }
    };
    if let Some(p) = port {
        if p.is_empty() || !p.chars().all(|c| c.is_ascii_digit()) || p.parse::<u16>().is_err() {
            return Err("a bad port".into());
        }
    }
    let host = host.to_ascii_lowercase();
    if !matches!(host.as_str(), "127.0.0.1" | "::1" | "localhost") {
        return Err(format!("{host} isn't this computer (only 127.0.0.1, [::1] or localhost)"));
    }
    Ok(url.to_string())
}

/// Sets the tests' server (`None`: none); a server that isn't on this
/// computer is refused and none is set.
pub fn set_test_server(url: Option<&str>) -> Result<(), String> {
    let checked = match url.map(str::trim).filter(|u| !u.is_empty()) {
        Some(u) => match loopback_url(u) {
            Ok(u) => Some(u),
            Err(e) => {
                TEST_SERVER.with(|s| *s.borrow_mut() = Some(None));
                return Err(e);
            }
        },
        None => None,
    };
    TEST_SERVER.with(|s| *s.borrow_mut() = Some(checked));
    Ok(())
}

/// The tests' server, if one is set (on the desktop, from
/// `RAPIDR_AI_TEST_SERVER` the first time it's asked).
pub fn test_server() -> Option<String> {
    TEST_SERVER.with(|s| {
    let mut s = s.borrow_mut();
    if s.is_none() {
        #[cfg(not(target_arch = "wasm32"))]
        let from_env = std::env::var("RAPIDR_AI_TEST_SERVER").ok().filter(|v| !v.trim().is_empty());
        #[cfg(target_arch = "wasm32")]
        let from_env: Option<String> = None;
        *s = Some(match from_env {
            Some(u) => match loopback_url(&u) {
                Ok(u) => Some(u),
                Err(e) => {
                    eprintln!("RAPIDR_AI_TEST_SERVER ignored: {e}");
                    None
                }
            },
            None => None,
        });
    }
    s.clone().flatten()
    })
}

/// Where `p`'s API is: its own address, or — a test server set — that
/// server's place for it. (`compatible`'s is always the user's.)
pub fn default_base(p: &Provider) -> String {
    if p.id != "compatible" {
        if let Some(server) = test_server() {
            let path = p.base_url.split_once("://").and_then(|(_, r)| r.find('/').map(|i| &r[i..])).unwrap_or("");
            return format!("{server}/{}{path}", p.id);
        }
    }
    p.base_url.to_string()
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

    // (Robert 2026-10-28: the context meter said "121,309 of 40,000 (100%)"
    // on claude-opus-5-5 while Sonnet said "of 700,000": no window was
    // known for it, so the 40,000 fallback was used)
    #[test]
    fn every_anthropic_model_has_a_window() {
        for (id, w) in ANTHROPIC_WINDOWS {
            assert!(*w >= ANTHROPIC_LEAST_WINDOW, "{id}");
            assert_eq!(known_window("anthropic", id), Some(*w), "{id}");
            assert_eq!(known_window("Anthropic", &format!("{id}-20260401")), Some(*w), "{id}, dated");
            assert_eq!(known_window("anthropic", &format!(" {} ", id.to_uppercase())), Some(*w), "{id}, as typed");
        }
        assert_eq!(known_window("anthropic", "claude-opus-5-5"), Some(1_000_000));
        assert_eq!(known_window("anthropic", "claude-sonnet-5-5"), Some(1_000_000));
        assert_eq!(known_window("anthropic", "claude-haiku-4-5-20251001"), Some(200_000));
        // (a newer Claude than the table: the smallest window, not a
        // sibling's — claude-opus-5-7 isn't claude-opus-5)
        assert_eq!(known_window("anthropic", "claude-opus-5-7"), Some(ANTHROPIC_LEAST_WINDOW));
        assert_eq!(known_window("anthropic", "mystery"), None);
        // (other providers' models lists say theirs)
        assert_eq!(known_window("openai", "claude-opus-5-5"), None);
        assert_eq!(known_window("compatible", "claude-opus-5-5"), None);
    }

    #[test]
    fn the_tests_server_is_on_this_computer_only() {
        for ok in ["http://127.0.0.1:18541", "http://localhost:9/", "https://[::1]:443/x", "http://127.0.0.1"] {
            assert!(loopback_url(ok).is_ok(), "{ok}");
        }
        for bad in ["http://127.0.0.1.evil.example:80", "http://127.0.0.1@evil.example", "http://evil.example/127.0.0.1", "http://localhost.evil.example", "ftp://127.0.0.1", "http://10.0.0.2:80", "http://[::2]:80", "http://127.0.0.1:99999", "http://127.0.0.1:8a", "127.0.0.1:80", "http://0.0.0.0:80"] {
            assert!(loopback_url(bad).is_err(), "{bad}");
        }
        assert!(set_test_server(Some("https://api.openai.com")).is_err());
        assert_eq!(test_server(), None, "a refused server sets none");
        assert_eq!(default_base(provider("openai").unwrap()), "https://api.openai.com/v1");
        set_test_server(Some("http://127.0.0.1:18541/")).unwrap();
        assert_eq!(default_base(provider("anthropic").unwrap()), "http://127.0.0.1:18541/anthropic/v1");
        assert_eq!(default_base(provider("openai").unwrap()), "http://127.0.0.1:18541/openai/v1");
        assert_eq!(default_base(provider("deepseek").unwrap()), "http://127.0.0.1:18541/deepseek");
        assert_eq!(default_base(provider("compatible").unwrap()), "http://localhost:11434/v1", "the user's server stays the user's");
        set_test_server(None).unwrap();
        assert_eq!(default_base(provider("anthropic").unwrap()), "https://api.anthropic.com/v1");
    }
}
