# RapidR programs that make more sense: what could change in `.rr`

Classic programs (`.bas`, `.rqw`, `.rqb`) behave exactly as RapidQ's own
compiler does. A RapidR program (`.rr`) may do better where the classic way
makes little sense. A change of a `.rr` **default** is Robert's to approve;
each row says what the classic way is (probed), what a `.rr` program does
today, and what we recommend. Additive members (a new property, an optional
argument) need no approval and are not listed here.

| # | Item | Classic (probed) | A `.rr` program today | Recommendation | Status |
|---|---|---|---|---|---|
| 1 | The rhythm of an `RTimer` | The system's timer: ticks every Interval rounded up to 15.625 ms steps, **phase-locked** — the next tick is one period from where the last was *due*, not from where its handler ended; a tick that fell due during a long handler waits and runs the moment it ends (at most one waits). Probed in the Windows 11 VM with a handler spinning 0, 24, 50 and 130 ms: `Interval = 20` with a 24 ms handler gave gaps of 31 ms; with 50 ms, 50 ms back to back; `Interval = 100` with 24 ms gave 109 to 116 ms | The next tick comes one period **after the handler ended** (so `Interval = 20` with a 24 ms handler ticks every 55 ms, 18 a second; an emulator frame timer runs slow and its sound queue runs dry). Opt in to the steady beat with `FixedRate = 1` (every Interval exactly, a late tick made up at once, up to 250 ms behind) | Make the `.rr` default the steady beat: **`FixedRate` true unless set to 0**, so `Interval = 20` means a tick every 20 ms by the clock, as a person reads it. The manual already described the timer as "every Interval milliseconds"; today it is not. Cost: a program tuned to the slow rhythm ticks faster (its handler runs once a turn, so the window stays responsive); a conformance pair `.bas` against `.rr` pins both | Awaiting Robert's decision |
