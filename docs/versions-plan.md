# Versions: Git in RapidR Studio, reinvented to be easy (plan, 2026-10-19)

Robert (2026-10-19): "A very good IDE without versioning is not good today." RapidR Studio gets **Git integration**:
- commits, diffs, merges, cherry-picks, rollbacks, blame;
- remotes on GitHub, GitLab, Bitbucket and others.

All of it **reinvented to be intuitive**, deeply integrated, accessible, safe, and very careful about licences. **Target: the next release** (after the M0 clean-up), as "Preview 4: Versions".

## 1. The idea: Git's power, in plain words, and nothing is ever lost

Professional tools expose Git's vocabulary (stage, HEAD, detached, rebase) and let you lose work with one wrong click. RapidR Studio does the opposite:

- **Two levels, one engine.**
  - **Simple** (the default): plain words and safe actions only.
  - **Pro:** the full Git vocabulary and power.

  It's a switch in Settings, remembered per project. Teachers can lock it to Simple. Every Simple action is a real Git operation, so a project moves between the two levels and to any other Git tool freely.
- **Learn without noticing.** Each plain word carries its Git name in a tooltip: "Save a version (Git calls this a *commit*)". Students learn real Git while using friendly words. That's Learn with RAI §9 in practice.
- **Nothing is ever lost.**
  - Before anything that rewrites or discards (a rollback, a merge, switching experiments, a force push in Pro), Studio saves a hidden safety point (`refs/rapidr/safety/…`).
  - Every repository action Studio performs goes in an **action history** with **one-click Undo**, even the next day.
  - Uncommitted work is never thrown away silently.
- **One timeline for you and the AI.** RAI's per-turn checkpoints (ai-ux-plan §3) are lightweight local versions (`refs/rapidr/checkpoints/…`) on the same timeline, shown in a lighter style and never pushed. Every AI change can be diffed, kept or undone with the same tools.

| You see (Simple) | Git does | Pro shows |
|---|---|---|
| **Start versioning this project** | `git init` + RapidR's `.gitignore` | Initialize repository |
| **Save a version…** (message, which files) | commit (staging all chosen files) | Commit, with staging of files and hunks, amend |
| **Timeline** | log with a graph | Log, graph, refs |
| **What changed?** | diff (working tree, versions) | Diff (any two refs, staged / unstaged) |
| **Go back to this version** (the project, or one file) | a new commit restoring that state (never a destructive reset) | Revert, reset (soft/mixed/hard, with a safety point), checkout |
| **Undo this version** | revert | Revert |
| **Try something new** (an *experiment*) | create and switch branch | Branches, switch, rename, delete |
| **Bring this experiment into main** | merge (visual conflict resolution) | Merge, rebase (later: interactive) |
| **Copy this change here** | cherry-pick | Cherry-pick |
| **Put my changes aside** | stash | Stash list, apply, pop, drop |
| **Mark this version** ("1.0") | tag | Tags (annotated and lightweight) |
| **Who wrote this line?** | blame | Blame, line history |
| **Share online / Get latest / Sync** | push / pull (fetch + merge) / both | Fetch, pull (merge or rebase), push, force-with-lease |
| **Copy a project from the internet** | clone | Clone (depth, branch) |

## 2. Where it lives in Studio (UI)

- **The Versions panel** (sidebar, beside Project):
  - the current experiment (branch), the changed files with badges, the **Save a version** box;
  - **Sync** (↑ to share, ↓ to get) with counts;
  - the Timeline below.
