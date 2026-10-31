# Legal/wording review, 2026-10-31: the long-form X Article ("BASIC? Yes. That's the point.")

Date: 2026-10-31. A careful wording review of the article and its pictures, from the files themselves. **Not legal advice** (US/EU practice; no lawyer consulted). Conservative choices throughout, per Robert's rule.

**Scope.** `dist/2.122.0/x-article/article-{en,es,pt,zh}.md` and the 24 pictures in `dist/2.122.0/x-article/images/` (git-ignored; made by `dist/2.122.0/x-article/src/*.py` from RapidR's own captures). The earlier short draft `dist/2.122.0/x-article.md` was read for what it got wrong. Every fact was checked against the repository; the table is at the end of `article-en.md`.

## Verdict: OK TO POST AFTER THE FIXES BELOW (all applied)

Nothing in the article names another software product as a competitor, claims a superlative without a measurement, quotes or implies anyone's endorsement, or uses a picture that isn't ours. The comparison is with programming languages, described fairly, with their strengths named first. Twelve wording fixes were needed, mostly to make true claims exactly true; they are applied in all four languages.

## Comparative advertising (EU Directive 2006/114/EC art. 4; US Lanham Act s. 43(a) and the FTC's truth-in-advertising rules)

A comparison must be objective, verifiable, about like-for-like features, not misleading and not denigrating.

- **Like with like.** The table compares each language "with its standard tools, as typically installed", says so in the article and in the picture, and says that separate and third-party tools (many excellent) add a lot to all of them. RapidR is a language plus its IDE; the frame makes that fair. OK.
- **Verifiable cells.** Each cell is a documented, checkable fact about the language's standard distribution (tkinter is in Python's standard library; C, C++ and Rust have no standard GUI; Rust's safe subset is memory-safe; JavaScript engines in browsers JIT-compile). OK.
- **F1 (fixed): a subjective row.** "First steps: Strict compiler; a steeper start" / "Large and powerful; more to learn" were opinions presented as facts. Replaced by an objective row, "Designed for first-time programmers", stating each language's main design aim (systems programming; performance and abstraction; safe, fast systems; Python "not its only aim; widely taught as a first language").
- **F2 (fixed): "Large and growing fast"** (Rust's ecosystem) is a rate claim with no source in the article: now "Large and growing".
- **F3 (fixed): the native-build cell** for RapidR left out that native builds need Rust installed (by `rapidr setup`, once), while Python's cell said "with extra tools". Now: "Yes, via generated Rust (setup installs Rust once); also standalone interpreted builds".
- **No denigration.** Each language gets a "where it shines" line, and the article lists where RapidR is behind (tiny ecosystem, Preview, small team, not code-signed, roadmap gaps) and says RapidR does not replace them. OK.
- **No superlatives.** No "fastest", "best", "only". Speed claims are RapidR against itself, with the machine (Apple M4 Max) and the note "your computer will differ, and the scripts are in the repository". No other tool's benchmark is cited (the data benchmark's comparison in the manual is deliberately left out).

## Truthfulness of RapidR's own claims

- **F4 (fixed, also in the old draft): "644 conformance programs".** 644 is runs: 322 programs (304 `.bas` + 18 `.rr`) on each of two engines. Now "322 conformance programs, each on both engines: 644 runs, 0 failures (macOS)".
- **F5 (fixed, also in the old draft): "hundreds of example programs".** 110 example programs ship; 766 / 556 / 560 are *checks*. Now "All 110 example programs checked on all three systems", with the Windows VM's 13 open cases stated.
- **F6 (fixed, also in the old draft): "Every download is signed".** The checksum file is signed; the apps are not code-signed. Now "listed in a signed checksum file … The apps themselves are not code-signed yet."
- **F7 (fixed): pixels "went from 78 ms to under 0.1 ms"** read as a before/after of the same code; it compares two methods (a `Pset` per pixel vs the new `SetPixels`) on a 256 x 192 picture. Reworded as such. The old draft's "a full screen of pixels" was imprecise and is gone.
- **F8 (fixed): Spanish counts.** The brief's 2,561 texts / 5,377 reference entries were older numbers; the installed 2.122.0 says 2,581 / 120 / 5,378 / 39, with 81 texts marked for review. The article says "a first pass".
- **F9 (fixed): privacy absolutes.** "RapidR sends nothing about your students anywhere" ignored that a cloud AI, when a teacher turns it on, receives the chat. Now: RapidR collects nothing; a cloud AI gets the chat, Studio shows what was sent, and **the provider's own terms and age rules apply**; a local model or AI off needs no account (relevant to children: COPPA/GDPR-K).
- **F10 (fixed): AI results.** Added "How well it builds depends on the model you choose", "the provider may charge for its use", and "The RAI screenshots here use a scripted demo model"; the local-models picture's caption says "example models shown". The tutorials' AI voice and scripted demo model were already disclosed.
- **F11 (fixed): scope details.** "Installers are per user, no admin" is true for the Windows installer and the Linux `.tar.gz`, not for every package: now says so. Versions is desktop-only (the browser Studio keeps the local history): now says so. "Every improvement we make for Studio, your apps get too" became "much of what we improve for Studio reaches your apps too".
- **F12 (fixed): accessibility.** The design claim (accessibility tree to the system's screen readers, ARIA mirror, WCAG AA/AAA contrast) is documented and tested by tree comparison, but this release's checks list "a screen reader" as not done by a person. The honest-status section now says a full walk-through with screen readers on each system is still on the list.
- **Support wording (fixed).** "Free and stays free" was a promise about the future: now "free and open source". Added the auto-renew sentence used elsewhere (monthly support renews monthly and is cancelled any time in the platform's settings), as for California's auto-renewal law.
- **Demo mode.** The article mentions that the tutorials were recorded by Studio in a demo mode "not in the release": true (behind the SEC-24 gate, review-2026-10-30-demo-mode.md). No promise that users get it.

## Trademarks and names

- **Language names** (Python: Python Software Foundation; Rust: Rust Foundation; JavaScript: Oracle; C and C++ are ISO standards) are used only descriptively, to name the languages, which both foundations' policies and nominative fair use allow. Each article and each table picture carries the line "used only to name the languages; no affiliation or endorsement". "Rust" is also used truthfully to say what RapidR is written in and builds through.
- **No other software products** are named in the article text: no IDEs, editors, app builders or AI vendors (checked by a scan of all four files; "minisign", named in the old draft's spirit, was removed). Named only where needed: the platforms (Windows, macOS, Linux; descriptive), the supported databases SQLite and MySQL (compatibility, nominative), standards (WebAssembly, HTML, ARIA, WCAG, MCP), the measuring machine (Apple M4 Max: a condition of a measurement), and the channels of the call to action (GitHub, GitHub Sponsors, YouTube, Buy Me a Coffee).
- **RapidQ** is named once per article, as "runs classic RapidQ programs too" (one Studio picture also shows the menu item "Import a RapidQ Program…": that is the product's own UI).
- **History.** BASIC's 1964 origin and purpose are public history; no person or institution is named in a way that implies endorsement.

## Pictures and fonts

- **All pictures are our own captures**: RapidR Studio and RapidR's example programs (from `docs/manual/images/`, `dist/2.122.0/screens/`), frames of our own tutorial videos (`dist/video/`), and two captures made today with the installed 2.122.0 (the code sample in Studio and its window, through the signed test-mode hooks). No stock, no third-party screenshots, no people. The example programs' content is made by the programs (manual: "no one's code was copied, the data is made up").
- **Checked for names inside the pictures:** no AI vendor name, model brand or personal path is visible in any picture used. (A capture made today showed a vendor's model name in Studio's AI pane; it was cropped to the editor only. A capture showing "Send shares this chat with Anthropic (Claude)" was cropped above that line.) Example model names (`example-small`, `demo-model`) and the test name "Ana" are fictional. `RSystemInfo`'s picture shows a Mac's model and memory, not personal data.
- **Fonts used to draw text into the pictures:** Inter and JetBrains Mono (SIL OFL 1.1, already in `crates/rapidr-value/fonts/`) and Noto Sans SC (SIL OFL 1.1, copied from the build cache with its licence). The OFL allows using a font to make images with no notice required.
- **Logo:** RapidR's own (`design/brand/logo/rapidr-lockup-dark.svg`), used as the brand guide says (dark background).

## Platform and posting (Robert, on the day)

- Post from Robert's own account; nothing in the article implies a third party wrote or approved it.
- X Articles have no tables: post the `comparison-table-<lang>.png` picture; the markdown table is for other sites.
- The Spanish trailer is still private: the Spanish article links the English trailer and says the Spanish videos come "in the next days". Re-check before posting; link `https://youtu.be/ONTWkfUaK3M` only once it is public.
- Re-read the "By the numbers" section against the release if a hotfix lands before posting.

## The old draft

`dist/2.122.0/x-article.md` repeats F4, F5 and F6 and says "Every download is signed": do not post it; it is marked as superseded.
