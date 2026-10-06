# RapidQ and RapidR: rights, terms, trademarks, and what RapidR takes

A review made on 2026-10-06. It covers who holds rights in RapidQ, RapidQ's
terms, trademarks, the law on re-implementing a language and its components,
RapidR's use of RapidQ's compiler for testing, and everything RapidR takes
from RapidQ and the products RapidQ itself used. It records the decisions
taken and the changes made.

> **This is research, not legal advice.** It was written without a lawyer,
> from public sources and the project's own files, and it may be wrong or
> incomplete. Where the law is unsettled or the facts are unknown, it says
> so. The project's policy, adopted with this review, is to remove anything
> that carries a plausible risk rather than argue about it, at a small cost
> in compatibility where needed. What remains is listed in §12 with the
> reasons it is considered low.

Quotations from RapidQ's materials are kept to a few words each, attributed,
only where a term has to be cited. The local sources (RapidQ's distribution
in `~/Downloads/Rapidq`, the downloaded manual in `.reference/`) stay out of
the repository.

## Summary

| Area | Finding | Risk | What protects RapidR |
|---|---|---|---|
| RapidQ's terms | Free to use for any purpose, programs may be sold; only selling copies of RapidQ itself is forbidden. No licence text forbids study, testing or compatible software (§1) | Low | RapidR doesn't distribute RapidQ; it only uses it as a black box (§5) |
| Rights holder | William Yu sold "the rights" (or the source code) to REAL Software in 2000, now Xojo, Inc. No public claim by Xojo about RapidQ was found (§2) | Low | No RapidQ code, text or media in RapidR (§9); interface and behaviour are not protected (§4) |
| Trademarks | "RapidQ", "Rapid-Q" and "RapidR" are not registered for software anywhere searched; RapidQ has been unused commercially since 2000 (§3) | Low (RapidQ owner); low to moderate (Rapidr.io, a different product) | "RapidQ" used only nominatively; non-affiliation statement (LEGAL.md, NOTICE) |
| Language, components, behaviour | Not protected by copyright in the EU, the US or under TRIPS; RapidR copies no code (§4) | Low | §4 law; clean-room record (docs/legal/clean-room.md) |
| Built-in `RAPIDQ.INC` constants | 415 of RAPIDQ.INC's 416 names and values, in largely the same order (§6) | Was low to moderate (arrangement); now low | Regrouped by public source in RapidR's own order; same names and numbers, pinned by a test |
| Library includes (qcgi.inc is GPL) | Only state and limit constants (§7) | Low | Facts; the objects are RapidR's own code |
| RC.EXE's error messages | About eight short messages reproduced (§8) | Low | Short functional phrases; long or creative ones would be reworded (none are) |
| Repository and its history | No RapidQ file ever committed; one 9-line routine from an example, a few manual quotes, manual-example data in three test fixtures, and RC.EXE output of three corpus examples (§9) | Was low; now very low | All rewritten or moved out of the repository |
| Products RapidQ used (Delphi, Windows, DelphiX, DirectX) | Names, numbers, formats and behaviour only (§10) | Low | Nominative use; trademark notices in LEGAL.md |

## 1. RapidQ's terms

**Local sources** (RapidQ's distribution on this machine, not
redistributed): the folder has no licence, EULA or readme of RapidQ's own
terms. The compiler's only notice is a copyright string naming William Yu
and 1999–2000 (`strings RC.EXE`). The help file `rapidq.chm` (William Yu's
manual, extended by community contributors) has the terms, in its FAQ,
section 1.6 "Do I need a license to distribute my programs?", and its
preface:

- Using RapidQ needs no licence ("it's free"), and neither does
  distributing or selling the programs made with it; it may be used for any
  task, "profit-seeking or otherwise", with no fee and no credit required.
- The one restriction: RapidQ itself may not be distributed for a charge
  (bundling it free with a program is allowed).
- The user takes all consequences of using it (no warranty).
- The preface (a later community edition) says the final release was a beta,
  that William Yu "sold the rights to the owner of RealBASIC", that he no
  longer supports it, and that it will not become open source. The help
  index says the rights are "currently ... owned by RealBasic".

Nothing in these terms mentions reverse engineering, benchmarking, testing,
or writing compatible or competing software, and no click-through licence
exists. The distribution also contains much third-party material with its
own terms (D. Glodt's and others' component documentation and includes,
Chris Warrington's GPL `qcgi.inc`, Michael Zito's LGPL QChart, JohnK's GPL
code in the FreeQ IDE, community-patched libraries); none of it is in RapidR.

