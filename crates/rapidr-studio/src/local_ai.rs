//! Local AI made easy (docs/local-ai-plan.md): the AI models already on
//! this computer, whether each fits it, an optional check of each, and the
//! best match — RPROJECT's `LocalAI…` members, Set Up RAI's "On this
//! computer" page.
//!
//! * **Found, never suggested.** Only when the user opens the page:
//!   servers on `127.0.0.1` that speak the OpenAI-compatible API are asked
//!   for their models at the usual ports and at an address the user types
//!   — never another computer ([`local_url`] refuses one). Each model is
//!   described as its server reports it (size, parameters, quantization,
//!   context, tools, layers and heads, its licence); a server that only
//!   lists names gets rows with what is known. RapidR names, ranks or
//!   suggests no model the user doesn't have: there is no list of models
//!   anywhere in RapidR.
//! * **The fit check** ([`fit`], instant, nothing runs): the weights, the
//!   conversation's cache for the context ([`kv_bytes`]: layers × key/value
//!   heads × head size) and the runtime's overhead against the memory free
//!   now and the memory a model could have with other apps closed —
//!   Fits / Fits if you close other apps / Too big. The context is chosen
//!   here: 8K by default, 16K when there's room, never below 4K.
//! * **Check my models** (the user starts it; one model at a time, Cancel
//!   at any moment, a thread of its own — the window never waits): how
//!   fast an answer starts and goes on, whether a tool call arrives as a
//!   call, and a tiny RapidR task through RAI's real loop and Studio's
//!   real tools ("a window with a button that says Hello": it compiles and
//!   the button is there — nothing the model wrote is run). Results are
//!   kept per model (its digest) and per Studio version and RAI's prompts
//!   ([`cache_key`]): a model is checked again only when it or they change.
//! * **The best match** ([`rank`]): passes the task › tools work › fast
//!   enough (an answer starts within [`FAST_ENOUGH_S`]) › memory to spare.
//!   RAI's small-model mode is chosen from the check ([`small_mode`]),
//!   never from a model's name.
//!
//! Everything stays here: the computer's facts, the models' and the
//! checks' results are never sent anywhere.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

use rapidr_value::sysinfo::{gb_words, Facts};
use rapidr_value::Value;
use serde_json::{json, Value as Json};

const MB: u64 = 1024 * 1024;
const GB: u64 = 1024 * MB;

/// The ports local AI servers usually answer on (the address the user
/// types is asked too).
pub const USUAL_PORTS: &[u16] = &[11434, 1234, 8080, 8000, 1337, 5001, 4891, 8081];
/// The context RAI is given by default, the one when there's room, the
/// least (tokens).
pub const CONTEXT_DEFAULT: u64 = 8192;
pub const CONTEXT_ROOMY: u64 = 16384;
pub const CONTEXT_LEAST: u64 = 4096;
/// The memory kept for RapidR Studio and the system before a model's
/// need is compared with what is free.
pub const KEEP_FREE: u64 = 1536 * MB;
/// An answer that starts within this many seconds is fast enough.
pub const FAST_ENOUGH_S: f64 = 10.0;
/// The tiny RapidR task of a check.
pub const TASK: &str = "Build me a window with a button that says Hello.";
/// The longest the task may take before it counts as not done (seconds).
const TASK_SECONDS: u64 = 240;
/// RAI's small-model mode below this context (as RAI decides it).
const LITE_BELOW: u64 = rapidr_ai::rai::LITE_BELOW;

// ---- the models found --------------------------------------------------------------

/// A model on a server on this computer, as the server reports it (`None`
/// / "": the server doesn't say).
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Model {
    pub id: String,
    /// The server's OpenAI-compatible address (`http://127.0.0.1:PORT/v1`).
    pub base: String,
    /// The model file's size in bytes.
    pub size: Option<u64>,
    pub params: Option<u64>,
    pub quant: String,
    pub family: String,
    /// The longest context it was made for (tokens).
    pub max_context: Option<u64>,
    /// It takes tools (`None`: the server doesn't say; the check tells).
    pub tools: Option<bool>,
    /// It chats (false: an embedding model, which only turns text into numbers).
    pub chat: bool,
    pub layers: Option<u64>,
    pub kv_heads: Option<u64>,
    pub head_dim: Option<u64>,
    /// Its licence's name as the server reports it ("apache-2.0", the
    /// licence text's first line).
    pub licence: String,
    pub licence_text: String,
    /// The server's digest of the model (what the checks are kept by).
    pub digest: String,
    /// It is in memory now (its memory already counted as used).
    pub loaded: bool,
    /// The server answers more than the OpenAI-compatible list (sizes,
    /// details, unloading).
    pub rich: bool,
    /// Its app runs it on a service elsewhere (it says so): what is asked
    /// goes there — never the best match, never checked, never picked.
    pub remote: bool,
}

impl Model {
    fn new(id: &str, base: &str) -> Model {
        Model { id: id.to_string(), base: base.to_string(), chat: true, ..Model::default() }
    }

    /// The weights' bytes: the file's size, else the parameters at the
    /// quantization's bits.
    pub fn weights(&self) -> Option<u64> {
        self.size.or_else(|| self.params.map(|p| p * quant_bits(&self.quant) / 8))
    }

    /// The server's root (its address without `/v1`).
    pub fn root(&self) -> String {
        root_of(&self.base)
    }
}

/// A quantization's bits a weight ("Q4_K_M" 4.5, "Q8_0" 8.5, "F16" 16;
/// 5 when unknown: most local models are 4-bit, and more is surer).
fn quant_bits(q: &str) -> u64 {
    let q = q.to_ascii_uppercase();
    let digits: String = q.chars().skip_while(|c| !c.is_ascii_digit()).take_while(|c| c.is_ascii_digit()).collect();
    match digits.parse::<u64>() {
        Ok(16) | Ok(32) if q.starts_with('F') || q.starts_with("BF") => digits.parse().unwrap_or(16),
        Ok(b @ 1..=8) => b + 1,
        _ => 5,
    }
}

/// A parameter count as a server writes it ("3.4B", "135M", "1.2T").
fn parse_params(t: &str) -> Option<u64> {
    let t = t.trim().to_ascii_uppercase();
    let (num, mul) = match t.chars().last()? {
        'B' => (&t[..t.len() - 1], 1e9),
        'M' => (&t[..t.len() - 1], 1e6),
        'K' => (&t[..t.len() - 1], 1e3),
        'T' => (&t[..t.len() - 1], 1e12),
        _ => (t.as_str(), 1.0),
    };
    num.trim().parse::<f64>().ok().filter(|n| *n > 0.0).map(|n| (n * mul) as u64)
}

// ---- the fit check -----------------------------------------------------------------

/// Whether a model fits this computer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    /// Comfortably, in the memory free now.
    Fits,
    /// In the memory a model could have with other apps closed.
    CloseApps,
    /// Not even then: shown, never picked.
    TooBig,
    /// Its size or the computer's memory isn't known.
    Unknown,
}

impl Verdict {
    pub fn word(self) -> &'static str {
        match self {
            Verdict::Fits => "fits",
            Verdict::CloseApps => "close",
            Verdict::TooBig => "toobig",
            Verdict::Unknown => "unknown",
        }
    }
}

/// The memory a model may have here: now (what is free, less what Studio
/// and the system keep) and at most (with other apps closed: on shared
/// memory the part a GPU may take, else all but the system's own).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Memory {
    pub now: u64,
    pub most: u64,
}

impl Memory {
    pub fn of(f: &Facts) -> Option<Memory> {
        let total = f.total_memory?;
        let free = f.free_memory.unwrap_or(total / 2);
        let most = if f.unified_memory { f.gpu_memory.unwrap_or(total / 4 * 3) } else { total.saturating_sub(3 * GB) };
        Some(Memory { now: free.saturating_sub(KEEP_FREE).min(most), most })
    }
}

/// The conversation's cache for `context` tokens: keys and values (2), in
/// 16-bit numbers (2 bytes), for each layer's key/value heads; a model
/// whose shape isn't known: 128 KB a token (an 8-billion-parameter model's).
pub fn kv_bytes(m: &Model, context: u64) -> u64 {
    match (m.layers, m.kv_heads, m.head_dim) {
        (Some(l), Some(h), Some(d)) if l > 0 && h > 0 && d > 0 => 2 * l * h * d * 2 * context,
        _ => 128 * 1024 * context,
    }
}

/// The memory a model needs with `context`: its weights, the cache, the
/// runtime's overhead (half a gigabyte and a tenth of the weights for its
/// working buffers).
pub fn need(m: &Model, context: u64) -> Option<u64> {
    let w = m.weights()?;
    Some(w + kv_bytes(m, context) + 512 * MB + w / 10)
}

/// A model's fit and the context RAI gets with it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Fit {
    pub verdict: Verdict,
    pub context: u64,
    pub need: Option<u64>,
}

/// The fit check: 16K when it needs at most 80% of what is free now, else
/// 8K, else 4K that fit now; else 8K or 4K with other apps closed; else too
/// big. A model already in memory counts its own memory as free.
pub fn fit(m: &Model, mem: Option<Memory>) -> Fit {
    let cap = |c: u64| m.max_context.filter(|mx| *mx > 0).map_or(c, |mx| c.min(mx));
    let (c16, c8, c4) = (cap(CONTEXT_ROOMY), cap(CONTEXT_DEFAULT), cap(CONTEXT_LEAST));
    let Some(n8) = need(m, c8) else { return Fit { verdict: Verdict::Unknown, context: c8, need: None } };
    let Some(mem) = mem else { return Fit { verdict: Verdict::Unknown, context: c8, need: Some(n8) } };
    let now = if m.loaded { (mem.now + m.size.unwrap_or(0)).min(mem.most) } else { mem.now };
    let n = |c: u64| need(m, c).unwrap_or(n8);
    let at = |verdict, context| Fit { verdict, context, need: Some(n(context)) };
    if n(c16) as f64 <= now as f64 * 0.8 {
        at(Verdict::Fits, c16)
    } else if n8 <= now {
        at(Verdict::Fits, c8)
    } else if n(c4) <= now {
        at(Verdict::Fits, c4)
    } else if n8 <= mem.most {
        at(Verdict::CloseApps, c8)
    } else if n(c4) <= mem.most {
        at(Verdict::CloseApps, c4)
    } else {
        at(Verdict::TooBig, c4)
    }
}

