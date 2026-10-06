# Databases

| Component | Database | Where |
|---|---|---|
| `RSQLITE` (RapidR's) | SQLite itself (rusqlite; compiled to WebAssembly in the browser) | everywhere |
| `QMYSQL` / `RMYSQL` (RapidQ's) | a MySQL or MariaDB server | desktop (browsers can't open raw TCP connections) |

Both share one model for results, parameters and events: what works with
one works with the other.

## SQLite

```basic
DIM db AS RSQLITE
db.OnError = DbError
db.Connect "app.db"                      ' a file; ":memory:" for none
db.Query "CREATE TABLE IF NOT EXISTS people (name TEXT, age INTEGER)"
db.Query "INSERT INTO people VALUES (?, ?), (?, ?)", "Ann", 34, "Bob", 25

db.AddParam 30
db.Query "SELECT name, age FROM people WHERE age > ? ORDER BY name"
PRINT "rows "; db.RowCount; " cols "; db.ColCount
WHILE db.FetchRow
    PRINT db.Row(0); " is "; db.Row(1)
WEND
PRINT db.QueryScalar("SELECT COUNT(*) FROM people")
db.Close

SUB DbError (Message AS STRING)
    PRINT "error: "; Message
END SUB
```

```
rows 1 cols 2
Ann is 34
2
```

- `Query sql [, values…]` runs every statement in `sql`, in order. A
  statement with result columns (`SELECT`, `PRAGMA`, `WITH … SELECT`,
  `INSERT … RETURNING`) gives the component its rows: `RowCount`,
  `ColCount` / `FieldCount`, `FetchRow` (1 while there is a row), `Row(i)`
  (column i of the current row, from 0, as text; NULL is ""), `RowSeek`,
  `FieldSeek`, `FetchField`.
- `QueryScalar(sql)` returns the first column of the first row.
- `EscapeString(s)` doubles single quotes — but prefer parameters.
- Events: `OnConnect`, `OnQueryDone`, `OnDisconnect`, `OnError (Message)`.
- In a browser, `Connect "app.db"` opens the project's `app.db` (built into
  the page) in memory for the session; changes aren't kept after the page
  closes.

## Parameters: no SQL injection

Values after the SQL, and values queued with `AddParam`, are bound to its
`?` placeholders in order (`ClearParams` empties the queue; an array
argument is spread out). A bound value is data, never SQL, so text from a
user can't change the statement. The number of values must match the
placeholders, or the query is an error and doesn't run.

## MySQL

RapidQ's QMYSQL, with its members:

```basic
DIM my AS QMYSQL
IF my.Connect("localhost", "user", "password") THEN
    my.SelectDB "shop"
    my.Query "SELECT id, name FROM items WHERE price < ?", 10
    WHILE my.FetchRow
        PRINT my.Row(0); " "; my.Row(1)
    WEND
    my.Close
END IF
```

`Connect(host, user, password [, database])` (or the `Host`, `User`,
`Password`, `DB` and `Port` properties; port 3306 by default), `Connected`,
`SelectDB`, `DB(i)` / `DBCount` (the server's databases), `EscapeString`,
and the same results, parameters and events as SQLite. The connection is
not encrypted (the client is built without TLS).
