"""Shared pieces of the RAI-TRAIN scripts: where things are, the real
checker (rai_check: the tools of RapidR Studio's assistant, replayed with no
model), the three conversation set-ups a model meets (RAI's small-window
set, RAI's full set, an outside AI app over Studio's MCP server), and the
turning of a conversation into training rows.

Our own material only (docs/legal): nothing here reads RapidQ's corpus, a
cloud model's output, or anything outside this repository.
"""

from __future__ import annotations

import hashlib
import json
import os
import re
import subprocess
import sys
from pathlib import Path

ROOT = Path(os.environ.get("RAI_TRAIN_ROOT", "/Volumes/RapidRBuild/rai-train"))
REPO = Path(__file__).resolve().parents[2]
BIN = ROOT / "bin"
META = ROOT / "meta"
DATA = ROOT / "data"

# ---- what the corpus may be made of (the legal rule) ---------------------------------------
ALLOWED_PREFIXES = (
    "examples/",
    "docs/manual/",
    "crates/rapidr-lang/data/",
    "tests/fixtures/",
    "templates:",  # programs written for this corpus (tools/rai_train/programs.py)
    "registry:",  # the registry, as `rapidr lang export` prints it
)
FORBIDDEN_FRAGMENTS = (
    "examples/rapidq/",
    ".reference",
    "Downloads/Rapidq",
    "dist/ai-eval",
    "/projects/",
    "Programming/RapidR/",
    "tests/ai-eval/",
    "transcript",
)

# The recipes that ARE the reference solutions of the eval's `apps/` tasks
# (tests/ai-eval/README.md: "Each test passes on its recipe"). Training on
# them with the eval's prompts would teach the answers: they stay out.
EVAL_RECIPES = {
    "calculator.rr", "tip_calculator.rr", "stopwatch.rr", "todo_list.rr", "unit_converter.rr",
    "quiz_game.rr", "drawing_pad.rr", "password_generator.rr", "csv_viewer.rr", "server_manager.rr",
}
# Recipes topically next to a school task (`school/cpu-emulator`: "a tiny 8-bit CPU emulator"):
# also out, so a score on that task cannot come from memory. See CORPUS.md.
HOLDOUT_ADJACENT_RECIPES = {"cpu_step.rr", "cpu_stack.rr", "cpu_timing.rr", "disassembler.rr"}


def rel_to_repo(p: Path) -> str:
    try:
        return str(Path(p).resolve().relative_to(REPO))
    except ValueError:
        return str(p)


def source_allowed(src: str) -> bool:
    if any(f in src for f in FORBIDDEN_FRAGMENTS):
        return False
    return src.startswith(ALLOWED_PREFIXES)


# ---- the real checker ------------------------------------------------------------------------
class Checker:
    """rai_check in a pipe: RapidR's own tools on in-memory projects. Answers are
    cached on disk (keyed with the rapidr version) so a rebuild is fast and
    deterministic."""

    def __init__(self, cache: bool = True) -> None:
        self.use_cache = cache
        self.version = (META / "rapidr-version.txt").read_text().strip() if (META / "rapidr-version.txt").exists() else "unknown"
        self.cache_file = DATA / f"check-cache-{hashlib.sha1(self.version.encode()).hexdigest()[:10]}.jsonl"
        self.cache: dict[str, list] = {}
        if self.use_cache and self.cache_file.exists():
            for line in self.cache_file.open():
                try:
                    d = json.loads(line)
                    self.cache[d["k"]] = d["r"]
                except Exception:
                    pass
        self.proc: subprocess.Popen | None = None
        self.hits = 0
        self.misses = 0
        self._out = None

    def _start(self) -> None:
        (ROOT / "tmp" / "prints").mkdir(parents=True, exist_ok=True)
        env = dict(os.environ, RAPIDR_REGISTRY=str(ROOT / "tmp" / "check.reg"), RAPIDR_PRINT_TO=str(ROOT / "tmp" / "prints"), RAPIDR_AI_KEYSTORE="memory")
        self.proc = subprocess.Popen([str(BIN / "rai_check")], stdin=subprocess.PIPE, stdout=subprocess.PIPE, stderr=subprocess.DEVNULL, env=env, text=True, bufsize=1)

    def run(self, files: list[tuple[str, str]], main: str, calls: list[tuple[str, dict]], apply: bool = True) -> list[dict]:
        req = {"id": "x", "main": main, "files": [{"path": p, "text": t} for p, t in files], "apply": apply, "calls": [{"tool": t, "input": i} for t, i in calls]}
        key = hashlib.sha1(json.dumps(req, sort_keys=True).encode()).hexdigest()
        if self.use_cache and key in self.cache:
            self.hits += 1
            return self.cache[key]
        self.misses += 1
        for attempt in range(2):
            if self.proc is None or self.proc.poll() is not None:
                self._start()
            try:
                assert self.proc and self.proc.stdin and self.proc.stdout
                self.proc.stdin.write(json.dumps(req) + "\n")
                self.proc.stdin.flush()
                line = self.proc.stdout.readline()
                if not line:
                    raise RuntimeError("rai_check closed")
                results = json.loads(line)["results"]
                break
            except Exception:
                self.proc = None
                if attempt == 1:
                    raise
        if self.use_cache:
            self.cache[key] = results
            DATA.mkdir(parents=True, exist_ok=True)
            with self.cache_file.open("a") as f:
                f.write(json.dumps({"k": key, "r": results}) + "\n")
        return results

    def compiles(self, files: list[tuple[str, str]], main: str) -> tuple[bool, str]:
        """(true when no line is an error, the checker's text) for the project's main file."""
        r = self.run(files, main, [("check_code", {"path": main})])[0]
        text = r["content"]
        bad = any(l.startswith("error") for l in text.splitlines()) or r["is_error"]
        return (not bad, text)

    def close(self) -> None:
        if self.proc and self.proc.poll() is None:
            try:
                self.proc.stdin.close()  # type: ignore[union-attr]
                self.proc.wait(timeout=5)
            except Exception:
                self.proc.kill()


