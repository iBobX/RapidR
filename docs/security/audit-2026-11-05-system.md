# RapidR security audit — SYSTEM area (Preview 6, 2026-11-05)

Audit of `development` at `1e996749` (workspace 2.122.1), the SYSTEM area of
the Preview 6 security brief (`dist/lanes/security-audit-brief.txt`): file and
path components, process running, sockets and the HTTP client, archives
(RArchive), the registry, file-type registration and installers, opening a
project or example from elsewhere (planted config / project files that start
things), **Versions (gitoxide)**, crash-report scrubbing, and print-to paths.

Method: map the trust boundaries, read each entry point end to end, and prove
the material finding with a PoC test that fails today (so the green build is
not broken; a fix lane removes `#[ignore]`). Nothing was run against any
external host, keychain or network — only this Mac's temp folders. No product
code was changed (audit brief §4).

Earlier audits that still hold and are not re-done here:
`docs/security-audit.md` (2026-10-08, SEC-14…SEC-20), `docs/security/audit-preview4.md`
(SEC-21…SEC-30), `docs/security/test-mode.md` (SEC-24). The DLL / VARPTR /
PEEK / POKE managed-memory model (SEC-19, §6 of the first audit) was spot-checked
and is unchanged; it is not re-audited here.

IDs continue the `SEC-` series (last used SEC-30).

## 1. Findings

| ID | Severity | Area | One line |
|---|---|---|---|
| SEC-31 | **High** | Versions (gitoxide) | A planted `.git/config` `core.worktree` makes Versions read and write files **outside** the opened project folder — opening the project is enough; one normal Versions action exfiltrates or overwrites outside files |
| SEC-32 | Medium | Versions (gitoxide) | "Get latest from a folder" copies and checks out commits from an **untrusted** shared folder into the project worktree; crafted trees (symlink entries, mode bits) are written by gitoxide's checkout with no RapidR-side confinement — needs verifying before Preview 6's clone/sync |
| SEC-33 | Low | Project loading | A planted `.rrproj` may set `main`, `icon`, `output` and `run.args` to absolute or `..` paths (`normalize_path` keeps them); opening is inert, but Run/Build then act on a file outside the project (still behind the download-quarantine prompt) |

Nothing Critical. **SEC-31 should block Preview 6** unless fixed: Preview 6 adds
GitHub publish / clone / sync (versions-plan G6), which makes opening someone
else's project folder the normal case, and SEC-31 turns that into a read/write
escape from a folder opened but never run.

---

### SEC-31 — High — Versions: a planted `core.worktree` escapes the project folder

