# Legal review, 2026-10-30: "AI on this computer" (Preview 5: Set Up RAI › On this computer, Check My Models, RSystemInfo, small-model mode, recipe fallback)

"Lawyer agent" review (standing rule since 2026-10-22). A careful, conservative risk review, **not legal advice** (US/EU practice; Uruguayan law not checked). Rule applied: remove or reword anything that could plausibly cause a legal issue for Ruta Internet SRL or for users who build commercial apps.

- **Scope:** `docs/local-ai-plan.md`, `docs/manual/local-ai.md`, `ide/aisetup.inc` (every `TR$`), `crates/rapidr-studio/src/local_ai.rs` (the working tree on top of 6c396e8e), `crates/rapidr-value/src/sysinfo.rs`, the RSYSTEMINFO entry in `objects.toml`, `web_facts` in `object_web.rs`, `loopback_url` (`rapidr-ai/src/provider.rs`), `http.rs` (redirects), `recipes.rs` and the fallback in `ai.rs`, `tools/manual/scenes/local-ai.mjs`, `examples/system_info.rr`, the Spanish strings tagged `# legal`, `LEGAL.md`, and the older pages this feature makes stale (`docs/manual/ai-assistant.md`, `crates/rapidr-ai/src/provider.rs` presets, `ai.toml`).
- **Not done:** nothing was run (no build, no app, no tests). `docs/manual/images/local-ai/` does not exist yet, so the pictures were not seen. Trademark registers were not searched. The field names Ollama-style apps use for models that run on their maker's servers were not checked against any API documentation (finding 1b). Only six of the Spanish strings were read.

## Verdict: MERGE AFTER FIXES

The design is sound: loopback only (checked, redirects off, IP literals, no credentials sent), user-started, no model named, ranked, bundled or downloaded, results stored locally as numbers, no new dependency, nothing third-party loaded. Three wordings must change before merge (a privacy promise RapidR cannot keep, what "Check My Models" really does, and what "never sent anywhere" means for developers); the rest is small.

| # | Item | Severity | Action |
|---|---|---|---|
| 1 | "Nothing you ask leaves this computer" is a promise the local app, not RapidR, controls | **must fix** | Reword 5 places (+ Spanish); mark models the app runs elsewhere (1b) |
| 2 | "tries each model that fits" is false: models that don't fit now, and size-unknown ones, are loaded too | **must fix** | Bulk check only `Fits`; say what the check does to the computer |
| 3 | RSystemInfo: "never sent anywhere" reads as covering the programs; no warning for developers | **must fix** | Reword 4 places; add a short "if your program sends these facts" paragraph |
| 4 | Licence display: "some allow anything", "the programs you write are yours", no "not checked" | should fix | 6 wording changes; one LEGAL.md bullet |
| 5 | "Best match" lines claim capability ("builds apps"), "for sure", "The one to choose" | should fix | 7 wording changes |
| 6 | Unload call and "let go after": inaccurate for some apps, can drop a model another app loaded | should fix | Re-read `/api/ps` first; reword |
| 7 | Web: the typed address is sent before `local_url` checks it | should fix | Check first |
| 8 | Stale page `ai-assistant.md` (old wizard, product names as "what to install") and old presets | should fix | Rewrite step 1; decision on names |
| 9 | Manual gaps: shared computers, "allow requests" advice, where checks are kept | should fix | 3 sentences |
| 10 | Product names: new feature clean; trademark lines missing in `LEGAL.md` | should fix | One bullet |
| 11 | Recipes: RapidR's own, but no licence statement for code copied into users' projects | should fix | One sentence (decision for Robert) |
| 12 | Probing, CFAA, security tools | note | Adequate |
| 13 | Six traps; dependencies; screenshots | note | Clean (details below) |

## 1. Privacy promise (must fix)

RapidR only talks to `127.0.0.1`/`[::1]`. What the local app does next is not RapidR's to promise: an app can be a proxy or gateway, an SSH tunnel to another machine's GPU can sit on `127.0.0.1:11434`, and some local apps list models that run on their maker's servers. The strings below say flatly that nothing leaves the computer; students and teachers will rely on that. A false "nothing leaves" is the kind of claim consumer and children's-privacy regulators read as deceptive.

