# Versions, lane G0: the engine, proved (2026-10-22)

Before building Versions (Git in RapidR Studio, [versions-plan.md](versions-plan.md)), lane G0 checked gitoxide, the pure-Rust Git engine the plan chose:
- what it can do today;
- its licences against our `deny.toml`;
- its cost in Studio's size;
- whether it builds for the web;
- a working spike, `crates/rapidr-vcs`, tested end to end.

**Verdict.** gitoxide is the right engine for everything *local*:
- status, saving versions, the timeline, diffs, blame, branches, three-way merges and tags work;
- licences pass;
- a release Studio gets +5.1 MiB (stripped).

Three things are missing or need care:
1. **gitoxide cannot push.** There is no send-pack in gitoxide at any level. Sharing with a *local* folder works in-process (the spike does it). Pushing to GitHub or any HTTPS/SSH server needs our own send-pack on gitoxide's transport and pack writer. That is a real piece of work (estimate below), and **Robert decides** whether G6 builds it.
2. **gitoxide runs programs from a repository's configuration**: filter drivers, diff `textconv`, merge drivers and credential helpers. The spike removes them from the in-memory configuration, and a test proves nothing runs. Without that step, a test proves gitoxide does run them.
3. **Its local transport runs the `git` command.** `file://` URLs and plain paths spawn `git-upload-pack`. So Studio never uses gitoxide's transport for folders. It copies objects itself, as the spike's "share with a folder" does.

Versions checked: **gix 0.89.0**, the latest on crates.io on 2026-10-22 (MSRV 1.88; our toolchain is 1.98). Upstream status is from `crate-status.md` on gitoxide's `main`, read the same day.

## 1. Capabilities

Legend:
- ✅ there, with an API;
- 🧩 the building blocks are there, and we assemble the operation ourselves (a few dozen to a few hundred lines);
- ❌ missing.

"Spike" means `crates/rapidr-vcs` does it and a test checks it.