- **The Timeline:** a vertical graph of versions with their messages, authors and relative times. AI checkpoints are smaller and lighter. Clicking a version opens it; right-click (or the version's buttons) offers Go back, Undo, Copy here, Mark, What changed.
- **The Diff tab:** docked in the editor area, the *same beautiful diff* the assistant uses (ai-ux-plan §6):
  - split or unified view, connecting ribbons, word-level highlights, folded unchanged lines;
  - a change minimap, and keyboard navigation (F7 / Shift+F7);
  - accessible: `+`/`−` markers and screen-reader names, never colour alone.
- **RapidR's WOW: diffs that understand RapidR.**
  - **Forms:** "What changed?" on a form shows the **designer view before and after**, with moved, resized and re-captioned controls highlighted and the changed properties listed. No other IDE shows form changes visually.
  - **Data files:** a CSV opens as a **table diff** (changed cells highlighted).
  - **Images:** side by side and onion skin.
- **The Merge tab:** a three-way merge editor ("Yours · Theirs · Result"):
  - per-change pick (yours, theirs, both, edit);
  - **form-aware merge** of `CREATE` blocks, property by property, so two people moving different buttons don't conflict;
  - an optional **"RAI, help me merge"**, which proposes a resolution you review (with permission, like every RAI action).
- **In the editor:**
  - gutter marks for added, changed and removed lines since the last version;
  - **blame on hover** (who, when, the version's message), and "Show this line's history".
- **The status bar:** the experiment's name, the sync state, and a one-click Sync.
- **The Project tree:** badges for modified, new, conflicted and ignored files.
- **Settings › Versions:** Simple/Pro, your name and email for versions, the default branch name (`main`), and **Accounts** (below).

## 3. Remotes and accounts

- **GitHub:** sign in with GitHub's **device flow**. Studio shows a code, you approve it on github.com, and the token goes into the **OS keychain**, the same way RAI keys are kept (keyring-core). Then:
  - **Publish this project** creates the repository (public or private, your choice), adds the remote and pushes;
  - **Clone** picks from your repositories or takes a URL.
  - Needs a GitHub OAuth App owned by Ruta Internet, with device flow enabled (its client ID is public by design). **Robert creates it** (an account-side action).
- **GitLab, Bitbucket, Gitea / Forgejo / self-hosted, any HTTPS remote:** a personal access token (or an app password) kept in the keychain. Their OAuth flows can come later.
- **SSH remotes:** through the system's `ssh` and `ssh-agent`. Studio never handles private keys itself.
- **Names, not logos:** "GitHub", "GitLab" and "Bitbucket" in plain text with a generic cloud icon, so we stay safely within each company's brand rules.
- **Large files:** Studio warns before committing a file over 50 MB, GitHub's limit. Git LFS comes later.

## 4. The engine: pure Rust, permissive, ours (licences first)

| Need | Choice | Licence | Why |
|---|---|---|---|
| Git itself | **gitoxide (`gix`)** | MIT OR Apache-2.0 | Pure Rust, fast, maintained; no C to build |
| Diff algorithms | **imara-diff** (gitoxide uses it) | Apache-2.0 | Myers and histogram diffs, fast |
| HTTPS transport | `gix`'s reqwest transport with **rustls** | MIT/Apache/ISC | No OpenSSL |
| SSH | the system's `ssh` (gix spawns it) | — | We ship no SSH code |
| Credentials | **keyring-core** (already used for RAI) | MIT/Apache | The OS keychain |
| Not used | ~~libgit2 / git2-rs~~ | GPL-2.0 with linking exception | It's copyleft, which our `deny.toml` rejects ("no copyleft of any strength"), and it's C |
| Not shipped | the `git` command | GPL-2.0 | Never bundled. It may be *used in our tests only* (`git fsck`, to check that the repositories we write are valid) |

- **`deny.toml` decides.** Lane G0 adds `gix` and runs `cargo deny check`. Every transitive licence must pass: no MPL, no copyleft. Anything that fails gets a feature turned off or a replacement, never an exception without Robert's OK.
- **Capability check first.** gitoxide's coverage grows release by release. G0 verifies, on the current version, clone, fetch, **push**, status, commit, merge (trees and blobs), blame, revert and cherry-pick (as a three-way merge), stash, and tags.
  - Anything missing is built *by us* on gix's lower-level crates (merge, protocol, transport), or postponed.
  - It is never filled in by calling the `git` command, because RapidR's rule is one implementation, no fallbacks.
- **A new crate, `rapidr-vcs`:** the plain-words API Studio uses:
  - `open`, `start`, `status`, `save_version`, `timeline`, `go_back`, `undo_version`;
  - experiments, `bring_into`, `copy_change`, `put_aside`, `mark`, `who_wrote`, `share`, `get_latest`, `sync`, `clone`;
  - safety points, the action history with undo, and checkpoints.

  It's pure Rust, tested on temporary repositories, and it never runs repository hooks or external filters (see security).

## 5. Security (decided conservatively)

- **Cloning is safe:** opening or cloning a repository never runs anything from it. No hooks, no filter drivers, no `core.fsmonitor`, nothing from the repo's config that executes. Pro mode can enable hooks for a project, with a clear warning.
- **Tokens:** the keychain only. Never in project files, logs or the audit log, and never shown after entry. Scopes are the minimum needed (GitHub: `repo` for private publishing, `public_repo` otherwise).
- **Pushing:**
  - a force push exists only in Pro, as **force-with-lease**, after a safety point;
  - publishing a repository defaults to **private**, so the user makes it public on purpose.
- **Secrets in commits:** before saving a version, Studio scans the changed text for things that look like keys or tokens (API key patterns, `-----BEGIN … PRIVATE KEY`) and asks before including them. It's a common beginner mistake, so we catch it.
- **Nothing is sent anywhere** except the remote the user chose. There's no telemetry.
- **Review:** a security review of credentials, transport and clone safety before the release, **Fable with Robert's OK**, or Opus if he prefers to save budget.

## 6. RAI and Versions (optional, with permission, like all RAI)

- **Suggest a message** for "Save a version" from the diff. It works with local models too, and you edit it before saving.
- **Explain these changes:** a plain-words summary of a version or a diff (also a Learn-with-RAI moment).
- **Help me merge:** a proposed resolution, shown in the Merge tab for you to accept piece by piece.
- **RAI's own changes** are checkpoints on the timeline, and RAI can be asked to "go back to before your last change".
- The registry exposes Versions' actions to RAI and MCP (every IDE block is a public, AI-operable component). The AI's permission mode applies: it never pushes or rewrites history without asking.

## 7. The web Studio

This release is desktop first.
- **Web Studio** gets local versions (the timeline, diffs, go back) stored in the browser.
- **GitHub sync on the web** comes through GitHub's REST API, because browsers can't speak Git's network protocol to GitHub without a proxy. That's planned for the following release, and said plainly in the manual.

## 8. Quality bar (what "done" means)

- **The engine:** unit tests on temporary repositories for every action, conflicts included. Every repository we write passes `git fsck` (in tests only). Interop: a repository made in Studio opens cleanly in other Git tools, and the reverse.
- **Remotes:**
  - end-to-end against a local bare repository;
  - against a **test repository on GitHub** owned by the RapidR Assistant account (with Robert's OK);
  - token and device-flow paths with mocked servers in CI-free tests.
- **Studio:**
  - flows through real events for every Simple action and the main Pro ones, on desktop and in the web's local mode;
  - light, dark and high contrast, keyboard only, screen-reader names;
  - **walked like a user:** start versioning, save, change, diff, go back, an experiment, a conflict, merge, publish to GitHub, clone it back.
- **Manual:** a "Versions" chapter with reproducible screenshots, written for beginners first, with a Pro section.
- **Performance:** status and timeline under 100 ms on a project with a thousand versions.

## 9. Lanes, models and budget

Sizes and costs are as in [master-plan.md](master-plan.md) §2. Estimated total: ~20–25% of a week, so one or two weeks with M0.

| # | Lane | Model, size | Depends on |
|---|---|---|---|
| G0 | Research and spike: gitoxide's current capabilities (push, merge, blame, stash…), `cargo deny` with gix, the binary-size impact on Studio, wasm build check; a spike that opens, commits, diffs, blames and pushes to a local bare repository | Haiku XS (survey) + Opus S (spike) | — |
| G1 | `rapidr-vcs`: the plain-words API, safety points, the action history with undo, checkpoints, the secret scan; tests with `git fsck` | Opus L | G0 |
| G2 | **The Diff tab** (shared with RAI's review: ai-ux-plan §6) + **form diff** + CSV table diff | Opus L (+ L for the form diff) | — |
| G3 | The Versions panel, the Timeline, Save a version, editor gutter marks, Project badges, the status bar | Opus L (visual) | G1 |
| G4 | Experiments + Bring into main + **the Merge tab** (three-way, form-aware) | Opus L (visual) | G1, G2 |
| G5 | Blame on hover, line history | Sonnet M | G1 |
| G6 | Accounts (keychain), GitHub device flow, Publish, Clone, Sync; token remotes (GitLab, Bitbucket, Gitea) | Opus L | G1; **Robert creates the GitHub OAuth App** |
| G7 | Pro mode: hunk staging, amend, stash, cherry-pick, tags, rebase (non-interactive), force-with-lease | Opus M–L | G1, G3 |
| G8 | RAI: message suggestions, explain changes, help me merge, checkpoints on the timeline | Sonnet M + Opus M | G1, G3, G4 |
| G9 | Security review (credentials, transport, clone safety) | **Fable, with Robert's OK** (or Opus) | G1, G6 |
| G10 | The manual chapter + screenshots, accessibility pass, the USE-AUDIT walk, the release, the video ("Never lose your work") | Sonnet M + coordinator | all |

**Order:**
1. G0 → G1 and G2 in parallel (different files).
2. Then G3, G5 and G6 (one visual lane at a time).
3. Then G4 and G7.
4. Then G8, G9 and G10.

## 10. Decided

- **Engine:** gitoxide (pure Rust, MIT/Apache). No libgit2 (copyleft, C) and no bundled `git`.
- **Naming:** Simple mode speaks plain words, with Git's names in tooltips; Pro speaks Git. The product never calls itself "Git" (Git is the Software Freedom Conservancy's trademark): the panel is **Versions**.
- **Safety:** destructive actions always save a safety point first, everything Studio does can be undone, and publishing defaults to private.
- **The diff is shared:** one diff and merge engine and UI for Versions and RAI.
