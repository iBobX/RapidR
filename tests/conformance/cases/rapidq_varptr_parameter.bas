' VARPTR doesn't see a SUB's parameters in RapidQ (RC.EXE, 2026-10-22,
' docs/rapidq-ground-truth.md): where a parameter has a global's name,
' VARPTR(name) inside the SUB is the global's address — the .expected is
' RC.EXE's output. (VARPTR of a parameter no global shares its name with is
' RC.EXE's "Unknown identifier X"; RapidR gives the parameter's address, an
' addition.) A SUB's own DIM is seen.
DEFSTR MyMsg = "global"
DEFINT N = 7
DEFLNG G, GN
G = VARPTR(MyMsg)
GN = VARPTR(N)
SUB S(MyMsg AS STRING, N AS INTEGER)
  PRINT "in S: "; MyMsg; " "; N
  PRINT "same as global: "; VARPTR(MyMsg) = G; " "; VARPTR(N) = GN
  PRINT "text there: "; VARPTR$(VARPTR(MyMsg))
  MyMsg = "changed"
  PRINT "param now: "; MyMsg
END SUB
SUB T
  DIM L AS INTEGER
  L = 5
  PRINT "local: "; VARPTR(L) <> 0
END SUB
S "param", 42
PRINT "main: "; MyMsg; " "; N
PRINT "main text there: "; VARPTR$(G)
T