**1a. Wording** (each also in `ide/lang/es.po`, tagged `# legal`, accepted by a person):
- `ide/aisetup.inc:131` (`AISuLocalText`), from "Nothing you ask leaves this computer, and it costs nothing. A model runs here in a local AI app — a free program you install that keeps AI models on your computer. Studio finds the models it already has and checks which ones fit this computer. Answers are slower and simpler than a cloud model's." to:
  > Studio sends what you ask only to a local AI app on this computer, and it costs nothing. A model runs here, in that app — a free program you install that keeps AI models on your computer — so what you ask stays here, unless the app is set up to pass it on to a service elsewhere (check its settings). When you open this page or press Look Again, Studio asks only this computer which models it has, and checks which ones fit. Answers are slower and simpler than a cloud model's.
- `ide/aisetup.inc:638` (`AISuNone`), "…and answers RAI here, so nothing you ask is sent anywhere." to "…and answers RAI here, so what you ask stays on this computer, unless the app is set up to pass it on."
- `ide/aisetup.inc:794` (the check's note, both plural forms), see item 2 for the whole sentence; it no longer says "nothing leaves this computer" and ends "Studio only talks to this computer."
- `docs/manual/local-ai.md:3`, "…and nothing you ask leaves the computer." to "…and what you ask stays on the computer, as long as the app itself isn't set up to pass it on to a service elsewhere."
- `docs/manual/local-ai.md:32`, "Nothing about your computer or your models is sent anywhere." to "RapidR Studio sends nothing about your computer or your models anywhere: it only talks to this computer. What a local AI app does with what it is asked is up to that app: if it is set up to use a service elsewhere, it sends your questions there."
- `docs/manual/ai-assistant.md:14` ("sends nothing out"), `:36` and `:587` ("nothing leaves it"): same sentence. Search the Help text for `leaves`, `sends nothing`, `sent anywhere` in anything about a local app.

**1b. Code (should fix, same pass).** In `server_models` (`local_ai.rs` ~line 499) read the description's remote-host fields (Ollama-style cloud models report a remote host/model in `/api/tags` and `/api/show`; confirm the exact names in that API's documentation, they were not verified here) and set `Model.remote`. A remote model: chip "Runs elsewhere", line "Its local AI app runs this model on a service elsewhere: what you ask is sent there.", never `best`, never in the check's queue, and Finish asks once: "{model} runs on a service elsewhere, not on this computer: what you ask RAI is sent there. Use it anyway?"

## 2. Check My Models (must fix)

`check(-1)` queues every row that is `usable()` (chats and not `TooBig`, `local_ai.rs:318,761`), which includes "Fits if you close other apps" and "Size not known". The UI (`aisetup.inc:794`), the manual (`local-ai.md:18,53`) and the plan say "each model that fits". Loading a model that doesn't fit the free memory can freeze or crash the computer (unsaved work), and a load can make the app unload a model another program is using. The user's consent (pressing the button) is only as good as that sentence.

- **Code:** queue only `Verdict::Fits` rows for the bulk check, in `check`, `state("estimate")` and `state("unchecked")` (`local_ai.rs:761,1161,1162`). "Fits if you close other apps" rows wait: close apps, Look Again, Check. Keep size-unknown rows in the bulk check (without them a name-only server is never ranked), and say so.
- `ide/aisetup.inc:794`, to (plural form likewise):
  > Optional, about {n} minutes: Studio loads each model that fits here, one at a time, and tries it — how fast it answers, whether it can use RAI's tools, and a tiny app — then lets it go (a model whose size isn't known is tried too). It uses this computer's memory and processor meanwhile, and may close a model another app has loaded. You can stop at any time. Studio only talks to this computer.