**Where.**
- `crates/rapidr-vcs/src/lib.rs:331-333` (`open_options` = `gix::open::Options::isolated()`);
- `crates/rapidr-vcs/src/lib.rs:395` (`gix::open_opts(dir, open_options())`) and `:400` / `:421` (`repo.workdir()` is taken as the project folder, unchecked);
- `crates/rapidr-vcs/src/lib.rs:1078-1115` (`neutralize_programs`: strips filters, hooks, credential helpers, `core.fsmonitor/sshcommand/…`, but **not** `core.worktree`);
- gitoxide 0.89 honours a repository-local `core.worktree` when `core.bare = false`: `gix-0.89.0/src/open/repository.rs:291-346`, section eligibility `:539-557` (the value must come from the repo's own `.git/config`, which is exactly what the attacker plants).

**Attacker.** Someone who gives the developer a project folder: a downloaded /
cloned / shared / USB-stick / example project that contains a `.git` directory.
After extraction the folder is owned by the developer, so RapidR's ownership
gate (`owned_by_me(dir) && owned_by_me(.git)`, `:386-394`) passes.

**Scenario, step by step.**
1. The attacker ships `evil-project/` with a real `.git` whose `.git/config` has:
   ```
   [core]
       bare = false
       worktree = /Users/victim        # or a relative ../../ path
   ```
   and at least one commit (so the repo is valid and has a timeline).
2. The developer opens `evil-project/` in RapidR Studio. **No run, no click on
   anything in Versions is needed:** opening a project calls
   `VersionsProjectChanged` → `VerRefresh` (`ide/versions.inc:711`, from
   `ide/project.inc:94`/`:174`), which sets `Repo.Folder = Proj.Folder`;
   `repository.rs` `refresh` → `read_state` → `Versions::open`
   (`crates/rapidr-studio/src/repository.rs:103-115`, `:387-416`).
3. `gix::open_opts` reads `evil-project/.git/config`, honours `core.worktree`,
   so `repo.workdir()` becomes `/Users/victim`. `neutralize_programs` does not
   touch `core.worktree`. `from_repo` only checks `workdir().is_none()`
   (`:400`), which is false — it passes.
4. `read_state` → `v.changes()` (`:398-416`) walks the index against the
   **redirected** worktree. The victim's home-folder files appear in the
   Versions pane as "new" changes — an immediate, no-click disclosure of the
   names (and later, of the contents) of files **outside the opened project**.
5. One ordinary Versions action then does more, all against `v.folder()` =
   the redirected path:
   - **Save a version** (`save_version`, `lib.rs:520-560`) reads those outside
     files through the filter pipeline and commits them; **Share to a folder**
     (`folders.rs`) copies those objects out → **exfiltration** of the victim's
     files to a USB stick / network drive / (Preview 6) GitHub.
   - **Go back** / **switch experiment** / **restore file** (`checkout.rs`
     `switch_files` `:114`, `restore_paths` `:150`, `write_entries` `:70`,
     `remove_file` `:95`) write the attacker's committed tree into the
     redirected worktree and **remove** files the tree lacks →
     **destructive overwrite** of the victim's files outside the project.

**Evidence (PoC, runs locally in temp folders).**
`crates/rapidr-vcs/tests/security_worktree_escape.rs` (two `#[ignore]`d tests
that assert the *secure* behaviour and fail today):
- `opened_project_cannot_point_its_worktree_outside_itself` — after
  `Versions::open`, `v.folder()` is the attacker's `victim-private` dir, not the
  opened `downloaded-project`.
- `status_does_not_read_files_outside_the_project` — `v.changes()` lists
  `secret.txt`, a file that lives outside the opened project.

Run: `cargo test -p rapidr-vcs --test security_worktree_escape -- --ignored`.
Observed: both fail; `v.folder()` escaped to `…/victim-private`, and `changes()`
returned `["secret.txt"]`.

**Recommended fix (for an Opus fix lane).**
- After `gix::open_opts`, canonicalise `repo.workdir()` and refuse to open (or
  open read-only with a clear reason) when it is not inside the folder RapidR
  was asked to open (`dir`). This is the single robust check; it also covers any
  other way a repo-local config could move the worktree.
- Belt and braces: in `neutralize_programs`, also strip `core.worktree` and
  normalise `core.bare` in the in-memory config (the file on disk is left
  alone, as the function already does for the others).
- Add a regression that a `core.worktree` pointing outside the project is
  refused (the PoC becomes that test, `#[ignore]` removed).
- gitoxide has no `open::Options` override to force the worktree (the env
  override is denied by `isolated()`), so the post-open check is the pragmatic
  guard.

---

### SEC-32 — Medium — Versions: "Get latest" checks out untrusted commits with no RapidR-side confinement

**Where.** `crates/rapidr-vcs/src/folders.rs` (`get_latest_from_folder`,
`open_shared` — a folder owned by another account is refused for **writing**
but allowed for **reading/get-latest**, `:66-71`); the copied history is
checked out through `crates/rapidr-vcs/src/checkout.rs` (`switch_files` /
`write_entries`, which calls `gix::worktree::state::checkout`), and the module
notes it writes "line-ending conversions, executable bits, **symbolic links**"
(`checkout.rs:14-18`).

**Attacker.** The owner of a shared folder (USB stick, network drive, and in
Preview 6 a remote the user clones/syncs): untrusted by design (the code already
treats such a folder as not-yours for writing), yet its commits are copied into
the project and written into the working folder on get-latest / fast-forward /
merge.

**Reasoning / scenario.** gitoxide's checkout writes the committed tree into the
project worktree. The classic git risk is a tree that writes a **symlink** entry
and then a file **through** that symlink (CVE-2021-21300-class), or uses `..` /
NTFS-stream / case-fold tricks to land a file outside the worktree. RapidR adds
no confinement of its own here; it relies entirely on gitoxide's checkout
guards. The archive component, by contrast, has an explicit write-through-link
guard (`crates/rapidr-value/src/objects/archive.rs:427-475`) and refuses link
entries outright — Versions has nothing equivalent, and symlink entries in a
tree **are** written as real symlinks.

**Evidence.** Not PoC'd (building a crafted multi-commit tree that triggers the
symlink-then-write ordering through gitoxide's own index/checkout is
substantial, and gitoxide may already guard it). This is a **to-verify** item,
raised because Preview 6 turns "get latest / clone / sync" from an untrusted
folder into a routine action.

**Recommended fix / next step.**
- Verify against gitoxide 0.89 whether `gix::worktree::state::checkout` refuses
  to follow a symlink created earlier in the same checkout and rejects path
  escapes; write a PoC that commits a `a -> /tmp/outside` symlink plus `a/x`
  and runs `get_latest_from_folder`, asserting nothing is written to
  `/tmp/outside`.
