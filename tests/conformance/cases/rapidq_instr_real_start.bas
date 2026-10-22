' INSTR with a start that isn't a whole number (RC.EXE, 2026-10-22,
' docs/rapidq-ground-truth.md): the search begins where the start rounds
' half to even, and the place found is counted from the start as given —
' so the answer is a real number: INSTR(2.5, "abcabc", "c") searches from 2
' and gives 3.5. A whole start gives a whole place, not found gives 0. The
' .expected is RC.EXE's output.
PRINT INSTR(2.5, "abcabc", "c"); " "; INSTR(2.7, "abcabc", "b"); " "; INSTR(3.5, "abcabc", "c"); " "; INSTR(4.5, "abcabc", "a")
PRINT INSTR(2.5, "abcabc", "z"); " "; INSTR(2.0, "abcabc", "c"); " "; INSTR(3, "abcabc", "c")
DIM s AS SINGLE
s = 1.5
PRINT INSTR(s, "hello", "l")