- `docs/manual/local-ai.md:18,53`: "tries each model that fits" to "tries each model that fits now, and each whose size isn't known, one at a time". Add after the table of lines: "A model that says *Fits if you close other apps* is not checked until it fits: close other apps, Look Again, then Check My Models."
- What the check sends is fine: RapidR's own instructions and three fixed prompts (`local_ai.rs:61,922,934,940`), no project (`projectfolder` and `projectfiles` empty, `:976`), no key (no `Authorization` header), to the user's own model; nothing it writes is run or saved. The checked model is never offered the running program's eyes and hands (`any_running()` is thread-local and false on the check's thread, `session.rs:901`); keep it so, with a test.

## 3. RSystemInfo: privacy for the programs people build (must fix)

Desktop: OS and version, CPU name, cores, memory total and free, GPU, disk size and free, battery (`sysinfo.rs`), read only when a program asks. Together these pick one computer out of thousands (a fingerprint). The registry text says "read on this computer and never sent anywhere", true of RapidR but easy to read as true of the user's program. Sending these facts to a server can be personal-data processing under the GDPR; reading device information for tracking can need consent under the ePrivacy rules (Art. 5(3)); in a program for children, device characteristics combined into an identifier are what COPPA's "persistent identifier" rule is about.

The web side is right (`web_facts`, `object_web.rs:3081`): only `hardwareConcurrency`, `deviceMemory` (rounded, at most 8 GB) and the platform, read only when a program uses RSystemInfo (nothing at page load), no GPU renderer string, no battery API. Studio's own web build reads nothing (`facts_now`, wasm branch). Keep the comment that explains why.

- `crates/rapidr-lang/data/components/objects.toml:252` (then regenerate `members.md:3365`), "…read on this computer and never sent anywhere." to "…read on this computer. RapidR never sends them anywhere. If your program does (to a server, in a log, in a report), tell its users first and send only what you need (`Class`, not every fact): together they can single out one computer, and a program for children shouldn't send them at all."
- `docs/manual/local-ai.md:112`, "— read on this computer, never sent anywhere." to "— read on this computer; RapidR never sends them anywhere." Add under the example:
  > **If your program sends these facts anywhere** (your server, a log, a crash report), they can be personal data: together they pick out one computer, and in the EU reading them to track people needs consent. Tell your users, ask first where the law says so, send only what you need (`Class` is usually enough), and never send them from a program for children. On the web the browser gives less on purpose (no graphics name, no battery, memory rounded to 8 GB at most): don't try to get round that.
- `examples/system_info.rr:4` and the `examples/README.md` row: "never sent anywhere" to "never sent anywhere by RapidR; if your program sends them, tell its users".

## 4. Licence display (should fix)

Reading `general.license` / the licence text from the user's own local app and showing it at the user's request, in memory only (not in `local-ai-checks.json`, not in the repo), is not a copyright problem; the screenshots use made-up licences ("Mock Huge Licence"). The risks are accuracy and implied vouching. The name comes from the model *file's* metadata or the text's first line, which a converter or uploader may have set wrongly; the text is cut at 200,000 characters (`local_ai.rs:563`) without saying so.

- `aisetup.inc:741`, to "Licence, as its local AI app reports it (Studio doesn't check it; it can be wrong or out of date): {licence}."
- `aisetup.inc:743`, to "Its local AI app doesn't say its licence: check it where you got the model." (the manual line 88 already promises this; the UI did not.)
- `aisetup.inc:745` and `:852` (and the three Spanish strings), "Building apps to sell? Check this model's licence." to "Building apps to sell? Read this model's licence first: it may limit commercial use, or what the model writes."
- `local_ai.rs:563`: when the text is cut, append "\n\n[The licence text is longer: Studio shows its first 200,000 characters.]".
- `local-ai.md:84`, "some allow anything, some limit commercial use or what the model may be used for" to "some are very open, others limit commercial use, what the model may be used for, or what you may do with what it writes, and some ask you to credit it". ("Allow anything" is never true.)
- `local-ai.md:88`, "…RapidR's own licence (MIT) says nothing about a model's: the programs you write are yours, but what a model may be used for is up to its licence." to "RapidR doesn't download or distribute any model, doesn't check what a licence says, and its own licence (MIT) says nothing about a model's or about what a model writes. Whether you may sell programs a model helped write, and who owns them, depends on that model's licence and on your country's law; ask the model's makers if you are unsure."
- `LEGAL.md`, under "What stays your responsibility", add: "**AI models and what they write.** RapidR doesn't provide, bundle or download any AI model. A model's licence, or an AI service's terms, apply to what it writes; whether AI-written code can be protected by copyright, and who owns it, depends on your country. RapidR makes no promise about either, nor about the accuracy of a model's licence as a local AI app reports it."

