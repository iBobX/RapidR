//! What a tool call may do, decided in ONE place for both callers — RAI
//! (Studio's assistant) and AI apps (Studio's MCP server) — from the
//! user's permission, the tool's permission class ([`crate::toolset`]'s
//! `grant`) and, for RAI, the Coach dial and School Lock
//! (docs/ai-tools.md "Permissions").
//!
//! The permission words of both are the same four levels:
//!
//! | AI apps (the pane's menu) | RAI (Settings > AI) | read | note | change | run, drive, rai |
//! |---|---|---|---|---|---|
//! | Read only | read-only | allow | refuse | refuse | refuse |
//! | Ask before changes | propose | allow | allow | ask | ask |
//! | Allow edits | — | allow | allow | allow | ask |
//! | Allow everything | auto ("Work on its own") | allow | allow | allow | allow |
//!
//! "Ask" is each caller's own way of asking the person: AI apps' calls wait
//! in the AI Apps pane (Allow Once, Allow for This Session, Deny); RAI's
//! changes are proposals the user reviews, its runs and its use of the
//! running program a card in the chat. A grant the person gave (Allow for
//! This Session, Allow for this run) turns "ask" into "allow" — the callers
//! keep those, they are the person's answers, not policy.
//!
//! Coach (RAI's dial, or School Lock keeping it there) refuses every change
//! and run — RAI gives hints, the student writes and runs the code — and
//! leaves reading and looking. School Lock's "no AI apps" keeps the MCP
//! server from starting at all (ide/schoollock.inc), so no call reaches here.

use crate::toolset::{self, Kind};

/// What a call gets.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Decision {
    Allow,
    /// The person is asked first (each caller's own way).
    Ask,
    Refuse,
}

impl Decision {
    pub fn word(self) -> &'static str {
        match self {
            Decision::Allow => "allow",
            Decision::Ask => "ask",
            Decision::Refuse => "refuse",
        }
    }
}

/// Who calls.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Caller {
    Rai,
    App,
}

/// A permission, in either caller's words, as one of the four levels:
/// "read", "ask", "edits", "all".
pub fn level(mode: &str) -> &'static str {
    match mode.trim().to_ascii_lowercase().as_str() {
        "read" | "read-only" | "readonly" => "read",
        "edits" => "edits",
        "all" | "auto" => "all",
        _ => "ask",
    }
}

/// The decision for a call of `tool` (or an old name of it) under `mode`;
/// `coach`: RAI's dial (or School Lock) is on Coach. An unknown tool is
/// refused.
pub fn decide(tool: &str, mode: &str, coach: bool) -> Decision {
    let Some(spec) = toolset::find(tool) else { return Decision::Refuse };
    decide_class(spec.name, spec.grant, spec.kind, mode, coach)
}

/// Tools that write the student's code for them, though they change no
/// file: never in Coach.
const COACH_NEVER: &[&str] = &["button_grid"];

/// The decision for a tool's permission class and kind.
pub fn decide_class(name: &str, grant: &str, kind: Kind, mode: &str, coach: bool) -> Decision {
    // (Coach: hints only — the student changes and runs the code)
    if coach && ((grant != "read" && grant != "note" && !(grant == "drive" && kind == Kind::Read)) || COACH_NEVER.contains(&name)) {
        return Decision::Refuse;
    }
    if grant == "read" {
        return Decision::Allow;
    }
    match (level(mode), grant) {
        ("read", _) => Decision::Refuse,
        ("all", _) => Decision::Allow,
        (_, "note") => Decision::Allow,
        ("edits", "change") => Decision::Allow,
        _ => Decision::Ask,
    }
}

/// What a refused call is told, in plain words, with what to do instead.
pub fn refusal(caller: Caller, tool: &str, mode: &str, coach: bool) -> String {
    let name = toolset::canonical(tool);
    if toolset::find(&name).is_none() {
        let names: Vec<&str> = toolset::TOOLS.iter().map(|t| t.name).collect();
        return format!("There is no tool `{name}`: the tools are {}.", names.join(", "));
    }
    if coach && caller == Caller::Rai {
        return crate::coach::refusal(&name);
    }
    match caller {
        Caller::App => "Not allowed: RapidR Studio lets AI apps only look at this project now, and not at the running program (its user chose Read only in the AI Apps pane), so nothing can be changed, run, looked at in the program or used. Tell the user what you would do instead; they can choose another mode there.".to_string(),
        Caller::Rai if level(mode) == "read" => format!("{name}: the user allowed reading only (Tools > Settings > AI > It may): you can't change, run or use their program. Say what you would do instead, in words."),
        Caller::Rai => format!("{name} isn't allowed now."),
    }
}

/// Writes a call to the audit log both callers share (beside Studio's
/// settings, never in the project): when, who (`source` "mcp" with the AI
/// app's name, or "rai"), which tool, the project's file it touched (never
/// a file's text) and how it went.
pub fn audit(source: &str, client: &str, tool: &str, target: &str, decision: &str) {
    let line = serde_json::json!({"time": rapidr_ai::audit::now_iso(), "source": source, "op": "tool", "client": client, "tool": tool, "target": target, "decision": decision});
    rapidr_ai::audit::append(&line.to_string());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_four_levels_decide_alike_for_both_callers() {
        for (rai, app) in [("read-only", "read"), ("propose", "ask"), ("auto", "all")] {
            for t in toolset::TOOLS {
                assert_eq!(decide(t.name, rai, false), decide(t.name, app, false), "{} under {rai} / {app}", t.name);
            }
        }
        assert_eq!(decide("read_file", "read", false), Decision::Allow);
        assert_eq!(decide("propose_edit", "read", false), Decision::Refuse);
        assert_eq!(decide("propose_edit", "ask", false), Decision::Ask);
        assert_eq!(decide("propose_edit", "edits", false), Decision::Allow);
        assert_eq!(decide("run_program", "edits", false), Decision::Ask);
        assert_eq!(decide("app_click", "all", false), Decision::Allow);
        assert_eq!(decide("app_layout", "ask", false), Decision::Ask, "looking at the program shows what it shows");
        assert_eq!(decide("app_wait", "read", false), Decision::Allow);
        assert_eq!(decide("report_gap", "ask", false), Decision::Allow);
        assert_eq!(decide("report_gap", "read", false), Decision::Refuse);
        assert_eq!(decide("restore_version", "ask", false), Decision::Ask, "restore is a change: it asks");
        // (old names decide as their tools)
        assert_eq!(decide("get_problems", "read", false), Decision::Allow);
        assert_eq!(decide("no_such_tool", "all", false), Decision::Refuse);
    }

    #[test]
    fn coach_reads_and_looks_never_changes_or_runs() {
        for t in toolset::TOOLS {
            let d = decide(t.name, "auto", true);
            match (t.grant, t.kind) {
                ("read", _) | ("note", _) if t.name != "button_grid" => assert_eq!(d, Decision::Allow, "{}", t.name),
                ("drive", Kind::Read) => assert_eq!(d, Decision::Allow, "{}: Coach may look", t.name),
                _ => assert_eq!(d, Decision::Refuse, "{}", t.name),
            }
        }
        assert!(refusal(Caller::Rai, "propose_edit", "propose", true).contains("Coach"));
        assert!(refusal(Caller::App, "propose_edit", "read", false).contains("Read only"));
        assert!(refusal(Caller::Rai, "nope", "auto", false).contains("There is no tool `nope`"));
    }
}
