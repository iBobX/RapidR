' RCODEEDITOR (RapidR's code editor, the IDE's) keeps its text in the
' shared text model, as a QMEMO does: Text set before the form shows (with
' '\n' line breaks, as the IDE reads it), LineCount, Line(i), AddStrings,
' SelStart / SelLength / SelText, WhereX / WhereY; GetSubList, GotoSub and
' GotoLine (the caret there, scrolled into view); the program's changes
' fire no OnChange. Its BASIC colours and line numbers show in captures.
DECLARE SUB Go
DECLARE SUB Changed
CREATE Form AS QFORM
  Caption = "code"
  Width = 480: Height = 320
  CREATE Ed AS RCODEEDITOR
    Left = 8: Top = 8: Width = 440: Height = 200
    OnChange = Changed
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 216: Width = 460
  END CREATE
  CREATE Btn AS QBUTTON
    Top = 248
    OnClick = Go
  END CREATE
END CREATE
DIM Changes AS INTEGER
Q$ = CHR$(34)
Ed.Text = "DIM a AS INTEGER" + CHR$(10) + "SUB Hello(x AS INTEGER)" + CHR$(10) + "  PRINT " + Q$ + "hi" + Q$ + " ' greet" + CHR$(10) + "END SUB" + CHR$(10) + "FUNCTION Twice(n)" + CHR$(10) + "  Twice = n * 2.5" + CHR$(10) + "END FUNCTION"
Lbl.Caption = STR$(Ed.LineCount) + "|" + Ed.Line(1) + "|" + STR$(LEN(Ed.Text)) + "|" + STR$(INSTR(Ed.Text, CHR$(13)))
Form.ShowModal

SUB Changed
  Changes = Changes + 1
END SUB

SUB Go
  S$ = Ed.GetSubList
  P = INSTR(S$, CHR$(10))
  S$ = LEFT$(S$, P - 1) + "/" + MID$(S$, P + 1)
  Ed.GotoSub "twice"
  A$ = STR$(Ed.WhereY) + "," + STR$(Ed.SelStart)
  Ed.GotoLine 2
  Ed.SelLength = 7
  T$ = Ed.SelText
  Ed.SelText = "  BEEP"
  FOR I = 1 TO 30
    Ed.AddStrings "' more " + STR$(I)
  NEXT I
  Ed.GotoLine 33
  Lbl.Caption = Lbl.Caption + "|" + S$ + "|" + A$ + "|" + T$ + "|" + Ed.Line(2) + "|" + STR$(Ed.LineCount) + "|" + STR$(Ed.WhereY) + "|" + STR$(Changes)
END SUB
