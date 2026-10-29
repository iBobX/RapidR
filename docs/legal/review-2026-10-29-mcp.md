# Legal review, 2026-10-29: Studio's MCP server and the `RMcpServer` component

This is the "lawyer agent" review (standing rule since 2026-10-22) for a new protocol, a new component name and a new mention of a third-party product. It is a careful risk review with sources, **not legal advice** (US/EU practice; Uruguayan law not checked).

- **What is being built:** an MCP server inside RapidR Studio (off by default, a switch in Settings, `127.0.0.1` only, bearer token, Streamable HTTP), written by us from the public specification with no SDK and no new dependencies; and a reusable component, `RMcpServer`, that users' own (possibly commercial) programs can use. It follows MCP revision 2026-07-28 and also answers 2025-11-25, 2025-06-18 and 2025-03-26.
- **Searches:** run on 2026-10-29. Pages were read through a summarising fetch tool, so the two decisive ones (the LF Projects trademark policy and Anthropic's brand pages) should be re-read once by a person before anything is quoted in public.
- **Not searched:** TMview, EUIPO and WIPO (no MCP filing turned up in a web search, but that is not a register search). The USPTO record was read through public mirrors, not TSDR itself.

## Verdict table

| # | Item | Risk | Decision |
|---|---|---|---|
| 1a | Implementing the MCP spec in our own code | None | Free for commercial use; no notice or attribution needed in our binaries if nothing is copied. |
| 1b | Short spec words in our docs (`HeaderMismatch`, `-32020`, field names) | None | Fine: interface facts, and the spec is under open licences anyway. Write prose and JSON examples ourselves. |
| 1c | Copying longer spec text, tables or the schema file | Low | Avoid. If ever done: keep the Apache-2.0 / CC-BY-4.0 notice (§1). |
| 2a | "Model Context Protocol" and "MCP" are marks of LF Projects, LLC | Low | Use them only as plain descriptive words. Never in a product name, logo, icon or domain. No endorsement or certification claim. |
| 2b | Component named `RMcpServer` | Low | **Allowed as an identifier**, under the conditions in §2. Optional zero-cost step: ask LF Projects in writing. |
| 3a | "Claude Code" and the `claude mcp add …` line in the **manual** | Low | **Allowed**, exactly once, in that one snippet, with the wording in "Wording to use". |
| 3b | "Claude Code" or a `claude …` command in the **Settings page** | Medium | **Not allowed.** The Settings page shows a generic address and header only. |
| 4a | Audit log of what AI apps did | Low | Allowed, local only; log paths and operation names, never contents, keys or tokens. |
| 4b | What AI apps can see (user's code goes to a third party) | Medium if unexplained | Add the "What AI apps can see" note and a one-time confirmation when the switch is turned on. |
| 4c | Children and schools | Low | Off by default; adult-service wording; a school can leave it off. |
| 4d | API keys and the token | Low if tested | Never in any tool result, log, error text or example. |
| 4e | `RMcpServer` inside users' commercial programs | Low | Loopback and token by default; the manual says the author is responsible for what the program exposes. |
| 5 | No third-party code, no new dependencies | Gate | Checklist in §5. |

## 1. The specification's licence

**Finding.** The MCP project is `Model Context Protocol a Series of LF Projects, LLC`. Its repository's `LICENSE` file says the project "is undergoing a licensing transition from the MIT License to the Apache License, Version 2.0":
- **Specification and code:** all new contributions are Apache-2.0; older contributions stay MIT until their authors consent to move. The copyright line reads "Copyright (c) 2024-2025 Model Context Protocol a Series of LF Projects, LLC".
- **Documentation (not the specification):** CC-BY-4.0.
- Governance says the same: "All outbound code and specifications will be made available under the Project License" (Apache-2.0), documentation under CC-BY-4.0.
- The repository README still says only "MIT". The `LICENSE` file is the better source. Either way, every one of these is a permissive licence on our `deny.toml` allowlist (no copyleft).
- Revision 2026-07-28 is the **current** one on the versioning page; 2025-11-25 and earlier are final.

**What this means for us.**
1. **Implementing is free.** Copyright protects text and code, not a protocol or its interface (17 USC §102(b); *SAS Institute v World Programming*, C-406/10, for the EU; the US Supreme Court's *Google v Oracle* (2021) treated reuse of an API's declarations as fair use). Apache-2.0 §4 (notices, a copy of the licence, a list of changes) only applies if we **redistribute the specification or a derivative of it**. A server written from it, not copying it, is neither.
2. **No notice is needed in our binaries or in programs built with `RMcpServer`** if no spec text, schema file or SDK code is copied. We add a courtesy line to `docs/licensing.md` (the section that lists protocols we implement), nothing in `THIRD_PARTY_NOTICES.md`.
3. **Apache-2.0 also carries a patent licence** from each contributor for their contributions (§3). That helps implementers. No patent claim on MCP was found; a deep patent search was not done (the willfulness rule of review 5a).
4. **Short verbatim names are fine.** Error names, numbers and field names (`HeaderMismatch`, `-32020`, `MCP-Protocol-Version`, `_meta`) are facts about an interface; single names and short phrases are not protected (US Copyright Office, 37 CFR §202.1(a)), and the licence would allow even more. The manual explains them in our own words.
5. **If we ever vendor the JSON schema** (for a test), it is a third-party file: keep its licence text beside it and add it to `THIRD_PARTY_NOTICES.md` (this repeats review 2026-10-22, item 4e). Preferably do not vendor it; test against our own cases.

## 2. Governance and trademarks

**Who owns what (verified 2026-10-29).**
- Anthropic donated MCP to the **Agentic AI Foundation (AAIF)**, a directed fund of the Linux Foundation, on 2025-12-09. Anthropic says MCP keeps its governance model. The project's trademark rules are LF Projects, LLC's.
- **"MODEL CONTEXT PROTOCOL"**: US reg. **8,025,467** (class 42, software services), registered 2025-11-11 on the **Supplemental Register** (a register for descriptive terms; "PROTOCOL" is disclaimed). Filed by Anthropic, PBC; **assigned to LF Projects, LLC** (a mirror shows the assignment recorded in March 2026). A second filing, serial 99031614 (class 9, reg. 8,025,468 per a mirror), was not verified. The LF Projects policy lists "Model Context Protocol®" as registered and "MCP™" as an unregistered mark.
- A third party's separate application (serial 99302201, "gateway software for managing MCP tools") also exists. That shows "MCP" is being claimed in several places; it does not concern us.
- A Supplemental Register mark is weak (no presumption that anyone owns the term exclusively). We do not rely on that.

**The LF Projects trademark policy allows** (`lfprojects.org/policies/trademark-policy/`):
- accurate, factual statements about the project and "compatible with" wording, as long as nobody is led to think LF Projects makes, endorses or supports the product;
- referential use ("works with MCP clients").

**It forbids:** using a mark in a product name, domain name or company logo; changing, abbreviating or combining a mark; using a mark as a verb, plural or possessive; using the LF Projects logos in promotion without written permission; claiming certification without testing and written permission; claiming ownership of the mark. Using open-source software under its licence grants no trademark right.

**Decision on wording ("AI apps (MCP)", "an MCP server").** Fine, used as an adjective in running text: "an MCP server", "MCP clients", "AI apps that support MCP", "serves tools over the Model Context Protocol". Not fine: "MCPs", "MCP's", "MCP-ify", "RapidR MCP" as a feature brand, "MCP Studio", `mcp`-anything in a domain, the MCP logo or a lookalike icon anywhere (Settings, manual, site, videos; use a generic plug or link glyph from our own icon set). Say "implements revision 2026-07-28 and earlier revisions" only because it is true. Never "certified", "official", "approved" or "MCP-compliant".

**Decision on the name `RMcpServer`: allowed, risk Low, conditions below.**
- Why it is defensible: it is an identifier inside a programming language that says which protocol the component serves (US nominative use, *New Kids on the Block v News America*, 9th Cir. 1992: the thing is not otherwise identifiable, only as much of the mark as needed, no implied endorsement; EU: Art. 14(1)(c) of Regulation 2017/1001, use needed to show a product's purpose, in line with honest practice). The project's own TypeScript SDK names the same concept `McpServer`. The R prefix is RapidR's own convention, like `RHttpServer`.
- Why not "None": the policy's text bars "product name" and "combining" a mark, and, unlike HTTP, MCP **is** a mark.
- **Conditions:** (a) the identifier is not used as a feature brand: Help title, UI label, release notes and videos say "serve tools to AI apps (MCP)", with `RMcpServer` only as the component's class name; (b) no logo; (c) the first mention on the manual page and in `LEGAL.md` follows the policy's attribution wording (below); (d) `RWebMCP` (planned) is a different specification from a different body and gets its own review when it is built.
- **Fallback if LF Projects ever objects:** rename and keep the old name as a silent alias (RapidR already does this for Q names). Cheap, so no need to rename now.
- **Optional, free, Robert's call:** ask `trademarks@lfprojects.org` (the policy's contact for written permission) whether the identifier `RMcpServer` and the `rapidr mcp` command are fine. A written yes closes the point. Do not hold the release for an answer.

## 3. Naming Claude Code, and the Settings page

**Sources.** Anthropic's trademark page: its marks may be used only as permitted, may not be altered, may not carry a trademark symbol, and may not suggest sponsorship, endorsement or affiliation. It has no rule for plain factual mentions. Anthropic's Agent SDK branding rules (for partners building agents) say "Claude Code" and "Claude Code Agent" are **not permitted** as names in the partner's product, and that a product "should maintain its own branding and not appear to be Claude Code or any Anthropic product". Claude Code's own MCP page gives the syntax `claude mcp add --transport http <name> <url> --header "Authorization: Bearer …"`.

**Manual page: allowed.**
- A plain-text sentence that names the client and shows the command it takes is the textbook nominative case: we cannot tell a reader how to connect without naming the command, we use only the name, and we say we are independent. The `claude` executable name is a functional fact, not a brand use.
- It is **not** a partner agent name, so the SDK rule is not triggered, but its spirit sets the limits: no logo, no icon, no orange, no screenshots of the client's window, no "Claude Code" in a heading, page title, menu, search keyword or button, and no ™ or ®.
- Exactly one place: that snippet. Plan documents in `docs/` that name Claude Code or Claude Desktop (`docs/ide-ai.md`, `docs/ide-plan.md`, `docs/preview4-plan.md`) are internal plans; reword when next touched, and never copy them into the manual, Help or the UI.
- "Tested with Claude Code <version> on <date>" is allowed **only** if it was really tested on that date. "Works with Claude Code" as a headline is not allowed anywhere (README, site, release notes, videos).

**Settings page: generic only.** Show the address, the header name and value, and buttons "Copy address", "Copy header" and "Reset token". No `claude mcp add …` line, no "Connect Claude Code" button, no client name. Reasons:
1. A named button looks like a built-in partnership; that is the implied-endorsement risk Anthropic's rules point at, and the Agent SDK rule is about exactly this kind of label.
2. RapidR's rule is no third-party product names in the UI.
3. The command's syntax belongs to Anthropic and changes; a stale line in Studio becomes our support problem, while the manual can say "see the client's own help for the current syntax".
4. A ready-made command invites pasting the token into a shell, where it stays in history.

## 4. Other exposure in this feature

**4a. The audit log.** It is on the user's own computer and RapidR never receives it, so Ruta is neither controller nor processor of it under GDPR. It can still hold personal data (a Windows or Mac user name inside a path, a client's name, file names), and the person or school running Studio may be a controller. The existing design already hashes arguments and keeps only the last 10,000 calls. Required:
- log the time, client name and version, operation, project-relative paths and the decision; **never** file contents, code, prompts, data rows, keys, the bearer token or the Authorization header (also not a wrong token someone tried);
- keep it outside the project folder (so it is not committed or shared) and out of any "report a problem" or export bundle, now or later;
- the manual says where it lives and how to clear it; a "Clear" button exists.

**4b. Showing the user's code to third-party AI apps.** It is the user's choice, and RapidR sends nothing anywhere. But once an AI app reads a file, that app's maker decides what happens next (many send it to their cloud). The spec itself says hosts must get the user's informed consent before exposing data. Required: the note below, in the manual and in the Settings page, plus a one-time confirmation dialog the first time the switch is turned on, with the safe button (Cancel) as the default. Every bullet in the note must be checked against the real tool list before merge.

**4c. Children and schools.** Studio is used by children. The feature collects nothing, so it triggers no COPPA or GDPR-child duty for Ruta; the risk is steering a child toward third-party AI services. The consumer terms of Anthropic's own app require users to be 18 or the local age of consent, and other AI apps have similar limits. Required: off by default; no prompt, banner or button inside Studio that pushes anyone to turn it on; the note says "many AI apps are for adults only; under 18, set this up with a parent or teacher" (same wording as RAI's key set-up and the manual's "Privacy and safety"). Optional but useful: a way for a school to lock the switch off (an environment variable or policy file), described in the teacher notes.

**4d. Keys and the token.** Provider keys and any password-flagged value never appear in a tool result, resource, error message, log or example (the planned canary test covers this). Studio never asks an AI app for, stores or forwards a login of another AI service (Anthropic does not let third parties offer claude.ai login in their products; this feature does not). The bearer token is shown only on the Settings page, is random per start or after "Reset token", and is compared in constant time.

**4e. `RMcpServer` in users' programs.** RapidR's MIT licence covers it; nothing extra to ship. The manual must say: it listens on `127.0.0.1` only by default and refuses to start without a token; opening it to a network is an explicit, warned choice; what the program exposes, and the privacy of the program's own end users' data, is the program author's responsibility; and the author should describe the program as "works with MCP clients", not put "MCP" in the program's name (same LF policy).

**4f. The six traps (CLAUDE.md), for this feature.**
| Trap | Result |
|---|---|
| 1 Children | No accounts or data collected by us; see 4c for the wording. |
| 2 Third-party assets | None. The Settings page uses the shipped fonts and icons. |
| 3 Analytics and replay | None. No call goes out; the server only listens. The audit log stays local (4a). |
| 4 Marketing email | Not applicable. |
| 5 Subscriptions | Not applicable. |
| 6 Uploads (DMCA) | Not applicable: nothing is hosted or shared; an AI app only reaches the user's own open project. |

Also required by the spec and worth a line of liability thinking: check the `Origin` header (403 when present and wrong), bind `127.0.0.1` only, authenticate every request. A hole here lets a website drive Studio.

## 5. No third-party code: the gate

The engineer states in the merge note, and the reviewer checks, that:
1. No code, comments or tests were copied or translated from the MCP SDKs (TypeScript, Python, Rust `rmcp`, Go, Java), their examples, the Inspector or the conformance suite, or from blog posts. Run external test tools from outside the repository; do not vendor them.
2. No spec prose or tables were pasted into code comments, Help or the manual; comments cite a section name or URL only.
3. `Cargo.toml` and `Cargo.lock` have no changes from this feature; `cargo deny check` passes; `THIRD_PARTY_NOTICES.md` and `LICENSES.md` are unchanged. If a dependency does arrive, this review is void and the notice generator and licence check run again.
4. Any new asset (an icon for the Settings page) is RapidR's own or from the existing icon set, never the MCP logo.

## Wording to use

**Settings page** (the "AI apps (MCP)" section; the switch is off by default):

> **Let AI apps use RapidR Studio.** Off. When on, an AI app on this computer that has the address and token below can work with your open project.
> Address: `http://127.0.0.1:<port>/mcp`   Header: `Authorization: Bearer <token>`   [Copy address] [Copy header] [Reset token]
> Paste these into your AI app's MCP settings. The manual shows an example.

**What AI apps can see** (Settings page, first-time confirmation and the manual; edit each bullet to match the real tools):

> **What AI apps can see.** With this switch on, an AI app that has your token can, through Studio:
> - list and read the files in your open project (your code, forms and data files), and nothing outside the project;
> - look at and change forms and components, and build and run your program (Studio asks you first before anything is deleted, written outside the project or sent over the network);
> - read what your program prints and Studio's error messages.
>
> It cannot see your AI provider keys or passwords (they stay in your system's keychain), or files outside the project.
> RapidR sends none of this anywhere. The AI app may pass what it reads to the company that makes it, under that company's terms and privacy rules. Many AI apps are for adults only; if you are under 18, set this up with a parent or teacher. Connect only AI apps you trust, and share only what you are allowed to share (schoolwork, an employer's code or someone else's licensed code may not be yours to share).
> Turn the switch off, or press Reset token, to disconnect every AI app. Studio keeps a list of what AI apps did on this computer only (Settings, AI apps, Activity); you can clear it.

**The manual's one named-client snippet:**

> Any AI app that supports MCP can connect with the address and token from Settings. As an example, here is how Claude Code (a product of Anthropic) takes them:
> `claude mcp add --transport http rapidr-studio http://127.0.0.1:<port>/mcp --header "Authorization: Bearer <token>"`
> Other AI apps ask for the same two things in their own settings; see their documentation for the current steps. RapidR is independent: it is not made, sponsored or endorsed by Anthropic, by LF Projects or by the MCP project.
> The token you type into a command stays in your shell history and in the AI app's own settings file. If you shared it by mistake, press Reset token in Settings.

**Attribution** (first mention on the manual page; also in `LEGAL.md`):

> Model Context Protocol® and MCP™ are marks of LF Projects, LLC (https://www.lfprojects.org/policies/). RapidR uses them only to say which protocol it speaks. Claude Code is a product of Anthropic, PBC. RapidR is independent of both and is not sponsored or endorsed by them.

(Anthropic's page says not to put a trademark symbol on its marks, so none is added to "Claude Code".)

**`docs/licensing.md` courtesy line:** "MCP: implemented from the public specification (Apache-2.0, older parts MIT, documentation CC-BY-4.0); no specification text, schema or SDK code is included."

## Required before merge

- [ ] No dependency or lockfile change from this feature; `cargo deny check` clean; notices files unchanged (§5.3).
- [ ] No copied or translated third-party code, test or prose; no vendored schema (§5.1, §5.2).
- [ ] Settings page: generic address and header only; no client name, no `claude` command, no MCP logo or lookalike icon (§2, §3).
- [ ] Settings page carries the "What AI apps can see" note; the first-time confirmation exists with Cancel as the default; the switch is off by default and cannot be turned on by an AI app or a URL (§4b).
- [ ] Every bullet of the note matches the real tool list; "Studio asks you first" is true for each case it names.
- [ ] Manual: the snippet above (one place only), the attribution line, the note, the audit-log location and how to clear it, the adult-service wording, the `RMcpServer` author-responsibility paragraph (§4e). No screenshot of any client's window.
- [ ] `LEGAL.md`: the attribution paragraph. `docs/licensing.md`: the courtesy line.
- [ ] Wording sweep: "MCP" only as an adjective in running text; no "MCPs", "MCP's", "RapidR MCP", "certified", "official", "works with Claude Code" in UI, Help, README, release notes or videos.
- [ ] `RMcpServer` is not used as a feature title or brand; no logo (§2 conditions).
- [ ] Audit log: no contents, keys, tokens or Authorization header; project-relative paths; outside the project folder; "Clear" works; a test greps a session's log for a canary key and a canary token (§4a, §4d).
- [ ] Protocol security: `Origin` check, `127.0.0.1` only, token on every request, constant-time compare; `RMcpServer` refuses to start without a token (§4e, §4f).
- [ ] The `serverInfo` and any `icons` Studio sends to clients name only "RapidR Studio" and RapidR's own icon.

## Decisions for Robert
1. **Optional:** write to `trademarks@lfprojects.org` to ask that the identifier `RMcpServer` and the `rapidr mcp` command are fine (free; a written yes ends the question). Not needed for the release.
2. **Confirm** the manual may name Claude Code once, as above. If you would rather have zero third-party names, replace the snippet with "an MCP client" and keep the generic address and header; nothing else in this review changes.
3. **Teacher lock:** do you want a way for a school to lock the switch off (§4c)? Recommended.

## Sources (read 2026-10-29)
- MCP repository licence: https://github.com/modelcontextprotocol/modelcontextprotocol and its `LICENSE` file (raw: https://raw.githubusercontent.com/modelcontextprotocol/modelcontextprotocol/main/LICENSE)
- MCP governance (LF Projects, licences, trademarks): https://modelcontextprotocol.io/community/governance
- Revisions and the current one: https://modelcontextprotocol.io/specification/versioning
- Streamable HTTP, 2026-07-28 (Origin, localhost, `HeaderMismatch` `-32020`): https://modelcontextprotocol.io/specification/2026-07-28/basic/transports/streamable-http
- Spec overview (user consent principles): https://modelcontextprotocol.io/specification/2025-11-25
- TypeScript SDK (`McpServer`, Apache-2.0 for new contributions, existing code MIT): https://github.com/modelcontextprotocol/typescript-sdk
- Donation to AAIF: https://www.anthropic.com/news/donating-the-model-context-protocol-and-establishing-of-the-agentic-ai-foundation and https://www.linuxfoundation.org/press/linux-foundation-announces-the-formation-of-the-agentic-ai-foundation
- LF Projects policies and trademark policy: https://www.lfprojects.org/policies/ and https://lfprojects.org/policies/trademark-policy/
- USPTO record, read through mirrors (check TSDR, serial 99031611): https://trademarkregistration.app/Trademark/Details/99031611 and https://www.trademarkia.com/model-context-protocol-99031611
- Anthropic trademark guidelines: https://www.anthropic.com/legal/trademark-guidelines
- Agent SDK branding guidelines: https://code.claude.com/docs/en/agent-sdk/overview
- `claude mcp add` syntax: https://code.claude.com/docs/en/mcp
- Anthropic consumer terms (minimum age): https://www.anthropic.com/legal/consumer-terms
- Earlier reviews: `docs/legal/review-2026-10-22.md` (4e, 1d), `docs/legal/six-traps-preview4.md`; design: `docs/ai-architecture.md` §4-§5, `docs/ide-ai.md` §4.
