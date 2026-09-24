' xfail: vm, codegen — INC/DEC silently ignored (ROADMAP Phase 1)
DIM i AS INTEGER
i = 5
INC i
INC i, 10
DEC i
PRINT STR$(i)