// ---- the check ------------------------------------------------------------------

/// What a check found.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Check {
    /// Seconds before RAI's first answer starts (the model already loaded).
    pub first_s: f64,
    /// Words a second after that.
    pub words_s: f64,
    /// Seconds the server took to load the model.
    pub load_s: f64,
    /// A tool call arrived as a call, with its arguments as JSON.
    pub tools: bool,
    /// Why not, in a few words ("" when it did).
    pub tools_why: String,
    /// The tiny task: it compiles and the window has its Hello button.
    pub task: bool,
    pub task_why: String,
    /// The context the server runs the model with, when it says (tokens).
    pub server_context: u64,
}

impl Check {
    fn to_json(&self) -> Json {
        json!({"firstS": self.first_s, "wordsS": self.words_s, "loadS": self.load_s, "tools": self.tools, "toolsWhy": self.tools_why, "task": self.task, "taskWhy": self.task_why, "serverContext": self.server_context})
    }

    fn of_json(v: &Json) -> Option<Check> {
        Some(Check {
            first_s: v["firstS"].as_f64()?,
            words_s: v["wordsS"].as_f64().unwrap_or(0.0),
            load_s: v["loadS"].as_f64().unwrap_or(0.0),
            tools: v["tools"].as_bool()?,
            tools_why: v["toolsWhy"].as_str().unwrap_or("").to_string(),
            task: v["task"].as_bool()?,
            task_why: v["taskWhy"].as_str().unwrap_or("").to_string(),
            server_context: v["serverContext"].as_u64().unwrap_or(0),
        })
    }
}

/// RAI's small-model mode for a model: from its check (full only when it
/// built the test app, its tool calls worked, it answers fast enough and
/// gets 16K of context); unchecked, by the context (as RAI decides).
pub fn small_mode(fit: &Fit, check: Option<&Check>) -> bool {
    let context = context_for(fit, check);
    match check {
        Some(c) => !(c.task && c.tools && c.first_s <= FAST_ENOUGH_S && context >= LITE_BELOW),
        None => context < LITE_BELOW,
    }
}

/// The context RAI gets: the fit's, never more than the server runs the
/// model with.
pub fn context_for(fit: &Fit, check: Option<&Check>) -> u64 {
    match check.map(|c| c.server_context).filter(|c| *c > 0) {
        Some(s) => fit.context.min(s),
        None => fit.context,
    }
}

/// A model, its fit and its check.
#[derive(Clone, Debug)]
pub struct Row {
    pub model: Model,
    pub fit: Fit,
    pub check: Option<Check>,
    /// The check came from an earlier one (kept).
    pub kept: bool,
    /// Why its check couldn't finish ("" none).
    pub error: String,
}

impl Row {
    fn tools_ok(&self) -> bool {
        self.check.as_ref().map_or(self.model.tools.unwrap_or(true), |c| c.tools)
    }
    fn usable(&self) -> bool {
        self.model.chat && !self.model.remote && !matches!(self.fit.verdict, Verdict::TooBig)
    }
    /// Checked by Check My Models: usable, it fits now or its size isn't
    /// known (a model that would need other apps closed waits), unchecked.
    fn to_check(&self) -> bool {
        self.usable() && matches!(self.fit.verdict, Verdict::Fits | Verdict::Unknown) && self.check.is_none()
    }
}

/// The rows' order, best first: usable (chats, not too big) › passes the
/// task › tools work › fast enough › memory to spare (what is free now
/// less what it needs); then as the servers listed them.
pub fn rank(rows: &[Row], mem: Option<Memory>) -> Vec<usize> {
    let spare = |r: &Row| -> i64 { mem.map_or(0, |m| m.now as i64) - r.fit.need.map_or(i64::MAX / 4, |n| n as i64) };
    let mut order: Vec<usize> = (0..rows.len()).collect();
    order.sort_by(|&a, &b| {
        let key = |r: &Row| {
            (
                r.usable(),
                r.check.as_ref().is_some_and(|c| c.task),
                r.tools_ok(),
                r.check.as_ref().is_none_or(|c| c.first_s <= FAST_ENOUGH_S),
                matches!(r.fit.verdict, Verdict::Fits),
            )
        };
        key(&rows[b]).cmp(&key(&rows[a])).then_with(|| spare(&rows[b]).cmp(&spare(&rows[a]))).then_with(|| a.cmp(&b))
    });
    order
}

/// The best match: the first row that is usable, takes tools and, when
/// checked, built the test app (`None`: none is).
pub fn best(rows: &[Row], order: &[usize]) -> Option<usize> {
    order.iter().copied().find(|&i| {
        let r = &rows[i];
        r.usable() && r.tools_ok() && r.check.as_ref().is_none_or(|c| c.task)
    })
}

/// The seconds a check of `m` takes, about: loading it and three short
/// conversations — more the larger it is.
pub fn estimate_s(m: &Model) -> u64 {
    m.weights().map_or(120, |w| 45 + 15 * w / GB)
}

// ---- where a model's checks are kept -------------------------------------------------------

fn fnv(text: &str) -> u64 {
    let mut h: u64 = 0xcbf2_9ce4_8422_2325;
    for b in text.bytes() {
        h ^= b as u64;
        h = h.wrapping_mul(0x0000_0100_0000_01b3);
    }
    h
}

/// What RAI tells a model and the task: a check is done again when they
/// change.
fn prompts_mark() -> String {
    format!("{:016x}", fnv(&format!("{}\u{0}{}\u{0}{TASK}", crate::ai::instructions(true), crate::ai::instructions_lite(true))))
}

/// A check's key: the model (its digest, else its server, name and size),
/// this Studio's version, RAI's prompts.
pub fn cache_key(m: &Model) -> String {
    let who = if m.digest.is_empty() { format!("{}#{}#{}", m.base, m.id, m.size.unwrap_or(0)) } else { m.digest.clone() };
    format!("{who}|{}|{}", env!("CARGO_PKG_VERSION"), prompts_mark())
}

/// (unit tests) The folder the checks are kept in, in place of Studio's
/// (Studio's own folder is RAPIDR_STUDIO_DATA's in its tests).
static TEST_CACHE_DIR: Mutex<Option<String>> = Mutex::new(None);
/// (unit tests) One test at a time uses the page's state and the kept checks.
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn cache_file() -> String {
    let dir = TEST_CACHE_DIR.lock().unwrap_or_else(|e| e.into_inner()).clone().unwrap_or_else(crate::recovery::studio_folder);
    format!("{dir}/local-ai-checks.json")
}

fn cache_read() -> serde_json::Map<String, Json> {
    rapidr_value::objects::read_file(&cache_file()).ok().and_then(|b| serde_json::from_slice::<Json>(&b).ok()).and_then(|v| v.as_object().cloned()).unwrap_or_default()
}

fn cache_write(key: &str, c: &Check) {
    let mut all = cache_read();
    all.insert(key.to_string(), c.to_json());
    // (the newest 200 checks)
    while all.len() > 200 {
        if let Some(k) = all.keys().next().cloned() {
            all.remove(&k);
        }
    }
    let path = cache_file();
    if let Some(dir) = std::path::Path::new(&path).parent() {
        let _ = std::fs::create_dir_all(dir);
    }
    let _ = std::fs::write(&path, serde_json::to_string_pretty(&Json::Object(all)).unwrap_or_default());
}

// ---- addresses -------------------------------------------------------------------

/// An address on this computer, as it is asked: `127.0.0.1` for
/// `localhost` (the name could point elsewhere), its OpenAI-compatible
/// path (`/v1`) added when missing. Any other computer is refused.
pub fn local_url(url: &str) -> Result<String, String> {
    let t = url.trim().trim_end_matches('/');
    let t = if t.contains("://") { t.to_string() } else { format!("http://{t}") };
    let ok = rapidr_ai::provider::loopback_url(&t).map_err(|_| rapidr_value::i18n::tr_ui_fill("{address} isn't an address on this computer (only 127.0.0.1, [::1] or localhost).", &[("address", url.trim())]))?;
    let (scheme, rest) = ok.split_once("://").unwrap_or(("http", ok.as_str()));
    let (auth, path) = rest.split_once('/').map_or((rest, ""), |(a, p)| (a, p));
    let auth = if auth.eq_ignore_ascii_case("localhost") || auth.to_ascii_lowercase().starts_with("localhost:") { format!("127.0.0.1{}", &auth[9..]) } else { auth.to_string() };
    let path = path.trim_matches('/');
    let path = if path.is_empty() { "v1".to_string() } else { path.to_string() };
    Ok(format!("{scheme}://{auth}/{path}"))
}

fn root_of(base: &str) -> String {
    base.trim_end_matches('/').strip_suffix("/v1").unwrap_or(base.trim_end_matches('/')).to_string()
}

/// The usual addresses.
pub fn usual() -> Vec<String> {
    USUAL_PORTS.iter().map(|p| format!("http://127.0.0.1:{p}/v1")).collect()
}

// ---- asking a server (the desktop) ------------------------------------------------------

#[cfg(not(target_arch = "wasm32"))]
mod net {
    use super::*;
    use rapidr_ai::http::{run_blocking, Options};
    use rapidr_ai::HttpRequest;

    /// A request to this computer only (the address checked again), its
    /// status and body.
    pub fn ask(method: &'static str, url: &str, body: Option<Json>, read_timeout: u64) -> Result<(u16, Vec<u8>), String> {
        let base = url.split("/v1").next().unwrap_or(url);
        local_url(base)?;
        let req = HttpRequest { method, url: url.to_string(), headers: vec![("content-type".into(), "application/json".into())], body: body.map(|b| b.to_string()) };
        let mut out = Vec::new();
        let (status, _) = run_blocking(&req, Options { connect_timeout: 2, read_timeout, retries: 0 }, |d| {
            // (a local server that sends too much: kept to 16 MB)
            if out.len() < 16 * 1024 * 1024 {
                out.extend_from_slice(d)
            }
        })?;
        Ok((status, out))
    }

    pub fn json(method: &'static str, url: &str, body: Option<Json>, read_timeout: u64) -> Option<Json> {
        match ask(method, url, body, read_timeout) {
            Ok((200, b)) => serde_json::from_slice(&b).ok(),
            _ => None,
        }
    }