- If gitoxide does not guard it, add a RapidR-side pass that refuses trees with
  symlink entries whose target leaves the worktree (as the archive component
  does), before checkout.

---

### SEC-33 — Low — A planted `.rrproj` can name files outside the project

**Where.** `crates/rapidr-project/src/project.rs:665-686` (`normalize_path`
keeps a leading `/` and leading `..`), used for `main` (`:549`), and for the
`icon` / `output` project paths and `run.args`.

**Reasoning.** Opening a project is inert (parsing only — confirmed; forms and
code are parsed, never executed at design time, per the Preview 4 audit). But a
crafted `.rrproj` with `main = "/etc/x.rr"` or `main = "../../x.rr"` makes Run
and Build act on a file outside the project folder. The impact is bounded: a
downloaded `main` still goes through the quarantine / "run it?" prompt
(`crates/rapidr-cli/src/launch.rs` `trust::check`, called by `run_session`), and
running a program the user chose to open is by design.

**Recommended fix.** When loading a `.rrproj`, reject (or clamp into the project
folder) a `main` / `icon` / `output` that is absolute or climbs out with `..`,
so a project cannot point Studio's Run/Build at a file the user never saw in the
tree. Low priority; mostly defence in depth.

---

## 2. Checked and found sound (so the next audit need not redo these)