## 5. "Best match" and the other claims (should fix)

Ranking only models the user already has, with objective tests, is not an endorsement or a warranty of any model; the lines are scoped ("here", "for this computer"). Three things overreach: present-tense capability ("builds apps") after one tiny "Hello" window, "for sure", and "The one to choose". Replace:
- `aisetup.inc:585`, "Best match for this computer: builds apps, answers in about {s} s." to "Best match for this computer: it built Studio's test app, answers in about {s} s."; `:587`, "Builds apps, answers in about {s} s." to "It built Studio's test app, answers in about {s} s."
- `aisetup.inc:595`, "Can chat, but can't build apps with RAI." (unchecked, from the app's own flag) to "Its local AI app says it takes no tools, so RAI can't build apps with it."
- `aisetup.inc:599`, "…Check My Models to be sure." to "…Check My Models to see how it does."
- `local-ai.md:69` ("The one to choose.") to "The best of your models for this computer, as Studio's check sees it. You choose."; add to the section's start: "This says how a model did on Studio's one small test on this computer today. It is not a verdict on the model: another computer or another task can differ."
- `local-ai.md:99`, "this page's chips tell you for sure once it is there" to "this page's chips estimate it once it is there"; table row *Fits* (`:44`) to "By Studio's estimate, it fits in the memory free now, comfortably."
- **Rule for posts, videos and docs:** screenshots and examples use made-up names (the scene does: `AI_MOCK_LOCAL_NAMES`); never publish a ranking or a "model X can't build apps" line naming a real model unless it is dated, reproducible and states the method (same rule as review 2026-10-22 item 8d).

## 6. The unload call (should fix)

`local_ai.rs:820` sends `POST /api/generate {"model": …, "keep_alive": 0}` for every model with `rich && !loaded`, where `loaded` was read when the list was made, not when the check starts: a model another app (or person) loaded in between is dropped. Only apps that answer that API are asked, so "A model the check loaded is let go after it" (`local-ai.md:59`) is untrue for the rest. Fix: re-read `/api/ps` at the start of each model's check and set `loaded`; reword to "When its app lets Studio ask, a model the check loaded is let go after it; some apps keep it until their own timer." The call is harmless on a server that doesn't know it (not sent unless `/api/tags` answered).

## 7. Web: validate before sending (should fix)

`AISuAddrLook`'s web branch (`aisetup.inc:810-822`) sets `BaseURL` and calls `RefreshModels` with whatever was typed; `local_url` refuses a non-local address only afterwards, in `LocalAIAddModels`. Today the page's CSP (`connect-src … http://localhost:* http://127.0.0.1:*`, `ide/web/index.html:22`) stops the request, but the CSP can be changed (`--csp`) and the manual says "refused before anything is sent". Check the address with `local_url` first (a `LocalAI("islocal")` state, or the existing refusal path) and show "Studio only asks this computer: …". Also `[::1]` is accepted by `local_url` but not by that CSP; say "127.0.0.1 or localhost" in the web paragraph. Note: from an https page, current Chrome shows its own "local network" permission prompt for `http://127.0.0.1`; that is a second consent and needs no change.

## 8. Stale and product-naming text (should fix)

`docs/manual/ai-assistant.md` still describes the Preview 4 wizard: lines 35-39 ("Ollama, LM Studio or MLX Serve (free programs you install)", "✓ Found on this computer: Ollama (3 models)"), the caption at :45 and `images/ai/setup.png` (it names Ollama, LM Studio, MLX Serve), lines 84-86, 646, 821-825. This now contradicts the page and the plan ("no product pushed", the test at `studio_flows.mjs:5375` forbids those names on the page). Replace step 1's first bullet with:
> **Free, on this computer.** A model runs on your own computer, in a local AI app (a free program you install that keeps AI models on your computer). It costs nothing, needs a fast computer with plenty of memory, and answers are slower and simpler than a cloud model's. Studio asks this computer which models it already has and says what it found — *✓ Found 3 models in a local AI app on this computer* — or that none is running. It never starts one for you. [AI on this computer](../manual/local-ai.md) has the rest.

Regenerate `setup.png` with the made-up names. For the reference text that must name a server to give its default address (`BaseURL`, `COMPATIBLE_PRESETS`, `ai.toml:56,58`, `wire.rs:598`) see "Decisions".

## 9. Manual sentences (should fix, `local-ai.md`)
- **Shared computers** (under "What Studio looks for, and where"): "On a computer shared with other people (a school lab, a remote desktop), the app that answers may belong to someone else, and Check My Models would use its memory and processor. Use Check My Models only on an app that is yours."
- **Web, allowing requests** (`:104`), after "…allowed origins or CORS setting)": "Allow only that address, never every site: an app that answers any page lets any website you visit use your models."
- **Where results are kept:** "The results are in `local-ai-checks.json` in Studio's own folder (the folder `ai-audit.jsonl` is in: see [the assistant's page](../manual/ai-assistant.md)). It holds timings and yes/no results, plus the model's digest, or its name and address when it has no digest. Nothing in it is sent anywhere; delete the file to forget it."

## 10. Names and trademarks (should fix)
- **New feature: clean.** No product or model name in code comments, UI, the manual page or the scene; API paths (`/api/tags`, `/api/show`, `/api/ps`, `/api/generate`, `/api/v0/models`) are functional interfaces, not names (same reasoning as `review-2026-10-29-mcp.md` §1). "OpenAI-compatible API" is descriptive use of a mark to say which interface an app offers; keep it out of titles, chips and any product name. Add a test that scans `docs/manual/local-ai.md` and the `TR$` strings of `aisetup.inc` for product names (the existing one checks only `aisulocaltext`).
- `LEGAL.md` › Trademarks, add: "OpenAI is a trademark of OpenAI, Inc.; Anthropic and Claude are trademarks of Anthropic, PBC; DeepSeek, Ollama, LM Studio, xAI, OpenRouter and Llama are names or marks of their owners. RapidR names them only to say which providers and servers its AI component can talk to, and which interface a local AI app offers ("OpenAI-compatible"); it is independent of all of them."

## 11. Recipes and the fallback (should fix, one sentence)
`examples/recipes/*.rr` are RapidR's own (written for RapidR; their headers cite no source and name no other product; each is click-tested; their provenance was not otherwise audited here; the system recipes' "tiny machine" uses instruction sets, which are facts). The fallback (`ai.rs:fall_back`) only creates a proposal the user reviews, no network, audit line `recipe-fallback`. The gap: a recipe copied unchanged into a user's project is MIT text with no notice, while `LEGAL.md` promises no notice is needed for the user's programs. Review 2026-10-22 item 4d decided MIT-0 for generated templates; do the same here. In `LEGAL.md` › "In short": "Code RapidR gives you to start from — the recipes RAI starts new programs from (`examples/recipes`) — is released under MIT-0: use it, change it and sell it, with no notice." Put an SPDX line `' SPDX-License-Identifier: MIT-0` in each recipe header (`Recipe::code` already strips only the search lines). Needs Robert's OK (it widens the licence of those files only).

