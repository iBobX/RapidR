' Forms RapidQ's compiler takes that RapidQ's music examples use — the
' .expected is RC.EXE's output: CONST values worked out from functions
' (CHR$), `&` stuck to decimal digits (&0 is 0, &10 is 10), a CONST with a
' dotted name read back by that name (CONST Application.Path = …), and
' a statement whose argument starts in parentheses (SLEEP(T * 2) / 600 is
' SLEEP (T * 2) / 600, not SLEEP(T * 2) and then a stray / 600).
$APPTYPE CONSOLE
CONST QCrlf=Chr$(13)+Chr$(10): CONST QQuote=Chr$(34): CONST Null=&0: CONST FALSE=0
CONST Application.Path = LEFT$("abc\def", 4)
PRINT LEN(QCrlf); QQuote; Null; FALSE
PRINT Application.Path
PRINT &0; " "; &10; " "; &17; " "; &19
DIM T AS INTEGER
T = 3
SLEEP(T * 2) / 600
PRINT "slept"