    /// Whether something listens at the address's port (a closed port is
    /// told at once: no request is made to it).
    pub fn listening(base: &str) -> bool {
        let rest = base.split_once("://").map_or(base, |(_, r)| r);
        let auth = rest.split('/').next().unwrap_or("");
        let Ok(addr) = auth.parse::<std::net::SocketAddr>().or_else(|_| format!("{auth}:80").parse()) else { return false };
        std::net::TcpStream::connect_timeout(&addr, std::time::Duration::from_millis(400)).is_ok()
    }
}

/// The models a server at `base` reports (`None`: no OpenAI-compatible
/// server answers there).
#[cfg(not(target_arch = "wasm32"))]
pub fn server_models(base: &str) -> Option<Vec<Model>> {
    if !net::listening(base) {
        return None;
    }
    let list = net::json("GET", &format!("{base}/models"), None, 5)?;
    let data = list["data"].as_array()?;
    let mut models: Vec<Model> = Vec::new();
    for d in data {
        let Some(id) = d["id"].as_str().filter(|s| !s.is_empty()) else { continue };
        let mut m = Model::new(id, base);
        // (a server that writes each model's facts into the list: its
        // parameters, file size, the context it was made for)
        let meta = &d["meta"];
        m.params = meta["n_params"].as_u64();
        m.size = meta["size"].as_u64();
        m.max_context = meta["n_ctx_train"].as_u64().or_else(|| d["context_length"].as_u64()).or_else(|| d["max_context_length"].as_u64());
        models.push(m);
    }
    let root = root_of(base);
    // (a server that also describes its models: sizes, digests, details)
    if let Some(tags) = net::json("GET", &format!("{root}/api/tags"), None, 5) {
        let loaded: Vec<String> = net::json("GET", &format!("{root}/api/ps"), None, 5).and_then(|v| v["models"].as_array().cloned()).unwrap_or_default().iter().filter_map(|m| m["name"].as_str().or_else(|| m["model"].as_str()).map(str::to_string)).collect();
        for t in tags["models"].as_array().cloned().unwrap_or_default() {
            let name = t["name"].as_str().or_else(|| t["model"].as_str()).unwrap_or("");
            let Some(m) = models.iter_mut().find(|m| m.id == name) else { continue };
            m.rich = true;
            m.size = t["size"].as_u64().or(m.size);
            m.digest = t["digest"].as_str().unwrap_or("").to_string();
            let det = &t["details"];
            m.params = det["parameter_size"].as_str().and_then(parse_params).or(m.params);
            m.quant = det["quantization_level"].as_str().unwrap_or("").to_string();
            m.family = det["family"].as_str().unwrap_or("").to_string();
            m.max_context = det["context_length"].as_u64().or(m.max_context);
            m.loaded = loaded.iter().any(|l| l == name);
            m.remote = ["remote_host", "remote_model"].iter().any(|k| t[*k].as_str().is_some_and(|v| !v.trim().is_empty()));
        }
        for m in models.iter_mut().filter(|m| m.rich) {
            if let Some(show) = net::json("POST", &format!("{root}/api/show"), Some(json!({"model": m.id})), 10) {
                describe(m, &show);
            }
        }
    } else if let Some(v0) = net::json("GET", &format!("{root}/api/v0/models"), None, 5) {
        // (a server whose own list says each model's kind, shape, its
        // quantization and context, and whether it is loaded)
        for d in v0["data"].as_array().cloned().unwrap_or_default() {
            let Some(m) = models.iter_mut().find(|m| Some(m.id.as_str()) == d["id"].as_str()) else { continue };
            m.chat = !matches!(d["type"].as_str(), Some("embeddings") | Some("embedding"));
            m.quant = d["quantization"].as_str().unwrap_or("").to_string();
            m.family = d["arch"].as_str().unwrap_or("").to_string();
            m.max_context = d["max_context_length"].as_u64().or(m.max_context);
            m.loaded = d["state"].as_str() == Some("loaded");
            m.size = d["size_bytes"].as_u64().or_else(|| d["size"].as_u64()).or(m.size);
            if let Some(caps) = d["capabilities"].as_array() {
                m.tools = Some(caps.iter().any(|c| c.as_str() == Some("tool_use") || c.as_str() == Some("tools")));
            }
        }
    }
    Some(models)
}

/// A model's description (`/api/show`'s answer): its shape, what it can
/// do, its licence.
pub fn describe(m: &mut Model, show: &Json) {
    let info = &show["model_info"];
    let arch = info["general.architecture"].as_str().unwrap_or("").to_string();
    let get = |k: &str| info[format!("{arch}.{k}")].as_u64();
    m.layers = get("block_count");
    m.kv_heads = get("attention.head_count_kv").or_else(|| get("attention.head_count"));
    m.head_dim = get("attention.key_length").or_else(|| match (get("embedding_length"), get("attention.head_count")) {
        (Some(e), Some(h)) if h > 0 => Some(e / h),
        _ => None,
    });
    m.max_context = get("context_length").or(m.max_context);
    m.params = info["general.parameter_count"].as_u64().or(m.params);
    if m.family.is_empty() {
        m.family = arch;
    }
    if let Some(caps) = show["capabilities"].as_array() {
        let has = |c: &str| caps.iter().any(|x| x.as_str() == Some(c));
        m.tools = Some(has("tools"));
        m.chat = has("completion") || !has("embedding");
    }
    let text = show["license"].as_str().unwrap_or("").trim().to_string();
    let first = text.lines().map(str::trim).find(|l| !l.is_empty()).unwrap_or("").chars().take(80).collect::<String>();
    m.licence = info["general.license"].as_str().map(str::to_string).filter(|l| !l.is_empty()).unwrap_or(first);
    m.licence_text = text.chars().take(200_000).collect();
    if text.chars().count() > 200_000 {
        m.licence_text.push_str("\n\n[The licence text is longer: Studio shows its first 200,000 characters.]");
    }
    m.remote = m.remote || ["remote_host", "remote_model"].iter().any(|k| show[*k].as_str().is_some_and(|v| !v.trim().is_empty()));
}

// ---- the page's state ------------------------------------------------------------------

#[derive(Default)]
struct State {
    looking: bool,
    rows: Vec<Row>,
    order: Vec<usize>,
    best: Option<usize>,
    servers: Vec<String>,
    facts: Facts,
    /// (tests) The computer's facts in place of this one's.
    test_facts: Option<Facts>,
    /// (tests) The addresses asked in place of the usual ones.
    addresses: Option<Vec<String>>,
    /// The row being checked, its step ("load", "speed", "tools", "task").
    checking: Option<usize>,
    step: String,
    /// The rows waiting for their check.
    queue: Vec<usize>,
    checked_now: usize,
    /// The last check was stopped before its end.
    stopped: bool,
    cancel: Option<Arc<AtomicBool>>,
    news: bool,
    error: String,
    /// The look that is the latest (an older one's result is dropped).
    generation: u64,
}

static STATE: Mutex<Option<State>> = Mutex::new(None);

fn with<R>(f: impl FnOnce(&mut State) -> R) -> R {
    let mut s = STATE.lock().unwrap_or_else(|e| e.into_inner());
    f(s.get_or_insert_with(State::default))
}

impl State {
    fn memory(&self) -> Option<Memory> {
        Memory::of(&self.facts)
    }

    fn reorder(&mut self) {
        let mem = self.memory();
        for r in &mut self.rows {
            r.fit = fit(&r.model, mem);
        }
        self.order = rank(&self.rows, mem);
        self.best = best(&self.rows, &self.order);
        self.news = true;
    }

    /// The row at place `i` of the order.
    fn at(&self, i: i64) -> Option<(usize, &Row)> {
        let k = *self.order.get(usize::try_from(i).ok()?)?;
        Some((k, self.rows.get(k)?))
    }
}

fn facts_now() -> Facts {
    if let Some(f) = with(|s| s.test_facts.clone()) {
        return f;
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        Facts::read(false)
    }
    #[cfg(target_arch = "wasm32")]
    {
        Facts { browser: true, ..Facts::default() }
    }
}

/// (tests) `total=16384,free=8192,unified=0,cores=8` (megabytes): the
/// computer's facts in place of this one's; "" this one's again.
fn set_test_facts(text: &str) {
    let mut f = Facts { os: "test".into(), cores: 8, ..Facts::default() };
    for part in text.split([',', '&']) {
        let (k, v) = part.split_once('=').unwrap_or((part, ""));
        let n = v.trim().parse::<u64>().unwrap_or(0);
        match k.trim() {
            "total" => f.total_memory = Some(n * MB),
            "free" => f.free_memory = Some(n * MB),
            "gpu" => f.gpu_memory = Some(n * MB),
            "unified" => f.unified_memory = n != 0,
            "cores" => f.cores = n as u32,
            _ => {}
        }
    }
    let set = !text.trim().is_empty();
    with(|s| {
        s.test_facts = set.then_some(f);
    });
}

/// Looks for the models on this computer (a thread: the page polls).
/// `extra`: an address the user typed ("" none). Refused for an address
/// that isn't this computer's.
pub fn find(extra: &str) -> Result<(), String> {
    let extra = extra.trim();
    let extra = if extra.is_empty() { None } else { Some(local_url(extra)?) };
    #[cfg(target_arch = "wasm32")]
    {
        let _ = extra;
        Err(rapidr_value::i18n::tr_ui("A web page can't look for AI apps on this computer by itself: type the address of yours."))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let facts = facts_now();
        let (generation, mut addresses) = with(|s| {
            s.generation += 1;
            s.looking = true;
            s.error.clear();
            s.news = true;
            (s.generation, s.addresses.clone().unwrap_or_else(usual))
        });
        if let Some(e) = extra {
            if !addresses.contains(&e) {
                addresses.push(e);
            }
        }
        std::thread::Builder::new()
            .name("rapidr-local-ai-find".into())
            .spawn(move || {
                // (every address at once: a closed port answers at once)
                let found: Vec<(String, Option<Vec<Model>>)> = std::thread::scope(|sc| {
                    let handles: Vec<_> = addresses.iter().map(|a| sc.spawn(move || (a.clone(), server_models(a)))).collect();
                    handles.into_iter().filter_map(|h| h.join().ok()).collect()
                });
                let kept = cache_read();
                with(|s| {
                    if s.generation != generation {
                        return;
                    }
                    s.facts = facts;
                    s.servers = found.iter().filter(|(_, m)| m.is_some()).map(|(a, _)| a.clone()).collect();
                    // (a check of the same model kept from before)
                    let old: Vec<Row> = std::mem::take(&mut s.rows);
                    for (_, models) in found {
                        for m in models.unwrap_or_default() {
                            let before = old.iter().find(|r| r.model.base == m.base && r.model.id == m.id && r.model.digest == m.digest).and_then(|r| r.check.clone());
                            let key = cache_key(&m);
                            let kept_check = kept.get(&key).and_then(Check::of_json);
                            let kept_flag = before.is_none() && kept_check.is_some();
                            s.rows.push(Row { fit: fit(&m, None), check: before.or(kept_check), kept: kept_flag, error: String::new(), model: m });
                        }
                    }
                    s.looking = false;
                    s.reorder();
                });
            })
            .map(|_| ())
            .map_err(|e| format!("can't look: {e}"))
    }
}

