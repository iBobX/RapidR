//! `rapidr ai`: RapidR's AI provider layer from the terminal (`rapidr-ai`,
//! the one RapidR Studio's assistant uses) — to check a provider, a key
//! and a model without opening Studio.
//!
//!   rapidr ai providers                                 the providers, where keys are kept
//!   rapidr ai models --provider P [--base-url URL]       the models the key can use
//!   rapidr ai ask --provider P [--model M] [--max-tokens N] [--base-url URL] "question"
//!
//! The key is the one RapidR Studio keeps for the provider (the OS
//! keychain's "RapidR Studio/<provider>"), else the provider's environment
//! variable (ANTHROPIC_API_KEY, OPENAI_API_KEY, DEEPSEEK_API_KEY;
//! Anthropic's workspace: ANTHROPIC_WORKSPACE_ID); `--key-source` names
//! another (`keychain:NAME|env:VAR`). It is never printed. No model is
//! assumed: `ask` needs `--model` (`models` lists them). `ask` sends only
//! the question (no project, no tools) and streams the answer; the tokens
//! used go to standard error.

use std::io::Write;
use std::process::ExitCode;

use rapidr_ai::http::{run_blocking, Options};
use rapidr_ai::stream::{Decoder, Event, Reply};
use rapidr_ai::wire::{self, Endpoint};
use rapidr_ai::{keys, ChatRequest, Message, PROVIDERS};

fn opt(args: &[String], name: &str) -> Option<String> {
    args.iter().position(|a| a == name).and_then(|i| args.get(i + 1)).cloned()
}

fn endpoint(args: &[String]) -> Result<Endpoint, String> {
    let id = opt(args, "--provider").ok_or("give --provider (rapidr ai providers lists them)")?;
    let p = rapidr_ai::provider(&id).ok_or_else(|| format!("no provider `{id}` (rapidr ai providers lists them)"))?;
    let mut ep = Endpoint::new(p);
    ep.base_url = opt(args, "--base-url").unwrap_or_default();
    ep.workspace = opt(args, "--workspace").unwrap_or_else(|| keys::env_workspace(p.id));
    match keys::resolve(&opt(args, "--key-source").unwrap_or_else(|| studio_source(p))).0 {
        Some(k) => ep.key = k,
        None if p.needs_key => return Err(format!("no {} key: set {} (or save one in RapidR Studio: Tools > Settings > AI)", wire::short_name(p), p.env_key)),
        None => {}
    }
    Ok(ep)
}

/// Where RapidR Studio keeps a provider's key, then its usual variable.
fn studio_source(p: &rapidr_ai::Provider) -> String {
    let mut s = format!("keychain:RapidR Studio/{}", p.id);
    if !p.env_key.is_empty() {
        s.push_str(&format!("|env:{}", p.env_key));
    }
    s
}

pub fn command(args: &[String]) -> ExitCode {
    let run = || -> Result<(), String> {
        match args.first().map(String::as_str) {
            Some("providers") => {
                for p in PROVIDERS {
                    let key = match keys::resolve(&studio_source(p)) {
                        (Some(k), keys::Source::Environment) => format!("key from {} ({})", p.env_key, keys::hint(&k)),
                        (Some(k), _) => format!("key kept ({})", keys::hint(&k)),
                        (None, _) if p.needs_key => "no key".to_string(),
                        (None, _) => "no key needed".to_string(),
                    };
                    println!("{:<11} {:<46} {key}", p.id, p.name);
                }
                println!("Keys are kept in {}.", keys::store_name());
                Ok(())
            }
            Some("models") => {
                let ep = endpoint(&args[1..])?;
                let req = wire::models_request(&ep);
                let mut body = Vec::new();
                let (status, retry) = run_blocking(&req, Options::default(), |d| body.extend_from_slice(d)).map_err(|e| wire::explain_connection(&ep, &e).message)?;
                let text = String::from_utf8_lossy(&body);
                if status != 200 {
                    return Err(wire::explain_error(ep.provider, status, &text, retry, "").message);
                }
                let models = wire::parse_models(ep.provider, &text)?;
                for m in &models {
                    let hint = wire::model_hint(&format!("{} {}", m.id, m.name));
                    println!("{}{}{}", m.id, if m.name.is_empty() { String::new() } else { format!("  ({})", m.name) }, if hint.is_empty() { String::new() } else { format!("  {hint}") });
                }
                eprintln!("{} models, the fast, low-cost ones first", models.len());
                Ok(())
            }
            Some("ask") => {
                let rest = &args[1..];
                let ep = endpoint(rest)?;
                let question = rest.iter().enumerate().filter(|(i, a)| !a.starts_with("--") && (*i == 0 || !rest[i - 1].starts_with("--"))).map(|(_, a)| a.as_str()).collect::<Vec<_>>().join(" ");
                if question.trim().is_empty() {
                    return Err("give the question".into());
                }
                let model = opt(rest, "--model").filter(|m| !m.is_empty()).ok_or("give --model: no model is assumed (rapidr ai models lists the provider's)")?;
                let max = opt(rest, "--max-tokens").and_then(|m| m.parse().ok()).unwrap_or(1024);
                let req = ChatRequest { model: model.clone(), system: String::new(), system_tail: String::new(), messages: vec![Message::user(&question)], tools: Vec::new(), max_tokens: max, reasoning_effort: None, no_stream_usage: false, thinking: None };
                let http = wire::chat_request(&ep, &req);
                let mut decoder = Decoder::new(ep.provider.format);
                let mut reply = Reply::default();
                let mut err_body = Vec::new();
                let mut out = std::io::stdout();
                let result = run_blocking(&http, Options::default(), |d| {
                    for e in decoder.feed(d) {
                        if let Event::Text(t) = &e {
                            let _ = out.write_all(t.as_bytes());
                            let _ = out.flush();
                        }
                        reply.apply(&e);
                    }
                    err_body.extend_from_slice(d);
                });
                let (code, retry) = result.map_err(|e| wire::explain_connection(&ep, &e).message)?;
                if code != 200 {
                    return Err(wire::explain_error(ep.provider, code, &String::from_utf8_lossy(&err_body), retry, &model).message);
                }
                for e in decoder.finish() {
                    reply.apply(&e);
                }
                println!();
                if let Some(e) = reply.error {
                    return Err(e);
                }
                eprintln!("[{model}: {} tokens in, {} out; stopped: {}]", reply.usage.input, reply.usage.output, reply.stop);
                Ok(())
            }
            _ => Err("usage: rapidr ai providers | models --provider P | ask --provider P [--model M] \"question\"".into()),
        }
    };
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("rapidr ai: {e}");
            ExitCode::FAILURE
        }
    }
}
