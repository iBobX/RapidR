# Legal review, 2026-10-22: the old diff routine (GPL)

*This is RapidR's lawyer-agent review: a careful risk review, not legal advice.*

**Finding.** RDiffView's Myers module (`crates/rapidr-value/src/objects/diffview/myers.rs`, added in 4865e459 on 2026-10-06) is a **derivative** of gnulib `lib/diffseq.h` and diffutils `src/analyze.c` (Copyright FSF, **GPL-3.0-or-later**). The evidence:
- **GNU's identifiers throughout:** `xoff/xlim/yoff/ylim`, `dmin/dmax`, `fmid/bmid`, `fmin/fmax/bmin/bmax`, `fd/bd`, `tlo/thi`, `too_expensive`, `fxybest/bxybest`.
- **GNU-only details:**
  - the diagonal-window updates, the tie rules and the overlap tests;
  - the scan order;
  - Eggert's "too expensive" heuristic, statement for statement;
  - the array offsets;
  - the cost-limit loop from diffutils 3.3.

The comments were rewritten in our own words, but the code is a translation.

**Exposure.** It shipped in **v2.118.0, v2.119.0 and v2.120.0** (pre-releases, 74 downloads in total), in:
- the CLI and Studio;
- the interpreted runners;
- the web runtime;
- the SDK's shipped source;
- almost certainly every native program built with those versions.

LEGAL.md's "nothing in your program is copyleft" was untrue for them.

**Risk: low in practice.** The FSF enforces to get compliance, not money. GPLv3 §8 restores the licence once the violation stops, and curing within 30 days of a first notice restores it too. It was short-lived, with a tiny user base.

**Steps:**
1. **Done:** replaced by `crates/rapidr-diff`, written from Myers 1986 (spot-checked as clean).
2. **Robert:** remove the binary assets of v2.118.0–v2.120.0 from GitHub releases, and any copy on rapidr.dev. Keep the pages and tags with a note; optionally publish a 2.120.1 with only the diff replaced.
3. **Done:** the file relabelled in docs/licensing.md §8 (GPL-3.0-or-later for those commits). No history rewrite.
4. **CHANGELOG, Preview 4:** "2.118.0–2.120.0 contained a diff routine derived from GNU diff (GPL-3.0-or-later), contrary to LEGAL.md; removed. Rebuild programs built with those versions."
5. **The same sentence on the old release pages.** No mailing.
6. **No GPL source offer** (the source is public and labelled), and **no relicensing** of the binaries.
7. **Don't contact the FSF.** Keep this record. If anyone writes, fix within 30 days.
8. **Prevention:**
   - a check that fails on other implementations' internal names (GNU, gnulib, xdiff, `compareseq`, `fxybest`, `too_expensive`…);
   - LEGAL.md reworded;
   - settle the `gix-imara-diff` (xdiff) question before Versions ships.

**Record of actions:**
- 2026-10-22: steps 1, 3 and 8 (LEGAL.md wording) done by the lead.
- Steps 2 and 5 wait for Robert.
