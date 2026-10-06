//! The tokenizer: one line at a time, from the state stack the line before
//! left, to the stack this line leaves.
//!
//! A stack is a list of state numbers above the implicit `root`; an entry
//! with [`EMBED_MARK`] set opens an embedded language's region (its low bits
//! index the enclosing language's embeds), and the entries after it are
//! that language's states. At each step the innermost region's end pattern
//! is searched first, then the current state's rules (one multi-pattern
//! regex: the leftmost match, ties to the earlier rule) before it.

use regex_automata::{Anchored, Input};

use super::{Embed, Language, Rule, Token, TokenKind};

/// Marks a stack entry that opens an embedded language.
pub const EMBED_MARK: u16 = 0x8000;
/// Pushes beyond this depth are ignored (a nesting bomb can't grow a stack
/// without bound).
pub const MAX_STACK: usize = 64;
/// Only a line's first this many bytes are tokenized (the rest is text): a
/// minified file's one huge line stays cheap.
pub const MAX_LINE_BYTES: usize = 20_000;
/// Empty matches in a row (rules that only change state) before the
/// tokenizer moves on anyway.
const MAX_EMPTY_STEPS: usize = 8;

struct Ctx<'a> {
    lang: &'a Language,
    state: u16,
    /// The innermost embedded region: its definition and its marker's
    /// index in the stack.
    region: Option<(&'a Embed, usize)>,
    /// Whether the top of the stack is one of `lang`'s states (not its
    /// implicit root).
    top_is_state: bool,
}