# ---- the three set-ups a model meets ------------------------------------------------------------
REMEMBER = "Remember: a change happens only through a tool call in your answer (create_project, write_file, propose_edit); asked to build, change or go on, call it now instead of asking or describing."
MCP_SYSTEM = "You are working in RapidR Studio through its tools; build the app the user asks for."
PROFILES = ("lite", "full", "mcp")


def _oai(tool: dict) -> dict:
    return {"type": "function", "function": {"name": tool["name"], "description": tool["description"], "parameters": tool["input_schema"]}}


class Setups:
    """The system prompts and tool lists, read from the programs themselves
    (never hand-copied): `rai_check --dump` for RAI, the MCP server's own
    initialize + tools/list (tools/ai_eval/studio_mcp_agent.mjs --list-tools)."""

    def __init__(self) -> None:
        dump_file = META / "rai_dump.json"
        out = subprocess.run([str(BIN / "rai_check"), "--dump"], capture_output=True, text=True, check=True).stdout
        dump_file.write_text(out)
        self.dump = json.loads(out)
        mcp_file = META / "mcp-list-tools.json"
        if not mcp_file.exists():
            res = subprocess.run(["node", str(REPO / "tools/ai_eval/studio_mcp_agent.mjs"), "--list-tools", "--out", str(ROOT / "tmp" / "listtools")], capture_output=True, text=True, env=dict(os.environ, RAPIDR=str(BIN / "rapidr"), RAPIDR_AI_KEYSTORE="memory"), check=True)
            mcp_file.write_text(res.stdout)
        self.mcp = json.loads(mcp_file.read_text())
        self.tools = {
            "lite": [_oai(t) for t in self.dump["tools_lite"]],
            "full": [_oai(t) for t in self.dump["tools"]],
            "mcp": [{"type": "function", "function": {"name": t["name"], "description": t["description"], "parameters": t.get("inputSchema") or {"type": "object", "properties": {}}}} for t in self.mcp["tools"]],
        }
        self.tool_names = {k: {t["function"]["name"] for t in v} for k, v in self.tools.items()}
        self.tool_schema = {k: {t["function"]["name"]: t["function"]["parameters"] for t in v} for k, v in self.tools.items()}

    def system(self, profile: str, project: tuple[str, str, list[str]] | None) -> str:
        """project: (name, main, files) of the open project, or None."""
        if profile == "mcp":
            return "\n\n".join([MCP_SYSTEM, self.mcp["instructions"]])
        base = self.dump["instructions_lite"] if profile == "lite" else self.dump["instructions"]
        if project is None:
            facts = "No project is open in Studio now."
        else:
            facts = f"The open project: {project[0]}; main file {project[1]}; files: {', '.join(project[2])}."
        return f"{base}\n\n{facts}\n\n{REMEMBER}"


# ---- conversations ------------------------------------------------------------------------------------
def call_msg(text: str, calls: list[tuple[str, dict]]) -> dict:
    return {"role": "assistant", "content": text, "tool_calls": [{"type": "function", "function": {"name": n, "arguments": a}} for n, a in calls]}


def mcp_applied(tool: str, call_input: dict, content: str, is_error: bool, open_project: bool) -> str:
    """What Studio's MCP server (mode: Allow everything) adds to a change's answer."""
    if is_error or tool not in ("write_file", "propose_edit"):
        return content
    path = call_input.get("path", "")
    return f"Applied in Studio's editor (unsaved, so the user can undo it with Edit > Undo; Run saves it): {path}. {content}"


class Step:
    """One assistant turn: some words (usually none) and tool calls (none: the last answer)."""

    def __init__(self, text: str = "", calls: list[tuple[str, dict]] | None = None, train: bool = True):
        self.text = text
        self.calls = calls or []
        self.train = train


