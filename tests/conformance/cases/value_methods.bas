' Obj.Member read without parentheses: a method of the object's type is
' called (RapidQ's `WHILE DB.FetchRow`), anything else is the property
' (`UpDown.Max`) — the same on native builds, the interpreter and the web.
DIM DB AS QSQLITE
DB.Connect(":memory:")
DB.Query("CREATE TABLE t (a INTEGER, b TEXT)")
DB.Query("INSERT INTO t VALUES (7, 'seven')")
DB.Query("INSERT INTO t VALUES (8, 'eight')")
DB.Query("SELECT a, b FROM t")
WHILE DB.FetchRow
    PRINT "row "; DB.Row(0); " "; DB.Row(1)
WEND
DB.Query("SELECT a FROM t")
DIM K AS INTEGER
K = DB.FetchRow
PRINT "first fetch "; K; " "; DB.Row(0)
WITH DB
    K = .FetchRow
END WITH
PRINT "second fetch "; K; " "; DB.Row(0)
K = DB.FetchRow
PRINT "past the end "; K

DIM T AS QTRACKBAR
T.Min = 5
T.Max = 50
PRINT "trackbar "; T.Min; " "; T.Max
DIM U AS QUPDOWN
U.Min = 3
U.Max = 9
PRINT "updown "; U.Min; " "; U.Max
DIM P AS QPROGRESSBAR
P.Max = 70
PRINT "progress "; P.Max
DIM SB AS QSCROLLBAR
SB.Min = 2
SB.Max = 40
PRINT "scrollbar "; SB.Min; " "; SB.Max
