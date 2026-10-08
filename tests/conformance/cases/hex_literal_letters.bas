' &H as RC.EXE reads it: the whole alphanumeric run after it, with the
' letters that aren't hex digits dropped — &hHE is 14 (RapidQ's own
' sound/SBLIB.BAS has it), &hG1 is 1, a bare &h is 0 (docs/windows-dll-calls.md §4)
PRINT &hHE; " "; &hHA; " "; &hH10; " "; &hHH1; " "; &hHHE
PRINT &hG1; " "; &hZ; " "; &h
A = &hHE: PRINT A; " "; &hHE + 1; " "; &hXE
PRINT &hhE; " "; &HhE; " "; &HHE