impl Language {
    fn context<'a>(&'a self, stack: &[u16]) -> Ctx<'a> {
        let mut ctx = Ctx { lang: self, state: 0, region: None, top_is_state: false };
        for (i, &e) in stack.iter().enumerate() {
            if e & EMBED_MARK != 0 {
                match ctx.lang.embeds.get((e & !EMBED_MARK) as usize) {
                    Some(emb) => {
                        ctx.region = Some((emb, i));
                        ctx.lang = &emb.lang;
                        ctx.state = 0;
                        ctx.top_is_state = false;
                    }
                    None => break,
                }
            } else {
                ctx.state = e;
                ctx.top_is_state = true;
            }
        }
        if ctx.state as usize >= ctx.lang.states.len() {
            ctx.state = 0;
        }
        ctx
    }

    /// Whether the stack is inside an embedded language's region.
    pub fn in_embedded(stack: &[u16]) -> bool {
        stack.iter().any(|e| e & EMBED_MARK != 0)
    }

    /// Tokenizes `line` (without its break) from `stack`, leaving in `stack`
    /// the state for the next line. Tokens other than plain text go to `out`
    /// (cleared first), in order, adjacent ones of one kind joined.
    pub fn tokenize_line(&self, line: &str, stack: &mut Vec<u16>, out: &mut Vec<Token>) {
        out.clear();
        if self.states.is_empty() {
            return;
        }
        let mut len = line.len().min(MAX_LINE_BYTES);
        while !line.is_char_boundary(len) {
            len -= 1;
        }
        let mut pos = 0;
        let mut empty_steps = 0;
        let mut word = String::new();
        loop {
            let ctx = self.context(stack);
            if ctx.lang.states.is_empty() {
                // (an embedded language without rules: text to its end)
                match ctx.region.and_then(|(emb, marker)| emb.end.search(&Input::new(line).span(pos..len)).map(|m| (m, emb, marker))) {
                    Some((m, emb, marker)) if !m.is_empty() || empty_steps < MAX_EMPTY_STEPS => {
                        empty_steps = if m.is_empty() { empty_steps + 1 } else { 0 };
                        emit(out, m.start(), m.end(), emb.end_token);
                        stack.truncate(marker);
                        pos = m.end();
                        continue;
                    }
                    _ => break,
                }
            }
            // the innermost region's end, if it is on this line
            let mut limit = len;
            let mut end_match = None;
            if let Some((emb, marker)) = ctx.region {
                if let Some(m) = emb.end.search(&Input::new(line).span(pos..len)) {
                    limit = m.start();
                    end_match = Some((m.start(), m.end(), emb, marker));
                }
            }
            let state = &ctx.lang.states[ctx.state as usize];
            let mut found = state.regex.as_ref().and_then(|re| re.search(&Input::new(line).span(pos..limit)));
            if let Some(m) = found {
                let rule = &state.rules[m.pattern().as_usize()];
                if m.is_empty() && (!rule.changes_state() || empty_steps >= MAX_EMPTY_STEPS) {
                    // an empty match that can't be used: one character of
                    // the state's text, or on to the end
                    if m.start() < limit {
                        let step = line[m.start()..].chars().next().map_or(1, char::len_utf8);
                        emit(out, pos, m.start() + step, state.default);
                        pos = m.start() + step;
                        empty_steps = 0;
                        continue;
                    }
                    found = None;
                }
            }
            if let Some(m) = found {
                let (ms, me) = (m.start(), m.end());
                let rule = &state.rules[m.pattern().as_usize()];
                empty_steps = if ms == me { empty_steps + 1 } else { 0 };
                emit(out, pos, ms, state.default);
                let mut kind = rule.token;
                if rule.keywords {
                    word.clear();
                    word.push_str(&line[ms..me]);
                    if let Some(k) = ctx.lang.keyword_kind(&word) {
                        kind = k;
                    }
                }
                match &rule.single {
                    Some(single) => {
                        let mut caps = single.create_captures();
                        single.search_captures(&Input::new(line).span(ms..limit).anchored(Anchored::Yes), &mut caps);
                        let mut at = ms;
                        for (g, gk) in rule.groups.iter().enumerate() {
                            if let Some(span) = caps.get_group(g + 1) {
                                if span.start < at || span.end > me {
                                    continue;
                                }
                                emit(out, at, span.start, kind);
                                emit(out, span.start, span.end, gk.unwrap_or(kind));
                                at = span.end;
                            }
                        }
                        emit(out, at, me, kind);
                    }
                    None => emit(out, ms, me, kind),
                }
                rule.apply(ctx.top_is_state, stack);
                pos = me;
                continue;
            }
            emit(out, pos, limit, state.default);
            match end_match {
                Some((es, ee, emb, marker)) => {
                    if es == ee {
                        if empty_steps >= MAX_EMPTY_STEPS {
                            break;
                        }
                        empty_steps += 1;
                    } else {
                        empty_steps = 0;
                    }
                    emit(out, es, ee, emb.end_token);
                    stack.truncate(marker);
                    pos = ee;
                }
                None => break,
            }
        }
    }
}

impl Rule {
    pub(crate) fn changes_state(&self) -> bool {
        self.pop || self.push.is_some() || self.next.is_some() || self.embed.is_some()
    }

    /// The rule's state change (`top_is_state`: the stack's top is a state of
    /// the current language, not its implicit root).
    fn apply(&self, top_is_state: bool, stack: &mut Vec<u16>) {
        if self.pop && top_is_state {
            stack.pop();
        }
        if let Some(n) = self.next {
            if top_is_state && !self.pop {
                stack.pop();
            }
            if stack.len() < MAX_STACK {
                stack.push(n);
            }
        }
        if let Some(p) = self.push {
            if stack.len() < MAX_STACK {
                stack.push(p);
            }
        }
        if let Some(e) = self.embed {
            if stack.len() < MAX_STACK {
                stack.push(EMBED_MARK | e);
            }
        }
    }
}

fn emit(out: &mut Vec<Token>, start: usize, end: usize, kind: TokenKind) {
    if end <= start || kind == TokenKind::TEXT {
        return;
    }
    if let Some(last) = out.last_mut() {
        if last.kind == kind && last.end as usize == start {
            last.end = end as u32;
            return;
        }
    }
    out.push(Token { start: start as u32, end: end as u32, kind });
}
