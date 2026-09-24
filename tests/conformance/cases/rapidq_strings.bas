' xfail: vm, codegen — REPLACESUBSTR$ missing from both backends (ROADMAP Phase 1 builtins)
' RapidQ-specific string builtins.
PRINT REPLACESUBSTR$("aXbX", "X", "-")
PRINT REVERSE$("abc")
PRINT STR$(TALLY("banana", "a"))