/// (the web) The models a server listed to an RAI (its Models text: an id,
/// a tab, a name, a tab, its window, a line each): rows with what is known.
pub fn add_models(base: &str, models: &str) -> Result<usize, String> {
    let base = local_url(base)?;
    let facts = facts_now();
    let kept = cache_read();
    let mut n = 0;
    with(|s| {
        s.facts = facts;
        s.rows.retain(|r| r.model.base != base);
        if !s.servers.contains(&base) {
            s.servers.push(base.clone());
        }
        for line in models.lines() {
            let mut f = line.split('\t');
            let Some(id) = f.next().map(str::trim).filter(|i| !i.is_empty()) else { continue };
            let mut m = Model::new(id, &base);
            m.max_context = f.nth(1).and_then(|w| w.trim().parse::<u64>().ok()).filter(|w| *w > 0);
            let check = kept.get(&cache_key(&m)).and_then(Check::of_json);
            s.rows.push(Row { fit: fit(&m, None), kept: check.is_some(), check, error: String::new(), model: m });
            n += 1;
        }
        s.looking = false;
        s.reorder();
    });
    Ok(n)
}

/// Starts checking the row at place `i` of the order (-1: every one that
/// fits and hasn't been checked), one at a time on a thread of its own.
pub fn check(i: i64) -> Result<usize, String> {
    #[cfg(target_arch = "wasm32")]
    {
        let _ = i;
        Err(rapidr_value::i18n::tr_ui("Checking models runs in RapidR Studio on the desktop."))
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        let (queue, cancel) = with(|s| {
            let queue: Vec<usize> = if i < 0 {
                s.order.iter().copied().filter(|&k| s.rows[k].to_check()).collect()
            } else {
                s.at(i).filter(|(_, r)| r.usable()).map(|(k, _)| vec![k]).unwrap_or_default()
            };
            if queue.is_empty() || s.checking.is_some() {
                return (Vec::new(), None);
            }
            let cancel = Arc::new(AtomicBool::new(false));
            s.cancel = Some(cancel.clone());
            s.queue = queue.clone();
            s.checked_now = 0;
            s.stopped = false;
            s.error.clear();
            s.news = true;
            (queue, Some(cancel))
        });
        let Some(cancel) = cancel else {
            return if with(|s| s.checking.is_some()) { Err(rapidr_value::i18n::tr_ui("A check is running: wait for it, or stop it.")) } else { Ok(0) };
        };
        let n = queue.len();
        with(|s| s.checking = queue.first().copied());
        std::thread::Builder::new()
            .name("rapidr-local-ai-check".into())
            .spawn(move || run_checks(queue, cancel))
            .map_err(|e| format!("can't check: {e}"))?;
        Ok(n)
    }
}

/// Stops the check now (the model being checked is left as it was).
pub fn cancel() {
    with(|s| {
        if let Some(c) = &s.cancel {
            c.store(true, Ordering::SeqCst);
        }
    });
}

#[cfg(not(target_arch = "wasm32"))]
fn run_checks(queue: Vec<usize>, cancel: Arc<AtomicBool>) {
    crate::ai::install();
    for k in queue {
        if cancel.load(Ordering::SeqCst) {
            break;
        }
        let Some((m, f)) = with(|s| {
            s.checking = Some(k);
            s.step = "load".into();
            s.news = true;
            s.rows.get(k).map(|r| (r.model.clone(), r.fit))
        }) else {
            continue;
        };
        let step = |what: &str| with(|s| {
            s.step = what.to_string();
            s.news = true;
        });
        // (in memory already — another app, or a person, uses it: it stays
        // after the check; asked now, not when the list was made)
        let was_loaded = m.rich && loaded_now(&m);
        let result = check_one(&m, &f, &cancel, &step);
        // (a model this check loaded is let go, as it was before)
        if m.rich && !was_loaded {
            let _ = net::ask("POST", &format!("{}/api/generate", m.root()), Some(json!({"model": m.id, "keep_alive": 0})), 10);
        }
        with(|s| {
            if let Some(r) = s.rows.get_mut(k) {
                match result {
                    Ok(c) => {
                        cache_write(&cache_key(&r.model), &c);
                        r.check = Some(c);
                        r.kept = false;
                        r.error.clear();
                    }
                    // (stopped by the user: left unchecked, no error)
                    Err(e) if e == "cancelled" => {}
                    Err(e) => r.error = e,
                }
            }
            s.checked_now += 1;
            s.reorder();
        });
    }
    with(|s| {
        s.checking = None;
        s.step.clear();
        s.queue.clear();
        s.stopped = s.cancel.as_ref().is_some_and(|c| c.load(Ordering::SeqCst));
        s.cancel = None;
        s.news = true;
    });
}

/// Whether the model is in its app's memory now (its app says which are).
#[cfg(not(target_arch = "wasm32"))]
fn loaded_now(m: &Model) -> bool {
    net::json("GET", &format!("{}/api/ps", m.root()), None, 5)
        .and_then(|v| v["models"].as_array().cloned())
        .unwrap_or_default()
        .iter()
        .any(|x| x["name"].as_str() == Some(m.id.as_str()) || x["model"].as_str() == Some(m.id.as_str()))
}

/// A model's answer: seconds to its first word, its text, its tool calls,
/// seconds after the first word.
#[cfg(not(target_arch = "wasm32"))]
type Answer = (f64, String, Vec<(String, String, Json)>, f64);

/// A streamed chat request to the model; `stop` ends it.
#[cfg(not(target_arch = "wasm32"))]
fn chat(m: &Model, req: &rapidr_ai::ChatRequest, stop: &AtomicBool) -> Result<Answer, String> {
    let provider = rapidr_ai::provider("compatible").ok_or("no compatible provider")?;
    let mut ep = rapidr_ai::Endpoint::new(provider);
    ep.base_url = m.base.clone();
    let http = rapidr_ai::wire::chat_request(&ep, req);
    local_url(&m.root())?;
    let started = std::time::Instant::now();
    let mut first: Option<f64> = None;
    let mut decoder = rapidr_ai::Decoder::new(provider.format);
    let mut reply = rapidr_ai::Reply::default();
    let mut text = String::new();
    let mut body = Vec::new();
    let mut stopped = false;
    let (status, _) = rapidr_ai::http::run_blocking(&http, rapidr_ai::http::Options { connect_timeout: 5, read_timeout: 180, retries: 0 }, |d| {
        if stopped || stop.load(Ordering::SeqCst) {
            stopped = true;
            return;
        }
        body.extend_from_slice(d);
        for e in decoder.feed(d) {
            match &e {
                rapidr_ai::Event::Text(t) | rapidr_ai::Event::Thinking(t) => {
                    if first.is_none() && !t.trim().is_empty() {
                        first = Some(started.elapsed().as_secs_f64());
                    }
                    if matches!(e, rapidr_ai::Event::Text(_)) {
                        text.push_str(t);
                    }
                }
                rapidr_ai::Event::ToolStart { .. } if first.is_none() => first = Some(started.elapsed().as_secs_f64()),
                _ => {}
            }
            reply.apply(&e);
        }
    })?;
    if stopped || stop.load(Ordering::SeqCst) {
        return Err("cancelled".into());
    }
    if status != 200 {
        let said = rapidr_ai::wire::provider_message(&String::from_utf8_lossy(&body));
        return Err(format!("the server answered {status}{}", if said.is_empty() { String::new() } else { format!(": {said}") }));
    }
    for e in decoder.finish() {
        reply.apply(&e);
    }
    reply.finish();
    if let Some(e) = &reply.error {
        return Err(e.clone());
    }
    let total = started.elapsed().as_secs_f64();
    let first = first.unwrap_or(total);
    Ok((first, text, reply.tool_calls(), (total - first).max(0.0)))
}

/// One model's check: loaded, its speed with RAI's real first request, a
/// tool call, the tiny task.
#[cfg(not(target_arch = "wasm32"))]
fn check_one(m: &Model, f: &Fit, stop: &AtomicBool, step: &dyn Fn(&str)) -> Result<Check, String> {
    use rapidr_ai::{ChatRequest, Message};
    let mut c = Check::default();
    let lite = f.context < LITE_BELOW;
    let req = |system: String, tools: Vec<rapidr_ai::Tool>, ask: &str, max: u32| ChatRequest { model: m.id.clone(), system, system_tail: String::new(), messages: vec![Message::user(ask)], tools, max_tokens: max, reasoning_effort: None, no_stream_usage: false, thinking: None };
    // (1. loaded: a one-word answer)
    let t0 = std::time::Instant::now();
    chat(m, &req(String::new(), Vec::new(), "Say OK.", 8), stop)?;
    c.load_s = t0.elapsed().as_secs_f64();
    // (the context the server runs it with, when it says)
    if m.rich {
        if let Some(ps) = net::json("GET", &format!("{}/api/ps", m.root()), None, 5) {
            c.server_context = ps["models"].as_array().into_iter().flatten().find(|x| x["name"].as_str() == Some(m.id.as_str()) || x["model"].as_str() == Some(m.id.as_str())).and_then(|x| x["context_length"].as_u64()).unwrap_or(0);
        }
    }
    // (2. speed: RAI's real first request — its instructions and tools —
    // and a short question)
    step("speed");
    let (system, tools) = if lite { (crate::ai::instructions_lite(true), crate::ai_tools::tools_lite(true)) } else { (crate::ai::instructions(true), crate::ai_tools::tools(true)) };
    let (first, text, calls, after) = chat(m, &req(system.clone(), tools.clone(), "Hello! In one short sentence: what can you help me with?", 120), stop)?;
    c.first_s = first;
    let words = text.split_whitespace().count() + calls.len() * 5;
    c.words_s = if after > 0.05 { words as f64 / after } else { 0.0 };
    // (3. a tool call: the file read as a call, its arguments JSON)
    step("tools");
    let (_, text, calls, _) = chat(m, &req(crate::ai::instructions_lite(true), crate::ai_tools::tools_lite(true), "Read the file main.rr of my project and tell me its first line.", 400), stop)?;
    match calls.iter().find(|(_, n, _)| n == "read_file") {
        Some((_, _, input)) if input.get("_invalid_json").is_none() && input.get("path").and_then(Json::as_str).is_some_and(|p| !p.trim().is_empty()) => c.tools = true,
        Some(_) => c.tools_why = "bad-arguments".into(),
        None if calls.is_empty() && (text.contains("read_file") || text.contains("\"path\"")) => c.tools_why = "as-text".into(),
        None if calls.is_empty() => c.tools_why = "no-call".into(),
        None => c.tools_why = "wrong-tool".into(),
    }
    // (4. the tiny task, through RAI's real loop and Studio's tools)
    step("task");
    let small = !c.tools || f.context < LITE_BELOW;
    let (task, why) = run_task(m, f.context.min(if c.server_context > 0 { c.server_context } else { u64::MAX }), small, stop)?;
    c.task = task;
    c.task_why = why;
    Ok(c)
}

