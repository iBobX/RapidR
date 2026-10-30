# Legal review, 2026-10-29: APP-EYES-HANDS (Studio's `app_*` tools, `RAppCapture`, `RAppDriver`)

This is the "lawyer agent" review (standing rule since 2026-10-22) for a new kind of power (AI apps and RAI seeing and driving the user's running program), two new public component names, and the conditions the MCP review's addendum set for `app_screenshot` and `app_click` (`docs/legal/review-2026-10-29-mcp.md`, "Coming soon"). It is a careful risk review, **not legal advice** (US/EU practice; Uruguayan law not checked).

- **Reviewed:** worktree `agent-ac77aa272fbb3b887` (branch `worktree-agent-ac77aa272fbb3b887`, merge commit `e6e2e61`): `docs/manual/app-eyes-and-hands.md`, the registry entries `RAPPCAPTURE` and `RAPPDRIVER` (`crates/rapidr-lang/data/components/ai.toml:142-230`), `ide/mcpapp.inc`, `tools/patches/app-eyes-mcp-inc.patch`, RAI's side (`crates/rapidr-studio/src/ai_tools.rs`, `ai.rs`, `session.rs`, `crates/rapidr-ai/src/rai.rs`), the runtime (`crates/rapidr-ui-app/src/{eyes,hands,link}.rs`), the host wiring (`interpreter/*/session.rs`, `lib.rs`) and `tests/app_eyes.mjs`.
- **Name searches:** web searches on 2026-10-29 through a summarising search tool. **Not searched:** USPTO TSDR itself, TMview, EUIPO, WIPO (a Justia page returned 403). The two decisive findings (APPDRIVER, APPEYES) come from third-party trademark mirrors and should be confirmed once on TSDR by a person before a public announcement.
- **Dependencies:** no new third-party crate (`serde_json`, `rapidr-ui-render`, `rapidr-lang` were already in the workspace); nothing copied from outside; icons are our own SVGs (`design/icons/src/components/app*.svg`). Nothing to add to `THIRD_PARTY_NOTICES.md`.
- **Six traps (CLAUDE.md):** no accounts, sign-up, analytics, email, payments or uploads are added, and nothing loads from a third-party server. Traps 1, 2, 4, 5, 6: not touched. Trap 3 (session replay / recording): nothing is recorded or sent by RapidR; the pictures exist only in memory, in a mode-0600 temp file until handed over, or in a `data:` URL. The manual says "Recording a video comes later": that needs its own review (CIPA / GDPR, royalty-free codecs only).

## Verdict: MERGE AFTER FIXES

The design is sound and the central promise is **true in the code**: the pictures are the kernel's own drawing of the program's forms (`crates/rapidr-ui-app/src/eyes.rs:1-14`: "never a read of the screen"), the input goes through the kernel's own routing (`hands.rs:1-10`), the window name is looked up in the program's own windows and an unknown one is an error listing them (`eyes.rs` `find_window`), and the requests reach the program only through the session protocol of a program Studio started (`interpreter/rapidr-vm-host-native/src/session.rs`, `run_session`). So the system's screen, other apps, system dialogs and Studio itself cannot be reached through these tools, and no Screen Recording or Accessibility permission is involved.

But **the Studio side does not yet meet the addendum**: looking is allowed in every mode (including Read only) without asking, the "Allow Once" is wider than the addendum's "each use", there is no consent re-ask, the audit log can receive text the program showed, typed text is shown in Studio's pane, announcements and ask question, and the wiring exists only as a stale patch against a version of `mcp.inc` that no longer exists on `development`. The names also need to change. All fixes are small.

## Verdict table

| # | Item | Risk | Decision |
|---|---|---|---|
| 1a | Name `RAppDriver` | **Medium** | **Rename to `RAppHands`.** US word mark APPDRIVER (Reg. 4868927, Adways Inc., class 9 software) is live; "AppDriver" is also a product word in this exact field (Silk AppDriver, WinAppDriver, the Appium family). |
| 1b | Name `RAppCapture` | Low | **Rename to `RWindowCapture`** (zero-risk policy; the rename is cheap before release). "AppCapture" is Microsoft's `Windows.Media.Capture.AppCapture` API class and a small app-store screenshot tool (appcapture.io); no mark found. |
| 1c | Names rejected | n/a | `RAppEyes` (APPEYES, Reg. 4961148, class 9 software: taken). `RAppRobot` (no hits, but not needed). |
| 2 | Addendum conditions | see §2 | 1 met; 2, 3, 4 (partly), 5 not met; fixes below. |
| 3 | Consent and Settings strings | Required | Strings in §3. |
| 4 | "Use it responsibly" box for developers | Required | Text in §4. |
| 5 | RAI side | Required before release | RAI asks once per run; RAI's setup text names what a running program shows; audit digest of typed text. §5. |
| 6 | Patch not applied | Blocks merge | Apply by hand to `development`'s `mcp.inc`, re-review the result. §6. |

## 1. Names

### 1a. `RAppDriver` -> **`RAppHands`**

Findings:
- **APPDRIVER, US Reg. No. 4868927** (serial 79167165, a Madrid designation, filed 2014-09-30, registered 2015-12-15): word mark, class 9, "computer programs for tracking websites of others to provide details about user click traffic or visits", plus advertising-distribution software; owner listed as **Adways Inc.** Whether it is still maintained (the 6-year declaration was due in 2021) was not found. Our `RAppDriver` contains the whole word, in the same class (software). The goods differ (click-traffic tracking vs. input automation), which keeps the risk below "likely confusion", but a registered identical word in class 9 is exactly what the zero-risk rule says to avoid when a free alternative exists.
- **Silk AppDriver** (Micro Focus; an implementation of the WebDriver protocol for native and mobile apps, Apache-2.0 repo) and **WinAppDriver** (Microsoft; "Windows Application Driver", a Selenium/Appium-style UI test service) use "AppDriver" for the *same job*. `RAppDriver` would look like a member of that family, which is both an association risk and a search-results risk. Also **Appium** is a registered OpenJS Foundation mark: never write "Appium-compatible" or similar anywhere (the lane's docs do not: checked).
- The component's `display` names (`AppDriver`, `AppCapture`, `ai.toml`) are what the designer and Help show, so they must change too.
- `RAppHands`: no software product, company or mark found in two searches ("AppHands", "App Hands"); it matches the manual's own wording ("eyes and hands"), and it is a plain English compound.

**Fallback if `RAppHands` is ever challenged:** `RAppRobot` (no hits found; "robot" is generic in this field).

### 1b. `RAppCapture` -> **`RWindowCapture`**

- `AppCapture` is the name of a Microsoft Windows API class (`Windows.Media.Capture.AppCapture`, Game Bar) and of a small product at appcapture.io (listing screenshots for app stores). No trademark filing was found for either. Risk is low, but "capture" + "App" is thin and a user searching "AppCapture" lands on those. `WindowCapture` is a plain descriptive term (Qt has a `WindowCapture` QML type; a 2012 utility used the phrase): nobody can own it, which makes `RWindowCapture` the safest choice, and it says what the component does (it captures *windows of the program*, never the screen).
- Keep the `R` prefix convention.

### What to rename (one pass, before merge)
`RAPPCAPTURE` -> `RWINDOWCAPTURE`, `display = "WindowCapture"`; `RAPPDRIVER` -> `RAPPHANDS`, `display = "AppHands"` (`ai.toml:142`, `:178`); `RAppCapture`/`RAppDriver` in `docs/manual/app-eyes-and-hands.md` (title, text, code), `docs/manual/reference/{components,members}.md`, `docs/manual/README.md`, the icons (`design/icons/src/components/appcapture*.svg` -> `windowcapture*.svg`, `appdriver*.svg` -> `apphands*.svg`; `design/icons/tools/components.py`, `docs/manual/icons/{light,dark}/components/*`, `design/icons/inventory.toml`), `crates/rapidr-value/src/component_defaults.rs`, `members.rs`, `crates/rapidr-ui-app/src/app_components.rs`, `script.rs`, the examples (`examples/gui/snapshot.rr`, `guided_tour.rr`, `examples/`), tests (`eyes_tests.rs`, `testhooks.rs`). Old names as silent aliases are **not** wanted (nothing shipped yet; one implementation, no fallbacks). Before the first public mention, run TSDR for `APPHANDS` and `WINDOWCAPTURE` in class 9 and 42.

The tool names `app_layout`, `app_screenshot`, `app_click`, `app_type`, `app_key`, `app_drag`, `app_scroll`, `app_wait` are plain words (no mark): keep.

## 2. The addendum's conditions, checked against the code

Addendum conditions for `app_screenshot` / `app_click` (`review-2026-10-29-mcp.md`, "Coming soon"):

| Cond. | Required | Found | Met? |
|---|---|---|---|
| 1 | Only the program's own windows that Studio started; never the desktop, other apps, a password or system dialog, or Studio itself | Kernel re-render of the program's `Desktop` (`eyes.rs:1-14`, `link.rs` `screenshot`: `find_window`/`capture_all` over `desk.forms`); input via `Desktop::mouse_down...` (`hands.rs:1-10`); the request path is `Session.App` -> `Command::App` -> the child's stdin (`session.rs` `"app"`, `run_session`), so only a program Studio started answers; a password box shows dots in the report (`eyes.rs:221-229`). `capture` of Studio's own window is the separate tool and its condition B2 is still open (see §6). | **Yes** |
| 2 | "Run" kind: never in Read only; under Ask before changes each use (or Allow for This Session) waits for the user; Allow edits does not allow it | `app_layout`, `app_screenshot`, `app_wait` are registered `"read"` (`ide/mcpapp.inc:51,52,58`) and `McpKindOf` maps only the five input tools to a new kind `drive` (patch, `McpKindOf`), so **looking is allowed in every mode, with no question, even in Read only** (`tests/app_eyes.mjs:307-310` asserts exactly that). Input is refused in Read only and asked in Ask / Edits (patch `McpDecision`), but **Allow Once covers the rest of the run** (`McpAppRunAllowed = RunStarted`, patch `McpDecide`; `tests/app_eyes.mjs` header: "the first app_click asks (Allow Once), the next in the same run doesn't"), not "each use". | **No** (looks); **partly** (input) |
| 3 | A new row or an extended row, and **one re-ask**: a consent version, so everyone who confirmed before sees the dialog once more | `McpConfirmed` is still a single flag (`ide/mcp.inc:137,515,525`, `McpSetOnClick`, `McpSettingsTake`); no version exists. The patch only edits `McpSeeText`, which `development` no longer has (the dialog now builds from `McpRowBody` rows, `mcp.inc:691-702`). | **No** |
| 4 | Audit log: the action only, **never the picture, the typed text or what the program showed**; canary test with text typed into the program | The call arguments are never written (`McpLogLine` -> `McpTools.AuditCall Client, Tool, Target, Outcome`; target is `""` for every `app_*` call; `crates/rapidr-studio/src/ai.rs:1130-1136`). The picture is never logged and its temp file is deleted after the reply (`mcpapp.inc:145-146`). **Gaps:** (a) on an error, `mcpapp.inc:150` passes `"error: " + LEFT$(Text, 80)` as the outcome, and `Text` is the program's own sentence, which quotes captions and lists the controls ("No button shows "Go" in form. Buttons now: "Add", "Clear"…"), so up to 80 characters of what the program shows are written to `ai-audit.jsonl`; (b) typed text goes into Studio's pane, status line, screen-reader announcement and the ask question (see below); (c) RAI's audit has a hash of the typed text (§5); (d) **no canary test**: `tests/app_eyes.mjs:74` sets `RAPIDR_AI_AUDIT` and never reads the file. | **Partly** |
| 5 | Pre-written row strings used | Not used (§3). | **No** |
| (manual) | The manual says plainly that a program under test may send real emails, make real purchases or delete real data: test with test data | `app-eyes-and-hands.md` has only "Be careful what you drive" in the `RAppDriver` part and nothing in the Studio section; `docs/manual/ai-apps.md` is untouched (it still says "A picture of the running program's own windows … isn't there yet", line ~179, and its tools table has no `app_*`). | **No** |

Where typed text leaks into Studio's own UI (not the audit): `ai_tools.rs:312` (`activity("app_type")` prints the first 40 characters) -> `McpTools.ToolActivity` -> `what` -> `McpLogLine` (the pane list; Copy copies it), `McpLastWhat` (status line), `McpWait`'s `Application.Announce` (read aloud) and `McpShowAsk` (`wants to use your running program: Typing "hunter2"…`). A password typed by an AI app into the program's password box would appear in clear in Studio, on a shared screen or a recording. Fix below.

### Fixes (all small)

**F1. Looking is "drive", too.** In the lane's `McpKindOf` hunk, map `app_layout` and `app_screenshot` to `drive` as well (only `app_wait` stays `"read"`), and register them with kind `"change"`-style hints, not `"read"` (`mcpapp.inc:51-52`; the MCP `readOnlyHint` would otherwise tell the AI app they are harmless). Consequences, all wanted: Read only refuses them (the existing Read-only refusal text is right: "…lets AI apps only look at this project…"; add "…and not at your running program"); Ask before changes and Allow edits ask; Allow everything doesn't. Flip the Read-only assertions in `tests/app_eyes.mjs:307-310` to "refused", and add "asks" for the look tools.

**F2. "Allow Once" per run: accepted, with conditions** (a deviation from the addendum's "each use", decided here). A per-use question for every picture and click would ask 20 or more times in one test loop, push people to "Allow for This Session" (which is wider), and the question itself already says what Allow Once covers. Accepted **only if** all hold: (i) the question names the scope in plain words (string below); (ii) the grant ends when the program's run ends and a new run asks again (it does: `McpAppRunAllowed = RunStarted`; test it); (iii) Deny for one `drive` call does not grant anything and the pane shows the denial (as for other kinds); (iv) "Allow for This Session" is cleared when the project changes or the server stops (`McpModeFollow` already empties `McpGrants`; test it for `drive`). The question (`McpShowAsk`, `CASE "drive"`), replace the patch's text with:

> `<AI app> wants to look at or use your running program: <what>. Allow Once covers pictures, clicks and typing until this run ends. It may see whatever the program shows, including private data.`

**F3. Consent version, one re-ask, and "Allow everything" reset.** Add a registry value `McpConsentVersion` (current value `2`), read and written beside `McpConfirmed` (`mcp.inc:515,525`).
- `McpSetOnClick`, `McpSettingsTake`, and `McpLoadSettings` (the line `IF McpOn AND McpConfirmed = 0 THEN McpOn = 0`) test `McpConfirmed AND McpConsentVersion >= 2`.
- At Studio start, when AI apps are on but the version is lower, show the consent dialog once (`McpConsent`, after the rows are updated). *Turn On*: version = 2, nothing else changes. *Cancel*: AI apps are turned off (`McpOn = 0`, `OutputLine "AI apps: off. Confirm again in Settings > AI Apps."`) and the server is not started. Until confirmed, `McpSyncTools` must not register `McpAppTools` (so an AI app cannot even see them).
- Projects stored with the mode `all` (Allow everything, `McpModes`) are set back to `ask` once at that upgrade: their "Allow everything" confirmation never said "click and type in your program".
- Test (`tests/studio_mcp.mjs` or `app_eyes.mjs`): registry with `McpConfirmed=1` and no version -> dialog once, tools absent before Turn On, present after, not asked again on the next start; a project in `all` is `ask` after the upgrade.

**F4. Audit: log the action only.**
1. `ide/mcpapp.inc:150`: log a fixed outcome: `IF Ok <> 0 THEN McpLogLine client, tool, what, "", "ok" ELSE McpLogLine client, tool, what, "", "error"`. (The sentence still goes to the AI app in the reply and nothing else needs it.)
2. `crates/rapidr-studio/src/ai_tools.rs:306-317` (`activity`): never put typed text in `what`. Replace the `app_type` arm with
   `"app_type" => format!("Typing {} characters{} in the running program", any("Text").unwrap_or_default().chars().count(), if on.is_empty() { String::new() } else { format!(" into{on}") })`
   and for `app_key` show the keys only when they are not text (it is `Enter`, `Ctrl+S`… by design, fine to keep).
3. A pane line for a look: `what` = `Looking at the running program` (already the activity text; keep).
4. **Canary test** in `tests/app_eyes.mjs` (all four modes it covers): type `CANARY-TYPED-7f3a` into the name box through `app_type`, cause one error whose sentence quotes a caption (`app_click` with `Caption: "CANARY-NOCAPTION"`; the sentence lists real captions, so also give the program a button captioned `CANARY-BUTTON`), take `app_screenshot` and `app_layout`; then read `RAPIDR_AI_AUDIT`'s file and the pane's text (`mcplog` item texts) and assert: none of the three canaries, no `iVBOR` (PNG base64) and no `data:image` appear; and `rapidr-app-*.png` is not left in the temp folder.

**F5. Temp pictures.** `session.rs:132-150` writes the PNG into the temp folder (mode 0600 on Unix) and `mcpapp.inc:146` deletes it after the reply. A crash or a killed Studio leaves it. At Studio start delete `rapidr-app-*.png` older than a minute from the temp folder (a loop with `DIR$`/`KILL`, dry-run first as per the disk-hygiene rule). On Windows the per-user temp folder is private to the user; no extra step.

**F6. Test that nothing outside Studio's run can use `app`.** Add to `tests/app_eyes.mjs` (or a Rust test): a built (`rapidr build`) program and an interpreted program started *without* `--session` ignore a line `{"seq":1,"type":"app","request":{"do":"screenshot"}}` on standard input (no reply, no picture). The code looks right (only `run_session` reads the protocol), but it is the base of condition 1 and it is cheap to prove.

## 3. Studio's consent text (privacy)

Why it matters: a picture or a layout report of the user's program shows whatever the program shows: the user's own data, or other people's (a teacher's grading program with students' names; a shop's customer list; a child's data in a school tool). Per-instance fines apply to disclosures of personal data, and the disclosure is to the AI app's maker (a third party, possibly outside the EU). RapidR sends nothing itself, but the dialog must say that the picture *is* given to the AI app, that it can hold other people's data, and that clicks are real.

These are the final rows for `McpRowBody` (`mcp.inc:691-698`), already **including** the MCP addendum's A, B1, C and E (the app-eyes parts are marked here in the review text only; the strings are what goes in the code).

Row 0, "It can see":
> `The files of your open project — your code, forms and data files — and nothing else on your computer. It also gets pictures of Studio's window and of your running program's windows, and a written list of what they show, as you see them.`

Row 1, "It can":
> `Check code, and read what your program prints and RAI's chat. It can also change forms and code, open other projects, run, debug, look at, click and type in your program, and ask RAI (which may cost money), each only as your permission mode allows; the default asks you first.`

Row 2, "It never sees": unchanged (`Your AI provider keys or passwords: your keychain keeps them.`). Note: this stays true for Studio's tools, but a password typed into the *running program's* own box by you is shown as dots, and an AI app that is allowed to type can type one: see row 3.

Row 3, "Keep in mind" (replace the whole string; the old text plus the addendum's sentence, shortened):
> `RapidR sends none of this anywhere, but the AI app may pass what it reads to the company that makes it, under that company's terms. A program can show private data, yours or other people's, and clicking in it acts as you: a real Send, Pay or Delete button does it for real, so use test data. Many AI apps are for adults only: if you are under 18, set this up with a parent or teacher. Connect only AI apps you trust.`

Footer (`McpFootText`), from the addendum's A:
> `Share only what you may share: schoolwork, an employer's code or someone else's licensed code may not be yours to share. Turn this off, or Reset Token, to disconnect every AI app. Studio keeps a list of what they did on this computer only; Clear Activity removes it.`

Allow everything (`McpConfirmAll`, `mcp.inc:528-536`):
- Lead: `Nothing waits for you: AI apps change files, run the program, look at it, click and type in it and ask RAI (which may cost money) on their own.`
- Row "Keep in mind": `A program they run can do anything you can on this computer, and what they click in it is real. Choose this only while you watch an AI app you trust.`
- Row "You still see": unchanged.
- Row "Never": `Studio's own tools never hand over your keys, or files outside the project.`

Mode notes (`McpModeNote`, `mcp.inc`):
- read: `AI apps can look — read the project, check code, look things up, see the program's output and Studio — but not change, run, look at or use your program.`
- edits: `Changes to files are made at once (each one Undo step, listed in the AI Apps pane); running the program, looking at or using it, and asking RAI still wait for you.`
- all: `Nothing waits for you: AI apps change files, run the program, look at and use it, and ask RAI on their own — and a program can do anything you can on this computer. Everything is still listed, Stop ends it, and Studio's own tools never hand over keys or files outside the project.`
- ask (default): `AI apps can look freely at the project; each change, run, look at your running program, or question to RAI waits for you in the AI Apps pane: Allow Once, Allow for This Session or Deny.`

Manual `docs/manual/ai-apps.md`: add to the tools table (kind column "run (looking at or using the running program)"): `app_layout`, `app_screenshot`, `app_click`, `app_type`, `app_key`, `app_drag`, `app_scroll` (and `app_wait`, look); extend "What AI apps can see" with the "It can see" / "It can" strings above; **delete** the note "A picture of the running program's own windows … isn't there yet"; in the activity log paragraph (line ~157-162) after "(never the file's text)" add "or a picture, the text an AI app typed, or anything your program showed"; apply the addendum's D and E edits that are still open. Link to `app-eyes-and-hands.md`.

## 4. Users' own apps: the "Use it responsibly" box

Put this box at the top of the `RWindowCapture` section and a shorter copy at the top of the `RAppHands` section of `docs/manual/app-eyes-and-hands.md`, and in the components' registry docs (one sentence). (Replaces "Be careful what you drive".)

> **Use it responsibly.**
> - **Your windows can show other people's private data.** A picture or a layout report holds all of it: names, addresses, grades, messages. Keep them on the user's own computer. Before one goes anywhere else (a server, an email, an AI service, an analytics tool) the person must know and agree, and you must only collect what you need. Tell people, in plain words, what is captured and where it goes.
> - **No hidden recording.** Never capture or record a person's windows without telling them before it starts and showing that it is on. Sending pictures or recordings of users' screens to a server is processing personal data: in the EU and UK it needs a legal basis (usually consent) and your privacy notice; some US states treat secretly recording what people do on a page or app as wiretapping, and fines are counted per person. Keep it off by default, mask password and personal fields, and do not do it in programs for children without a parent's or school's permission.
> - **It acts only inside your own program, and must not be used to get around anything.** It plays input into your program's own windows and nothing else: not the system, not other programs. Do not use it to click "I agree", "Pay" or "Delete" for someone who has not chosen to, to answer a security question, a password or a permission prompt, to imitate people (fake votes, likes, game bots), or to automate another service against its terms.
> - **What it clicks happens for real.** Tours, demos and self-tests can send real emails, charge real cards and delete real data. Run them on test data.

And in the "In RapidR Studio" section add:

> **What the AI sees.** A picture or the layout report shows whatever your program shows: your data, or other people's. With an AI app it goes to that app and, under its maker's terms, maybe to the company behind it; with RAI it goes to the AI provider you chose. RapidR sends nothing itself, keeps no picture (it is deleted as soon as it is handed over) and writes only that a picture was asked for into the activity list, never the picture, never what was typed. Use test data, and close what you don't want seen. A program under test may send real email, make real purchases or delete real data when it is clicked: test with test data.

Table in that section: change the Permission column to `app_layout` and `app_screenshot`: "counts as running it (asks like using it)"; `app_click …`: "counts as running it"; `app_wait`: always. Replace "For AI apps the permission menu decides: **Read only** lets them look but not use the program" with "**Read only** lets them neither look at nor use the program".

Also in the registry docs for `RWINDOWCAPTURE` and `RAPPHANDS`, add the sentence: `Pictures and clicks of a program show and do what its users' data and buttons are: tell users what is captured, and never record or automate without their knowledge.` (`ai.toml:142` and `:178`; keep `doc` lines under the length the registry check allows.)

## 5. RAI's side (Studio's own assistant)

RAI uses the user's chosen AI provider and is started by the user in its chat, so it is not "an outside AI app", but it sends the program's layout (control captions and the text in edit boxes) to that provider (`ai_tools.rs` `app_layout`; "a picture isn't one of RAI's"). Required:

- **R1 (before release).** RAI's setup and settings texts say what it sends: `ide/aisetup.inc:122, 500, 502, 504` currently say "your question, the chat and the parts of your project RAI reads". Replace by "your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show". (Line 506 likewise: `What you send — including, while your program runs, what its windows show — goes to it, under its privacy terms.`) Add one sentence to `docs/manual/ai-assistant.md` (privacy paragraph, line ~12): `While your program runs, RAI can read what its windows show and click and type in them; that goes to your provider like the rest of the chat.`
- **R2 (before release).** RAI uses the program with no question at all unless its permission is "read-only" (`ai_tools.rs:229-232` `app_call`; `ai.rs:177` `may_propose`). Text the *program* shows goes straight into RAI's context and can contain instructions ("click Delete all"). Give RAI the same rule as the AI apps: the first input tool call in a run of the program waits for the user (a chat card: `RAI wants to click and type in your running program. Allow for this run / Don't allow`), a Deny is answered to the model; looking at the layout follows the same card. Until then RAI's input tools must not be offered (remove them from `app_tools`'s `input` branch). The merge may land with R2 open only because nothing is released; it is a release blocker.
- **R3 (before merge).** RAI's audit line for a tool carries `digest(input)`, an FNV-1a hash of the whole JSON (`rai.rs:2537`, `:965`), which for `app_type` includes the typed text; a short secret (a PIN) is recoverable by brute force from a 64-bit non-cryptographic hash, and the comment at `rai.rs:967` says "nothing secret is digested". For `app_type` and `app_key` write `"digest": ""`. Also the `decision` is `"proposed"` for every non-read risk (`rai.rs:2537`), but a `ui` action *ran*: write `"decision": "ran in your program"` when `out.risk == "ui"` (the audit must say what happened).
- RAI's chat history and the transcript tool keep the layout text the model received (that is the model's context); the Clear history control already exists, and the privacy sentence in R1 covers it.

## 6. Anything else that blocks merge

1. **The Studio wiring is not in the branch.** `tools/patches/app-eyes-mcp-inc.patch` was written against a version of `ide/mcp.inc` from before the DESIGN-GUIDE merge; `development` now has `ide/consent.inc`, `McpRowBody`, `McpFootText`, `McpConsent` and no `McpSeeText`, so hunks 8 and 9 (the `McpSeeText` sentence) cannot apply and the consent change (F3) is not in it at all. The lead must apply the hunks by hand on the merged tree, add F1-F4 and §3, then **re-review the applied `ide/mcp.inc`** (this review is of the design and of `mcpapp.inc`; the mode logic that decides `ask` / `refuse` is in the not-yet-applied hunks, `McpKindOf`, `McpDecision`, `McpDecide`, `McpShowAsk`). Delete the patch file after (its own instructions say so).
2. **Mode-logic test.** `tests/app_eyes.mjs` must cover, per mode: Read only (all seven `app_*` that touch the program refuse, `app_wait` works), Ask (first look asks, Deny answers an error and grants nothing, Allow Once covers pictures/clicks/typing until the run ends, a second run asks again), Edits (asks), Allow everything (no question), "Allow for This Session" cleared by a project change.
3. **Condition B2 of the MCP addendum is still open** and is not this lane's: `capture` of Studio's own window must refuse while Settings, a dialog or a visible token / key is on screen. The `app_*` tools are not `capture` and do not need it, but the two must not be confused in the texts (row 0 says "pictures of Studio's window and of your running program's windows").
4. **Debugger-paused behaviour** (a look works, input refused with where it is paused) is fine: pausing does not widen anything. Nothing to change.
5. **Web Studio.** Only RAI reaches the program there (an AI app cannot listen); the same R1-R3 apply; pictures are `data:` URLs in the page and are not stored. No third-party asset is loaded by the lane (trap 2 clean).
6. **Docs naming rule.** The lane's docs name no other product (checked: no Appium, Selenium, WinAppDriver, etc.). Keep it so when renaming; do not add "like Appium" lines anywhere.

## Required before merge (checklist)

- [ ] Rename `RAppDriver` -> `RAppHands` and `RAppCapture` -> `RWindowCapture` everywhere (registry id and display, docs, examples, icons, tests); TSDR check for `APPHANDS` and `WINDOWCAPTURE` before the first public mention (§1).
- [ ] `app_layout` and `app_screenshot` are `drive` kind: refused in Read only, asked in Ask and Edits, free in Allow everything; `app_wait` free; MCP annotations not `readOnlyHint` for the looking tools (F1).
- [ ] The ask question for `drive` uses the string in F2; the grant ends with the run; tested (F2, §6.2).
- [ ] `McpConsentVersion` = 2; one re-ask for everyone who confirmed before; tools hidden until confirmed; `all` projects reset to `ask` once (F3).
- [ ] `mcpapp.inc:150` logs the fixed outcome `error`; typed text is never put in `what` (`ai_tools.rs:312`); canary test reads `ai-audit.jsonl` and the pane (F4).
- [ ] Rows 0, 1, 3, the footer, Allow everything and the mode notes use the strings in §3; `docs/manual/ai-apps.md` updated (§3).
- [ ] "Use it responsibly" box and "What the AI sees" paragraph in `app-eyes-and-hands.md`; one sentence in the two registry docs (§4).
- [ ] RAI: `digest` empty and honest `decision` for `app_type`/`app_key`/`ui` risks (R3); setup text and manual sentence (R1).
- [ ] Temp picture sweep at start (F5); the "`app` request ignored outside Studio's run" test (F6).
- [ ] The patch applied by hand to the current `ide/mcp.inc`, re-reviewed, and `tools/patches/app-eyes-mcp-inc.patch` deleted (§6.1).

## Required before any release (not before merge to development)

- [ ] RAI asks once per run before it looks at or uses the program (R2), or its input tools are removed.
- [ ] Recording video of a program's windows (announced as "later" in the manual): its own legal review (CIPA / GDPR; masks; off by default; royalty-free codecs only).

---

# Addendum 2: applied code (re-review of branch head 4d69714f, 2026-10-29)

Scope: the applied `ide/mcp.inc` (§6.1 asked for this), `ide/mcpapp.inc`, `docs/manual/ai-apps.md`, `docs/manual/app-eyes-and-hands.md`, `tests/app_eyes.mjs`, `tests/studio_mcp.mjs`, plus a spot check of the other checklist items. Read from the code; I did not run the test suites. Same caveat as above: a careful risk review, not legal advice.

## Verdict: MERGE AFTER FIXES

One required change (the owner's "carry the permission" rule must not carry **Allow everything**), with a test and one manual sentence. Everything else in the checklist is met.

## A. F1-F6, §3 and R1-R3 against the applied code

| Item | Found | Met? |
|---|---|---|
| F1 looking is "drive" | `McpKindOf` (`mcp.inc:439-447`) maps `app_layout`, `app_screenshot`, `app_click`, `app_type`, `app_key`, `app_drag`, `app_scroll` to `drive`; only `app_wait` is `read`. `McpDecision` (`:491-508`): Read only refuses `drive`, Ask and Allow edits ask (edits allows only `change`), Allow everything allows. The tools are registered with kind `"change"`, not `"read"` (`mcpapp.inc:57-63`), so no `readOnlyHint`. Refusal text says "and not at the running program" (`mcp.inc:1171`). | Yes |
| F2 Allow Once per run | Question string in `McpShowAsk` (`:1270`) is exactly the required one. `McpDecide` (`:1314`) sets `McpAppRunAllowed = RunStarted` only for `Choice > 0` on `drive` (Deny grants nothing; test at `app_eyes.mjs` Allow edits section). `McpDecision` allows only while `RunStarted <> 0 AND McpAppRunAllowed = RunStarted`; `RunStarted = TIMER` is set at each run start (`debug.inc:799`), so a new run asks again (tested: restart). `McpModeFollow` empties `McpGrants` on a project change (tested in the `session` section). The stale value after a run ends is harmless: there is no program to look at or click. | Yes |
| F3 consent version | `MCP_CONSENT = 2`, `McpConsentVersion` read and written (`:636`, `:666`, `:696`); `McpConsentOk` (`:655`) is used by `McpSetOnClick`, `McpSettingsTake`, `SetUpMcp` and `McpSyncTools` (`:920`: `IF McpConsentOk THEN McpAppTools`), so the `app_*` tools are not even listed before version 2 is confirmed. Start with `McpConfirmed` but no version: timer `McpReaskTick` shows `McpConsent` once; Turn On sets version 2 and starts the server; Cancel sets `McpOn = 0`, never starts the server and prints "AI apps: off. Confirm again in Settings > AI Apps." (`:671-686`). The upgrade (`McpUpgrade < 2`) turns every stored `\tall` into `\task` once (`:645-652`) and is written at once (`McpKeepConsent`), so a killed Studio before the answer still re-asks (version still 0). Tests: `app_eyes.mjs` "Upgrade" (Cancel, Turn On, next start, `all` -> `ask`, tools present only after). | Yes |
| §3 strings | Rows 0, 1, 3, footer, `McpConfirmAll` (lead, "Keep in mind", the rest) and the four `McpModeNote`s are character-for-character the strings in §3. | Yes |
| F4 audit | `mcpapp.inc:155` logs `"ok"` / `"error"` only; `ai_tools.rs` `activity` prints only a character count for `app_type` (never the text) and key names only when every token is longer than one character; the audit target is `""`. Canary test (`canaryCalls`, `checkTrail`) reads the audit file and the pane in all four modes, checks the typed canary, the shown canary, the asked-for caption in the audit, no PNG/`data:image`, and no `rapidr-app-<pid>-*.png` left. | Yes |
| F5 temp sweep | `sweep_pictures` (`crates/rapidr-studio/src/session.rs:133`, only plain files named `rapidr-app-*.png`, older than 60 s, never links) called at session start (`:361`), with a unit test. | Yes |
| F6 | `a_program_run_outside_studio_ignores_app_requests` (`crates/rapidr-cli/tests/app_eyes.rs:212`). | Yes |
| R1 / R3 | `aisetup.inc:506` and `docs/manual/ai-assistant.md:15` carry the sentences; `rai.rs:2568-2577`: `digest` is empty for `ui` risk and the decision reads "ran in your program" / "not done in your program". R2 is implemented (the `rai` section of `app_eyes.mjs` asserts the card and the refusal); it was only a release blocker anyway. | Yes |
| §6.1 | `tools/patches/` is gone; the old names `RAppDriver` / `RAppCapture` appear nowhere outside this `docs/legal` folder. | Yes |
| Manuals | `ai-apps.md` has the `app_*` rows, the "A program can show private data ... use test data" sentence, the activity-log sentence ("or a picture, the text an AI app typed, or anything your program showed"), the re-ask and the `all` reset paragraph; the stale "isn't there yet" note is gone. `app-eyes-and-hands.md` has the "Use it responsibly" box ("what it clicks happens for real ... run them on test data") and the "What the AI sees" paragraph. | Yes |

### The AI app's own caption on the pane line (the lane kept it on purpose): acceptable

The pane (and the status bar, and the screen-reader announcement) say `Clicking "<Caption>" in the running program` / `... the box beside "<Label>"` using the words the **AI app itself sent** in its request, never the program's answer; the audit log has none of it (target is `""`, outcome `ok` / `error`), and the test proves the program's sentence ("Buttons now: ...") never reaches the pane. The AI app already has those words, the pane is local and cleared by Clear Activity, and typed text is only a count. Accepted, no change. Optional tidy, not blocking: in the `app_key` arm, a single multi-character token that is not a key name (the program rejects it: `hands.rs:519`) is echoed in the pane as `Pressing <token>`; showing only `Pressing a key` unless the token is a known key name would remove even that.

## B. The new rule: a project an AI app creates or opens takes the permission of the project open before

Code: `McpCarryMode` (`mcp.inc:546-555`), called after `OpenProject` in `McpApply` (create_project) and `McpOpenProject`; a project with a kept mode (`McpModeIsKept`) keeps its own; nothing open before gives the default; the pane line, the status bar and `Application.Announce` say "New project: same permission as before (<mode>)"; `McpSetMode` also writes the audit line `mode` and persists the mode.

**Carrying Read only, Ask before changes and Allow edits without a dialog: acceptable.** None is more permissive than something the user already chose and watched, runs and the running program still ask in the last two, and it is said in three places. The one extra effect (Allow edits applied to an existing project that was opened) is limited to undoable, listed file changes inside a project the AI app could already read.

**Carrying Allow everything without a dialog: NOT acceptable.** Reasons, in the code:

1. The consent for Allow everything is per project and says so: `McpConfirmAll` asks "Allow everything for AI apps **in this project**?" and "Choose this only while you watch an AI app you trust." `McpModeFollow` has always dropped back to Ask before changes on a project change, and that drop is the safety boundary the confirmation relied on. The carry removes it with no question.
2. `open_project` can open any existing project in the Projects folder, RapidR's examples or the recent list **without asking in Allow everything** (`McpOpenAllowed`), and a project outside them after one Allow Once. That project may hold a user's real data (a shop's customers, a teacher's students, a child's data) and real-action code (send, pay, delete). With the carried mode the AI app then runs it (`run_program`), looks at it (`app_screenshot`: a picture of private data given to a third party), clicks in it (real Send / Pay / Delete) and asks RAI (money), all without one question and without the person having confirmed any of that for that project. This is exactly what F3 reset the old `all` projects for.
3. `McpSetMode` **persists** it (`McpModes`, last 50 projects): the project stays in Allow everything after Studio restarts and when the user later opens it by hand, long after "while you watch". A mode the user never picked for that project then looks like one they picked, and `McpModeIsKept` protects it as "their own".
4. The one-line pane message is an after-the-fact notice, not consent, and the badge only appears after the fact.

### Minimal safe rule (required)

Carry everything **except Allow everything**, which becomes **Allow edits**. Allow edits is the closest to the owner's intent (an AI app working alone is not stopped for its file changes, and a new program it creates is writable at once) while a run, looking at or using the running program and RAI still ask, and nothing private is disclosed or acted on without a question. (Ask before changes would be stricter and is also acceptable; this review requires only that **Allow everything is never passed on**.) The user can still choose Allow everything for the new project in the pane, with the confirmation dialog, as now.

Exact code change in `McpCarryMode` (`ide/mcp.inc`):

```basic
SUB McpCarryMode(Before AS STRING, BeforeKey AS STRING)
  DIM key AS STRING, carried AS STRING
  IF Before = "" OR ProjectOpen = 0 THEN EXIT SUB
  key = ProjectFile
  IF key = BeforeKey OR McpModeIsKept(key) THEN EXIT SUB
  carried = Before
  ' (Allow everything was confirmed for one project, while the user
  ' watched: never passed on — docs/legal/review-2026-10-29-app-eyes.md,
  ' Addendum 2)
  IF carried = "all" THEN carried = "edits"
  McpModeOf = key
  McpMode = ""
  McpSetMode carried
  IF Before = "all" THEN
    McpCarrySaid = "New project: Allow edits. Allow everything is not passed on to a new project: choose it again here if you want it."
  ELSE
    McpCarrySaid = "New project: same permission as before (" + McpModeName(carried) + ")"
  END IF
END SUB
```

(Update the comment above the SUB: "... takes Before, the mode of the project open before it, except Allow everything, which it never takes (it starts with Allow edits) ...".)

Manual: in `docs/manual/ai-apps.md` (after "A project an AI app creates or opens ... keeps the permission of the project that was open before"), change the sentence to:

> A project an AI app creates or opens (`create_project`, `open_project`) keeps the permission of the project that was open before — the pane and the status bar say "New project: same permission as before (Allow edits)" — unless it has a permission of its own. **Allow everything** is the one exception: it is never passed on, so the new project starts with **Allow edits**; choose Allow everything again for it in the pane if you want it. With no project open before, it starts with **Ask before changes**.

Also adjust the header comment of `mcp.inc` (the mode list) and any test/manual text that says the permission is always the same.

Tests to add in `tests/studio_mcp.mjs`, section `carry` (today only `edits` and "nothing open" are covered):
1. `mcp:mode=all` start, `create_project`: the new project reports `Permission: Allow edits` (not Allow everything), the pane has the "Allow everything is not passed on" line, the menu says "Allow edits", the badge is not shown; the registry/`McpModes` has no `?all` for the new project.
2. `mcp:mode=all`, `open_project` of an existing example: same result (this is the case that matters).
3. A project with a kept mode (seed `McpModes` with `<project>?read`): opened by `open_project` from an `edits` project it keeps `Read only` and no "same permission" line.
4. Carry of `read` and `ask` (one assertion each; `read` then refuses `app_screenshot` in the new project).

## C. Anything else blocking merge

Nothing. Reminders that are **not** merge blockers (unchanged from above): TSDR check of `APPHANDS` and `WINDOWCAPTURE` before the first public mention; condition B2 of the MCP addendum (`capture` of Studio's own window with Settings or a token on screen) is a separate lane's; recording video of a program's windows needs its own review before it ships.

## Required changes (checklist)

- [ ] `McpCarryMode`: Allow everything is never carried; the new project gets Allow edits with the pane/status/announcement text above.
- [ ] `docs/manual/ai-apps.md` (and the `mcp.inc` header comment): the exception sentence above.
- [ ] `tests/studio_mcp.mjs` `carry`: the four cases above; run `node tests/studio_mcp.mjs carry` and the full file once.
