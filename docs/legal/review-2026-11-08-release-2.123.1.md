# Legal review, 2026-11-08: additions for release 2.123.1

Lawyer-agent review (standing rule 2026-10-22) of five additions. A careful risk review with sources, **not legal advice** (US/EU practice; Uruguayan law not checked). Zero-risk preference applied: where two options are close, the safer one is recommended. Read-only: nothing in the code or docs was changed by this review.

- **Read:** `crates/rapidr-package/src/readme.rs`, `macho_sign.rs`, `info.rs`, `macos.rs`, `windows.rs`, `pack.rs`; `docs/macho-signer-design.md`; `docs/manual/building-apps.md` (+ es), `build-for-every-system.md`, `getting-started.md`, `agent-tools.md` (+ es), `ai-apps.md`; `LEGAL.md`; `docs/licensing.md`; `docs/legal/webmcp-name-review.md`, `review-2026-10-22.md` (7b, 7c, 7d, 7e), `six-traps-preview4.md`; the four proposed SVGs in `design/build-dialog/icons-proposed/` (they are in the **main checkout**, not in the dialog lane's worktree, where the folder does not exist yet) and the dialog design README; the company site's `src/frontend/pages/PrivacyPage.jsx` (read only).
- **Public web checked today:** Apple's pages on opening an app from an unidentified developer (EN, es-es, es-lamr); Apple's guidelines for third parties using its trademarks; AWS CloudFront pricing-plan and standard-logging pages; the open-source XNU copy of `cs_blobs.h`; the Go, LLVM and apple-codesign signer sources (similarity check below); the CIPO record for WEBMCP; live `curl` of `studio.rapidr.dev` and `rapidr.dev`.

## Verdict table

| # | Item | Verdict | In one line |
|---|---|---|---|
| 1 | First-launch README texts (`readme.rs`) | **Change needed** (small, none blocks) | Soften four claims, correct the macOS steps for older versions and Spain's Spanish, add a trademark credit and a "RapidR does not vouch" line. Ruta's identity does not appear as signer: OK. |
| 2 | Own ad-hoc Mach-O signer | **OK** (+ documentation changes) | Clean-room claim is credible and I tested it; no anti-circumvention problem as built; three red lines to keep. |
| 3 | Build dialog icons | **OK** | Four generic window drawings; no flag, apple or penguin. Keep the macOS dots one colour. |
| 4 | WebMCP / `RAgentTools` docs | **OK** (+ two small changes) | Follows `webmcp-name-review.md` rule by rule. |
| 5 | Runners from `studio.rapidr.dev` (CloudFront) | **Change needed before the host is public** | IP addresses reach AWS and the privacy policy does not say so; the privacy page is also unreachable today. One manual sentence + one policy section. |

No item is a **block** on shipping the code. Item 5 is a block on *announcing* the runners host until its privacy page is reachable.

---

## 1. First-launch README texts (`crates/rapidr-package/src/readme.rs`)

**Verdict: change needed (small).**

### What is fine

- **Ruta is not the signer, anywhere.** The README prints only what the app's author sets (`company`, `copyright`; empty by default, so no line). The only "Ruta Internet SRL" strings in the package crate are inside unit tests (`macos.rs:153,166`, `windows.rs:275,333,340`). The ad-hoc signature carries no identity; its identifier is the bundle id, which defaults to `dev.rapidr.app.<name>` (`info.rs:155`): a namespace, not a publisher, and the manual tells authors to set their own (`--bundle-id com.example.tips`). The texts say "if you trust whoever gave it to you", never "RapidR" or "Ruta". This satisfies review 7c.
- **Review 7e is met:** official names and steps only (Windows: "More info", "Run anyway"; macOS: "Open Anyway"); nothing tells anyone to switch a protection off (no `xattr`, no `spctl`, no "turn off SmartScreen").
- **The steps match Apple's current pages:** Privacy & Security, "Open Anyway", a login password, then "Open"; Apple says the button stays "for about an hour"; the Control-click route was removed in macOS 15, so "macOS 14 and earlier" is right.
- **Trademarks:** every use is nominative (naming the system the app runs on, quoting what a dialog says), no logos. US: nominative fair use (*New Kids on the Block*, 9th Cir. 1992); EU: Art. 14(1)(c) Reg. 2017/1001; Apple's own guidelines allow a word mark in a compatibility phrase ("runs on", "compatible with") if it is not part of a product name and never a logo. Spelling matches Apple's list ("macOS", "Mac", "Apple silicon", "Touch ID"). No implication of endorsement.

### Changes

**C1. "It runs on every Mac" is an overclaim** (a Mac on macOS 10.12 or older does not). Also, per `docs/build-for-every-platform-plan.md` line 88, only macOS (Apple silicon), Windows 11 ARM, Windows 7 32-bit and Ubuntu ARM are opened by hand; "runs on Windows 8/8.1", "Intel Mac on 10.13 to 12" are claims nobody has seen. Say "made for", which is what the build sets (`LSMinimumSystemVersion`, the target):

| Where | Now | Replace with |
|---|---|---|
| macOS universal, EN | `It runs on every Mac: Apple silicon (macOS 11 or later) and Intel\n(macOS 10.13 or later).` | `It is made for Macs with Apple silicon (macOS 11 or later) and Intel\nMacs (macOS 10.13 or later).` |
| macOS universal, ES | `Funciona en todos los Mac: Apple silicon (macOS 11 o posterior) e Intel\n(macOS 10.13 o posterior).` | `Está hecha para Mac con Apple silicon (macOS 11 o posterior) y Mac con\nIntel (macOS 10.13 o posterior).` |
| macOS arm64 / x64 | `It runs on Macs with Apple silicon ...` / `It runs on Intel Macs ...` (+ ES `Funciona en`) | `It is made for ...` / `Está hecha para ...` |
| Windows x86 | `It runs on Windows 7, 8, 8.1, 10 and 11, 32-bit and 64-bit.` | `It is made for Windows 7, 8, 8.1, 10 and 11, 32-bit and 64-bit.` (ES: `Está hecha para Windows 7, 8, 8.1, 10 y 11, de 32 y de 64 bits.`) |
| Windows x64 | `It runs on 64-bit Windows 10 and 11 (most PCs).` | `It is made for 64-bit Windows 10 and 11 (most PCs).` (ES: `Está hecha para Windows 10 y 11 de 64 bits (la mayoría de los PC).`) |
| Windows ARM64 | `... with an ARM processor (Snapdragon and others).` | `... with an ARM processor.` (ES: `Es para PC con Windows y procesador ARM.`) Drops one more third-party mark (Qualcomm's) for nothing. |

**C2. "Windows remembers it."** SmartScreen's memory is per file and not guaranteed. EN: `Windows normally doesn't ask again for that file.` ES: `Windows normalmente no vuelve a preguntar por ese archivo.` (macOS "remembers it" is Apple's own wording, "saved as an exception": keep.)

**C3. The macOS steps quote one macOS version.** "Done" and the sentence "Apple could not verify ... is free of malware" are macOS 15+ wording (Apple's older page says "cannot check ... for malicious software"); the app is built for macOS 10.13 and up, where the pane is *System Preferences > Security & Privacy*. Replace the windowed-app paragraph with (EN):

```
The first time, macOS may say it did not open {name} because Apple could not
check it for malware (the exact words depend on your macOS version). That is
because the app is not signed with an Apple Developer ID and notarized by
Apple. If you trust whoever gave it to you:
  1. Click "Done" (or "Cancel"), not "Move to Trash".
  2. Open System Settings > Privacy & Security, scroll down to the message
     about {name} and click "Open Anyway" (the button stays for about an
     hour after the attempt); confirm with your password or Touch ID, then
     click "Open". macOS remembers it.
  (On macOS 12 and earlier: System Preferences > Security & Privacy >
  General > "Open Anyway". On macOS 14 and earlier, right-click the app,
  choose "Open", then "Open" again, works too.)
```

The unit test at `readme.rs:191` asserts the old strings (`"Tip Calculator" Not Opened`, `Click "Done" (not "Move to Trash")`): change it with the text.

**C4. Spanish labels differ by country in Apple's own pages.** Apple Spain: "Ajustes del Sistema", "Abrir igualmente". Latin America: "Configuración del Sistema", "Abrir de todos modos" (and "Listo", "Mover a la papelera" in the alert). The ES README uses only the Latin American set (the dialog design README, question 8, already flags this as unchecked). Use both, and check them on a Spanish macOS before release (ES):

```
La primera vez, macOS puede decir que no abrió {name} porque Apple no pudo
comprobar que esté libre de software malicioso (las palabras exactas
dependen de tu versión de macOS). Es porque la app no está firmada con un
Developer ID de Apple ni notarizada por Apple. Si confías en quien te la dio:
  1. Haz clic en "Listo" (o "Cancelar"), no en "Mover a la papelera".
  2. Abre Configuración del Sistema (en España: Ajustes del Sistema) >
     Privacidad y seguridad, baja hasta el aviso sobre {name} y haz clic en
     "Abrir de todos modos" (en España: "Abrir igualmente"; el botón queda
     disponible cerca de una hora después del intento); confirma con tu
     contraseña o Touch ID y haz clic en "Abrir". macOS lo recuerda.
  (En macOS 12 y anteriores: Preferencias del Sistema > Seguridad y
  privacidad > General > "Abrir igualmente". En macOS 14 y anteriores también
  sirve: clic derecho sobre la app, "Abrir", y otra vez "Abrir".)
```

**C5. The notices file is inside the macOS app**, not beside it (`pack.rs:235`: `Contents/Resources`). For the macOS README only, replace the closing paragraph's first sentence with (EN): `THIRD-PARTY-NOTICES.txt (inside the app: right-click it, Show Package Contents > Contents > Resources) lists the open-source software inside this app and its licences; it stays inside the app when you share it.` (ES: `THIRD-PARTY-NOTICES.txt (dentro de la app: clic derecho, Mostrar contenido del paquete > Contents > Resources) lista el software de código abierto que esta app lleva dentro y sus licencias; se queda dentro de la app al compartirla.`)

**C6. Trademark credit and "RapidR does not vouch".** Not legally required for a nominative use, but Apple's guidelines ask for a credit line where its marks appear and the cost is two lines in the README's last paragraph, above "Made with RapidR." Per target:

| Target | EN | ES |
|---|---|---|
| Windows | `Windows and SmartScreen are trademarks of the Microsoft group of companies.` | `Windows y SmartScreen son marcas del grupo de empresas de Microsoft.` |
| macOS | `Apple, Mac, macOS and Touch ID are trademarks of Apple Inc., registered in the U.S. and other countries and regions. Intel is a trademark of Intel Corporation or its subsidiaries.` | `Apple, Mac, macOS y Touch ID son marcas de Apple Inc., registradas en EE. UU. y en otros países y regiones. Intel es una marca de Intel Corporation o sus filiales.` |
| Linux | `Linux is the registered trademark of Linus Torvalds in the U.S. and other countries.` | `Linux es una marca registrada de Linus Torvalds en EE. UU. y en otros países.` |
| Web | none | none |

then, for every target (EN / ES):

> `These names belong to their owners and are used only to say where the app runs. The app and RapidR are independent of them and not endorsed by them. RapidR is the tool the app was made with; it does not vouch for what the app does.`
>
> `Esos nombres pertenecen a sus dueños y se usan solo para decir dónde funciona la app. La app y RapidR son independientes de ellos y ninguno los respalda. RapidR es la herramienta con la que se hizo la app; no garantiza lo que la app hace.`

(The last sentence answers review 7c: "Made with RapidR" must never read as RapidR or Ruta being the app's publisher.) Optional style point: Apple asks that "Mac" be followed by a generic noun and not pluralised; "Macs" is common and harmless, but "Mac computers" costs nothing.

---

## 2. The ad-hoc Mach-O signer (`macho_sign.rs`, `docs/macho-signer-design.md`)

**Verdict: OK to ship. Documentation changes below.**

### Is the clean-room claim credible? Yes, and I tested it.

- **Process evidence:** the design doc lists its sources (§1); the lane ran without network (so nothing was fetched from other signers); behaviour is pinned byte for byte against `codesign`'s output by golden tests; SHA-1 and base64 are written in the file, no new crate; no comment or identifier in `macho_sign.rs` or the example mentions another signer.
- **Similarity check (my own, today):** word-shingle comparison (6 and 8 consecutive words of code, comments and identifiers, split on camelCase and underscores) of `macho_sign.rs` against Go's `cmd/internal/codesign/codesign.go` (BSD-3), LLVM's `lld/MachO/SyntheticSections.cpp` (Apache-2.0 with LLVM exception) and six files of apple-codesign (MPL-2.0: `code_directory.rs`, `code_resources.rs`, `macho_signing.rs`, `macho_universal.rs`, `bundle_signing.rs`, `embedded_signature_builder.rs`). Result: **no shared 8-word run with Go, lld or five of the six apple-codesign files**; the only overlap is 31 six-word runs (0.5% of ours) with apple-codesign's `code_resources.rs`, and every one is Apple's plist `DOCTYPE` line or Apple's default resource-rule patterns (`Frameworks|SharedFrameworks|PlugIns|...|Library/(Automator|Spotlight|LoginItems)`). That is data macOS expects, present in every signed app's `CodeResources`, not code. `ldid` was not compared.
- **Limits, said plainly:** shingles find copied wording, identifiers and data, not a faithful translation of someone's structure; and the code was written by an AI assistant, so "no other signer's code was read" is true of this lane's process but cannot exclude patterns the model may remember. That is why the check matters. Repeat it (same method as the 2026-10-06 RapidQ review) whenever the file changes materially.
- **Format constants (review 7b: "copy nothing from ldid or xnu"):** magic numbers, slot numbers and field layouts are facts needed to interoperate; the same `cs_blobs.h` is public in Apple's open-source XNU (APSL 2.0, "(c) 2017 Apple Computer") and the Go and LLVM linkers use it the same way. `macho_sign.rs` re-expresses the values as Rust constants with its own comments and copies no code or header text. Good. The design doc cites the Xcode SDK copy as the source; say the header is also public in XNU, so nobody has to rely on the SDK licence.
- **`RULES`/`RULES2` (`macho_sign.rs:385-407`):** the design doc says "Apple's default rule sets, verbatim (TN2206)". Short functional data (regular expressions and weights) that `codesign` writes into every bundle: Low. But "verbatim from a technical note" reads like copying prose; change the wording (below).
- **Tests:** the golden test reads `/usr/bin/true` at run time on macOS only (`macho_sign.rs:1395`); no Apple binary is committed (`git ls-files` shows no fixtures). Keep it so.

### Anti-circumvention (DMCA 17 U.S.C. §1201; EU Directive 2001/29 Art. 6)

No concern as built:
1. An ad-hoc signature is a documented, valid kind: Apple's own linker adds one by default on arm64 and `codesign --sign -` is Apple's documented way. Making one is using the mechanism as designed, not "avoiding, bypassing, removing or deactivating" it (§1201(a)(3)(A)).
2. There is no certificate, no private key, no Developer ID, no notarisation ticket, no CMS signature: nothing is forged or impersonated.
3. Gatekeeper, quarantine and notarisation are untouched; the README sends people through Apple's own documented override ("Open Anyway").
4. What is signed is the author's own program; no Apple copyrighted work is accessed. If it ever mattered, §1201(f) (interoperability of an independently created program) is the fallback.

**Three red lines (add them to the design doc, §0):** (a) never remove or rewrite the quarantine attribute from RapidR code; (b) never produce a Developer ID, Team ID, CMS or "Apple-signed" claim, or reuse anyone's certificate; (c) never touch entitlements or library validation to weaken what macOS checks. Any of these changes the analysis and needs a new review.

### Patents

I cannot do a freedom-to-operate search here. Hashing the pages of a file and storing the hashes in a table is a generic technique; Apple's toolchain gives it to every developer; open-source Mach-O signers have shipped for years (Go's linker since Go 1.16, LLVM's lld since 2021, apple-codesign, ldid and others) and a web search today found no report of Apple asserting patents or sending takedowns over any of them (Apple's anti-circumvention actions I found concerned jailbreak and virtualisation products, not signers). Residual risk Low and unquantified. Mitigation: stay ad-hoc only; if a letter ever arrives, remove the feature and let Mac users sign with Apple's tool.

### Apple wording in the docs

Nominative and accurate ("Apple's `codesign`", "macOS 27.2", "Xcode 27", "Developer ID", "Rosetta"). `LEGAL.md:169` credits only "Apple and macOS". Neither `LEGAL.md` nor `docs/licensing.md` mentions the signer at all.

### Changes

1. `LEGAL.md`, the paragraph at line 166-170: replace `Apple and macOS are trademarks of Apple Inc.;` with
   `Apple, Mac, macOS, Xcode, Rosetta and Touch ID are trademarks of Apple Inc., registered in the U.S. and other countries and regions; Intel is a trademark of Intel Corporation or its subsidiaries; Snapdragon is a trademark of Qualcomm Incorporated; Raspberry Pi is a trademark of Raspberry Pi Ltd;` (the manual's build tables name Intel, Snapdragon and Raspberry Pi).
2. `docs/licensing.md`, new section after 3.4:
   > ### 3.5 RapidR's own ad-hoc Mach-O signer
   > `crates/rapidr-package/src/macho_sign.rs` writes ad-hoc code signatures (no certificate, no identity) for the Mac programs and apps RapidR builds. It is RapidR's own code, written from Apple's published file-format descriptions (the Mach-O and code-signature header files, Apple's technical notes on code signing) and from the observed output of Apple's `codesign`; no other signer's code was used. It adds no dependency (SHA-256 comes from the `sha2` crate already listed). It does not remove or work around macOS's quarantine check, Gatekeeper or notarisation. Record: `docs/macho-signer-design.md` section 1 and `docs/legal/review-2026-11-08-release-2.123.1.md`.
3. `docs/macho-signer-design.md`: §4, replace "Apple's default rule sets, verbatim (TN2206)" with "the default rule sets `codesign` writes into every bundle (TN2206 prints them too): data macOS expects, so they are reproduced as they are"; §1, add "Source 1: the same headers are public in Apple's open-source XNU"; add the similarity-check result above and the three red lines.

---

## 3. Build dialog icons

**Verdict: OK.**

Reviewed: `design/build-dialog/icons-proposed/target-windows.svg`, `target-macos.svg`, `target-linux.svg`, `target-phone.svg` (main checkout; I rendered them). Web (the existing globe, `design/icons/src/groups/web.svg`) and board (a chip) are not among the four; the globe and chip I saw only in an earlier proposal sheet and they are generic.

| Icon | What it draws | Evokes a protected mark or trade dress? |
|---|---|---|
| Windows | rounded window, title-bar line, minimise dash and a close X at the right, a content bar | No. No four-pane flag, no Windows 11 shape. Minimise/close glyphs are a UI convention every desktop uses (functional, not protectable). |
| macOS | same frame, three dots at the left of the title bar, all in the one accent blue | No apple, no Apple window-control colours. "Dots at the left" is a layout convention. |
| Linux | same frame with a `>_` prompt | No penguin. A terminal prompt is generic; coordinates are not those of the common open icon sets I know. |
| Phone | rounded rectangle, screen, home bar | No notch, no logo; generic. |

The labels "Windows", "macOS", "Linux" beside them are plain-text nominative references, which is what the Microsoft, Apple and Linux mark guidelines allow; the design README (decision 12) already says the name is always beside the picture, and the picture is never a logo.

Conditions: (a) **keep the macOS dots one colour**; an earlier drawing in the scratchpad (`icons/sheet-4.png`) used red, amber and green, which is much closer to Apple's window controls: do not go back to it. (b) Never use these icons as "the Windows logo" in posts, videos or the site. (c) Add the four to `design/icons/inventory.toml` as RapidR's own (origin recorded), as the dialog README asks.

---

## 4. WebMCP / `RAgentTools` docs against `docs/legal/webmcp-name-review.md`

**Verdict: OK, with two small changes.**

| Review rule | Found | |
|---|---|---|
| Neutral class name, no "MCP" in it (§4.1) | `RAgentTools` in code, manual, Help, example `examples/web/agent_tools.rr`, tests, components data | OK |
| §5 manual first paragraph | `docs/manual/agent-tools.md` and es: word for word, class name filled in | OK |
| §5 trademark line, first mention on the manual page and in `LEGAL.md` beside the MCP paragraph | Manual (`<!-- legal -->` block) and `LEGAL.md:205-214`; the LEGAL.md version adds "(the `RAgentTools` component, RapidR Studio's AI apps in a browser)": accurate | OK |
| §5 `docs/licensing.md` courtesy line | `licensing.md:358-364`: the approved text plus a pointer to `agent_tools_web.rs`; the sentence "No specification text, IDL or code is included" is true (the Rust and the test only use the API's names and call shape, with their own comments and their own mock) | OK |
| Rule 1: WebMCP only in plain running text; not in a title, menu, button, UI string, product-page heading, domain, repo, crate, package, public file name, video title | Manual title "Offer your app's tools to AI agents"; file `agent-tools.md`; section "Which browsers offer it"; README index row says "(the WebMCP draft)" in a sentence; Settings screen text "This browser doesn't give web pages a way to offer tools to AI agents" (screenshot `settings-web-unsupported.png`); no WebMCP string in `ide/lang` | OK |
| Rule 2: no (R)/(TM) on WebMCP | Symbols only on the two LF marks | OK |
| Rule 3: "draft", the Community Group named; never "W3C standard", "-ready", "-compatible", "certified", "official" | Grep over docs, LEGAL, README, component data, ide: no such phrase | OK |
| Rule 4: never call WebMCP "MCP"; a page never "becomes an MCP server" | The only "over MCP" sentence is in the desktop section ("`RAgentTools` serves AI apps on this computer over MCP, as `RMcpServer` does"), which is true of the desktop transport | OK |
| Rule 5: no logos; browser names only in the dated factual table; "tested with" only if tested | Table "Which browsers offer it (2026-11-07)" is the only place; screenshots show Studio only, no browser frame or logo; nothing says "tested with" | OK |
| Rule 6: do not contact the Canadian owner | Nothing in the repo does | OK |

**Re-checked today:** the CIPO record for WEBMCP (TMA1439877, class 42) is still "Issued and Active", no proceedings listed: the review's reading stands, and the neutral name remains the right call. **Name clash check for `RAgentTools`** (the review asked for one before release): a web search finds a small PyPI package `ragentools` (a retrieval-and-agents helper) and a .NET framework `Ragent` with a `Ragent.Tools` add-on; no component library or mark of that name; different ecosystems and no confusion with a RapidR component. Low. I did not search trademark registers for it.

### Changes

1. **`CHANGELOG.md` has no entry yet** for `RAgentTools` or Studio's browser transport. When written it must follow rule 1: say "offer tools to AI agents in the browser", WebMCP only in plain prose. Suggested:
   ```markdown
   - **`RAgentTools`: your web app offers its own tools to AI agents in the browser** (RapidR's; [the manual](docs/manual/agent-tools.md)): `AddTool`, `Start`, `OnToolCall`, `Reply`; a change is asked of the person first; what the windows show only when you say so. It works with browsers that implement the WebMCP draft (a draft, today offered by only some browsers); where there is none, `Available` is 0 and the program goes on. RapidR Studio on the web offers its own tools the same way ([AI apps](docs/manual/ai-apps.md)).
   ```
   (No browser names, no "WebMCP support" badge wording, no "W3C".)
2. `docs/licensing.md:358`: heading `### 3.4 The browser's AI agents (WebMCP)` → `### 3.4 The browser's AI agents`. Not a product page, so not required; the name stays in the body, as the review intends.

Still open from the review itself: a **person re-reads the W3C page** before the trademark paragraph is quoted publicly (the review read it through a summarising tool); refresh the browser table on release day (it ages). If the page is ever translated to pt-BR or zh-Hans, the legal paragraph goes in with the same meaning (`docs/legal/i18n-pt-legal-texts.md`, `i18n-zh-legal-texts.md`).

---

## 5. Runners downloaded from `studio.rapidr.dev` (AWS CloudFront)

**Verdict: change needed before the host is announced.** Code: OK.

### Facts (checked today)

- `curl` of `studio.rapidr.dev` (2026-11-08): `server: AmazonS3`, `via ... (CloudFront)`, edge `EZE50-P4`, S3 encryption header, **no `Set-Cookie`**. So Ruta's files are an S3 bucket behind CloudFront.
- AWS's plan table (CloudFront flat-rate plans): **Access Logs are listed for Pro, Business and Premium, not Free**, as you said. But the flat-rate plans bundle AWS WAF/DDoS protection and bot management, and the **Security dashboard is marked for Free**, so AWS still processes each request's address to deliver and to defend (I did not confirm which of those two features the Free tier switches on), and I found no retention period for that. So "no logs at all" must not be promised; "RapidR keeps no access log" can be. S3 server-access logging and CloudTrail data events are separate settings: confirm both are off (they are by default).
- The CLI's request carries the address it connects from, the time, the path and `User-Agent: RapidR/<version>` (`net.rs:50`); `check_base_url` rejects query strings and user-info, so nothing else rides on it.
- A visitor's IP address is personal data for a site operator that can obtain identifying information (CJEU C-582/14, *Breyer*); the operator is the controller (Ruta), AWS its processor (AWS's GDPR addendum, part of the customer agreement; confirm in AWS Artifact), with transfers outside the EU under its standard clauses. Chance of enforcement against a free tool is small; the cost of the fix is two paragraphs.
- This is the item the earlier reviews parked: `six-traps-preview4.md` ("name the host and add a privacy page when it's hosted") and `review-2026-10-22.md` 7d ("name the host in the privacy page"). It is now due.

### What is wrong today

1. **The privacy policy does not cover this host.** `PrivacyPage.jsx` says the whole site "is served from our own server; your browser loads nothing from third parties" and describes *our* server's logs. Both are true of that site and false of `studio.rapidr.dev`, which is a different server run by a processor and which is what the web Studio and the runner download talk to.
2. **The privacy page is not reachable from here.** `rapidr.dev/*` answers 302 to `studio.rapidr.dev/<same path>` (including `/privacidad` and `/en/privacy`), and `studio.rapidr.dev/en/privacy` answers 403 (an S3 miss). `rutainternet.company/en/privacy` also answered 403 to `curl` (that may be a filter on non-browser clients: check in a browser). A manual line that links to a dead privacy page is worse than none: make a privacy page load before the release.
3. The web Studio page has no link to a privacy page (no "privacy" string in `ide/web/`).

### Changes

**A. Manual, "What is sent"** (`docs/manual/building-apps.md`, replace the paragraph):

> The download asks studio.rapidr.dev for three files (`https://studio.rapidr.dev/runners/2.123.1/`: the list, its signature and the runner) like a web page does. Any web server sees the address your computer connects from, the time, the file asked for and the program's name, `RapidR/2.123.1`. This one is served through Amazon CloudFront (Amazon Web Services), which handles those requests for RapidR to deliver the files and to defend against attacks; RapidR keeps no access log of it. Nothing else leaves your computer: not your program, not your name, not what you build; no cookie and no identifier. Nothing is sent when the runner is in the install or already kept. Who is responsible and how to ask about your data: the privacy policy at https://rapidr.dev/en/privacy.

Spanish (`docs/manual/es/building-apps.md`, "Qué se envía"):

> La descarga le pide a studio.rapidr.dev tres archivos (`https://studio.rapidr.dev/runners/2.123.1/`: la lista, su firma y el ejecutor), como hace una página web. Cualquier servidor web ve la dirección desde la que se conecta tu computadora, la hora, el archivo pedido y el nombre del programa, `RapidR/2.123.1`. Este se sirve a través de Amazon CloudFront (Amazon Web Services), que atiende esas solicitudes por cuenta de RapidR para entregar los archivos y defenderse de ataques; RapidR no guarda un registro de accesos. Nada más sale de tu computadora: ni tu programa, ni tu nombre, ni lo que compilas; ni cookies ni identificadores. No se envía nada cuando el ejecutor está en la instalación o ya guardado. Quién es el responsable y cómo consultar tus datos: la política de privacidad en https://rapidr.dev/privacidad.

(Say "RapidR keeps no access log" only after you confirm the distribution has standard logging off, no WAF logging and the bucket no access logging. The "Where runners come from" section itself needs no privacy line beyond this one; do not add one in the numbered list.)

**B. Privacy policy** (`src/frontend/pages/PrivacyPage.jsx`, ES original, EN translation; PT and ZH to follow the policy's glossary):

1. In `summary`, replace the bullet "The whole site (fonts included) is served from our own server; your browser loads nothing from third parties." with:
   - ES: `Las páginas de este sitio (incluidas las fuentes tipográficas) se sirven desde nuestro propio servidor; el navegador no carga nada de terceros. RapidR Studio en la web y las descargas de RapidR (studio.rapidr.dev) se sirven a través de Amazon CloudFront: ver más abajo.`
   - EN: `The pages of this site (fonts included) are served from our own server; your browser loads nothing from third parties. RapidR Studio on the web and RapidR's downloads (studio.rapidr.dev) are served through Amazon CloudFront: see below.`
2. New block after `browsing`:
   - ES, título `RapidR Studio en la web y descargas (studio.rapidr.dev)`: `Estas páginas y archivos (RapidR Studio en el navegador y los ejecutores que RapidR descarga al compilar para otro sistema) se sirven desde un almacenamiento de Amazon Web Services a través de Amazon CloudFront, con quien tenemos un contrato de tratamiento de datos. Al pedir un archivo, el servidor recibe tu dirección IP, la fecha y hora, el archivo pedido y el navegador o programa (en RapidR, "RapidR/versión"). Amazon trata esos datos para entregar los archivos y proteger el servicio de ataques, y puede tratarlos fuera de tu país, incluso fuera de la Unión Europea, bajo las cláusulas contractuales tipo de su contrato; el tiempo que los conserva lo fija Amazon. Nosotros no guardamos un registro de accesos de este servidor y no usamos esos datos para otra cosa; las herramientas de seguridad de Amazon pueden mostrarnos la dirección de una solicitud que bloquea como ataque. No usamos cookies ni herramientas de seguimiento en estas páginas. Base legal: nuestro interés legítimo en entregar los archivos que pediste y mantener el servicio seguro (para visitantes de la Unión Europea, artículo 6.1.f del RGPD).`
   - EN, title `RapidR Studio on the web and downloads (studio.rapidr.dev)`: `These pages and files (RapidR Studio in the browser, and the runners RapidR downloads when you build for another system) are served from Amazon Web Services storage through Amazon CloudFront, with which we have a data-processing agreement. When a file is asked for, the server receives your IP address, the date and time, the file asked for and the browser or program (in RapidR, "RapidR/version"). Amazon processes this to deliver the files and protect the service from attacks, and may process it outside your country, including outside the European Union, under the standard contractual clauses of its agreement; how long it keeps it is set by Amazon. We keep no access log of this server and use the data for nothing else; Amazon's security tools may show us the address of a request it blocks as an attack. We use no cookies or tracking tools on these pages. Legal basis: our legitimate interest in delivering the files you asked for and keeping the service secure (for visitors in the European Union, GDPR Article 6(1)(f)).`
3. Keep the policy's list of rights and complaint authorities as it is (they already cover EU, Brazil and Uruguay).

**C. Web Studio page:** add a "Privacy" link to the same policy (About or Help in the web Studio; `ide/web/index.html`). Information is owed where the data is collected.

**D. Before publishing A and B, confirm and record** (next to the runners upload step in `docs/release-packaging.md`, around line 475): standard logging off; no WAF or CloudWatch logging; S3 server-access logging and CloudTrail data events off; the AWS data-processing terms apply to the account (Artifact) and say which mechanism covers transfers outside the EU (I took "standard contractual clauses" from third-party descriptions of AWS's addendum, not from AWS's own current text: read it and fix the policy sentence if it differs); the bucket's region. If any log is on, the text above must change (logs kept by Ruta need a retention period).

### Related, not legal

- The live response headers carry `content-security-policy: frame-ancestors 'self'`, COOP, Referrer-Policy and nosniff, but **not** `Permissions-Policy: tools=(self)` and **no HSTS**. `ide/web/_headers` has the first and CloudFront does not read that file: add them to the distribution's response-headers policy. The "own origin only" promise for Studio's agent tools depends on it.
- The 2.123.1 runners are not uploaded yet (`/runners/2.123.1/runners-manifest.json` answers 403): expected before the release, but it means the download path was not exercised against the live host in this review.
- Rule check (CLAUDE.md trap 2, "never load scripts or fonts from third-party servers or CDNs"): serving RapidR's own files through its own CloudFront distribution is a processor, not a third-party origin loaded into someone else's page, so it complies once the privacy text above is live. The full six-trap pass for the web-Studio-on-CloudFront hosting is still due before the release if it has not run; traps 1, 4, 5 and 6 do not change (no accounts, email, payments or uploads were added by these five items).
- "RapidR has no telemetry" (`why-rapidr.md`, `ai-assistant.md`, `troubleshooting.md`) stays true: a runner download is a file fetch the person asks for and the manual says what it sends. No change needed there.

---

## Outside the five items, noticed while reading

- `docs/manual/getting-started.md:99` and `es/getting-started.md:114` (and the mention in `troubleshooting.md:10`, `es/troubleshooting.md:15`) give `xattr -dr com.apple.quarantine "/Applications/RapidR Studio.app"` as a Terminal alternative to "Open Anyway". Removing the quarantine flag is switching Apple's check off for that app; review 7e says never to tell users to do that, and Apple's own page offers only "Open Anyway". Recommend removing those lines (the README in item 1 deliberately does not have them). Low risk, but it is the one place the project's own text goes against its rule.

## Before-release checklist (from this review)

1. README: C1 to C6 and the test at `readme.rs:191`; check the Spanish macOS labels on a Spanish Mac.
2. `LEGAL.md` trademark sentence; `docs/licensing.md` §3.5; `docs/macho-signer-design.md` edits; keep `/usr/bin/true` out of the repo.
3. Icons: keep the macOS dots one colour; add the four to the inventory.
4. `CHANGELOG.md` entry for `RAgentTools` in the review's wording; a person re-reads the W3C page.
5. Confirm the distribution settings (item 5 D); make a privacy page reachable; update the manual (EN, ES) and the policy (ES, EN, PT, ZH); add the Studio link; add `Permissions-Policy` and HSTS to the headers policy.

## Sources (read 2026-11-08)

- Apple, open an app from an unidentified developer: https://support.apple.com/en-us/102445 ; https://support.apple.com/guide/mac-help/open-a-mac-app-from-an-unknown-developer-mh40616/mac ; Spanish: https://support.apple.com/es-es/guide/mac-help/open-a-mac-app-from-an-unknown-developer-mh40616/mac ; https://support.apple.com/es-lamr/102445
- Control-click override removed in macOS 15: https://www.macrumors.com/2024/08/06/macos-sequoia-gatekeeper-security-change/ ; https://appleinsider.com/articles/24/08/06/apple-removes-control-click-option-for-skipping-gatekeeper-in-macos-sequoia
- Apple, guidelines for using Apple trademarks: https://www.apple.com/legal/intellectual-property/guidelinesfor3rdparties.html
- XNU `cs_blobs.h` (APSL 2.0): https://github.com/apple-oss-distributions/xnu/blob/main/osfmk/kern/cs_blobs.h
- Go ad-hoc signer: https://go.dev/cmd/internal/codesign ; LLVM lld: https://reviews.llvm.org/D96164 ; apple-codesign: https://gregoryszorc.com/docs/apple-codesign/main/
- AWS CloudFront flat-rate plans: https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/flat-rate-pricing-plan.html ; standard logging: https://docs.aws.amazon.com/AmazonCloudFront/latest/DeveloperGuide/standard-logging.html ; AWS DPA: https://docs.aws.amazon.com/whitepapers/latest/navigating-gdpr-compliance/aws-data-processing-addendum-dpa.html
- CIPO record, WEBMCP: https://ised-isde.canada.ca/cipo/trademark-search/2414219
- Earlier reviews: `webmcp-name-review.md`, `review-2026-10-22.md` (7b, 7c, 7d, 7e), `six-traps-preview4.md`, `review-2026-10-29-mcp.md`.