#[cfg(not(target_arch = "wasm32"))]
#[derive(Clone, Copy)]
struct Quiet;

#[cfg(not(target_arch = "wasm32"))]
impl rapidr_ai::rai::Fire for Quiet {
    fn fire(self, _name: &str, _event: &str, _args: &[Value]) {}
}

/// The tiny task: RAI (the model, the context and the mode Studio would
/// give it) with Studio's project tools asked [`TASK`] for a new program;
/// passed when the program it proposes compiles and has a button that
/// says Hello. Nothing it wrote is run or saved.
#[cfg(not(target_arch = "wasm32"))]
fn run_task(m: &Model, context: u64, small: bool, stop: &AtomicBool) -> Result<(bool, String), String> {
    static N: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let n = N.fetch_add(1, Ordering::Relaxed);
    let (rai, tools) = (format!("rapidr-local-check-{n}"), format!("rapidr-local-tools-{n}"));
    let sv = |v: &str| Value::String(v.to_string().into());
    for (p, v) in [("permission", "propose"), ("projectfolder", ""), ("projectfiles", ""), ("mainfile", ""), ("projectname", "")] {
        crate::ai::set(&tools, p, &sv(v));
    }
    for (p, v) in [("provider", "compatible"), ("baseurl", m.base.as_str()), ("model", m.id.as_str())] {
        rapidr_ai::rai::set(&rai, p, &sv(v));
    }
    rapidr_ai::rai::set(&rai, "maxtokens", &Value::Integer(rapidr_ai::rai::LOCAL_REPLY as i64));
    rapidr_ai::rai::set(&rai, "smallmodel", &Value::Integer(if small { 1 } else { 0 }));
    rapidr_ai::rai::set(&rai, "contextsize", &Value::Integer(context as i64));
    if rapidr_ai::rai::call(&rai, "attachcomponent", &[sv(&tools), sv("project")], |v| v.to_string_val()).map(|v| v.to_i64()).unwrap_or(0) == 0 {
        return Err("RAI's tools couldn't be attached".into());
    }
    if rapidr_ai::rai::call(&rai, "ask", &[sv(TASK)], |v| v.to_string_val()).map(|v| v.to_i64()).unwrap_or(0) == 0 {
        return Err(rapidr_ai::rai::get(&rai, "error").map(|v| v.to_string_val()).unwrap_or_default());
    }
    let started = std::time::Instant::now();
    let busy = || rapidr_ai::rai::get(&rai, "busy").map(|v| v.to_i64()).unwrap_or(0) != 0;
    loop {
        rapidr_ai::rai::poll(Quiet);
        if !busy() {
            rapidr_ai::rai::poll(Quiet);
            break;
        }
        if stop.load(Ordering::SeqCst) {
            rapidr_ai::rai::call(&rai, "cancel", &[], |v| v.to_string_val());
            return Err("cancelled".into());
        }
        if started.elapsed().as_secs() > TASK_SECONDS {
            rapidr_ai::rai::call(&rai, "cancel", &[], |v| v.to_string_val());
            return Ok((false, "too-slow".into()));
        }
        std::thread::sleep(std::time::Duration::from_millis(40));
    }
    let get = |p: &str| crate::ai::get(&tools, p).map(|v| v.to_i64()).unwrap_or(0);
    if get("fellback") != 0 {
        return Ok((false, "recipe".into()));
    }
    let count = get("proposalcount");
    let mut main_text = None;
    for i in 0..count {
        let info = crate::ai::call(&tools, "proposalinfo", &[Value::Integer(i)]).map(|v| v.to_string_val()).unwrap_or_default();
        let f: Vec<&str> = info.split('\t').collect();
        if f.first() == Some(&"project") && f.get(4) == Some(&"1") {
            main_text = crate::ai::call(&tools, "proposalnew", &[Value::Integer(i)]).map(|v| v.to_string_val());
        }
    }
    let Some(text) = main_text else { return Ok((false, "no-program".into())) };
    if get("projecterrors") > 0 {
        return Ok((false, "errors".into()));
    }
    Ok(if has_hello_button(&text) { (true, String::new()) } else { (false, "no-button".into()) })
}

/// Whether a program's window has a button that says Hello: a button
/// made with CREATE and a Caption with "hello" in it, or a button whose
/// Caption is set to one.
pub fn has_hello_button(code: &str) -> bool {
    let mut stack: Vec<bool> = Vec::new();
    let mut buttons: Vec<String> = Vec::new();
    let is_button = |t: &str| t.trim().to_ascii_uppercase().ends_with("BUTTON");
    let hello = |l: &str| l.to_ascii_lowercase().contains("hello");
    for raw in code.lines() {
        let l = raw.trim();
        let low = l.to_ascii_lowercase();
        if low.starts_with("create ") {
            let ty = low.split(" as ").nth(1).unwrap_or("");
            let name = l[7..].split_whitespace().next().unwrap_or("").to_ascii_lowercase();
            let b = is_button(ty);
            if b {
                buttons.push(name);
            }
            stack.push(b);
        } else if low.starts_with("end create") {
            stack.pop();
        } else if low.starts_with("dim ") && is_button(low.split(" as ").nth(1).unwrap_or("")) {
            buttons.push(l[4..].split_whitespace().next().unwrap_or("").to_ascii_lowercase());
        } else if stack.last() == Some(&true) && low.starts_with("caption") && low.contains('=') && hello(l) {
            return true;
        } else if let Some((lhs, rhs)) = l.split_once('=') {
            let lhs = lhs.trim().to_ascii_lowercase();
            if let Some(obj) = lhs.strip_suffix(".caption") {
                if buttons.iter().any(|b| b == obj.trim()) && hello(rhs) {
                    return true;
                }
            }
        }
    }
    false
}

// ---- RPROJECT's members ------------------------------------------------------------------

fn s(v: impl Into<String>) -> Value {
    Value::String(v.into().into())
}

fn secs(x: f64) -> String {
    format!("{x:.1}")
}

/// A field of the row at place `i` (the best first).
fn field(i: i64, what: &str) -> String {
    with(|st| {
        let mem = st.memory();
        let Some((k, r)) = st.at(i) else { return String::new() };
        let m = &r.model;
        let c = r.check.as_ref();
        let opt = |v: Option<u64>| v.map_or(String::new(), |n| n.to_string());
        let flag = |b: bool| if b { "1".to_string() } else { "0".to_string() };
        match what {
            "id" => m.id.clone(),
            "base" => m.base.clone(),
            "server" => m.base.split_once("://").map_or(m.base.as_str(), |(_, r)| r).split('/').next().unwrap_or("").to_string(),
            "size" => opt(m.size),
            "sizewords" => m.weights().map(gb_words).unwrap_or_default(),
            "params" => m.params.map(|p| if p >= 1_000_000_000 { format!("{:.1}B", p as f64 / 1e9).replace(".0B", "B") } else { format!("{}M", p / 1_000_000) }).unwrap_or_default(),
            "quant" => m.quant.clone(),
            "family" => m.family.clone(),
            "maxcontext" => opt(m.max_context),
            "context" => context_for(&r.fit, c).to_string(),
            "need" => opt(r.fit.need),
            "needwords" => r.fit.need.map(gb_words).unwrap_or_default(),
            "spare" => match (mem, r.fit.need) {
                (Some(mm), Some(n)) => gb_words(mm.now.saturating_sub(n)),
                _ => String::new(),
            },
            "fit" => r.fit.verdict.word().to_string(),
            "tools" => m.tools.map(flag).unwrap_or_default(),
            "chat" => flag(m.chat),
            "licence" => m.licence.clone(),
            "licencetext" => m.licence_text.clone(),
            "loaded" => flag(m.loaded),
            "remote" => flag(m.remote),
            "digest" => m.digest.clone(),
            "checked" => flag(c.is_some()),
            "kept" => flag(r.kept),
            "first" => c.map(|c| secs(c.first_s)).unwrap_or_default(),
            "words" => c.map(|c| format!("{:.0}", c.words_s)).unwrap_or_default(),
            "load" => c.map(|c| secs(c.load_s)).unwrap_or_default(),
            "toolsok" => c.map(|c| flag(c.tools)).unwrap_or_default(),
            "toolswhy" => c.map(|c| c.tools_why.clone()).unwrap_or_default(),
            "task" => c.map(|c| flag(c.task)).unwrap_or_default(),
            "taskwhy" => c.map(|c| c.task_why.clone()).unwrap_or_default(),
            "servercontext" => c.map(|c| c.server_context.to_string()).unwrap_or_default(),
            "fast" => c.map(|c| flag(c.first_s <= FAST_ENOUGH_S)).unwrap_or_default(),
            "mode" => (if small_mode(&r.fit, c) { "small" } else { "full" }).to_string(),
            "best" => flag(st.best == Some(k)),
            "checking" => flag(st.checking == Some(k)),
            "queued" => flag(st.queue.contains(&k) && st.checking != Some(k) && c.is_none()),
            "step" => if st.checking == Some(k) { st.step.clone() } else { String::new() },
            "estimate" => estimate_s(m).to_string(),
            "error" => r.error.clone(),
            _ => String::new(),
        }
    })
}

