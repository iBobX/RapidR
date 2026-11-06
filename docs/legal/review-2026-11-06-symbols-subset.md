# Legal review: RapidR Symbols Subset (commit 847ce288), 2026-11-06

Reviewer: RapidR's conservative legal reviewer (zero-risk rule; not a lawyer, strict).
Subject: `RapidR Symbols Subset`, a 9,744-byte subset (58 symbols) of Noto Sans Symbols 2 2.008 (OFL 1.1), built into every desktop binary (`include_bytes!` in `crates/rapidr-ui-kernel/src/text/mod.rs`), drawn only when the computer offers no system fonts at all (plain Windows XP).
Precedent: `docs/legal/i18n-zh-font.md` and `docs/legal/review-2026-11-06-zh.md` part B (conditions B-1 .. B-8).

## Verdict (one line)

**PASS, no blocker.** Every OFL 1.1 condition and every one of B-1 .. B-8 is met; one small docs fix (F1) before release, three recommended hardening fixes (F2 to F4), one cosmetic note (F5).

## How I checked (and what I could not do)

- `fontTools` is **not installed** on this Mac (`python3 -I -c "import fontTools"` fails on all six Pythons). So `python3 tools/fonts.py check` ran only its text part and printed "fonts: ok (not checked on a build: fontTools isn't installed)"; **its checks of the subset file itself did not run**. I did them by hand instead, with a small pure-Python reader of the font's tables (name, cmap, CFF, hmtx, OS/2) in the scratchpad: nothing was installed and the repository was not touched.
- I read the commit's whole diff, the font and its OFL text, `fonts/fallback/fonts.toml`, `tools/fonts.py`, `tools/third_party_notices.py`, `LICENSES.md` §4, `THIRD_PARTY_NOTICES.md`, `crates/rapidr-cli/src/notices.rs`, `package.rs`, `tools/release/{stage,home,sbom}.py`, `tools/check_notices.py`, `docs/licensing.md`.
- I fetched the upstream licence (`github.com/notofonts/symbols`, `OFL.txt`, main branch) to confirm "no Reserved Font Name" from the source of the font, not only from the repository's own statement.
- Not done: I did not download the pinned release zip to confirm the committed source file is byte-identical to the zip's member (a download needs Robert's yes). The committed source's SHA-256 does match the pin (`9b0f186f...017c0`), and the zip itself is pinned in `fonts.toml` by `release_sha256`.

## Condition by condition

### OFL 1.1 itself

| Clause | Result | Evidence |
|---|---|---|
| Reserved Font Name (clause 3) | **PASS** | The upstream `OFL.txt` has one copyright line and no name "specified as such after the copyright statement". The font's own name ID 0 is the same line with no "Reserved Font Name". The subset's family is `RapidR Symbols Subset`, so it would be allowed even if one existed. |
| Copyright notice and licence in each copy (clause 2) | **PASS** | In the font: ID 0 "Copyright 2022 The Noto Project Authors (https://github.com/notofonts/symbols)", ID 13 (licence sentence) and ID 14 (licence URL). Beside the font in the source tree and in the SDK's copied crate sources (`home.py` copies the whole crate folder): `RapidRSymbolsSubset-OFL.txt`, whose body (from "PREAMBLE" on) is byte-identical to `fonts/fallback/OFL.txt`. In every output a user gets: `THIRD-PARTY-NOTICES.txt` with the whole OFL (see "Distribution" below). |
| Not sold by itself (clause 1) | **PASS** | Only inside software. `rapidr-value` and `rapidr-ui-kernel` are `publish = false`; `stage.py` copies only `OFL-1.1.txt` from that folder to the archives' docs, never the `.otf` alone; no release asset, package or download of the font exists. Notices, `LICENSES.md`, the fonts README and `fonts.toml` all say "never offered on its own". |
| Modified Version (definition; FAQ: removing glyphs is a modification) | **PASS** | Named a Modified Version in the OFL file, the notices component, `LICENSES.md` and the README. |
| Whole font stays under the OFL (clause 5) | **PASS** | `used: OFL-1.1` in the notices; `LICENSES.md` says OFL-1.1 only, not MIT. The crate's `license.workspace` line is crate metadata; the folder's README and `LICENSES.md` say the font files are OFL (same as Liberation, Inter and JetBrains Mono). |
| No promotion by holders' names (clause 4) | **PASS** | OFL file: "The names of the copyright holders and authors are used only to acknowledge their work, not to promote this font, which they have not endorsed." The notices say the same in other words; nothing says "Google's font". Naming the source ("a Modified Version of Noto Sans Symbols 2", the source URL) is an acknowledgement. |

