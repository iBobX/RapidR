' RSQLITE parameter binding (RapidR's): the values after the SQL, and those
' queued with AddParam, are bound to its ? placeholders — never spliced
' into the SQL, so a value can't change what the statement does.
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

SUB Connected
  PRINT "OnConnect"
END SUB

SUB Done
  PRINT "OnQueryDone"
END SUB

SUB Gone
  PRINT "OnDisconnect"
END SUB

DB.OnError = Failed
DB.Connect(":memory:")
DB.Query("CREATE TABLE users (id INTEGER PRIMARY KEY, name TEXT, pass TEXT, age INTEGER, score REAL)")

PRINT "-- values after the SQL"
DIM nm AS STRING
DIM age AS INTEGER
DIM score AS DOUBLE
nm = "O'Neil, ""Bob"" = x AND y"
age = 42
score = 2.5
r = DB.Query("INSERT INTO users (name, pass, age, score) VALUES (?, ?, ?, ?)", nm, "secret", age, score)
PRINT r
DB.Query("INSERT INTO users (name, pass, age, score) VALUES (?, ?, ?, ?)", "Ann", "pw", 30, 1.25)
DB.Query("SELECT id, name, age, score FROM users WHERE name = ?", nm)
ShowRows

PRINT "-- each value's SQL type"
DIM nothing AS VARIANT
DB.Query("SELECT typeof(?), typeof(?), typeof(?), typeof(?), typeof(?), typeof(?)", 7, 7.5, "7", 3.0, "", nothing)
ShowRows
DB.Query("SELECT ? IS NULL, ?", nothing, 1 = 1)
ShowRows
DB.Query("SELECT ? * 2, ? || 'x', ?", 2.5, 10, 3.0)
ShowRows

PRINT "-- AddParam and ClearParams"
DB.AddParam "Ann"
DB.AddParam "pw"
DB.Query("SELECT id FROM users WHERE name = ? AND pass = ?")
ShowRows
DB.AddParam "Ann"
DB.Query("SELECT age FROM users WHERE name = ? AND pass = ?", "pw")
ShowRows
DB.AddParam "stale"
DB.ClearParams
DB.Query("SELECT COUNT(*) FROM users WHERE age > ?", 1)
ShowRows

PRINT "-- an array gives its elements"
DIM ids(1) AS INTEGER
ids(0) = 2
ids(1) = 1
DB.Query("SELECT name FROM users WHERE id IN (?, ?) ORDER BY id", ids)
ShowRows

PRINT "-- an injection attempt is only a value"
DIM typed AS STRING
typed = "x' OR '1'='1"
DB.Query("SELECT COUNT(*) FROM users WHERE pass = ?", typed)
ShowRows
DB.Query("SELECT COUNT(*) FROM users WHERE pass = '" + typed + "'")
ShowRows
DB.Query("SELECT name FROM users WHERE name = ?", "Ann'; DROP TABLE users; --")
PRINT DB.RowCount; " rows"
DB.Query("SELECT COUNT(*) FROM users")
ShowRows

PRINT "-- QueryScalar"
PRINT DB.QueryScalar("SELECT name FROM users WHERE age = ?", 30)
PRINT "["; DB.QueryScalar("SELECT name FROM users WHERE age = ?", 99); "]"

PRINT "-- wrong counts"
r = DB.Query("INSERT INTO users (name) VALUES (?)")
PRINT r
r = DB.Query("INSERT INTO users (name) VALUES (?)", "a", "b")
PRINT r
r = DB.Query("SELECT ?, ?", 1)
PRINT r
r = DB.Query("SELECT 1", 1)
PRINT r
DB.Query("SELECT COUNT(*) FROM users")
ShowRows

PRINT "-- events"
DB.OnConnect = Connected
DB.OnQueryDone = Done
DB.OnDisconnect = Gone
r = DB.Connect("")
PRINT "connect"; r
r = DB.Query("SELECT ?", 1)
PRINT "query"; r
r = DB.Query("SELECT * FROM nope")
PRINT "query"; r
DB.Close
PRINT "closed"
