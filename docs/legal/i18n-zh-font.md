# Simplified Chinese: fonts, and the licence check for the web subset

Lane I18N-ZH, 2026-11-06. Sections 1 to 5 are what I sent for the lawyer check before anything was added; the lawyer's review (`docs/legal/review-2026-11-06-zh.md`, part B) answered **OK with conditions B-1 to B-8**, and section 6 says how each one is met. The subset is now made by `python3 tools/fonts.py build` and checked by `python3 tools/fonts.py check` (part of `tools/regress.sh legal`).

The subset's family name is **`RapidR Han Subset`**, not "Noto Sans SC Common" as in the first measurement (section 4): the OFL grants no right to Google's trademark "Noto" (section 5), so no font RapidR cuts out of a Noto font carries the name.

## 1. What was decided (the lead, on Robert's delegation)

1. **macOS and Windows**: the system's own CJK fonts, through the kernel's font fallback. RapidR never bundles or copies Apple's or Microsoft's fonts: their licences forbid redistribution.
2. **Linux**: if no CJK system font is found, Studio says so in Chinese and English and names the free package (`fonts-noto-cjk`). It detects; it installs nothing.
3. **Web**: a subset of Noto Sans SC (OFL-1.1), common characters only, loaded only when the UI is Chinese; licence and Reserved Font Name checked first.

## 2. What the kernel did before this lane, per platform

RapidR's bundled fonts (Liberation, Inter, JetBrains Mono) have no Han glyphs. The kernel asks the system for a font for what they lack (parley with its `system` feature: CoreText on macOS, DirectWrite on Windows, fontconfig on Linux). `crates/rapidr-ui-kernel/examples/font_probe.rs` lays Chinese text out and prints the font that drew each run and how many characters came out as the empty box (glyph 0); `crates/rapidr-ui-host-web/examples/zh_capture.rs` paints a form with Chinese text through the desktop host's CPU renderer, as a PNG.

| Platform (the check ran on) | Font the system gave Han characters | Characters drawn as a box |
|---|---|---|
| macOS, this Mac (system language en-US) | Heiti SC (黑体-简), Simplified forms | 0 |
| Windows 11 Pro ARM64 (VM, en-US) | **Yu Gothic UI** (a Japanese font) | **11 of the sample's 51 Chinese characters** (编 辑 视 图 项 调 试 为 户 说 …: Simplified characters it lacks) |
| Ubuntu 26.04 ARM64 (VM, en_US, `fonts-noto-cjk` installed) | **Noto Sans CJK JP** (Japanese forms of shared characters: 直 骨 角 刃) | 0 |
| Ubuntu with no CJK font (fontconfig limited to DejaVu, to show it) | none | **51 of 51** |

So on an English Windows, a Chinese Studio showed boxes in many menu words (the capture `win-before`), and on Linux it drew Japanese forms. The system's own choice for Han depends on the system's language, and a program's `Application.Language` did not change it (the probe also passed `zh-Hans` as the text's language: no difference on Windows or Linux).

## 3. The fix in this lane (system fonts only)

`styles()` in `crates/rapidr-ui-kernel/src/text/mod.rs`: when the language the program speaks is Simplified Chinese (`zh`, `zh-Hans`, `zh-CN`, `zh-SG`; Traditional has its own fonts and falls to nothing on purpose), the computer's own Simplified Chinese fonts are named **before the generic family** (fontconfig's list for it has the Japanese Noto CJK first):

- Windows: Microsoft YaHei UI, Microsoft YaHei, SimSun;
- macOS: PingFang SC, Heiti SC (PingFang SC is not listed by name by the macOS backend: Heiti SC draws, same Simplified forms);
- Linux: Noto Sans CJK SC, Noto Sans SC, Source Han Sans SC, WenQuanYi Micro Hei, WenQuanYi Zen Hei, Droid Sans Fallback;
- the web: none (no system fonts; its Noto chunks, section 4).

Names only: no font file is bundled or copied. A family the computer lacks is skipped.

| Platform | After |
|---|---|
| macOS | Heiti SC, 0 boxes (the whole Studio in Chinese, captured: `main`, Settings, Problems, run error, RAI panel, School Lock, consent) |
| Windows 11 ARM64 | **Microsoft YaHei UI, 0 boxes** (capture `win-after`) |
| Ubuntu 26.04 ARM64 | **Noto Sans CJK SC (face 2 of the collection), 0 boxes** (capture `linux-after`) |