### The Chinese subset's conditions B-1 .. B-8

| # | Condition | Result | Evidence |
|---|---|---|---|
| **B-1** | OFL text shipped with it; keep name IDs 0, 7, 8, 9, 13, 14 | **PASS** | Name table of the subset: IDs 0, 7, 8, 9, 10, 11, 12, 13, 14 all present (copyright, trademark notice, "Monotype Imaging Inc.", "Monotype Design Team", licence text, licence URL). OFL file beside it. `fonts.py check` fails if the OFL file is missing or incomplete (it reads it). See F2: its name-table part needs fontTools. |
| **B-2** | Neutral family name written inside the file; keep ID 7 | **PASS** | ID 1 `RapidR Symbols Subset`, ID 3 `RapidRSymbolsSubset-Regular;symbols`, ID 4 `RapidR Symbols Subset Regular`, ID 6 `RapidRSymbolsSubset-Regular`; the CFF font name is `RapidRSymbolsSubset-Regular`; no IDs 16/17; only Windows-platform names, so no leftover Mac-platform copy. ID 7 "Noto is a trademark of Google Inc." kept. I repeated `fonts.py`'s own scan on the bytes (ASCII, UTF-16, Mac Roman): once the two notice strings are removed, **no "Noto" is left**. Case-insensitive, the only "noto" left is inside Google's page address in ID 11 (`http://www.google.com/get/noto/`), an attribution link, not a name (F5). |
| **B-3** | Not sold, offered or packaged by itself | **PASS** | See clause 1 above. |
| **B-4** | OFL only; no MIT header on the font | **PASS** | `LICENSES.md` §4 new paragraph, `notices.rs` (`used: "OFL-1.1"`), fonts README. |
| **B-5** | Wording: acknowledgement, no endorsement | **PASS** | See clause 4 above. |
| **B-6** | Notices name it: `third_party_notices.py`, `THIRD_PARTY_NOTICES.md`, `LICENSES.md` §4, `notices.rs`, with a test | **PASS, one gap** | All four named, with a unit test of the component (`the_desktop_notices_name_the_symbols_subset_with_its_licence`; I read its assertions against the text and they hold). `fonts.py check` also requires the name in the README. Gap: `docs/licensing.md` was not updated (F1). |
| **B-7** | Same origin, no third party, no new request | **PASS (nothing to load)** | The font is `include_bytes!`, compiled for non-wasm targets only; no URL, no CDN, no request. The web build is unchanged (`cfg(not(target_arch = "wasm32"))`; the notices code adds the component only when the kind is not Web). GDPR trap 2 does not apply. |
| **B-8** | Provenance: cut only from the pinned file; characters from the build file, not a downloaded list | **PASS** | Source = committed `fonts/fallback/NotoSansSymbols2-Regular.otf`, SHA-256 equals the `[symbols] source_sha256` pin. The character list is in `fonts.toml` (58 unique), equal to the subset's cmap. **All 58 glyph outlines are identical to the source's** (I expanded the subroutines and compared the drawing instructions glyph by glyph) and all 58 advance widths are identical. No glyph from anywhere else. |

### Distribution: every desktop binary, and programs built with RapidR