| Capability | gix 0.89 | Evidence | Spike | Plan |
|---|---|---|---|---|
| Init / open | ✅ | `ThreadSafeRepository::init_opts`, `open_opts` with `open::Options::isolated()`. The default branch is `main` (`gix/src/init.rs`, `DEFAULT_BRANCH_NAME`) | ✅ `start`, `open` | as is |
| Status (worktree ↔ index, untracked files) | ✅ | `Repository::status()` → `into_index_worktree_iter`, with rename tracking. Tree ↔ index too (`tree_index_status`) | ✅ `changes` | G1: tree ↔ index for Pro staging |
| Staging whole files (`add -A`) | 🧩 | No `add` command. `crate-status.md`: "[ ] add files with `.gitignore` handling". The blocks: `filter::Pipeline::worktree_file_to_object` (writes the blob), `gix_index::State` entry edits, `File::write` | ✅ in `save_version` | G1. Pitfall found: path lookups fail after `dangerously_push_entry` until `sort_entries` (fixed: push after lookups) |
| Staging hunks | 🧩 | No API. Build the staged blob from the chosen hunks (imara-diff's `Hunk` ranges) and set the index entry | — | G7 |
| Commit | ✅ | `commit_as` ("[x] create new commit from tree"). **No index → tree:** built with `Repository::edit_tree` (the tree editor) | ✅ `save_version` | as is |
| Log / timeline | ✅ | `rev_walk`: sorting, first-parent, hidden tips, commit-graph | ✅ `timeline` | G1: < 100 ms on 1,000 versions, to measure |
| Diff tree ↔ tree | ✅ | `diff_tree_to_tree`, with rename tracking | ✅ `compare_versions` | as is |
| Diff of file contents | ✅ | `gix-imara-diff` 0.3.1 (gitoxide's imara-diff, Apache-2.0): Myers and histogram, slider heuristics, `latin_word_diff` for word highlights, a unified printer | ✅ `hunks`, the same `@@` headers as `git diff` (tested) | G2 uses the word diff |
| Diff worktree ↔ version | 🧩 | Status plus the blob diff above (the file converted to its stored form first) | ✅ `what_changed` | as is |
| Blame | ✅ | `blame_file` (gix-blame 0.19: ranges, since, renames). `crate-status.md` says its performance is not yet on par with git's | ✅ `who_wrote`, equal to `git blame` (tested) | G5: measure on a big file |
| Branches | ✅ | Reference transactions with compare-and-swap (`PreviousValue`), `validate::reference::branch_name`. Reflogs need a committer: an isolated repository has none, so pass one (`edit_references_as`) | ✅ `start_experiment` | as is |
| Checkout / switch / reset | 🧩 | `crate-status.md`: "[ ] checkout, switch, restore and reset orchestration". The blocks: `index_from_tree` + `worktree::state::checkout` (what clone uses) | ✅ `switch_to` (rewrites all files, removes the old ones, refuses with unsaved changes) | G1: rewrite only the changed paths; a safety point first |
| Merge | ✅ trees, blobs, commits · ❌ orchestration | `merge_commits`, `merge_trees`, `merge_base`: conflicts listed, conflict markers with labels, `apply_index_entries` for stage 1–3 index entries. "[ ] merge workflow orchestration", "[ ] persist and resume conflicted merges with MERGE_HEAD" | ✅ `bring_into_current`: fast-forward, a two-parent version, conflicts reported with nothing touched | G4: write conflicts to the index and files, MERGE_HEAD, the Merge tab |
| Stash | ❌ (🧩) | "[ ] stash workflow orchestration". The blocks: commits + `refs/stash` + its reflog, restored with `merge_trees` | — | G7: in git's own format, so other tools see it |
| Cherry-pick / revert | ❌ (🧩) | "[ ] cherry-pick and revert workflow orchestration". The blocks: `merge_trees(parent, HEAD, commit)`; revert swaps the sides | — | G1 (Undo this version), G7 |
| Go back to a version | 🧩 | A new commit with the old tree (`commit_as`), then checkout | — | G1 (trivial with the spike's parts) |
| Tags | ✅ | `tag()` (annotated), `tag_reference()` (lightweight) | — | G7 |
| Rebase | ❌ | "[ ] rebase" | — | G7, built on cherry-pick |
| Fetch / clone over HTTPS | ✅ | `gix-transport`: "[x] via `curl` (blocking only)", "[x] via `reqwest` (blocking only)". The `Http` trait (`client/blocking_io/http/traits.rs`) also takes **our own HTTP client** | — (no network in tests) | G6 (see §3) |
| Fetch / clone over SSH | ✅ | Spawns the system's `ssh` (`client/blocking_io/ssh`) | — | G6, as the plan says |
| Fetch / clone from a local folder | ⚠️ runs `git` | `client/blocking_io/file.rs`: `SpawnProcessOnDemand::new_local` spawns `git-upload-pack` from PATH (line 291), or `git upload-pack` (`prepare_fallback_command`, line 139) | ✅ in-process instead (`share_to_folder`) | G1: "Get latest" from a folder in-process too |
| **Push** | ❌ | `gix`: "[ ] push". `gix-protocol`: "[ ] push", "[ ] send-pack / receive-pack client plumbing". What exists: the transport's `Service::ReceivePack` handshake (`gix-transport/src/lib.rs:50`), pack writing (`gix_pack::data::output`), pkt-lines | ✅ to a **local** bare repository, in-process: missing objects copied, the branch moved with compare-and-swap, never forced (`NeedsGetLatest`) | **Decision for Robert** (§6). Our own send-pack: Opus M–L |
| Credential helpers | ✅, which is the risk | `gix-credentials` runs `credential.helper` programs and `git credential` ("[x] run `git credential` directly") | ✅ removed from the configuration | G6: our keychain through `Connection::with_credentials` (`remote/connection/access.rs:27`), never helpers |
| Submodules | ✅ read | `gix-submodule`: list, status. No update | — | Shown read-only. Later |
| LFS | ❌ | "gix-lfs" is a goal only | — | Not needed: warn over 50 MB (plan §3) |
| Hooks | never run ✅ | No hook support at all ("[ ] execute hooks", "[ ] respect core.hooksPath"). `init` writes the usual inert `*.sample` files | ✅ tested: 7 executable hooks, none ran | A test pins it for when gitoxide adds hooks |
| Filter drivers (`.gitattributes` `filter=`) | ⚠️ **run** | `gix/src/filter.rs:225` `extract_drivers` reads `filter.<name>.clean/smudge/process` from the configuration. Every pipeline uses it: status, add, checkout, diff, merge | ✅ removed; the control test proves plain gitoxide runs them | G1 keeps the removal and the tests |
| Diff / merge drivers | ⚠️ **run** | `diff.<name>.textconv/command` (gix-diff `pipeline.rs`), `merge.<name>.driver` (gix-merge `platform/merge.rs:79`, `prepare_external_driver`) | ✅ removed, tested | as is |
| Built-in conversions | ✅ in-process | End-of-line/CRLF, `ident`, `working-tree-encoding` (encoding_rs) run in gitoxide itself, with no programs | ✅ | as is |
| `core.fsmonitor` | not supported | No fsmonitor code (only the sample hook text) | removed anyway | — |
| Ownership (safe.directory) | ✅ different from git | `gix-sec` computes trust from ownership. A repository owned by someone else opens with **reduced trust**: its configuration is ignored, but it is not refused. `open::Options::bail_if_untrusted(true)` refuses it. In isolated mode git's `safe.directory` (global configuration) is not read | — | **Decision** (§6). Recommended: refuse with an explanation and an "Open anyway, read-only" choice |
| Signed commits | ❌ | "[ ] create signed commits and tags" | — | Not needed (plan "Next") |
| Thread safety | ✅ | `ThreadSafeRepository`. The `parallel` feature makes the object store shareable (needed by `worktree::state::checkout`) | ✅ | Status off the UI thread |

## 2. Security, as the spike does it (versions-plan §5)

- **Isolated open.** `gix::open::Options::isolated()` reads only the repository's own configuration. It never reads the system's, the user's (`~/.gitconfig`), the environment's (`GIT_DIR`, `GIT_SSH_COMMAND`…) or included files, and it never runs the `git` binary to find its install configuration (`git_binary: false`). Who saves a version comes from Studio (`Person`), not from git's settings. A side effect found by the spike: reflogs need an identity, so every reference edit passes one (`edit_references_as`).
- **Nothing configured runs.** At open, `neutralize_programs` removes these from the *in-memory* configuration:
  - `filter.*`, `credential.*`, `gpg.*` sections;
  - `diff.<driver>` and `merge.<driver>` subsections;
  - `core.fsmonitor`, `sshCommand`, `askPass`, `gitProxy`, `hooksPath`, `editor` and `pager`;
  - `remote.*.uploadpack`, `receivepack` and `vcs`;
  - `ssh.variant`.

  The file on disk is never rewritten, and a test compares it byte for byte.
- **The tests:**
  - `never_runs_configured_programs_or_hooks`: a repository asks for programs everywhere (each would `touch` a mark file) and has seven executable hooks. The test opens it, checks status, saves, diffs, starts an experiment, switches, merges (content merge), runs blame and compares versions, and shares. No mark appears.
  - With the removal disabled (tried once, by hand), the same test fails at the first save: gitoxide ran `/bin/sh -c "touch …/filter-process"`.
  - `plain_gitoxide_runs_a_configured_filter_driver` keeps that proof as a regular test, so a gitoxide upgrade that changes the behaviour is noticed.
- **No `git` command, ever, in product code.** Folders are shared in-process. The tests use the system's `git` only as an independent judge, and skip those checks without it.
- **For G9's review:**
  - the removal list must grow with gitoxide (check every upgrade's changelog for new config keys that run programs);
  - `url.*.insteadOf` rewrites are kept (they run nothing, but they redirect: show the real URL before Sync);
  - a URL can't name a program: `ext::…` and `<helper>::…` remote URLs are refused by gix-transport's `connect` ("currently unsupported", `client/blocking_io/connect.rs`).

## 3. Licences

Command: `cargo deny check licenses bans` with the repository's `deny.toml`, for the whole workspace. Also run with `--features rapidr-vcs/network`. **Both pass** (`bans ok, licenses ok`).

- **One trap, avoided.** gix's usual `max-performance-safe` feature turns on `pack-cache-lru-static`, which pulls **uluru (MPL-2.0)**. The spike uses `parallel` + `pack-cache-lru-dynamic` instead (clru, MIT). The manifest says why.
- **TLS.** Plan §4 said "reqwest with **rustls**", but `deny.toml` bans `rustls`, `ring`, `aws-lc-*` and `webpki-roots` ("TLS is the system's"). The `network` feature therefore uses **reqwest + native-tls** (Security.framework, SChannel, OpenSSL 3 on Linux), the same TLS as RHTTP and RAI. No rustls or ring ends up in the graph.
- **No copyleft, no MPL, no C library.** Hashing is `sha1dc` (collision-detecting SHA-1, MIT/Apache) and zlib is `zlib-rs` (Zlib), both pure Rust.
- `THIRD_PARTY_NOTICES.md` lists every workspace crate's dependencies, so it is regenerated: 598 → 682 libraries (`tools/third_party_notices.py --check` passes). What built programs carry (`rapidr notices`) is unchanged, because rapidr-vcs is in no program and not yet in Studio. When it joins Studio (`rapidr-cli`), the `tools-<os>` notices pick it up by themselves.
- `cargo deny check advisories` reports nothing from gitoxide's crates. Its failures are already in the tree, outside this lane:
  - `zip` 0.6.6: RUSTSEC-2026-0367/0368/0369, all in `extract()`, which RapidR never calls;
  - `zbus_names`, `zvariant` and `zvariant_derive`: unmaintained (RUSTSEC-2026-0370/0371/0372);
  - a yanked `inout`.

**New crates in Studio's binary.** These are compared with `cargo tree -p rapidr-cli` (the Studio binary), all targets.

*Local features (82 crates + rapidr-vcs):*

| Licence | Crates |
|---|---|
| MIT OR Apache-2.0 | gix 0.89.0, gix-actor 0.44.0, gix-attributes 0.37.0, gix-bitmap 0.6.0, gix-blame 0.19.0, gix-chunk 0.10.0, gix-command 0.12.0, gix-commitgraph 0.41.0, gix-config 0.62.0, gix-config-value 0.21.0, gix-date 0.18.0, gix-diff 0.69.0, gix-dir 0.31.0, gix-discover 0.57.0, gix-error 0.5.1, gix-filter 0.36.0, gix-fs 0.24.0, gix-glob 0.29.0, gix-hash 0.28.0, gix-hashtable 0.18.0, gix-ignore 0.24.0, gix-index 0.57.0, gix-lock 26.0.0, gix-macros 0.2.0, gix-merge 0.22.0, gix-object 0.66.0, gix-odb 0.86.0, gix-pack 0.76.0, gix-packetline 0.24.0, gix-parallel 0.50.0, gix-path 0.14.0, gix-pathspec 0.22.0, gix-protocol 0.67.0, gix-quote 0.10.0, gix-ref 0.69.0, gix-refspec 0.47.0, gix-revision 0.51.0, gix-revwalk 0.37.0, gix-sec 0.16.0, gix-shallow 0.15.0, gix-status 0.36.0, gix-submodule 0.36.0, gix-tempfile 26.0.0, gix-trace 0.3.0, gix-transport 0.61.0, gix-traverse 0.63.0, gix-url 0.40.0, gix-utils 0.5.1, gix-validate 0.13.0, gix-worktree 0.58.0, gix-worktree-state 0.36.0, gix-zlib 0.3.0, bstr 1.13.1, crossbeam 0.8.5, crossbeam-deque 0.8.8, crossbeam-epoch 0.9.21, filetime 0.2.29, hash32 0.3.1, hashbrown 0.14.5 (a second version), heapless 0.8.0, io-close 0.3.7, sha1dc 0.1.5 |
| MIT | clru 0.6.3, dashmap 6.2.1, dua-core 4.2.0, faster-hex 0.10.1, nonempty 0.12.0, prodash 31.0.1 |
| Apache-2.0 | gix-imara-diff 0.3.1, unicode-bom 2.0.3 |
| Unlicense OR MIT | jiff 0.2.38, jiff-core 0.1.1, jiff-static 0.2.38, jiff-tzdb 0.1.9, jiff-tzdb-platform 0.1.3, same-file 1.0.6, walkdir 2.5.0, winapi-util 0.1.11 |
| Zlib | zlib-rs 0.6.8 |
| CC0-1.0 OR MIT-0 OR Apache-2.0 | dunce 1.0.5 |
| (Apache-2.0 OR MIT) AND BSD-3-Clause | encoding_rs 0.8.35 (already in the lockfile, now in Studio) |
| (MIT OR Apache-2.0) AND Unicode-3.0 | unicode-normalization 0.1.26 |

*The `network` feature adds 28 more:*

| Licence | Crates |
|---|---|
| MIT | h2 0.4.20, http-body 1.0.1, http-body-util 0.1.3, hyper 1.9.0, hyper-util 0.1.20, mio 1.2.0, tokio-native-tls 0.3.1, tokio-util 0.7.18, tower 0.5.3, tower-http 0.6.11, tower-layer 0.3.3, tower-service 0.3.3, try-lock 0.2.5, want 0.3.2 |
| MIT OR Apache-2.0 | gix-credentials 0.42.0, gix-negotiate 0.37.0, gix-prompt 0.19.0, futures-channel 0.3.32, futures-sink 0.3.32, http 1.4.0, httparse 1.10.1, hyper-tls 0.6.0, ipnet 2.12.2, mime 0.3.17, reqwest 0.13.5, rustls-pki-types 1.15.1 (types only, not rustls) |
| Apache-2.0 | sync_wrapper 1.0.2 |
| Apache-2.0 WITH LLVM-exception OR Apache-2.0 OR MIT | wasi 0.11.1 |

## 4. Size

Measured, not guessed:
- `rapidr` (the Studio binary, `rapidr-cli`) built `--release` and stripped with `strip`;
- macOS arm64, CARGO_BUILD_JOBS=3;
- the same checkout, once without and once with rapidr-vcs linked.

To link it, `main` called a probe behind an environment variable. The probe used every spike function, plus, for the network row, a clone over HTTPS, so nothing was optimized away. The probe was temporary and is not committed:

```rust
if let Some(dir) = env::var_os("RAPIDR_G0_PROBE") { g0_probe(dir); }
// g0_probe: Versions::start, changes, save_version, what_changed, timeline,
// who_wrote, compare_versions, start_experiment, switch_to,
// bring_into_current, create_shared_folder, share_to_folder
// (+ network: gix::clone::PrepareFetch::new(url…).fetch_then_checkout → main_worktree)
```

| Build | Stripped | Unstripped | vs. today |
|---|---|---|---|
| Studio today | 38,238,408 B (36.5 MiB) | 48,003,056 B | — |
| + rapidr-vcs, local features | 43,601,880 B | 55,214,480 B | **+5,363,472 B (+5.1 MiB, +14.0%)** |
| + the `network` feature (reqwest + native-tls) | 47,049,048 B | 60,039,008 B | **+8,810,640 B (+8.4 MiB, +23.0%)**; the network alone is +3.3 MiB |

The stripped probe binary was also run once in a scratch folder. It made a repository, saved a version and shared it with a bare folder whose `main` pointed at the same commit.

**Making it smaller:**
- **One HTTP stack.** Implement gix-transport's `Http` trait over **ureq + native-tls**, which Studio already links for RHTTP and RAI, instead of reqwest. Most of the network row would disappear: hyper, h2, the tower family and the tokio glue. This is also "one implementation", and it is recommended for G6.
- Lean builds (M3): Versions lives only in Studio, never in users' programs, so their runners don't grow.

## 5. The web (wasm32-unknown-unknown)

- **`gix` does not compile for the browser.** `gix-sec` reads file ownership with `std::os::unix` or Windows APIs (`identity.rs:36`), and `io-close` (through gix-worktree-state) has no wasm path.
- **gitoxide's plumbing does compile.** A throwaway crate (deleted after the check) built for wasm32-unknown-unknown with:
  - `gix-object`, `gix-hash` (sha1), `gix-index`, `gix-pack`;
  - `gix-diff` (blob: imara-diff);
  - `gix-merge` (tree and blob merges);
  - and what they pull in: `gix-revwalk`, `gix-traverse`, `gix-revision`…
- **So the web Studio's local versions (plan §7) are possible on gitoxide's plumbing**, with our own object store in the browser (IndexedDB or OPFS) behind `gix_object::Find`/`Write`. Diffs and merges would be the same code as the desktop.
- GitHub sync on the web stays REST, as planned.

## 6. Decisions for Robert

1. **Push.** gitoxide has none. Options:
   - **(a) Recommended:** G6 builds our own send-pack on gitoxide's parts:
     - the transport's receive-pack handshake (HTTPS through our client, SSH through the system);
     - a pack of the missing objects (`gix_pack::data::output`);
     - the command list, report-status and side-band;
     - force-with-lease as a compare-and-swap.

     About 600–1,000 lines plus tests against a local `git-receive-pack`, the system's git used only in tests. **Opus M–L.**
   - (b) Wait for gitoxide upstream. Push is unchecked in both `gix` and `gix-protocol`, and no date is announced.

   Without push, "Share online" can't ship. Local folders, clone and fetch can.
2. **Size:** +5.1 MiB for local Versions, +3.3 MiB more for the network with reqwest (Studio 36.5 → 44.9 MiB stripped). Is that acceptable? Implementing gitoxide's HTTP trait over ureq should take most of the network part away. Recommended.
3. **TLS wording:** plan §4 said rustls, but `deny.toml` (which "decides") bans it. Native TLS is used. The plan's table is corrected to say so.
4. **Repositories owned by someone else** (a USB stick, another account's folder): refuse with an explanation and an "Open read-only" choice (recommended), or open with reduced trust as gitoxide does by default.

## 7. Recommendation for G1

- **Engine:** gix 0.89 with the spike's features. Pin the minor version, because gitoxide's 0.x releases break APIs. Upgrade on purpose, re-running the security tests and re-reading the new config keys.
- **Start from `crates/rapidr-vcs`.** Its parts are proved:
  - isolated open and program removal;
  - whole-file staging;
  - index → tree;
  - commit with a given person;
  - timeline, hunks, blame;
  - experiments with compare-and-swap reference edits;
  - checkout of a tree;
  - fast-forward and three-way merge with conflicts reported untouched;
  - in-process sharing with a folder.
- **Build on gitoxide's primitives in G1:**
  - Go back (a new version with an old tree) and Undo this version (revert as `merge_trees`);
  - safety points under `refs/rapidr/safety/…`, written before every switch, merge and go-back;
  - the action history with undo (a small journal under `.git/rapidr/`);
  - AI checkpoints (`refs/rapidr/checkpoints/…`: commits that never touch HEAD or the index);
  - the secret scan before saving;
  - "Get latest" from a folder, in-process;
  - packs instead of loose objects when sharing many objects;
  - checkout of changed paths only.
- **Keep the tests' rule:** every repository written passes `git fsck --strict` and `git status`, and agrees with `git diff` and `git blame`, when git is installed.
- **Measure in G1:** status and timeline on a project with 1,000 versions (plan §8: under 100 ms).
- **Later lanes:**
  - G6: the HTTP client over ureq, credentials from the keychain only, push per decision 1;
  - G7: stash in git's format, cherry-pick, tags, hunk staging, rebase on cherry-pick;
  - web: plumbing on a browser object store.