MODE_NOTE = "Nothing waits for you: AI apps change files, run the program and ask RAI on their own — and a program can do anything you can on this computer. Everything is still listed, Stop ends it, and Studio's own tools never hand over keys or files outside the project."


def studio_status(project: tuple[str, str, list[tuple[str, str]]] | None, version: str, dirty: bool = False) -> str:
    """studio_status as Studio's MCP server words it (ide/mcp.inc McpStatusText), mode Allow everything.
    It is Studio's own tool, not one of rai_check's: written from that function."""
    t = f"RapidR Studio {version}\n"
    if project:
        names = ", ".join(p for p, _ in project[2])
        t += f"Project: {project[0]} (~/Projects/{project[0]}); main file {project[1]}; files: {names}\n"
        t += f"Documents: {project[1]} (active, Code view{', unsaved changes' if dirty else ''})\n"
    else:
        t += "Project: none open (open_project opens one; examples/gui/hello_form.rr is one of RapidR's examples)\n"
        t += "Documents: none open\n"
    t += "Program: not running\n"
    if project:
        t += "Errors: 0 (get_problems lists them)\n"
    t += "RAI: not set up (the user sets it up in Studio)\n"
    t += f"Permission: Allow everything — {MODE_NOTE}"
    return t


def rapidr_version() -> str:
    f = META / "rapidr-version.txt"
    m = re.search(r"(\d+\.\d+\.\d+)", f.read_text()) if f.exists() else None
    return m.group(1) if m else "2.121.3"


CANNED = {"studio_status"}  # tools of Studio's MCP server that rai_check does not have


def conversation(checker: Checker, setups: Setups, profile: str, project: tuple[str, str, list[tuple[str, str]]] | None, user: str, steps: list[Step], context: str = "") -> dict:
    """Plays the steps against the real tools; returns {messages, tools, train (assistant message
    indices to learn), results (every tool answer)}."""
    files = list(project[2]) if project else []
    main = project[1] if project else ""
    flat = [c for s in steps for c in s.calls if c[0] not in CANNED]
    results = checker.run(files, main, flat, apply=True) if flat else []
    sys_proj = (project[0], project[1], [p for p, _ in files]) if project else None
    msgs: list[dict] = [{"role": "system", "content": setups.system(profile, sys_proj)}, {"role": "user", "content": (user + ("\n\n" + context if context else ""))}]
    train: list[int] = []
    slips: list[int] = []
    k = 0
    for s in steps:
        if s.calls:
            msgs.append(call_msg(s.text, s.calls))
        else:
            msgs.append({"role": "assistant", "content": s.text})
        if s.train:
            train.append(len(msgs) - 1)
        elif s.calls:
            slips.append(len(msgs) - 1)
        for name, inp in s.calls:
            if name == "studio_status":
                content = studio_status(project, rapidr_version())
                results_entry = {"tool": name, "content": content, "is_error": False, "note": ""}
            else:
                results_entry = results[k]
                k += 1
                content = results_entry["content"]
                if profile == "mcp":
                    content = mcp_applied(name, inp, content, results_entry["is_error"], project is not None)
            msgs.append({"role": "tool", "content": content})
    return {"messages": msgs, "tools": setups.tools[profile], "train": train, "slips": slips, "results": results}


def rows_of(conv: dict) -> list[dict]:
    """Training rows: mlx-lm's chat format with tools, one per learned assistant turn (its
    `--mask-prompt` learns only the last message of a row)."""
    rows = []
    for i in conv["train"]:
        rows.append({"messages": conv["messages"][: i + 1], "tools": conv["tools"]})
    return rows


# ---- tokens ----------------------------------------------------------------------------------------------
_tok = None


def tokenizer():
    global _tok
    if _tok is None:
        from transformers import AutoTokenizer  # the training venv has it

        _tok = AutoTokenizer.from_pretrained(str(ROOT / "models" / "Qwen3.5-2B"))
    return _tok


def row_tokens(row: dict) -> int:
    return len(tokenizer().apply_chat_template(row["messages"], tools=row["tools"], return_dict=False))


# ---- small text helpers ---------------------------------------------------------------------------------
def norm(text: str) -> str:
    return re.sub(r"\s+", " ", text.lower()).strip()


def words(text: str) -> list[str]:
    return re.findall(r"[a-z0-9áéíóúñçãõâêôàü]+", text.lower())


def shingles(text: str, n: int = 5) -> set[str]:
    w = words(text)
    return {" ".join(w[i : i + n]) for i in range(max(0, len(w) - n + 1))}


def sha(text: str) -> str:
    return hashlib.sha1(text.encode()).hexdigest()[:12]


def log(*a) -> None:
    print("[corpus]", *a, file=sys.stderr, flush=True)