**Linux with no CJK font** (item 2): Studio draws a character no font has as "?" on a canvas, so it draws 你 and ? on two small bitmaps at start-up (only a desktop Studio, on Linux, in Chinese) and compares the pixels; the same means no font. It then writes two lines in the Output pane, in English and in Chinese: *No font for Chinese was found on this computer, so Chinese text shows as empty boxes. Install the free package fonts-noto-cjk (for example: sudo apt install fonts-noto-cjk), then start Studio again.* (`ChineseFontNote` in `ide/chrome.inc`; the Chinese line is a catalog text; capture `no-cjk-note`, made with the test hook `RAPIDR_TEST_NO_CJK=1`, since the check Mac has fonts.) The probe above, with the fonts hidden, proves the system-level fact the test relies on (glyph 0 for every character). Nothing is installed.

Open point, not changed: a program that shows Chinese text while its own language isn't Chinese (English Studio, a program's text in Chinese) still gets the system's choice for Han: on an English Windows, the Japanese font, with boxes for some Simplified characters (11 of the sample's 51, section 2). Tying the families to a text's language (not the UI's) is the next step; the safe workaround today is `Application.Language = "zh-Hans"`.

## 4. The web: Noto Sans SC

A browser has no system fonts the wasm can draw with. `tools/fonts.py` already splits **Noto Sans SC** (Noto CJK Sans 2.004, SHA-256 of the release archive `4d107c09ada479d3e48b6e78c83835773cbd9214bf6e12cdb7b60f8e068292ec`, pinned in `fonts/fallback/fonts.toml`) into 34 chunks by Unicode range, and the page fetches the chunk for each character it lacks. Measured in a real browser (Chromium, the prebuilt web Studio plus the Chinese catalog):

| | Files fetched | Bytes | Done at (localhost) | at 10 Mbit/s, 40 ms | at 4 Mbit/s, 80 ms |
|---|---|---|---|---|---|
| **As shipped** (chunks by Unicode range) | 24 of 34 | 5.03 MiB | 0.40 s | 4.6 s | 11.0 s |
| **Subset** (one file) | 1 | **0.78 MiB (803 KiB)** | 0.34 s | **1.0 s** | **2.1 s** |

(The times are for the fonts alone, through a modelled shared link; the page itself loads at full speed. Raw OTF; with gzip served, the chunks are 4.07 MiB and the subset 698 KiB.) The Chinese Studio's 823 distinct non-ASCII characters (803 Han) sit scattered over the whole font, so 24 of 34 chunks load; one subset loads instead.

**The subset**: the 3,755 characters of GB 2312 level 1 (the common Simplified Chinese characters: nearly all of everyday text), every character the Chinese catalogs use (803 Han; 5 of them outside level 1: 榄 橄 浏 镖 骰), and the CJK and full-width punctuation: **4,102 characters asked for, 4,070 of them in the font, 803 KiB**. A catalogs-only subset (1,118 characters) would be 177 KiB; a character in neither falls to the existing chunks. Made with fontTools' `pyftsubset` (`--layout-features= --no-hinting --name-IDs=* --notdef-outline`) from the pinned release file (SHA-256 of `NotoSansSC-Regular.otf`: `faa6c9df652116dde789d351359f3d7e5d2285a2b2a1f04a2d7244df706d5ea9`). The subset keeps the font's name table: copyright, trademark, designer, licence description and URL (name IDs 0, 7, 8, 9, 13, 14).

**It must have a family name of its own**: a font family holds one face, and the page picks one font per family, so a subset named exactly `Noto Sans SC` would hide the chunks of the same name (the existing chunks are renamed `Noto Sans SC NNN` for that). The test above named it `Noto Sans SC Common`; the shipped name is `RapidR Han Subset` (section 6).

It loads only when a character is missing, which on the web means a Chinese (or other CJK) character on screen: an English page never fetches it. The language isn't consulted; nothing loads at page start.

## 5. Licence and Reserved Font Name: what the files say

Checked on the pinned archive `18_NotoSansSC.zip` (not changed, from `target/fonts-src`) and the font inside it:

