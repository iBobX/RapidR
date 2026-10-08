' `?` and `@` in a hex number are the digit 0 to RapidQ's compiler — the
' .expected is RC.EXE's output (docs/rapidq-ground-truth.md). RapidQ's own
' CommCtrl.inc has `Const TVI_ROOT = &HFFFF0000???`; past 8 digits a hex
' number keeps its low 32 bits.
Const TVI_ROOT = &HFFFF0000???
Const TVI_FIRST = &HFFFF0001???
PRINT TVI_ROOT; " "; TVI_FIRST
PRINT &H1?
PRINT &H1??
PRINT &HA?B
PRINT &H7?F
PRINT &HFFFFFFFF?
PRINT &H12345678?
PRINT &H?1
PRINT &H1@
PRINT &h1?
PRINT &H1?? + 1
PRINT &H123456789
PRINT (&H1<2)
PRINT VAL("&H1?")
' (and RapidQ's keyboard example's `&HH1`: the H twice)
PRINT &HH1; " "; &HHA
