#!/usr/bin/env python3
"""A big, valid RapidR program for the editor benchmarks (SPEED-AUDIT): a
form with controls, TYPEs, CONSTs, DECLAREs and many SUBs and FUNCTIONs
with locals, loops, IFs, SELECT CASE and calls between them - the shape of
a real program, about LINES lines (deterministic: the same file each time).

  python3 tools/bench/ui/gen_source.py [LINES] > big.rr
"""
import sys

LINES = int(sys.argv[1]) if len(sys.argv) > 1 else 5000


class Rng:
    def __init__(self, seed):
        self.s = seed

    def next(self):
        self.s ^= (self.s << 13) & 0xFFFFFFFFFFFFFFFF
        self.s ^= self.s >> 7
        self.s ^= (self.s << 17) & 0xFFFFFFFFFFFFFFFF
        return self.s

    def below(self, n):
        return self.next() % n


rng = Rng(0x9E3779B97F4A7C15)
out = []
out.append("' A big program for the editor benchmarks (tools/bench/ui/gen_source.py).")
out.append("$APPTYPE GUI")
out.append("")
for i in range(40):
    out.append(f"CONST LIMIT_{i} = {100 + i * 7}")
out.append("")
for t in range(6):
    out.append(f"TYPE Record{t}")
    out.append("  Id AS INTEGER")
    out.append("  Name AS STRING * 32")
    out.append("  Amount AS DOUBLE")
    out.append("  Count AS LONG")
    out.append("END TYPE")
    out.append("")

# (how many routines: about 21 lines each, with its DECLARE and blank line)
routines = max(4, (LINES - 150) // 21)
for r in range(routines):
    if r % 3 == 0:
        out.append(f"DECLARE FUNCTION Compute{r}(Value AS INTEGER, Scale AS DOUBLE) AS DOUBLE")
    else:
        out.append(f"DECLARE SUB Handle{r}(Value AS INTEGER, Note AS STRING)")
out.append("")
out.append("DIM Totals(1000) AS DOUBLE")
out.append("DIM Names(1000) AS STRING")
out.append("DIM Counter AS INTEGER")
out.append("DIM Recs(50) AS Record0")
out.append("")
out.append("CREATE MainForm AS RForm")
out.append("  Caption = \"Big program\"")
out.append("  ClientWidth = 640: ClientHeight = 480")
for c in range(12):
    out.append(f"  CREATE Btn{c} AS RButton")
    out.append(f"    Caption = \"Button {c}\": Left = {8 + (c % 4) * 150}: Top = {8 + (c // 4) * 32}")
    out.append("  END CREATE")
out.append("  CREATE Log AS RMemo")
out.append("    Left = 8: Top = 120: Width = 620: Height = 340")
out.append("  END CREATE")
out.append("END CREATE")
out.append("")

for r in range(routines):
    if r % 3 == 0:
        out.append(f"FUNCTION Compute{r}(Value AS INTEGER, Scale AS DOUBLE) AS DOUBLE")
        out.append("  DIM i AS INTEGER, total AS DOUBLE, text AS STRING")
        out.append("  total = 0")
        out.append(f"  FOR i = 1 TO Value MOD LIMIT_{r % 40}")
        out.append("    total = total + i * Scale")
        out.append("    IF total > 1000 THEN")
        out.append("      total = total / 2")
        out.append("    ELSEIF total < -1000 THEN")
        out.append("      total = -total")
        out.append("    END IF")
        out.append("  NEXT")
        out.append("  text = \"total \" + STR$(total)")
        out.append("  SELECT CASE Value MOD 4")
        out.append("    CASE 0: total = total + LEN(text)")
        out.append("    CASE 1: total = total - 1")
        out.append("    CASE ELSE: total = total * 1.5")
        out.append("  END SELECT")
        out.append("  Result = total")
        out.append("END FUNCTION")
    else:
        callee = (r // 3) * 3
        out.append(f"SUB Handle{r}(Value AS INTEGER, Note AS STRING)")
        out.append("  DIM k AS INTEGER, s AS STRING, x AS DOUBLE")
        out.append(f"  x = Compute{callee}(Value, {1 + rng.below(9)}.5)")
        out.append("  s = UCASE$(Note) + \" \" + LTRIM$(STR$(x))")
        out.append("  FOR k = 0 TO 10")
        out.append("    Totals(k) = Totals(k) + x / (k + 1)")
        out.append("    IF Totals(k) > 1E6 THEN Totals(k) = 0")
        out.append("  NEXT")
        out.append("  WHILE LEN(s) < 40")
        out.append("    s = s + \".\"")
        out.append("  WEND")
        out.append("  Names(Value MOD 1000) = s")
        out.append("  Counter = Counter + 1")
        out.append(f"  IF Counter MOD {2 + rng.below(20)} = 0 THEN")
        out.append("    Log.AddStrings s")
        out.append("  END IF")
        out.append("  Recs(Value MOD 50).Amount = x")
        out.append("  Recs(Value MOD 50).Count = Counter")
        out.append("END SUB")
    out.append("")

out.append("MainForm.ShowModal")
print("\n".join(out))