## 12. Localhost probing (note: adequate)
- **What is asked:** TCP connect (400 ms) to 8 fixed loopback ports, in parallel, then one `GET /v1/models`; only a port that answers that with a model list gets `/api/tags`, `/api/ps`, `/api/v0/models` and one `POST /api/show` per model. No credentials, no cookies, redirects off (`http.rs:198`), no body from a non-AI service is kept. A non-AI service on 8080, 8000, 5001 or 8081 sees one GET it doesn't understand; at worst a line in its log.
- **Loopback enforcement:** `local_url` + `loopback_url` accept only `127.0.0.1`, `[::1]`, `localhost` (rewritten to `127.0.0.1`: no DNS), refuse `@` in the address, bad ports, other schemes; tested with `localhost.evil.com`, `user@127.0.0.1`, `10.0.0.1`. `ask()` checks again before every request.
- **Consent:** started only when the user opens the page, presses Look Again or Look; not at start-up, not in the background; School Lock turns it off. Page 1 now says so (item 1a).
- **CFAA and similar laws:** connecting to a port the user's own computer exposes, from a program the user runs, with no authentication tried or bypassed, is not unauthorised access. A scan of 8 loopback ports does not leave the machine. A security tool may flag the burst; the manual lists the ports. The shared-computer case is item 9.
- Not a legal matter, for the next security audit: `net::ask` keeps the whole response body with no size cap (a misbehaving local server can fill memory).