**Archives — RArchive** (`crates/rapidr-archive/`, `crates/rapidr-value/src/objects/archive.rs`).
- Zip-slip: `names::safe_path` (`names.rs:19-56`) refuses absolute paths,
  drives, `..`, `:` (drive / NTFS stream), control chars, trailing dot/space,
  and Windows device names; `\` is a separator. Battery of refusals tested
  (`names.rs:73-100`).
- Links and devices are **never written**: `refuse_kind` (`lib.rs:487-499`)
  and `extract_all` skip `Kind::Link`/`Special`; tar type flags `1`/`2` and
  7z reparse/symlink bits map to `Kind::Link` (`tar.rs:174-181`,
  `sevenz.rs:43-48`); zip `is_symlink` → `Kind::Link` (`zipfmt.rs:38-41`).
- Write-through-link guard on the desktop: `write_tree` (`archive.rs:427-475`)
  checks every path component with `symlink_metadata` before writing and
  refuses "written through a link"; it writes nothing if any name is bad
  (plan-then-write, with rollback of files it created). Tested:
  `extract_all_stays_in_its_folder` (`archive.rs:693-708`) plants a symlink and
  asserts nothing escapes.
- Zip bombs: `limit.rs` caps real output (never the declared size) at MaxSize
  and MaxRatio × packed (≥ 1 MB floor); 7z checks declared totals at open
  (`sevenz.rs:72-81`) and real bytes on extract; `extract_all` caps the sum
  (`archive.rs:464-485`, `sevenz.rs:130-171`). Web MaxSize is 256 MB.
- 7z encrypted archives refused (`sevenz.rs:18-33`). Reads are via Rust crates
  (`zip` write-only in shipped code per the Preview 4 audit; `sevenz-rust2`,
  Apache-2.0).

**Process running** (`crates/rapidr-runtime-core/src/builtins.rs:385-430`).
SHELL / RUN / SHELLWAIT run the author's command string (`cmd /S /C "…"` /
`sh -c`) — the program author's power, by design (threat model §5). On Windows
the command is passed with `raw_arg` so `cmd` parses it, not Rust's quoter
(documented at the call site). No untrusted-data path reaches these except the
program's own source.

**Opening URLs / revealing files**
(`crates/rapidr-runtime-core/src/panels.rs:67-98` `open_in_browser`,
`studio.rs:104-121` `reveal`, `crates/rapidr-cli/src/serve.rs:128-153`
`open_browser`). A link opened from an RMarkdownView is checked
`is_web_link` (http/https/mailto only, `markdown.rs:722-725`) before `open` /
`rundll32 url.dll,FileProtocolHandler` / `xdg-open`, each given the URL as one
argument (no shell). `reveal` passes the path as a single `open -R` /
`explorer /select,` / `xdg-open` argument. Under the test hooks, opening is only
reported, not performed.

**`rapidr serve` (Run in Browser)** (`crates/rapidr-cli/src/serve.rs`).
127.0.0.1 only, random 128-bit path prefix, `Host` must be the server's own
loopback (`421` otherwise: DNS rebinding), GET/HEAD only (`405`), bundle CSP +
`nosniff` + `no-store` + `no-referrer`, served from memory, ends when stdin
closes. Sound.

**HTTP client — RHTTP** (`crates/rapidr-runtime-core/src/network.rs:644-730`).
`ureq` with the OS TLS (`native_tls`: Security.framework / SChannel / OpenSSL),
so the certificate chain is verified by default; the URL is fully
author-controlled (the program's own GET/POST). No default auth header, so
SEC-27's x-api-key-across-redirects concern (a different crate, `rapidr-ai`)
does not apply here. HTTP **server** (`httpserver.rs`, `netframe.rs`) was
audited in the Preview 4 pass (Host/Origin/token/limits/smuggling) and is
unchanged.

**Printing** (`crates/rapidr-value/src/objects/mod.rs:170-236`). `lp` and
`lpstat` get `job.printer` / `job.title` as separate args (no shell); the
PDF filename is the title filtered to alphanumerics + ` -_`. `RAPIDR_PRINT_TO`
is a dev/test convenience, not program-controllable. By design.

**Registry — QREGISTRY** (`crates/rapidr-value/src/registry.rs`). One store
file in RapidR's config dir or `RAPIDR_REGISTRY`; key paths are joined with
`\\` as registry subkeys (bounded to the mapped RapidQ roots), never as
filesystem paths. The program's own registry, by design. No filesystem
traversal.

**win7 stand-ins** (`tools/win7/shim/win7_stand_ins.c`). The six WinRT/COM
stand-ins resolve the real function only from an already-loaded module
(`GetModuleHandleW`) or from Windows' **system directory** by absolute path
(`GetSystemDirectoryW` + `LoadLibraryW`), explicitly "never a same-named file
next to the program" — no DLL-planting. Missing functions return ordinary error
codes. Sound.

**Installers / file-type registration.**
- Linux desktop entry (`crates/rapidr-package/src/linux.rs`): values escaped
  (`entry()` escapes `\` and newlines); `Exec` uses `%F` (file args kept
  separate) and `exec_quoted` for the program path.
- macOS `Info.plist` (`crates/rapidr-package/src/macos.rs:35-66`): every value
  XML-escaped via `xml()`; `AppInfo::check` (`info.rs:128-146`) rejects a name
  with `/ \ :` (it names the `.app` folder) and validates the bundle id.
- These run on `rapidr build` of a project (an explicit action); untrusted
  project names are escaped and the folder-naming name is validated.

**Crash-report scrubbing** (`crates/rapidr-crash/src/scrub.rs`, `report.rs`,
`lib.rs`). A report carries only RapidR's own messages, names and places, never
file contents or program data (`lib.rs:14-17`). Every field and log line is
`scrub`bed: the home folder → `~` (all spellings, canonicalised, longest-first),
provider-key and token shapes, labelled secrets (`password=`, `Bearer …`,
`api_key:`), JWTs, a password in a URL, and PRIVATE KEY / certificate blocks;
plus the host masker for the program's noted RSecrets. The "Last log lines" are
fed only by RapidR's own `note`/run-time-error text and Studio's own status
lines (`ai_tools.rs:468`, `project.rs:855/877`), not program output. The
"Report on GitHub" URL carries the scrubbed report (user reviews before
sending). Sound.

**Versions — the rest** (`crates/rapidr-vcs/`). Repositories open
`isolated()` (no system/user/global config, no env, no includes);
`neutralize_programs` strips filters, credential helpers, diff/merge drivers,
gpg, and `core.fsmonitor/sshcommand/askpass/gitproxy/hookspath/editor/pager`,
`remote.uploadpack/receivepack/vcs`, `ssh.variant` from the in-memory config;
gitoxide runs no hooks and never shells out (local transport is refused — the
objects are copied in-process, `folders.rs`); a folder owned by another account
is refused for changes (`NotYours`) and only opened read-only; `git` is run only
in a test (`repository.rs:887`, `#[cfg(test)]`). The secret scan runs before a
save. The two gaps are SEC-31 (`core.worktree`) and SEC-32 (untrusted checkout).

## 3. Note for the fix pass

- **SEC-31** should block Preview 6: one small post-open check
  (`repo.workdir()` must canonicalise inside the opened folder), plus stripping
  `core.worktree` in `neutralize_programs`. Turn the PoC into the regression.
- **SEC-32** is a verify-then-maybe-fix before Preview 6's clone/sync: PoC a
  symlink-in-tree checkout from a shared folder; add a RapidR-side link guard if
  gitoxide does not already refuse it.
- **SEC-33** is defence in depth: clamp `.rrproj` `main`/`icon`/`output` into the
  project folder.
- No secret value appears in this document; the PoC uses only temp folders and
  placeholder data.
