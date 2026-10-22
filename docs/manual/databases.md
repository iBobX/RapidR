# Databases

| Component | Database | Where |
|---|---|---|
| `RSQLite` (RapidR's) | SQLite itself (rusqlite; compiled to WebAssembly in the browser) | everywhere |
| `RMySQL` (older name: `QMYSQL`) | a MySQL or MariaDB server | desktop (browsers can't open raw TCP connections) |

Both share one model for results, parameters and events: what works with
one works with the other.

## SQLite

```basic
DIM db AS RSQLite
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

`RMySQL` (older name `QMYSQL`), with its members:

```basic
DIM my AS RMySQL
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
and the same results, parameters and events as SQLite.

Keep the password in the keychain with [RSecrets](secrets.md), not in the
program: `my.Connect("db.example.com", "shop", Secrets.Read("MySQL", "shop"))`.

### Encrypted connections

RapidR encrypts a MySQL or MariaDB connection with the operating system's
TLS (Security.framework on macOS, SChannel on Windows, OpenSSL 3 on
Linux). `SSLMode` says how, with the same modes as MySQL's own client.
Set it before `Connect`:

| SSLMode | The connection |
|---|---|
| `Preferred` (the default) | Encrypted when the server can, else plain. The server's certificate isn't checked. |
| `Disabled` | Never encrypted. |
| `Required` | Encrypted, or no connection. The certificate isn't checked. |
| `VerifyCA` | Encrypted, and the server's certificate is signed by `SSLCA`, or by an authority the system trusts. |
| `VerifyFull` | As `VerifyCA`, and the certificate names `Host`. |

```basic
DIM my AS RMySQL
my.SSLMode = "Required"
IF my.Connect("db.example.com", "shop", Secrets.Read("MySQL", "shop")) THEN
    PRINT "Encrypted: "; my.Encrypted
ELSE
    PRINT "Not connected."     ' OnError says why
END IF
```

- `Encrypted` is 1 while the connection is encrypted, and 0 when it is plain
  or there is none.
- `SSLCA` is a certificate authority's file, `.pem` or `.der` (such as
  the server's `ca.pem`), for `VerifyCA` and `VerifyFull`. When it is empty,
  the system's trusted authorities are used.
- A server without TLS, under `Required` or stricter, gives `OnError` "the
  server doesn't offer encrypted connections (TLS), and SSLMode asks for
  one".
- A refused certificate gives "the server's certificate wasn't accepted".
  A server's own self-made certificate is one the system doesn't trust:
  give its `ca.pem` as `SSLCA`.
- macOS refuses any server certificate valid for more than 825 days, even
  with the right `SSLCA`. The certificates MySQL makes for itself are valid
  for ten years, so on a Mac `VerifyCA` and `VerifyFull` need a server
  certificate of your own; `Required` still encrypts.
- A Unix socket or a named pipe (`RealConnect`'s `UnixSock`) never carries
  TLS. `Preferred` connects plain through it; `Required` and stricter refuse
  it.
- A server on this same computer (`127.0.0.1`, `localhost`): `Preferred`
  may go through the server's own local socket, which never leaves the
  computer, and `Encrypted` then says 0. `Required` and stricter always
  use TLS.
- `RealConnect`'s `CLIENT_SSL` flag (2048), as RapidQ programs pass it,
  asks for `Required`.

Before, the connection was never encrypted. With `Preferred`, a program
that worked before still connects to the same servers. It is now
encrypted wherever the server has TLS.
