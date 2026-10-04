' RSQLITE is SQLite itself on every runtime (native, interpreter, web):
' a query decides by its statement whether it returns rows, values come as
' the same text everywhere, quoted values may hold anything.
DIM DB AS RSQLITE
DIM r AS INTEGER

SUB ShowRows
  DIM i AS INTEGER
  DIM line AS STRING
  WHILE DB.FetchRow
    line = ""
    FOR i = 0 TO DB.ColCount - 1
      IF i > 0 THEN line = line + "|"
      line = line + DB.Row(i)
    NEXT
    PRINT line
  WEND
END SUB

SUB Failed(Msg AS STRING)
  PRINT "OnError: "; Msg
END SUB

r = DB.Connect(":memory:")
PRINT "connect"; r; " connected"; DB.Connected
r = DB.Query("CREATE TABLE people (id INTEGER PRIMARY KEY, name TEXT, note TEXT, score REAL, n INTEGER)")
PRINT "create"; r
DB.Query("INSERT INTO people (name, note, score, n) VALUES ('Ann', 'a=b', 2.5, 10)")
DB.Query("INSERT INTO people (name, note, score, n) VALUES ('Bob', 'x AND y', 3, NULL)")
DB.Query("INSERT INTO people (name, note, score, n) VALUES ('O''Neil', 'it''s, a list', 0.1, -7)")
DB.Query("INSERT INTO people (name, note, score, n) VALUES ('Eve', NULL, 1e20, 123456789012)")

PRINT "-- all"
DB.Query("SELECT id, name, note, score, n FROM people ORDER BY id")
PRINT DB.RowCount; " rows,"; DB.ColCount; " columns,"; DB.FieldCount; " fields"
ShowRows

PRINT "-- quoted values holding = AND ' ,"
DB.Query("SELECT name FROM people WHERE note = 'a=b'")
ShowRows
DB.Query("SELECT name FROM people WHERE note = 'x AND y'")
ShowRows
DB.Query("SELECT name, n FROM people WHERE note = 'it''s, a list' AND n = -7")
ShowRows
DB.Query("SELECT name FROM people WHERE name = 'O''Neil' OR note = 'a=b' ORDER BY id")
ShowRows

PRINT "-- integers and reals as text"
DB.Query("SELECT typeof(score), typeof(n), score * 2, n / 4, 7 / 2.0, 10 / 4, 1.0 * 3 FROM people WHERE id = 1")
ShowRows

PRINT "-- WITH, RETURNING, EXPLAIN"
DB.Query("WITH rich AS (SELECT name, score FROM people WHERE score > 1) SELECT name FROM rich ORDER BY score DESC")
ShowRows
DB.Query("INSERT INTO people (name, score) VALUES ('Zed', 4.25) RETURNING id, name, score")
PRINT DB.RowCount; " returned"
ShowRows
DB.Query("EXPLAIN SELECT * FROM people")
IF DB.RowCount > 0 AND DB.ColCount > 0 THEN PRINT "explain gives rows"

PRINT "-- several statements, the last rows"
r = DB.Query("CREATE TABLE tags (t TEXT); INSERT INTO tags VALUES ('red'); INSERT INTO tags VALUES ('blue'); SELECT COUNT(*) FROM tags")
PRINT r
ShowRows

PRINT "-- a statement without rows keeps the last ones"
DB.Query("SELECT name FROM people ORDER BY id")
DB.FetchRow
PRINT DB.Row(0)
DB.Query("UPDATE people SET n = 1 WHERE id = 5")
PRINT DB.RowCount
DB.FetchRow
PRINT DB.Row(0)

PRINT "-- RowSeek, FetchField, Row out of range"
DB.RowSeek(2)
DB.FetchRow
PRINT DB.Row(0)
DB.RowSeek(0)
DB.FetchRow
PRINT DB.Row(0); "["; DB.Row(5); "]"
DB.Query("SELECT 1, 2, 3")
DB.FetchRow
r = 0
WHILE DB.FetchField
  r = r + 1
WEND
PRINT r; " fields"
DB.FieldSeek(2)
PRINT DB.FetchField; DB.FetchField
PRINT DB.FetchRow

PRINT "-- the desktop's SQLite settings"
DB.Query("PRAGMA foreign_keys")
ShowRows
DB.Query("SELECT soundex('Robert')")
ShowRows

PRINT "-- errors, worded as SQLite words them"
DB.OnError = Failed
PRINT DB.Query("SELECT * FROM nope")
PRINT DB.Query("SELEC 1")
PRINT DB.Query("SELECT nofunc(1)")
PRINT DB.Query("INSERT INTO people (id, name) VALUES (1, 'again')")
PRINT DB.EscapeString("it's")
DB.Close
r = DB.Query("SELECT 1")
PRINT "connected"; DB.Connected; " fetch"; DB.FetchRow; " query"; r