- **`LICENSE` in the archive** (4,301 bytes, dated 2022-03-19): the SIL Open Font License 1.1, in full (text below). It has **no copyright line and no "Reserved Font Name" statement**: the words appear only in the licence's own definitions and clause 3 (`grep -n -i reserved` finds lines 21, 32 and 62, all boilerplate).
- **The font's own name table** (`NotoSansSC-Regular.otf`, version 2.004): name ID 0 (copyright) is `© 2014-2021 Adobe (http://www.adobe.com/).`; ID 7 (trademark) `Noto is a trademark of Google Inc.`; ID 8 `Adobe`; ID 13 (licence) `This Font Software is licensed under the SIL Open Font License, Version 1.1. … See the SIL Open Font License for the specific language, permissions and limitations governing your use of this Font Software.`; ID 14 `http://scripts.sil.org/OFL`. **No Reserved Font Name is declared anywhere.**
- **What follows** (OFL 1.1): clause 2 lets a Modified Version (glyphs removed: a subset) be used and distributed, with the copyright notice and the licence kept and the font under the OFL alone; clause 3 forbids a *Reserved Font Name* for a modified font, and there is none to forbid; clause 5 requires it be distributed under the OFL only. The repository's `fonts/fallback/OFL.txt` (shipped beside every chunk) carries the notice of every Noto font used.
- **The trademark**: OFL grants no trademark right, and name ID 7 says "Noto" is Google's. The existing chunks are already named `Noto Sans SC NNN` (LICENSES.md §4 says renaming is allowed since no RFN is declared). The zero-risk alternative, which the lawyer preferred and section 6 does: name the subset family neutrally (`RapidR Han Subset`) and keep the credit in the notice ("a Modified Version of Noto Sans SC, (c) 2014-2021 Adobe"); the notice names the font either way.
- **Not verified by me**: I could not fetch the upstream repository (no network in this lane). The lawyer fetched the upstream `Sans/LICENSE` and confirmed it opens the same way, with no copyright line and no Reserved Font Name (review, part B, "Facts confirmed").

## 6. Done, after the lawyer's OK: the eight conditions