/// The page's state, `what`.
fn state(what: &str) -> String {
    with(|st| {
        let mem = st.memory();
        match what {
            "busy" => (if st.looking || st.checking.is_some() { "1" } else { "0" }).into(),
            "looking" => (if st.looking { "1" } else { "0" }).into(),
            "checking" => st.checking.and_then(|k| st.order.iter().position(|&o| o == k)).map_or("-1".into(), |p| p.to_string()),
            "step" => st.step.clone(),
            "count" => st.rows.len().to_string(),
            "chatcount" => st.rows.iter().filter(|r| r.model.chat).count().to_string(),
            "servers" => st.servers.len().to_string(),
            "serverlist" => st.servers.iter().map(|b| b.split_once("://").map_or(b.as_str(), |(_, r)| r).split('/').next().unwrap_or("").to_string()).collect::<Vec<_>>().join(", "),
            "best" => st.best.and_then(|k| st.order.iter().position(|&o| o == k)).map_or("-1".into(), |p| p.to_string()),
            "news" => {
                let n = std::mem::take(&mut st.news);
                (if n { "1" } else { "0" }).into()
            }
            "error" => st.error.clone(),
            "total" => st.facts.total_memory.map_or(String::new(), |t| (t / MB).to_string()),
            "free" => st.facts.free_memory.map_or(String::new(), |t| (t / MB).to_string()),
            "totalwords" => st.facts.total_memory.map(gb_words).unwrap_or_default(),
            "freewords" => mem.map(|m| gb_words(m.now)).unwrap_or_default(),
            "mostwords" => mem.map(|m| gb_words(m.most)).unwrap_or_default(),
            "unified" => (if st.facts.unified_memory { "1" } else { "0" }).into(),
            "queue" => st.queue.len().to_string(),
            "done" => st.checked_now.to_string(),
            "stopping" => (if st.checking.is_some() && st.cancel.as_ref().is_some_and(|c| c.load(Ordering::SeqCst)) { "1" } else { "0" }).into(),
            "stopped" => (if st.stopped { "1" } else { "0" }).into(),
            "estimate" => st.order.iter().filter(|&&k| st.rows[k].to_check()).map(|&k| estimate_s(&st.rows[k].model)).sum::<u64>().to_string(),
            "unchecked" => st.order.iter().filter(|&&k| st.rows[k].to_check()).count().to_string(),
            "web" => (if cfg!(target_arch = "wasm32") { "1" } else { "0" }).into(),
            _ => String::new(),
        }
    })
}

