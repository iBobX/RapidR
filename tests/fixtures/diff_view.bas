' RDIFFVIEW (docs/ide-components.md 3.2): two versions of a program
' compared — three hunks (a changed line, an added comment, a changed
' line). The user clicks the first hunk's Accept button, goes to the next
' hunk with F7 and rejects it with Backspace (the third becomes the
' current one); OnHunkChange hears both. The program reads HunkCount,
' each HunkState, ResultText (the left text with the accepted hunk's
' lines; the undecided one counts as rejected), CurrentHunk, then
' switches to inline mode (the capture shows it).
DECLARE SUB HunkChanged (Index AS INTEGER, Accepted AS INTEGER)
DECLARE SUB Report
DIM Log AS STRING
CREATE Form AS QFORM
  Caption = "diff"
  Width = 660: Height = 430
  CREATE Diff AS RDIFFVIEW
    Left = 8: Top = 8: Width = 636: Height = 300
    OnHunkChange = HunkChanged
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 8: Top = 316: Width = 636
  END CREATE
  CREATE Info AS QLABEL
    Left = 8: Top = 336: Width = 636
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 8: Top = 360
    Caption = "Report"
    OnClick = Report
  END CREATE
END CREATE
NL$ = CHR$(10)
Q$ = CHR$(34)
Diff.LeftText = "SUB Greet(Name AS STRING)" + NL$ + "  PRINT " + Q$ + "Hello, " + Q$ + "; Name" + NL$ + "END SUB" + NL$ + "" + NL$ + "SUB Count" + NL$ + "  FOR I = 1 TO 10" + NL$ + "    PRINT I" + NL$ + "  NEXT I" + NL$ + "END SUB"
Diff.RightText = "SUB Greet(Name AS STRING)" + CHR$(13) + NL$ + "  PRINT " + Q$ + "Hello, " + Q$ + "; Name; " + Q$ + "!" + Q$ + CHR$(13) + NL$ + "END SUB" + NL$ + "" + NL$ + "' Counts to five" + NL$ + "SUB Count" + NL$ + "  FOR I = 1 TO 5" + NL$ + "    PRINT I" + NL$ + "  NEXT I" + NL$ + "END SUB"
Log = "hunks" + STR$(Diff.HunkCount) + " cur" + STR$(Diff.CurrentHunk) + " |"
Form.ShowModal

SUB HunkChanged (Index AS INTEGER, Accepted AS INTEGER)
  Log = Log + " " + STR$(Index) + ":" + STR$(Accepted)
END SUB

FUNCTION Flat$ (S AS STRING) AS STRING
  R$ = ""
  FOR I = 1 TO LEN(S)
    C$ = MID$(S, I, 1)
    IF C$ = CHR$(10) THEN R$ = R$ + "/" ELSE R$ = R$ + C$
  NEXT I
  Flat$ = R$
END FUNCTION

SUB Report
  S$ = ""
  FOR I = 0 TO Diff.HunkCount - 1
    S$ = S$ + STR$(Diff.HunkState(I))
  NEXT I
  Lbl.Caption = Log + " | states" + S$ + " | cur" + STR$(Diff.CurrentHunk) + " acc" + STR$(Diff.AcceptedCount) + " rej" + STR$(Diff.RejectedCount)
  Info.Caption = Flat$(Diff.ResultText)
  Diff.Mode = "inline"
END SUB