- `include_bytes!` is unconditional on non-wasm targets, so the font is in **every** desktop binary that links the UI kernel, whether or not the fallback ever runs. Notices must therefore always list it, and they do: `extras()` adds `symbols_component()` when `rapidr-ui-kernel` is in the kind's crate graph and the kind is not Web.
- I ran `cargo tree` (locked, offline) for the Desktop roots (`rapidr-runtime-core`, `rapidr-runner-stub`) on x86_64 Windows, aarch64 macOS and x86_64 Linux: `rapidr-ui-kernel` is in each graph (runtime-core's default `gui` feature). The Tools roots (`rapidr-cli`, `rapidr-launcher`) include it too. `rapidr-studio` and `rapidr-designer` are libraries inside those graphs, not separate shipped programs. So Studio, the CLI, the runners and every program built with RapidR (interpreted or native: `package::finish` calls `notices::write` for the .app's `Contents/Resources`, the Linux AppDir and the plain folder, and prints a "don't distribute without its notices" error if it cannot) get the notices that include the subset and the OFL text.
- Installs: the notices are generated by this same code at release time (`lib/rapidr/notices/<kind>.txt`, `share/doc/rapidr/THIRD-PARTY-NOTICES.txt`); a release built from this commit or later has them. (The Windows 7 x86 alpha bundle, `tools/win7/alpha.sh`, builds its notices from `rapidr notices tools-windows`, so it has it too; I did not inspect the separate XP flavour's packaging, which `docs/winxp-plan.md` says shares this kernel code.) The SDK also ships the crate sources with the font and its OFL file in one folder.
- Residual, accepted in the precedent for Liberation, Inter and JetBrains Mono: someone who copies a built program's executable away from its notices file has the font's copyright and licence only as metadata inside it. Nothing new here.

### Trademark

**PASS.** "Noto is a trademark of Google Inc." is kept in the font (ID 7 and the CFF notice) and quoted in the OFL file; the family name has no "Noto"; the source name appears only to say what the subset was cut from (nominative use, as `RapidR Han Subset` did); nothing implies Google, Monotype or the Noto authors endorse RapidR.

### Other sources, brand logos, emoji

**PASS.**
- No glyph comes from another font (see B-8).
- The 58 code points are ordinary Unicode characters (bullet, check marks and crosses, card suits, arrows, geometric shapes, media controls such as pause/stop/record, hourglass, warning sign, envelope, pencil, sun/cloud/umbrella, a few punctuation signs). None is in a Private Use area (no vendor logo characters), none is a brand symbol.
- The font has **no** colour, bitmap or SVG tables (tables: CFF, GPOS, GSUB, OS/2, VORG, cmap, head, hhea, hmtx, maxp, name, post, vhea, vmtx): even the code points that other systems draw as emoji (raised hand, hourglass, high voltage, no entry, exclamation and question marks) are this font's own black-and-white outlines under the OFL, not any vendor's emoji artwork and not Noto Color Emoji (which is a separate file RapidR does not use here).

## Fixes

| # | Priority | Fix |
|---|---|---|
| **F1** | Before the next release (docs only) | `docs/licensing.md` §3.1 (its "Noto fallback fonts" bullet says Symbols 2 is "in the repository, unmodified" and the desktop "Not crates" row, around line 109, lists only Liberation among fonts). Add: "RapidR Symbols Subset: 58 symbols cut from Noto Sans Symbols 2 2.008 (Modified Version, family name of its own, OFL-1.1), built into the desktop UI kernel and drawn only on a computer with no system fonts; its OFL text is in `THIRD-PARTY-NOTICES.txt` (`notices.rs`, `symbols_component`)". Also name it in the desktop and tools "Not crates" rows. |
| **F2** | Recommended | `tools/fonts.py check`: the symbols name-table, size and character checks sit inside `if TTFont:`, so on a machine without fontTools (this one) `tools/regress.sh legal` passes without looking at the committed `.otf`. Read that one committed file with a small pure-Python name/cmap reader (or fail loudly when fontTools is missing and the file changed), so a hand-replaced binary cannot slip through. I verified the file by hand today; its SHA-256 is `a6407db241513c5c44f8470839c203d7e57a175b2c2c0a90aeaacc1b2c4b6e63`. |
| **F3** | Recommended | `tools/check_notices.py`: add one check that the generated notices for every desktop kind and every tools kind contain "RapidR Symbols Subset" and "SIL OPEN FONT LICENSE", and that the web notices do not. Today only a unit test of the component's text exists, not of the generated file (and no font is checked in any kind). |
| **F4** | Optional | `tools/release/sbom.py` lists only Liberation and JetBrains Mono among fonts (Inter and the Noto fonts are already missing). Add Inter and "RapidR Symbols Subset" (OFL-1.1, derived from Noto Sans Symbols 2 2.008) so the SBOM matches the notices. |
| **F5** | Optional, cosmetic | The subset keeps the original OS/2 vendor tag `GOOG` and name ID 11 (Google's Noto page address). Both are attributions, not names used to promote, and the OFL does not require removing them; if zero risk is wanted, neutralise the vendor tag when the font is next regenerated and keep ID 11 or drop it. No change needed to ship. |

No BLOCK on any condition. Nothing in this review needs Robert's decision except choosing whether to do F4 and F5.