## 13. The six traps, and dependencies, for this feature
| Trap | Result |
|---|---|
| 1 Children | No accounts, no personal data collected by RapidR. Risk is in programs built with RSystemInfo: item 3. |
| 2 Third-party assets | None loaded (system facts only; web CSP unchanged). Not applicable. |
| 3 Analytics and replay | None. Local files only (`local-ai-checks.json`). When the planned crash reports land (`crash-reports-plan`), they must show the user any hardware facts they include before sending. |
| 4 Marketing email | Not applicable. |
| 5 Subscriptions | Not applicable. |
| 6 Uploads (DMCA) | Not applicable: no hosting or sharing. |

No new crates: `libc` and `windows-sys` (MIT OR Apache-2.0) were already in the tree (`Cargo.lock`, `THIRD_PARTY_NOTICES.md` unchanged). `pmset` (macOS battery) is the system's own tool. The new icon is RapidR's own drawing (`design/icons/tools/components.py`). The manual pictures must be made from the scene before merge (the folder is empty today): made-up names, a made-up 16 GB computer; `system-info.png` shows the maker's real machine, so check it shows no user name, host name or home path (the example prints none).

## Required before merge
- [ ] 1a/1b: the five privacy sentences (+ Spanish, a person accepts them); remote-model marking.
- [ ] 2: bulk check = `Fits` only (+ size-unknown), the new note, manual sentences.
- [ ] 3: RSystemInfo wording in four places and the developer paragraph.
- [ ] 4 and 5: the licence and result-line wordings; `LEGAL.md` AI-models bullet.
- [ ] 6, 7: re-read `/api/ps` before a check; validate the web address first.
- [ ] 8, 9, 10: `ai-assistant.md` step 1 and picture; the three manual sentences; `LEGAL.md` trademark bullet; name-scan test.
- [ ] 11: recipe licence sentence (after Robert's OK).
- [ ] The local-ai pictures made and looked at, no real model names or user data in them.

## Decisions for Robert
1. **Product names in the older reference text** (`COMPATIBLE_PRESETS` "Ollama (this computer)" / "LM Studio (this computer)", the provider's display name "OpenAI-compatible (Ollama, LM Studio, xAI …)", `ai.toml` and `members.md` for `BaseURL`, the example `Model = "llama3.2"`, `wire.rs:598`'s error text, `examples/README.md:194`). Recommended (zero risk, small cost): keep the names only where a user needs a default port, as plain text with the `LEGAL.md` line (item 10); change the example to read the first model from `AI.Models` instead of naming one; label the presets "This computer, port 11434" and "This computer, port 1234". Stricter: remove them everywhere.
2. **MIT-0 for the recipes** (item 11): yes, recommended.
3. **Remote models** (1b): refuse to list them, or list them marked "Runs elsewhere". Recommended: list and mark.

## Sources (read 2026-10-30)
The files named above in this worktree; `docs/legal/review-2026-10-22.md` (4d, 8d), `review-2026-10-29-mcp.md` (§1), `review-2026-10-29-app-eyes.md` (§5 R1: what RAI sends), `review-2026-10-29-rarchive.md` (format), `six-traps-preview4.md`; `LEGAL.md`. External rules cited from general knowledge, not re-read today: GDPR, ePrivacy Art. 5(3), COPPA (persistent identifiers), CFAA, FTC deceptive-claims practice.
