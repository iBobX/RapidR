' A QRICHEDIT's positions count in its Text, a line break two characters,
' as RapidQ's (RichEdit 1.0): SelLength = LEN(Text) selects it all, and a
' word INSTR finds in Text is selected at SelStart (RapidQ's syntax example;
' it looped forever when a line break counted one). The .expected is
' RC.EXE's output.
$APPTYPE CONSOLE
DIM F AS QFONT
CREATE Form AS QFORM
  CREATE R AS QRICHEDIT
    WordWrap = False
    Clear
    AddStrings("10 CLS", "20 PRINT 1", "30 PRINT 2")
  END CREATE
END CREATE
PRINT LEN(R.Text)
R.SelStart = 0
R.SelLength = LEN(R.Text)
PRINT R.SelStart; " "; R.SelLength
R.SelAttributes = F
PRINT R.SelStart; " "; R.SelLength
N = INSTR(UCASE$(R.Text), "PRINT") - 1
PRINT N
R.SelStart = N
R.SelLength = 5
R.SelAttributes = F
PRINT R.SelStart; " "; R.SelLength
N = INSTR(N + R.SelLength, UCASE$(R.Text), "PRINT") - 1
PRINT N
R.SelLength = 0
PRINT R.SelStart; " "; R.SelLength
