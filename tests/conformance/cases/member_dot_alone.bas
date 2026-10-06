' A `.` with no member right after it: RC.EXE (RapidQ 2006, run in the
' Windows VM: docs/rapidq-ground-truth.md) stops with "Member  not part of
' class FORM" (the member it read is empty) whether the dot ends the line
' (the next line is never joined to it: `Form.` then `Nope x`), a space
' follows it (`Form. Caption`), or it is WITH's (`.` alone); after a member
' the members read so far are named (`Form.Font.`: "Member FONT. not part
' of class FORM"). Each line below was one probe (RC stops at the first
' error): RC said the same for each, at that line.
DIM Form AS QFORM
Form.
Nope x
PRINT Form.
Form. Caption = "hi"
Form.Font.
WITH Form
.
END WITH