**Web sources** (accessed 2026-10-06):

- William Yu's own site, archived in 2001 by the Wayback Machine
  (<https://web.archive.org/web/20010625115058/http://www.basicguru.com:80/rapidq/>):
  describes RapidQ as a beta that is "fully functional and FREE!", and his
  "About" page (<https://web.archive.org/web/20010730194433/http://www.basicguru.com:80/rapidq/about.html>)
  lists "It's FREE" among its features. No licence terms beyond that.
- William Yu's documentation of 29 August 2000, as mirrored by the RapidQ
  Documentation Project (<http://www.wildgardenseed.com/RQDP/pref01.html>):
  he planned to open-source RapidQ "when I've done all I can", without a
  promise. It never happened.
- The community's download site, rapidq.phatcode.net
  (<https://rapidq.phatcode.net/>, FAQ at
  <https://rapidq.phatcode.net/docs/faq.html>): the same FAQ terms; the
  package is free, no licence is needed to make programs, and "RealBASIC(c)
  owns the rights to the compiler". Its page on the RapidQ2 help file
  (<https://rapidq.phatcode.net/docs/about_rapidq2.html>) asks that the help
  file not be distributed with any commercial product, out of respect for
  "the rights of Realbasic (tm) and William Yu".
- Wikipedia (<https://en.wikipedia.org/wiki/RapidQ>): William Yu sold the
  source code to REAL Software in 2000 (no source cited by the article).

**Conclusion.** RapidQ is freeware whose author, and later its rights
holder, never granted an open-source licence and never placed it in the
public domain; copyright in the compiler, its libraries and the original
manual still exists (it lasts decades), and is probably held by Xojo, Inc.
(§2). RapidQ's terms allow free use for any purpose, which covers running it
to test programs. They say nothing against compatible software, and could
not stop what the law allows anyway (§4, §5).

## 2. Who holds the rights

- William Yu wrote RapidQ in 1999–2000, then joined REAL Software, maker of
  REALbasic. Community sources say he sold the rights, or the source code,
  to REAL Software in 2000, and development stopped (sources in §1).
- REAL Software renamed itself Xojo, Inc. on 4 June 2013
  (<https://en.wikipedia.org/wiki/Xojo>). William Yu is listed as a Xojo
  speaker in Xojo's conference sessions (<https://www.xojo.com/xdc/sessions/>).
- No company is known to have acquired RapidQ since. No public statement by
  Xojo, Inc. claiming anything about RapidQ, or objecting to the community's
  continued distribution, was found (web searches, 2026-10-06).
- RapidQ's documentation was extended by community authors (John Kelly,
  D. Glodt, Ben Laws and others), who hold rights in their additions.

So the likely holder of rights in RapidQ's compiler and original manual is
Xojo, Inc., a US company. What matters for RapidR is the same whoever holds
them: RapidR uses none of the protected material (§9), and what it does use
(interface, behaviour) isn't protected (§4).

## 3. Trademarks and the name

**Searches** (2026-10-06): USPTO trademark search (tmsearch.uspto.gov) and
TMview (<https://www.tmdn.org/tmview/>), which covers EUIPO, the UK IPO,
WIPO's Madrid register, the USPTO and many national offices including
Uruguay, Argentina, Brazil, Paraguay, Chile and Mexico. Uruguay's DNPI has
its own search, which needs an account and was not used; TMview's Uruguay
data may be incomplete.

| Name | Result |
|---|---|
| RAPIDQ | No mark for software anywhere. Unrelated: "RapidQ Quality, Simplified" (WIPO 1880911, Canada 2431145; quality-accreditation and laboratory services, classes 42/44), RAPIDQC (Siemens, class 1), RapidQS (UK, classes 35/42) |
| RAPID-Q | Only USPTO 97081693 (quantum-dot diagnostics, class 42), abandoned |
| RAPIDR | No word mark in any office. Similar but different: RAPIDRF (Miller MMIC, class 42, US and EU), RAPIDRX (US class 9, ended), RAPID R (Lisi Automotive Rapid: fasteners, classes 6/7/8/17) |
| Xojo | XOJO and X XOJO registered in the US to Xojo, Inc. (serials 85674706, 85750063) |

**Unregistered rights.** "RapidQ" was never registered. Its owner has not
used it in trade since 2000; in the US, three years of non-use is presumed
abandonment (15 U.S.C. §1127). The hobby community still uses the name.
In the EU an unregistered sign can block another only if used in trade
beyond local significance (EUTMR 2017/1001 art. 8(4)). A claim based on
"RapidQ" is therefore unlikely.

**"RapidQ" in RapidR's text** is nominative: it names the product RapidR is
compatible with, uses only the word (never a logo or styling), and comes
with a non-affiliation statement — the use allowed in the US (*New Kids on
the Block v. News America*, 971 F.2d 302, 9th Cir. 1992; *Toyota v. Tabari*,
610 F.3d 1171, 9th Cir. 2010) and the EU (EUTMR art. 14(1)(c); CJEU
*Gillette*, C-228/03, 2005). Checked: the README, LEGAL.md, the release
notes, the announcement drafts, UI strings, file names and the brand
(design/brand: RapidR's own "Run" mark; the banner's tagline mentions
RapidQ in plain text). RapidR never calls itself "the new RapidQ",
"official" or a successor. The `Q…` component names are interface (programs
use them); RapidR's own are `R…`.

**"RapidR" itself** is one letter from "RapidQ", for the same kind of
product, and chosen as a nod to it ("R" for Roberto and for Rust). Under
US likelihood-of-confusion factors (*Polaroid*, *Sleekcraft*) and the EU's
global assessment (*Sabel*, C-251/95; *Canon*, C-39/97), that would weigh
against RapidR if "RapidQ" had a live owner using it; it doesn't (above).
The other names to watch are **Rapidr.io**, a customer-feedback SaaS using
the same word (unregistered; different product and buyers), and the crowded
field of RAPID… marks, which makes "RapidR" a weak mark to protect.
Assessment: **low** risk from RapidQ's owner, **low to moderate** from
Rapidr.io. Keeping the name is defensible.

**If the name is changed anyway** (a decision for the owner, best made
before the first release): candidates with no conflicting mark in TMview or
the USPTO for classes 9/42, free on crates.io, npm and PyPI, and with free
.com and .dev domains on 2026-10-06 — **Ferrobasic** (an expired French
FERROBASIC welding mark only), **RutaBASIC** (also the company's own name),
**Ombasic**. Weaker: Rubasic (a GitHub user of that name), OxBASIC (OXBase,
DE, registered, classes 9/42), Minuano (domains taken); avoid Zorzal and
Pampero (live marks in Uruguay or Argentina). Domains: `rapidr.dev` and
`rapidr.org` were free, `rapidr.com` parked, `rapidr.io` is Rapidr.io's.

## 4. The law on re-implementing a language and its interface

**International baseline.** TRIPS art. 9(2) (binding on every WTO member,
Uruguay included): copyright protects expressions, "not ... ideas,
procedures, methods of operation or mathematical concepts as such". In
Uruguay, software is protected as a literary work by Ley 9.739 as amended by
Ley 17.616 (2003), within that baseline.

**European Union.** Directive 2009/24/EC on the legal protection of computer
programs (<https://eur-lex.europa.eu/eli/dir/2009/24/oj>):
art. 1(2) protects the expression of a program, not the ideas and principles
underlying it, including those of its interfaces; art. 5(3) lets a person
with a right to use a copy observe, study or test it to find those ideas and
principles while loading and running it; art. 6 allows decompilation when
indispensable for interoperability (not used by RapidR); art. 8 makes
contract terms against arts. 5(3) and 6 void. The CJEU's *SAS Institute v
World Programming* (C-406/10, 2 May 2012,
<https://curia.europa.eu/juris/liste.jsf?num=C-406/10>) is RapidR's case
almost exactly: WPL re-implemented the SAS language so that SAS users'
programs would run, after studying SAS under its learning licence. The
Court held that the functionality of a program, its programming language and
its data file formats are not protected; that a licensee may observe, study
and test the program to find its ideas and principles; and that keywords,
syntax, commands, options and defaults are not, in isolation, the author's
intellectual creation — while the **manual** is protected, so copying its
text could infringe. The English Court of Appeal applied this and found no
infringement ([2013] EWCA Civ 1482).

**United States.** 17 U.S.C. §102(b) excludes ideas, procedures, processes,
systems and methods of operation. *Lotus v. Borland* (49 F.3d 807, 1st Cir.
1995, affirmed by an equally divided Supreme Court, 516 U.S. 233, 1996):
a program's menu command hierarchy, which users' macros depend on, is an
uncopyrightable method of operation, so Borland could reproduce it for
compatibility. *Google LLC v. Oracle America* (593 U.S. 1, 5 April 2021,
<https://www.supremecourt.gov/opinions/20pdf/18-956_d18f.pdf>): copying
about 11,500 lines of Java's declaring code, so that programmers' knowledge
carried over, was fair use; the Court did not decide whether such code is
copyrightable (the Federal Circuit had said it was, 750 F.3d 1339, 2014).
*Sega v. Accolade* (977 F.2d 1510, 9th Cir. 1992) and *Sony v. Connectix*
(203 F.3d 596, 9th Cir. 2000): studying a program, even by intermediate
copying, to make compatible products is fair use. *Feist v. Rural* (499 U.S.
340, 1991): facts are free; only an original selection or arrangement of
them is protected. Words and short phrases are not registrable (37 C.F.R.
§202.1(a)).

**Applied to RapidR:**

| What RapidR takes | Status | Notes |
|---|---|---|
| Syntax, keywords, statements, operators | Not protected (SAS; Lotus) | Implemented in Rust from scratch |
| Component, property, method, event, function and constant names | Not protected in isolation (SAS ¶66–67; short phrases); method of operation (Lotus) | Even the US worst case (*Oracle* at the Federal Circuit) concerned copied declaring *code*; RapidR re-expresses names in its own code and data structures. Google v Oracle would cover it in any case |
| Behaviour (what programs print and do) | Not protected (SAS: functionality) | Observed by testing (§5) |
| File formats (`.bas`, `.inc` names, `.DXG`, `.X`, RapidQ's stream layouts) | Not protected (SAS: data file formats) | |
| Constant numbers (RAPIDQ.INC) | Facts (Feist) | The arrangement was the only point to watch; regrouped (§6) |
| KEYWORD.LST names | Facts | RapidR's keyword lists share the words only: different order and format, none of its descriptions (§9) |
| Compiler error messages | Short phrases; functional | §8 |
| RapidQ's manual | **Protected** | RapidR's documentation is its own; quotes trimmed to a few words (§9) |
| RapidQ's code, examples, includes, images | **Protected** | None in RapidR (§9) |

## 5. Using RC.EXE as the ground truth

RapidR's developer runs a locally installed copy of RapidQ's compiler in his
own Windows test VM (`tools/rc_probe.sh`, `tools/rapidq_truth.py`,
[docs/rapidq-ground-truth.md](../rapidq-ground-truth.md)). It compiles test
programs written for RapidR, the programs run, and their output is compared
with RapidR's.

- **RapidQ's terms** allow use for any purpose and say nothing against
  testing or compatible software (§1). The copy came from RapidQ's free
  public distribution, which its terms let anyone pass on free of charge.
- **EU**: observing, studying and testing a program while running it, to
  find the ideas and principles behind it, is the art. 5(3) right *SAS*
  confirmed, and no contract may exclude it (art. 8). RapidR does nothing
  more: no decompiling or disassembling (art. 6 isn't needed).
- **US**: running a lawfully obtained copy and recording what the programs
  print reproduces nothing protected; studying a program to make compatible
  software is fair use even where copying is involved (*Sega*, *Connectix*).
- **Text strings read from the binary** (`strings RC.EXE`): the compiler's
  error messages and the names of its built-in objects and members, used as
  interface facts. The extracted list stays local (`.reference/`, ignored by
  git). This is the closest RapidR comes to examining the binary itself, and
  it reproduces nothing but names and short messages.
- **Never redistributed**: no RapidQ file is in the repository, a build, a
  bundle or an installer; the tools copy test programs to the VM and read
  output back, and the release scripts read only the repository (§9).
- **What the tests keep**: RapidQ's output of RapidR's own test programs
  (the conformance cases' `.expected`) — output of programs RapidR's
  developer wrote. The output of RapidQ's own *example* programs, which is
  other authors' text, was kept in `tests/rapidq_golden/`; it was moved out
  of the repository with this review (§11).

## 6. RAPIDQ.INC

RapidQ programs begin with `$INCLUDE "RAPIDQ.INC"`. RapidR doesn't ship
RapidQ's file; when it isn't next to the program, the preprocessor supplies
the constants itself (`RAPIDQ_INC_CONSTANTS`,
`crates/rapidr-preprocessor/src/lib.rs`).

**(a) Comparison with RapidQ's RAPIDQ.INC** (670 lines, 416 constants): the
table held 483 names; 415 are RAPIDQ.INC's, with identical values (RAPIDQ.INC
also has `AF_OSI`, an alias the table lacked); 68 are Windows or Delphi
names RAPIDQ.INC doesn't define (`MB_*`, `ID*`, `VK_*`, eleven named
colours, `clHighlight`, `mbLeft`…). No comment, layout or grouping text of
RapidQ's was in the table, but the **order** largely followed RAPIDQ.INC's
(two runs of 124 and 118 names in the same order; sequence similarity 0.79).
Roughly 170 of the names are Windows SDK names with Microsoft's values, about
235 are names of Delphi VCL types with their ordinal values, and about 75
are RapidQ's own numbers (TRUE = 1, `Num_*`, the shift-state bits,
QComPort's codes, the MessageDlg button bits, a few component options).
Several names and values are RapidQ's quirks (`clGreen` = &H00FF00,
`clPurple` = &HFF00FF, the misspelt `clInfoBk3DDkShadow`).

**(b) Assessment.** Each name and number is a fact a program uses, and the
names are dictated by compatibility: a program that says `clBtnFace` or
`goRowSelect` needs exactly that word and number (merger: when an idea can
be expressed only one way, the expression isn't protected; scènes à faire:
what an interface requires). SAS ¶66–67 and *Lotus* cover names and option
values; *Feist* leaves only an original **selection or arrangement**
protectable. The selection here is not RapidR's choice either — it is what
existing programs may reference — but the arrangement (RapidQ's order) was a
choice, and following it was the one element a rights holder could point to.
The EU database right (Directive 96/9/EC) protects substantial investment in
obtaining or verifying contents; a list of constants copied from public
headers involves no such investment by RapidQ, and Xojo is a US company (the
right is granted to EU makers). Risk before: low to moderate on the
arrangement; now low.

**(d) What was done:**

- The table was **regrouped by public origin**, alphabetically within each
  group, with a comment per group naming its source: Windows SDK numbers
  (message boxes and results, virtual keys, PlaySound flags, character sets,
  Windows Sockets, raster operations, colours as COLORREFs, system colours);
  values of Delphi VCL types (one sub-group per type, by the type's name in
  Embarcadero's documentation); RapidQ's own numbers. Nothing of RapidQ's
  file's order, grouping or comments remains.
- **No name or value changed**, and none was dropped: every name may be used
  by an existing program, and RapidQ-exact behaviour is a project rule.
  A unit test pins the 483 names and values (a hash of the sorted pairs),
  besides the existing test that no name appears twice — which is also why
  the order has no effect on any program. The conformance suites (both
  backends) and the GUI suites passed after the change (§11).
- Code comments elsewhere that cite RAPIDQ.INC's values ("RAPIDQ.INC's
  `crDefault` 0 …") name the interface, and stay.
- Where RapidQ's number differs from today's Delphi or Windows, RapidQ's is
  kept and the comment says so.

## 7. Library includes (`LIBRARY_INCLUDES`)

RapidR builds in the objects of six RapidQ library includes (QCGI, QDOWNLOAD,
QMIDI, QWAVE, QVIDEO, QCDAUDIO), and supplies their constants when a program
includes a file that isn't there: player states (`MD_PLAY` 1 …), a wave's
format (`WV_MONO` 1, `WV_KHZ44` 44100 …), and QCGI's input limits
(`CGI_INPUT_DEFAULT` 32767, `CGI_INPUT_LARGE` 65535, `CGI_INPUT_SMALL` 255,
`CGI_MAX_PAIRS` 256).

**(c) qcgi.inc** is Chris Warrington's library, GPL-2.0-or-later. RapidR
contains none of its code: `rapidr_value::objects::cgi` is RapidR's own
implementation of the behaviour, specified from RapidQ's documentation and
from running programs (its doc comment describes it in RapidR's words). What
is shared is four constant names with plain numbers (sizes and a count) that
programs pass to `MaxInput` — facts with no expression, outside what the GPL
can reach — and the object's member names, which programs call. The other
five libraries contribute state numbers 0–4 and standard audio rates. All
are kept (programs depend on them), listed in numeric order, and the comment
now records that they are interface facts and that `qcgi.inc` isn't needed.
Risk: low.

## 8. Compiler error messages

RC.EXE's message strings were extracted to `.reference/rapidq-compiler-messages.txt`
(255 messages; local only). RapidR reproduces about eight, because programs'
users, tests and tools match on them:

- `Member X not part of class Y`
- `X is a read-only value`
- `Identifier X already used, try another name` and `Usertype X already used, try another name`
- `Datatype X not supported in STRUCT`
- `Array of QREGISTRY is not supported!` (and of QRECT, QDXJOYSTICK, in docs)
- `Return type X is not supported` (a general phrase)

Each is a handful of functional words, not protectable as such (short
phrases; SAS: commands and messages in isolation). **Decision: keep them.**
No reproduced message is long or creative; if one ever is, it is to be
reworded in RapidR's words and its tests updated (CONTRIBUTING.md). The
messages RapidR writes otherwise are its own.

## 9. The repository and its history

An audit on 2026-10-06 compared **every blob in the git object store** —
7,583 blobs, 637 MB, the full history, other branches and unreachable
objects — with RapidQ's distribution (3,969 files including the members of
its zip archives), the downloaded manual (`.reference/`) and the extracted
help file:

- **Files**: no byte-identical, text-normalised or pixel-identical match;
  one perceptual near-match of an icon was a false positive (an "R" logo
  against a calendar icon).
- **Text** (runs of 12 or more words, 6,972 text blobs): no copied prose.
  Matches were third-party licence texts (MIT, Apache, GPL… also bundled in
  RapidQ's distribution), interface facts (event signatures, member lists,
  Win32 declarations, default column captions and widths), and, in the
  current tree, attributed quotes from RapidQ's manual of 12 to 22 words in
  code comments and docs.
- **Code** (comment-stripped tokens, runs of 40 tokens or more): one real
  copy — the 9-line `ArrayInsert` routine of RapidQ's example `arrins.bas`
  in the conformance case `stream_arrays` (commit 6e97695). Three test
  fixtures and a unit test also used the manual's example data (QOUTLINE's
  and QTREEVIEW's item strings) and said so.
- **Example programs** (281 `.bas` / `.rr` / `.inc` files against 1,286
  RapidQ programs and includes): highest similarity 0.16 (Jaccard), i.e.
  nothing derived.
- **QDockForm.inc / QDirListView.inc** (RapidR's versions of community
  libraries, rewritten before this review): QDockForm is independent
  (similarity 0.16); QDirListView keeps the original's method names and
  decomposition — members programs call, so interface — with rewritten
  bodies; its longest shared run is the default column captions and widths
  (observable defaults). Low risk.
- **KEYWORD.LST**: RapidR's keyword lists share the words only, in a
  different order and format, without its descriptions.
- **`.reference/`** is ignored (`.gitignore`) and never appears in the
  history; **no release or packaging script** reads RapidQ's folder or
  `.reference/`; the tools that do (`rc_probe.sh`, `rapidq_truth.py`, a few
  test-only readers of the example corpus, which skip when it is absent)
  copy nothing into the repository or a build.

**Corrected in this review**: `ArrayInsert` replaced by RapidR's own
`InsertAt` (same output); the outline and tree-view fixtures and the tree
unit test now use RapidR's own item strings; the manual quotes in comments
and docs longer than a short phrase were paraphrased (environ, input,
LPRINT, numeric types, QDXSCREEN, QDXSOUND, QGLASSFRAME, the console kind,
undeclared variables, PRINT's comma); `examples/rapidq/notepad.bas`'s header
no longer calls itself "RapidQ code" (it is RapidR's own program in RapidQ's
dialect); the corpus examples' RC.EXE output moved to `.reference/`.

**History.** The earlier versions of the two libraries and four test
programs that followed RapidQ material too closely were already purged from
every commit on 2026-10-06 (`git filter-repo`; docs/licensing.md §8). The
`ArrayInsert` routine (9 lines), the fixtures' example strings, the quotes
and the three golden outputs remain in older commits; they are small, and
rewriting published history again has its own costs, so they are left
there. Clones made before 2026-10-06, and GitHub's cache of commits by
hash, may still hold the purged versions (§12).

## 10. Products RapidQ used: Delphi, Windows, DelphiX, DirectX, Xojo

| Owner | What RapidR uses | Status | Action |
|---|---|---|---|
| Embarcadero (Delphi's VCL) | Type and constant names with their ordinal values (§6); behaviour modelled on documented VCL behaviour (Format / FloatToStrF digit rules, TRegistry-like methods, Align / Anchors, MessageDlg); "Delphi" in comments and planning docs (~240 mentions) | Names and behaviour: not protected (SAS, Lotus — Borland's own case, Google v Oracle). Free Pascal / Lazarus has re-implemented the VCL's names for over 20 years without action. No VCL source used. "Delphi" is Embarcadero's trademark | Mentions kept nominative; dropped from release messaging ("not a clone of RapidQ or Delphi" → "not a clone of RapidQ"); trademark line in LEGAL.md. Never use "Delphi" or "VCL" in a product, component or package name |
| Microsoft (Win32, DirectX) | Win32 constant names and values (§6); the DirectX `.X` model format (documented by Microsoft, <https://learn.microsoft.com/en-us/windows/win32/direct3d9/x-files--legacy->); Direct3D Retained Mode's object model as RapidQ's QD3D* exposes it; a classic Windows-like look | API facts and formats: not protected. Microsoft publishes the same constants under MIT / Apache-2.0 (windows-rs). Look and feel of generic controls: low risk (*Apple v. Microsoft*, 35 F.3d 1435, 9th Cir. 1994). Microsoft's trademark guidelines allow saying a product is compatible, require no use of Microsoft marks in product names or logos, and an ownership line (<https://www.microsoft.com/en-us/legal/intellectualproperty/trademarks>) | Ownership line in LEGAL.md now also names Direct3D; README says "RapidQ's DirectX 2D and Direct3D objects (reimplemented by RapidR …)". RapidQ's `windows.inc` (which says it derives from Microsoft headers and carries a "(c) RealSoftware" line) is not in RapidR |
| Hiroyuki Hori (DelphiX) | The `.DXG` image-library format and the QDX* objects' behaviour (DelphiX's TDXDraw, TDXImageList…). Test and example `.dxg` files are generated by RapidR's scripts | DelphiX is freeware whose archive may not be modified or redistributed; RapidR redistributes none of it. A format reader is interoperability (SAS) | Named in LEGAL.md |
| Xojo, Inc. | Mentioned in planning docs as a comparison ("in the tradition of"); an example's comment said "RealBasic-style" | Nominative | The example's comment reworded; Xojo / REALbasic trademark line in LEGAL.md; never describe RapidR as related to Xojo |
| Visual Basic, QBasic | VB's `#If` / `#Const`, `i++`, `+=`; mentions | Syntax isn't protected; nominative mentions | Keep |

One example used `$THEME AquaClassic` (an alias RapidR accepts, which selects
the Modern theme); it now says `$THEME Modern`, the same theme, so as not
to put Apple's "Aqua" in RapidR's own programs. The theme aliases programs
may write (`Windows`, `Win95`, `Aqua`, …) stay accepted for compatibility.

## 11. Decisions and changes made with this review

| Change | Where | Effect on programs |
|---|---|---|
| RAPIDQ.INC table regrouped by public origin, comments in RapidR's words, provenance documented, names and values pinned by a test | `crates/rapidr-preprocessor/src/lib.rs` | None: same 483 names and values; no duplicates, so order is irrelevant |
| Library-include constants documented as interface facts | same file | None |
| `ArrayInsert` (copied from a RapidQ example) rewritten as RapidR's own `InsertAt`; `StrCat` (the manual's example) renamed and rewritten | `tests/conformance/cases/stream_arrays.bas`, `operators_rapidq.bas` | Tests only; same `.expected`, passing on both backends |
| Manual-example data replaced by RapidR's own | `tests/fixtures/outline.bas`, `tree_view.bas`, `tests/gui_parity_cases.mjs`, `crates/rapidr-value/src/objects/tree.rs` (unit test) | Tests only |
| Manual quotes paraphrased | `environ.rs`, `input.rs`, `lprint.rs`, `numeric.rs`, `objects/directx.rs`, `objects/glass.rs`, `rapidr-ast`, `rapidr-bytecode`, `docs/desktop-host-plan.md`, `docs/rapidq-ground-truth.md`, `CHANGELOG.md`, a conformance case's comment | Comments only |
| RC.EXE output of RapidQ's example programs moved out of git | `tests/rapidq_golden/` → `.reference/rapidq_golden/` (`RAPIDQ_GOLDEN`), `tools/rapidq_truth.py` | Developer tool only |
| Non-affiliation, interoperability and trademark statements | `LEGAL.md`, `README.md`, `NOTICE` (new, shipped in every package), release notes and template | None |
| Contributor rule against copying RapidQ material | `CONTRIBUTING.md` (new) | None |
| Clean-room process record | `docs/legal/clean-room.md` (new), `docs/rapidq-ground-truth.md` ("The boundaries") | None |
| Wording: "not a clone of RapidQ or Delphi" → "not a clone of RapidQ"; DirectX wording; example comments | release-notes template, ROADMAP, docs/ide-plan.md, README, `examples/ide.rr`, `examples/rapidq/notepad.bas` | `examples/ide.rr` writes `$THEME Modern` (the same theme as before) |

**Verified** after the changes (2026-10-06): the unit tests of the
preprocessor (with the new pin), rapidr-value, rapidr-ast and
rapidr-bytecode; the conformance suite on both backends (326 passed, and the
four edited cases again after their edits); the web conformance suite (140
passed, 3 known failures as before); the GUI event suites, native and
interpreted, at 1× and 2× (all checks passed); the web GUI parity of the two
edited fixtures (pixels and accessibility trees identical to the desktop's);
`tools/manual_reference.py --check`.

## 12. Residual risks (what can't be removed, and why it is low)

1. **Interface names and numbers** (component, member and constant names,
   RapidQ's quirky values, the method names of QDirListView, the event
   signatures in test fixtures). They are what compatibility means; the law
   above treats them as unprotected, and RapidR's arrangement and code are
   its own.
2. **Old commits.** The purged library versions may survive in clones made
   before 2026-10-06 and in GitHub's cache of commit hashes; small items
   (§9) remain in the history. *Recommended (owner's action, nothing
   automated)*: ask GitHub Support to purge cached views of the rewritten
   commits ("Removing sensitive data from a repository" in GitHub's docs).
3. **The name "RapidR"** (§3). Low from RapidQ's owner; low to moderate from
   Rapidr.io. A rename is the only way to remove it entirely; see §13.
4. **Short compiler messages** reproduced for compatibility (§8).
5. **AI-assisted development.** Code written with AI assistants could, in
   principle, reproduce text a model has seen; the repository-wide scans
   (§9) are the check, and they are to be repeated before each release
   (docs/legal/clean-room.md §5).
6. **Behaviour modelled on Delphi's and RapidQ's documentation.** Behaviour
   isn't protected; the documentation is, and RapidR's is written in its own
   words.

## 13. Decisions left to the owner

- **Rename or keep "RapidR"** (§3). The review's view: keeping it is
  defensible; if a rename is wanted, do it before the first public release
  (Ferrobasic, RutaBASIC and Ombasic had the clearest availability).
- **Register `rapidr.dev`** (and `.org`) while free, whatever the decision.
- **Trademark filing.** If the name is kept, filing RAPIDR in classes 9 and
  42 in Uruguay (DNPI), then by the Madrid system, would give the project
  its own right; expect a narrow scope in a crowded field.
- **Who holds RapidR's copyright.** `LICENSE` names Roberto Berrospe; if the
  company (Ruta Internet SRL) is meant to hold it, the notices should say so
  consistently.
- **Optional courtesy**: a note to Xojo, Inc. or William Yu asking for no
  objection would be the strongest assurance, but it isn't needed for
  anything RapidR does, and it is the owner's call whether to make contact.
- **GitHub cache purge** (§12.2).

## 14. If a legal opinion is ever sought

The questions worth asking first, in order of value: (1) a Uruguayan IP
lawyer (derecho de autor y marcas) on the name and a DNPI filing, and on
the copyright holder of RapidR; (2) only if a claim ever arrives, a US
software-copyright lawyer (the likely rights holder is a Texas company) on
the points in §4–§6. Bring this document and docs/legal/clean-room.md.

## Sources

Local (not redistributed): `~/Downloads/Rapidq` (RC.EXE, `help/rapidq.chm`
FAQ §1.6 and preface, `include/RAPIDQ.INC`, `include/qcgi.inc`, the
examples); `.reference/` (rapidq.phatcode.net mirror; RC.EXE's messages).

Web (accessed 2026-10-06): <https://rapidq.phatcode.net/>,
<https://rapidq.phatcode.net/docs/faq.html>,
<https://rapidq.phatcode.net/docs/preface.html>,
<https://rapidq.phatcode.net/docs/about_rapidq2.html>,
<https://web.archive.org/web/20010625115058/http://www.basicguru.com:80/rapidq/>,
<https://web.archive.org/web/20010730194433/http://www.basicguru.com:80/rapidq/about.html>,
<http://www.wildgardenseed.com/RQDP/pref01.html>,
<https://en.wikipedia.org/wiki/RapidQ>, <https://en.wikipedia.org/wiki/Xojo>,
<https://www.xojo.com/xdc/sessions/>, <https://tmsearch.uspto.gov/>,
<https://www.tmdn.org/tmview/>, <https://eur-lex.europa.eu/eli/dir/2009/24/oj>,
<https://curia.europa.eu/juris/liste.jsf?num=C-406/10>,
<https://www.supremecourt.gov/opinions/20pdf/18-956_d18f.pdf>,
<https://en.wikipedia.org/wiki/Lotus_Development_Corp._v._Borland_International,_Inc.>,
<https://www.microsoft.com/en-us/legal/intellectualproperty/trademarks>,
<https://learn.microsoft.com/en-us/windows/win32/direct3d9/x-files--legacy->,
<https://delphi.fandom.com/wiki/DelphiX>, <https://rapidr.io/>.