| # | Condition (review part B) | How it is met |
|---|---|---|
| **B-1** | each copy carries the copyright notice and the licence | `fonts/fallback/OFL.txt` (the whole OFL 1.1 with Adobe's line "(c) 2014-2021 Adobe" and now the credit of the Modified Versions) is copied into the web fonts folder by `build`, beside the subset and the chunks. The fonts keep name IDs 0 (copyright), 7 (trademark notice), 8, 9 (designers), 13 (licence) and 14 (its URL). `fonts.py check` fails when a font is in a folder without `OFL.txt`, or when a file lost ID 0 (Adobe), 7, or 13. |
| **B-2** | a neutral family name, written into the file | `RapidR Han Subset` for the subset, `RapidR Han NNN` and `RapidR Hangul NNN` for the chunks (the lawyer's optional point, done in the same pass), in name IDs 1, 3, 4, 6 and 16 and the CFF names (`rename`, `tools/fonts.py`); the names are in `fonts/fallback/fonts.toml` (`chunk_family`, `[subset] family`). `check` fails when any of those IDs has "Noto" in it. The trademark notice (ID 7) stays: it is a notice, not a use. |
| **B-3** | never sold by itself | No release asset, package or download of the font alone. It is in `lib/rapidr/web/fonts`, web bundles and `rapidr build --web` sites, as the chunks are. |
| **B-4** | OFL only, not MIT | `LICENSES.md` §4, `docs/licensing.md` and the shipped notices say the fonts are OFL-1.1 Modified Versions; no MIT header is put on the font folder. |
| **B-5** | the holders' names do not promote it | `OFL.txt` says the names are used to acknowledge their work and that they have not endorsed these fonts. |
| **B-6** | notices | `tools/third_party_notices.py` (and the regenerated `THIRD_PARTY_NOTICES.md`), `LICENSES.md` §4 (the subset, its source SHA-256, how it is cut) and `crates/rapidr-cli/src/notices.rs` (every web build's `THIRD-PARTY-NOTICES.txt`; a unit test checks it names `RapidR Han Subset` and carries Adobe's line and the whole OFL). `check` fails when any of them stops naming the subset. |
| **B-7** | same origin only | The loader (`crates/rapidr-runtime-web/src/fonts_web.rs`) fetches from the page's own `fonts/` folder (or the `RAPIDR_FONTS` the page sets); `check` fails when the file names a URL. No font CDN, no Google Fonts. |
| **B-8** | provenance | The character list is built in `build` from Python's `gb2312` codec (level 1: rows 16 to 55) and the three Chinese catalogs, not from a downloaded word list; the font file comes only from the release archive pinned by SHA-256 in `fonts.toml`; `check` fails if that pin is missing. |

The lead's own rule, **loaded only when the UI is Chinese**: the index marks the subset `"for": "zh-Hans"`, and the loader skips a file marked so unless the UI language is Simplified Chinese (`rapidr_ui_kernel::text::chinese_ui`, which is false for Traditional). A page in another language never fetches the subset, and one that shows a Han character takes the chunk as before. Nothing is fetched at page load for any language. In a Chinese UI the subset comes first, so one file serves the common characters and the chunks stay the fallback for a rarer one.

**What the build makes** (`python3 tools/fonts.py build target/web/fonts`): the subset is cut right before the chunks of the font it comes from, so it is listed first among the files with Han characters; the chunks are unchanged apart from their names and still hold every character (an English page that shows Chinese text needs them). Size and speed are as measured in section 4; the real numbers of the shipped file are in the report with the web capture.

## 7. What is left (not blocking)

- **System fonts at run time** (review: OK, nothing to add). Keep: no system font reaches a PDF (`printer.rs` writes the standard Helvetica); if Chinese printing is added, embed the OFL subset, never a system font.
- **The Chinese manual** should say once that Microsoft YaHei, SimSun, PingFang and Heiti are licensed for the computers they come with, and that an app which ships a Chinese font should use an open one such as Noto Sans SC (review, "User apps"). No Chinese manual exists yet.
- **The hero screenshots of the Chinese Studio** can be taken on Linux with Noto Sans CJK SC if zero risk is wanted (review, "Screenshots and videos").

## Appendix: the licence text, exactly as in the archive (`LICENSE`, 92 lines)

```
This Font Software is licensed under the SIL Open Font License,
Version 1.1.

This license is copied below, and is also available with a FAQ at:
http://scripts.sil.org/OFL

-----------------------------------------------------------
SIL OPEN FONT LICENSE Version 1.1 - 26 February 2007
-----------------------------------------------------------

PREAMBLE
The goals of the Open Font License (OFL) are to stimulate worldwide
development of collaborative font projects, to support the font
creation efforts of academic and linguistic communities, and to
provide a free and open framework in which fonts may be shared and
improved in partnership with others.

The OFL allows the licensed fonts to be used, studied, modified and
redistributed freely as long as they are not sold by themselves. The
fonts, including any derivative works, can be bundled, embedded,
redistributed and/or sold with any software provided that any reserved
names are not used by derivative works. The fonts and derivatives,
however, cannot be released under any other type of license. The
requirement for fonts to remain under this license does not apply to
any document created using the fonts or their derivatives.

DEFINITIONS
"Font Software" refers to the set of files released by the Copyright
Holder(s) under this license and clearly marked as such. This may
include source files, build scripts and documentation.

"Reserved Font Name" refers to any names specified as such after the
copyright statement(s).

"Original Version" refers to the collection of Font Software
components as distributed by the Copyright Holder(s).

"Modified Version" refers to any derivative made by adding to,
deleting, or substituting -- in part or in whole -- any of the
components of the Original Version, by changing formats or by porting
the Font Software to a new environment.

"Author" refers to any designer, engineer, programmer, technical
writer or other person who contributed to the Font Software.

PERMISSION & CONDITIONS
Permission is hereby granted, free of charge, to any person obtaining
a copy of the Font Software, to use, study, copy, merge, embed,
modify, redistribute, and sell modified and unmodified copies of the
Font Software, subject to the following conditions:

1) Neither the Font Software nor any of its individual components, in
Original or Modified Versions, may be sold by itself.

2) Original or Modified Versions of the Font Software may be bundled,
redistributed and/or sold with any software, provided that each copy
contains the above copyright notice and this license. These can be
included either as stand-alone text files, human-readable headers or
in the appropriate machine-readable metadata fields within text or
binary files as long as those fields can be easily viewed by the user.

3) No Modified Version of the Font Software may use the Reserved Font
Name(s) unless explicit written permission is granted by the
corresponding Copyright Holder. This restriction only applies to the
primary font name as presented to the users.

4) The name(s) of the Copyright Holder(s) or the Author(s) of the Font
Software shall not be used to promote, endorse or advertise any
Modified Version, except to acknowledge the contribution(s) of the
Copyright Holder(s) and the Author(s) or with their explicit written
permission.

5) The Font Software, modified or unmodified, in part or in whole,
must be distributed entirely under this license, and must not be
distributed under any other license. The requirement for fonts to
remain under this license does not apply to any document created using
the Font Software.

TERMINATION
This license becomes null and void if any of the above conditions are
not met.

DISCLAIMER
THE FONT SOFTWARE IS PROVIDED "AS IS", WITHOUT WARRANTY OF ANY KIND,
EXPRESS OR IMPLIED, INCLUDING BUT NOT LIMITED TO ANY WARRANTIES OF
MERCHANTABILITY, FITNESS FOR A PARTICULAR PURPOSE AND NONINFRINGEMENT
OF COPYRIGHT, PATENT, TRADEMARK, OR OTHER RIGHT. IN NO EVENT SHALL THE
COPYRIGHT HOLDER BE LIABLE FOR ANY CLAIM, DAMAGES OR OTHER LIABILITY,
INCLUDING ANY GENERAL, SPECIAL, INDIRECT, INCIDENTAL, OR CONSEQUENTIAL
DAMAGES, WHETHER IN AN ACTION OF CONTRACT, TORT OR OTHERWISE, ARISING
FROM, OUT OF THE USE OR INABILITY TO USE THE FONT SOFTWARE OR FROM
OTHER DEALINGS IN THE FONT SOFTWARE.
```