/// RPROJECT's `LocalAI…` members: the value and the error ("" none);
/// `None`: not one of them.
pub fn call(method: &str, args: &[Value]) -> Option<(Value, String)> {
    let a = |i: usize| args.get(i).map(Value::to_string_val).unwrap_or_default();
    let n = |i: usize| args.get(i).map(Value::to_i64).unwrap_or(-1);
    let flag = |r: &Result<(), String>| Value::Integer(if r.is_ok() { -1 } else { 0 });
    Some(match method {
        // LocalAIFind(Address): looks for the models (Address: one the
        // user typed, "" none) — True when it started
        "localaifind" => {
            let r = find(&a(0));
            (flag(&r), r.err().unwrap_or_default())
        }
        // LocalAI(What): the page's state
        "localai" => (s(state(&a(0).to_ascii_lowercase())), String::new()),
        // LocalAIModel(Index, Field): a model's field, the best first
        "localaimodel" => (s(field(n(0), &a(1).to_ascii_lowercase())), String::new()),
        // LocalAICheck(Index): checks a model (-1 every one that fits): how many
        "localaicheck" => match check(n(0)) {
            Ok(k) => (Value::Integer(k as i64), String::new()),
            Err(e) => (Value::Integer(0), e),
        },
        "localaicancel" => {
            cancel();
            (Value::Integer(-1), String::new())
        }
        // LocalAIAddModels(Address, Models): (the web) an RAI's list
        "localaiaddmodels" => match add_models(&a(0), &a(1)) {
            Ok(k) => (Value::Integer(k as i64), String::new()),
            Err(e) => (Value::Integer(0), e),
        },
        // LocalAIAddress(Address): the address as it is asked (this
        // computer's only: "" and Error for another computer's) — a web
        // page checks the address typed before anything is sent
        "localaiaddress" => match local_url(&a(0)) {
            Ok(u) => (s(u), String::new()),
            Err(e) => (s(""), e),
        },
        // LocalAIClear: the page forgets what it found (not the checks kept)
        "localaiclear" => {
            with(|s| {
                s.generation += 1;
                s.rows.clear();
                s.order.clear();
                s.best = None;
                s.servers.clear();
                s.looking = false;
                s.news = true;
            });
            (Value::Integer(-1), String::new())
        }
        // (tests) LocalAITest(What, Value): "addresses" (url|url, "" the
        // usual ones), "facts" (total=…,free=… in MB, "" this computer's)
        "localaitest" => {
            match a(0).to_ascii_lowercase().as_str() {
                "addresses" => {
                    let list: Vec<String> = a(1).split('|').map(str::trim).filter(|x| !x.is_empty()).filter_map(|x| local_url(x).ok()).collect();
                    with(|s| s.addresses = (!list.is_empty()).then_some(list));
                }
                "facts" => set_test_facts(&a(1)),
                _ => return Some((Value::Integer(0), "LocalAITest: addresses or facts".into())),
            }
            (Value::Integer(-1), String::new())
        }
        _ => return None,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A model of known shape: `gb` of weights, 32 layers, 8 key/value
    /// heads of 128 (an 8-billion-parameter model's).
    fn model(id: &str, gb: f64) -> Model {
        Model { id: id.into(), base: "http://127.0.0.1:1/v1".into(), size: Some((gb * GB as f64) as u64), layers: Some(32), kv_heads: Some(8), head_dim: Some(128), max_context: Some(131072), tools: Some(true), chat: true, ..Model::default() }
    }

    fn mem(total_gb: u64, free_gb: u64) -> Option<Memory> {
        Memory::of(&Facts { total_memory: Some(total_gb * GB), free_memory: Some(free_gb * GB), ..Facts::default() })
    }

    #[test]
    fn the_cache_is_the_shape_times_the_context() {
        // (2 × 32 layers × 8 heads × 128 × 2 bytes = 128 KB a token)
        assert_eq!(kv_bytes(&model("m", 4.0), 8192), 1024 * MB);
        assert_eq!(kv_bytes(&model("m", 4.0), 16384), 2048 * MB);
        // (a shape not known: 128 KB a token too)
        let mut m = model("m", 4.0);
        m.layers = None;
        assert_eq!(kv_bytes(&m, 4096), 512 * MB);
        // (weights 4 GB + cache 1 GB + 0.5 GB + 0.4 GB)
        assert_eq!(need(&model("m", 4.0), 8192), Some(4 * GB + GB + 512 * MB + (4 * GB) / 10));
        // (no size: parameters at the quantization's bits)
        let p = Model { params: Some(8_000_000_000), quant: "Q4_K_M".into(), ..Model::default() };
        assert_eq!(p.weights(), Some(5_000_000_000));
        assert_eq!(parse_params("3.4B"), Some(3_400_000_000));
        assert_eq!(parse_params("135M"), Some(135_000_000));
    }

    #[test]
    fn the_fit_check_says_fits_close_apps_or_too_big_and_picks_the_context() {
        // (16 GB, 12 free: 4 GB of weights take 16K comfortably)
        let f = fit(&model("small", 4.0), mem(16, 12));
        assert_eq!((f.verdict, f.context), (Verdict::Fits, CONTEXT_ROOMY));
        // (8 GB free: 8K fits, 16K would take most of it)
        let f = fit(&model("small", 4.0), mem(16, 8));
        assert_eq!((f.verdict, f.context), (Verdict::Fits, CONTEXT_DEFAULT));
        // (6.5 GB free: only 4K fits now)
        let f = fit(&model("small", 4.0), mem(16, 7));
        assert_eq!((f.verdict, f.context), (Verdict::Fits, CONTEXT_LEAST));
        // (3 GB free of 16: fits if other apps close)
        let f = fit(&model("small", 4.0), mem(16, 3));
        assert_eq!((f.verdict, f.context), (Verdict::CloseApps, CONTEXT_DEFAULT));
        // (a 40 GB model on 16 GB: too big)
        assert_eq!(fit(&model("huge", 40.0), mem(16, 12)).verdict, Verdict::TooBig);
        // (a computer whose memory isn't known, a model whose size isn't)
        assert_eq!(fit(&model("m", 4.0), None).verdict, Verdict::Unknown);
        let mut m = model("m", 4.0);
        m.size = None;
        assert_eq!(fit(&m, mem(16, 12)).verdict, Verdict::Unknown);
        // (a model made for 2,048 tokens gets them, not more)
        let mut m = model("old", 1.0);
        m.max_context = Some(2048);
        assert_eq!(fit(&m, mem(16, 12)).context, 2048);
        // (a model already in memory counts its own memory as free)
        let mut m = model("loaded", 4.0);
        assert_eq!(fit(&m, mem(16, 5)).verdict, Verdict::CloseApps);
        m.loaded = true;
        assert_eq!(fit(&m, mem(16, 5)).verdict, Verdict::Fits);
        // (shared memory: a GPU's part is the most a model gets)
        let shared = Memory::of(&Facts { total_memory: Some(48 * GB), free_memory: Some(40 * GB), unified_memory: true, gpu_memory: Some(36 * GB), ..Facts::default() });
        assert_eq!(fit(&model("big", 30.0), shared).verdict, Verdict::Fits);
        let busy = Memory::of(&Facts { total_memory: Some(48 * GB), free_memory: Some(20 * GB), unified_memory: true, gpu_memory: Some(36 * GB), ..Facts::default() });
        assert_eq!(fit(&model("big", 30.0), busy).verdict, Verdict::CloseApps);
        assert_eq!(fit(&model("bigger", 36.0), shared).verdict, Verdict::TooBig);
    }

    fn row(m: Model, check: Option<Check>, mm: Option<Memory>) -> Row {
        Row { fit: fit(&m, mm), model: m, check, kept: false, error: String::new() }
    }

    fn passed(first: f64) -> Check {
        Check { first_s: first, words_s: 20.0, tools: true, task: true, ..Check::default() }
    }

    #[test]
    fn ranking_puts_the_task_then_tools_then_speed_then_memory_first() {
        let mm = mem(16, 12);
        let mut chatty = model("chatty", 2.0);
        chatty.tools = Some(false);
        let mut embed = model("embedder", 0.3);
        embed.chat = false;
        let rows = vec![
            row(model("huge", 40.0), None, mm),
            row(model("unchecked-small", 2.0), None, mm),
            row(model("passes-slowly", 3.0), Some(passed(40.0)), mm),
            row(model("passes-fast", 5.0), Some(passed(3.0)), mm),
            row(chatty, None, mm),
            row(model("failed-task", 1.0), Some(Check { first_s: 2.0, tools: true, task: false, ..Check::default() }), mm),
            row(embed, None, mm),
            row(model("unchecked-big", 6.0), None, mm),
        ];
        let order = rank(&rows, mm);
        let names: Vec<&str> = order.iter().map(|&k| rows[k].model.id.as_str()).collect();
        assert_eq!(names, ["passes-fast", "passes-slowly", "failed-task", "unchecked-small", "unchecked-big", "chatty", "embedder", "huge"]);
        // (the best: built the app; not checked yet, the one with the most to spare that takes tools)
        assert_eq!(best(&rows, &order).map(|k| rows[k].model.id.as_str()), Some("passes-fast"));
        let unchecked: Vec<Row> = rows.iter().filter(|r| r.check.is_none()).cloned().collect();
        let order = rank(&unchecked, mm);
        assert_eq!(best(&unchecked, &order).map(|k| unchecked[k].model.id.as_str()), Some("unchecked-small"));
        // (none usable: no best)
        let none = vec![row(model("huge", 40.0), None, mm)];
        assert_eq!(best(&none, &rank(&none, mm)), None);
    }

    #[test]
    fn the_small_model_mode_comes_from_the_check_not_the_name() {
        let roomy = Fit { verdict: Verdict::Fits, context: CONTEXT_ROOMY, need: None };
        let tight = Fit { verdict: Verdict::Fits, context: CONTEXT_DEFAULT, need: None };
        // (unchecked: by the context, as RAI decides)
        assert!(!small_mode(&roomy, None));
        assert!(small_mode(&tight, None));
        // (checked: full only when it built the app, fast, with tools, at 16K)
        assert!(!small_mode(&roomy, Some(&passed(3.0))));
        assert!(small_mode(&roomy, Some(&passed(30.0))));
        assert!(small_mode(&roomy, Some(&Check { task: false, ..passed(3.0) })));
        assert!(small_mode(&roomy, Some(&Check { tools: false, ..passed(3.0) })));
        assert!(small_mode(&tight, Some(&passed(3.0))));
        // (the server runs it with 4K: RAI keeps within that, and goes small)
        let c = Check { server_context: 4096, ..passed(3.0) };
        assert_eq!(context_for(&roomy, Some(&c)), 4096);
        assert!(small_mode(&roomy, Some(&c)));
    }

    #[test]
    fn only_this_computer_is_asked() {
        assert_eq!(local_url("localhost:11434").unwrap(), "http://127.0.0.1:11434/v1");
        assert_eq!(local_url("http://127.0.0.1:1234/v1/").unwrap(), "http://127.0.0.1:1234/v1");
        assert_eq!(local_url("http://[::1]:8080").unwrap(), "http://[::1]:8080/v1");
        for bad in ["http://192.168.1.20:11434", "example.com:11434", "http://localhost.evil.com:80/v1", "http://user@127.0.0.1:1/v1", "http://10.0.0.1", "ftp://127.0.0.1"] {
            assert!(local_url(bad).is_err(), "{bad} should be refused");
        }
        assert!(find("http://192.168.1.20:11434").is_err());
        #[cfg(not(target_arch = "wasm32"))]
        {
            let refused = net::ask("GET", "http://93.184.216.34/v1/models", None, 1);
            assert!(refused.is_err());
        }
    }

    #[test]
    fn checks_are_kept_by_digest_version_and_prompts() {
        let mut m = model("m", 2.0);
        m.digest = "sha256:abc".into();
        let k = cache_key(&m);
        assert!(k.starts_with("sha256:abc|") && k.contains(env!("CARGO_PKG_VERSION")));
        // (the same model at another server: the same digest, the same key)
        let mut other = m.clone();
        other.base = "http://127.0.0.1:2/v1".into();
        assert_eq!(cache_key(&other), k);
        // (another digest: another key; no digest: the server, name and size)
        other.digest = "sha256:def".into();
        assert_ne!(cache_key(&other), k);
        m.digest.clear();
        assert!(cache_key(&m).starts_with("http://127.0.0.1:1/v1#m#"));
        // (kept and read again: a folder of the test's own)
        let _one = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("rapidr-local-ai-{}", std::process::id()));
        *TEST_CACHE_DIR.lock().unwrap() = Some(dir.to_string_lossy().to_string());
        let c = passed(4.0);
        cache_write(&k, &c);
        assert_eq!(cache_read().get(&k).and_then(Check::of_json), Some(c));
        let _ = std::fs::remove_dir_all(&dir);
        *TEST_CACHE_DIR.lock().unwrap() = None;
    }

    #[test]
    fn the_task_wants_a_button_that_says_hello() {
        assert!(has_hello_button("CREATE Form AS RForm\n  CREATE B AS RButton\n    Caption = \"Hello\"\n  END CREATE\nEND CREATE"));
        assert!(has_hello_button("DIM Btn AS QButton\nBtn.Caption = \"Say Hello\""));
        assert!(!has_hello_button("CREATE Form AS RForm\n  Caption = \"Hello\"\n  CREATE B AS RButton\n    Caption = \"OK\"\n  END CREATE\nEND CREATE"));
        assert!(!has_hello_button("CREATE L AS RLabel\n  Caption = \"Hello\"\nEND CREATE"));
    }
}

/// (tests) A mock local AI server: an OpenAI-compatible list and chat, and
/// the richer description endpoints, on 127.0.0.1.
#[cfg(all(test, not(target_arch = "wasm32")))]
mod mock_tests {
    use super::*;
    use std::io::{BufRead, BufReader, Read, Write};
    use std::net::TcpListener;

    const HELLO: &str = "CREATE Form AS RForm\n    Caption = \"Hi\"\n    CREATE HelloButton AS RButton\n        Caption = \"Hello\"\n        Left = 20: Top = 20\n    END CREATE\nEND CREATE\nForm.ShowModal\n";

    fn sse(chunks: &[Json]) -> String {
        let mut out = String::new();
        for c in chunks {
            out.push_str(&format!("data: {c}\n\n"));
        }
        out.push_str("data: [DONE]\n\n");
        out
    }

    /// Answers like a local AI server; `tools`: whether its model calls tools.
    fn answer(path: &str, body: &str, tools: bool) -> (u16, &'static str, String) {
        let req: Json = serde_json::from_str(body).unwrap_or(Json::Null);
        match path {
            "/v1/models" => (200, "application/json", json!({"object": "list", "data": [{"id": "tiny:1b", "object": "model"}, {"id": "huge:70b", "object": "model"}, {"id": "embed:small", "object": "model"}]}).to_string()),
            "/api/tags" => (200, "application/json", json!({"models": [
                {"name": "tiny:1b", "size": 1_000_000_000u64, "digest": "sha256:tiny", "details": {"parameter_size": "1.2B", "quantization_level": "Q4_K_M", "family": "test"}},
                {"name": "huge:70b", "size": 42_000_000_000u64, "digest": "sha256:huge", "details": {"parameter_size": "70B", "quantization_level": "Q4_K_M"}},
                {"name": "embed:small", "size": 300_000_000u64, "digest": "sha256:embed", "details": {}}]}).to_string()),
            "/api/ps" => (200, "application/json", json!({"models": []}).to_string()),
            "/api/show" => {
                let m = req["model"].as_str().unwrap_or("");
                let caps = if m.starts_with("embed") { json!(["embedding"]) } else if tools { json!(["completion", "tools"]) } else { json!(["completion"]) };
                (200, "application/json", json!({"capabilities": caps, "license": "Test Licence 1.0\n\nYou may use it.", "model_info": {"general.architecture": "t", "t.block_count": 16, "t.attention.head_count_kv": 4, "t.attention.head_count": 16, "t.embedding_length": 1024, "t.context_length": 32768}}).to_string())
            }
            "/api/generate" => (200, "application/json", "{}".into()),
            "/v1/chat/completions" => {
                let msgs = req["messages"].as_array().cloned().unwrap_or_default();
                // (a message's text: a string, or its parts' texts)
                let text_of = |c: &Json| c.as_str().map(str::to_string).unwrap_or_else(|| c.as_array().into_iter().flatten().filter_map(|p| p["text"].as_str()).collect::<Vec<_>>().join(""));
                let last = msgs.iter().rev().find(|m| m["role"] == "user").map(|m| text_of(&m["content"])).unwrap_or_default();
                let results = msgs.iter().filter(|m| m["role"] == "tool").count();
                let text = |t: &str| sse(&[json!({"choices": [{"delta": {"content": t}}]}), json!({"choices": [{"delta": {}, "finish_reason": "stop"}]})]);
                let call = |name: &str, args: Json| sse(&[json!({"choices": [{"delta": {"tool_calls": [{"index": 0, "id": "c1", "type": "function", "function": {"name": name, "arguments": args.to_string()}}]}}]}), json!({"choices": [{"delta": {}, "finish_reason": "tool_calls"}]})]);
                let body = if !tools {
                    text("I would call read_file with {\"path\": \"main.rr\"}.")
                } else if last.starts_with("Read the file") {
                    call("read_file", json!({"path": "main.rr"}))
                } else if last.starts_with("Build me") && results == 0 {
                    call("create_project", json!({"name": "Hello", "main": "main.rr", "files": [{"path": "main.rr", "text": HELLO}]}))
                } else {
                    text("Hello! I can help you build RapidR programs.")
                };
                (200, "text/event-stream", body)
            }
            _ => (404, "application/json", "{}".into()),
        }
    }

    /// A server on a port of its own (a thread a connection).
    fn serve(tools: bool) -> String {
        serve_slow(tools, 0)
    }

    /// A server whose chat answers wait `slow_ms` first.
    pub(super) fn serve_slow(tools: bool, slow_ms: u64) -> String {
        let l = TcpListener::bind("127.0.0.1:0").unwrap();
        let port = l.local_addr().unwrap().port();
        std::thread::spawn(move || {
            for c in l.incoming().flatten() {
                std::thread::spawn(move || {
                    let mut r = BufReader::new(c.try_clone().unwrap());
                    let mut line = String::new();
                    if r.read_line(&mut line).is_err() {
                        return;
                    }
                    let path = line.split_whitespace().nth(1).unwrap_or("/").to_string();
                    let mut len = 0usize;
                    loop {
                        let mut h = String::new();
                        if r.read_line(&mut h).is_err() || h.trim().is_empty() {
                            break;
                        }
                        if let Some(v) = h.to_ascii_lowercase().strip_prefix("content-length:") {
                            len = v.trim().parse().unwrap_or(0);
                        }
                    }
                    let mut body = vec![0u8; len];
                    let _ = r.read_exact(&mut body);
                    let (status, ty, out) = answer(&path, &String::from_utf8_lossy(&body), tools);
                    if path.ends_with("/chat/completions") {
                        std::thread::sleep(std::time::Duration::from_millis(slow_ms));
                    }
                    let mut c = c;
                    let _ = write!(c, "HTTP/1.1 {status} OK\r\ncontent-type: {ty}\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{out}", out.len());
                });
            }
        });
        format!("http://127.0.0.1:{port}/v1")
    }

    fn wait_idle() {
        for _ in 0..600 {
            if state("busy") == "0" {
                return;
            }
            std::thread::sleep(std::time::Duration::from_millis(50));
        }
        panic!("still busy");
    }

    /// One test walks it all: the state is Studio's one page.
    #[test]
    fn found_fitted_checked_and_ranked_against_a_mock_server() {
        let _one = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
        let dir = std::env::temp_dir().join(format!("rapidr-local-ai-mock-{}", std::process::id()));
        *TEST_CACHE_DIR.lock().unwrap() = Some(dir.to_string_lossy().to_string());
        // (never the user's keychain)
        std::env::set_var("RAPIDR_AI_KEYSTORE", "memory");
        let good = serve(true);
        let chatty = serve(false);
        // (the ports asked: the two mocks and a closed one — never the
        // user's own servers)
        call("localaitest", &[s("addresses"), s(format!("{good}|{chatty}|http://127.0.0.1:9/v1"))]);
        call("localaitest", &[s("facts"), s("total=16384,free=12288,cores=8")]);
        let started = std::time::Instant::now();
        assert_eq!(call("localaifind", &[s("")]).unwrap().0.to_i64(), -1);
        wait_idle();
        assert!(started.elapsed().as_secs() < 5, "finding took {:?}", started.elapsed());
        assert_eq!(state("servers"), "2");
        assert_eq!(state("count"), "6");
        // (the order: the two that fit and take tools first; the embedding
        // model and the 42 GB one last)
        let rows: Vec<(String, String, String)> = (0..6).map(|i| (field(i, "id"), field(i, "fit"), field(i, "server"))).collect();
        assert_eq!(rows[0].0, "tiny:1b");
        assert_eq!(rows[0].1, "fits");
        assert_eq!(field(0, "context"), "16384");
        assert_eq!(field(0, "licence"), "Test Licence 1.0");
        assert_eq!(field(0, "params"), "1.2B");
        assert!(rows[4..].iter().all(|r| r.0 == "huge:70b" || r.0 == "embed:small"), "{rows:?}");
        assert!(rows.iter().filter(|r| r.0 == "huge:70b").all(|r| r.1 == "toobig"));
        let all: Vec<String> = (0..6).map(|i| ["id", "server", "fit", "tools", "chat", "checked", "task", "best"].iter().map(|f| field(i, f)).collect::<Vec<_>>().join(" ")).collect();
        assert_eq!(state("best"), "0", "{all:#?}");
        // (an address that isn't this computer: refused, nothing asked)
        let (v, e) = call("localaifind", &[s("http://192.168.0.9:11434")]).unwrap();
        assert_eq!(v.to_i64(), 0);
        assert!(e.contains("isn't an address on this computer"), "{e}");
        // (Check every model that fits: one at a time; the one that calls
        // tools builds the app, the other writes its call as text)
        let n = call("localaicheck", &[Value::Integer(-1)]).unwrap().0.to_i64();
        assert_eq!(n, 2);
        wait_idle();
        let by_id = |id: &str, base: &str| (0..6).find(|&i| field(i, "id") == id && field(i, "base") == base).unwrap();
        let g = by_id("tiny:1b", &good);
        assert_eq!((field(g, "checked"), field(g, "toolsok"), field(g, "task")), ("1".into(), "1".into(), "1".into()), "{}", field(g, "error"));
        let c = by_id("tiny:1b", &chatty);
        assert_eq!((field(c, "toolsok"), field(c, "toolswhy"), field(c, "task")), ("0".into(), "as-text".into(), "0".into()));
        assert_eq!(field(c, "mode"), "small");
        // (the one that built the app is the best match, first)
        assert_eq!((state("best"), field(0, "base")), ("0".into(), good.clone()));
        // (kept: found again, the checks come back without asking)
        call("localaiclear", &[]);
        call("localaifind", &[s("")]);
        wait_idle();
        let g = by_id("tiny:1b", &good);
        assert_eq!((field(g, "checked"), field(g, "kept")), ("1".into(), "1".into()));
        // (Cancel: a check stopped at once leaves the model unchecked)
        call("localaiclear", &[]);
        std::fs::remove_dir_all(&dir).ok();
        call("localaifind", &[s("")]);
        wait_idle();
        call("localaicheck", &[Value::Integer(-1)]);
        call("localaicancel", &[]);
        wait_idle();
        assert!((0..6).filter(|&i| field(i, "checked") == "1").count() <= 1);
        call("localaitest", &[s("addresses"), s("")]);
        call("localaitest", &[s("facts"), s("")]);
        std::fs::remove_dir_all(&dir).ok();
        *TEST_CACHE_DIR.lock().unwrap() = None;
    }
}

/// (tests) Stop Checking while a slow model answers: the check ends once
/// its answer comes, the model left unchecked, no error.
#[cfg(all(test, not(target_arch = "wasm32")))]
#[test]
fn a_check_stopped_while_a_model_answers_ends_unchecked() {
    let _one = TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner());
    *TEST_CACHE_DIR.lock().unwrap() = Some(std::env::temp_dir().join(format!("rapidr-local-ai-cancel-{}", std::process::id())).to_string_lossy().to_string());
    std::env::set_var("RAPIDR_AI_KEYSTORE", "memory");
    let slow = mock_tests::serve_slow(true, 1500);
    call("localaitest", &[s("addresses"), s(slow)]);
    call("localaitest", &[s("facts"), s("total=16384&free=12288")]);
    call("localaiclear", &[]);
    call("localaifind", &[s("")]);
    for _ in 0..100 {
        if state("busy") == "0" {
            break;
        }
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(call("localaicheck", &[Value::Integer(-1)]).unwrap().0.to_i64(), 1);
    std::thread::sleep(std::time::Duration::from_millis(300));
    call("localaicancel", &[]);
    assert_eq!(state("stopping"), "1");
    let t = std::time::Instant::now();
    while state("busy") == "1" && t.elapsed().as_secs() < 8 {
        std::thread::sleep(std::time::Duration::from_millis(50));
    }
    assert_eq!(state("busy"), "0", "still checking after {:?}", t.elapsed());
    assert_eq!((state("stopped"), field(0, "checked"), field(0, "error")), ("1".into(), "0".into(), String::new()));
    call("localaitest", &[s("addresses"), s("")]);
    call("localaitest", &[s("facts"), s("")]);
    *TEST_CACHE_DIR.lock().unwrap() = None;
}

/// (tests) RapidR names no local AI app or model anywhere in the page, its
/// code or its manual: it says "local AI app" (docs/local-ai-plan.md §5).
#[cfg(test)]
#[test]
fn no_local_ai_app_or_model_is_named() {
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let names = ["ollama", "lm studio", "lmstudio", "mlx serve", "mlx-serve", "llama.cpp", "llamafile", "gpt4all", "koboldcpp", "vllm", "jan.ai", "qwen", "gemma", "llama3", "mistral", "phi-3", "deepseek-r1"];
    for f in ["ide/aisetup.inc", "docs/manual/local-ai.md", "docs/manual/es/local-ai.md", "crates/rapidr-studio/src/local_ai.rs", "tools/manual/scenes/local-ai.mjs"] {
        let text = std::fs::read_to_string(root.join(f)).unwrap_or_default().to_ascii_lowercase();
        // (this test's own list isn't a mention)
        let text = text.split("fn no_local_ai_app_or_model_is_named").next().unwrap_or("").to_string();
        for n in names {
            assert!(!text.contains(n), "{f} names {n}");
        }
    }
}

/// (tests) A model its app runs on a service elsewhere: marked, never the
/// best match, never checked; a licence text cut says so.
#[cfg(test)]
#[test]
fn a_model_run_elsewhere_is_marked_and_never_picked() {
    let mut m = Model::new("far:big", "http://127.0.0.1:1/v1");
    m.size = Some(1_000_000_000);
    describe(&mut m, &json!({"remote_host": "https://service.example", "capabilities": ["completion", "tools"], "license": "x".repeat(200_010)}));
    assert!(m.remote && m.chat);
    assert!(m.licence_text.ends_with("Studio shows its first 200,000 characters.]"));
    let mem = Memory::of(&Facts { total_memory: Some(16 * GB), free_memory: Some(12 * GB), ..Facts::default() });
    let mut near = Model::new("near:small", "http://127.0.0.1:1/v1");
    near.size = Some(2_000_000_000);
    near.chat = true;
    let rows = vec![Row { fit: fit(&m, mem), model: m, check: None, kept: false, error: String::new() }, Row { fit: fit(&near, mem), model: near, check: None, kept: false, error: String::new() }];
    let order = rank(&rows, mem);
    assert_eq!(rows[order[0]].model.id, "near:small");
    assert_eq!(best(&rows, &order).map(|k| rows[k].model.id.as_str()), Some("near:small"));
    assert!(!rows[0].to_check() && rows[1].to_check());
}
