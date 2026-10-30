<!-- translated-from: databases.md 2f990f8da72b711c -->
# Bases de datos

| Componente | Base de datos | Dónde |
|---|---|---|
| `RSQLite` (el de RapidR) | el propio SQLite (rusqlite; compilado a WebAssembly en el navegador) | en todas partes |
| `RMySQL` (nombre anterior: `QMYSQL`) | un servidor MySQL o MariaDB | escritorio (los navegadores no pueden abrir conexiones TCP directas) |

Los dos comparten un solo modelo de resultados, parámetros y eventos: lo que
funciona con uno funciona con el otro —las consultas en segundo plano y un
`RDBGrid` con las filas de una consulta también—.

## SQLite

```basic
DIM db AS RSQLite
db.OnError = DbError
db.Connect "app.db"                      ' un archivo; ":memory:" para ninguno
db.Query "CREATE TABLE IF NOT EXISTS people (name TEXT, age INTEGER)"
db.Query "INSERT INTO people VALUES (?, ?), (?, ?)", "Ann", 34, "Bob", 25

db.AddParam 30
db.Query "SELECT name, age FROM people WHERE age > ? ORDER BY name"
PRINT "filas "; db.RowCount; " columnas "; db.ColCount
WHILE db.FetchRow
    PRINT db.Row(0); " tiene "; db.Row(1)
WEND
PRINT db.QueryScalar("SELECT COUNT(*) FROM people")
db.Close

SUB DbError (Message AS STRING)
    PRINT "error: "; Message
END SUB
```

```
filas 1 columnas 2
Ann tiene 34
2
```

- `Query sql [, values…]` ejecuta todas las instrucciones de `sql`, en orden. Una
  instrucción con columnas de resultado (`SELECT`, `PRAGMA`, `WITH … SELECT`,
  `INSERT … RETURNING`) le da al componente sus filas: `RowCount`,
  `ColCount` / `FieldCount`, `FetchRow` (1 mientras hay una fila), `Row(i)` (la
  columna i de la fila actual, desde 0, como texto; NULL es ""), `RowSeek`,
  `FieldSeek`, `FetchField`.
- `QueryScalar(sql)` devuelve la primera columna de la primera fila.
- `ColName(i)` y `ColType(i)` dan el nombre de la columna `i` de la última
  consulta (desde 0) y su tipo declarado (consulta
  [Nombres y tipos de columnas](#nombres-y-tipos-de-columnas)).
- `Schema` da las tablas, las vistas y las columnas de la base de datos como
  JSON, para el autocompletado de un editor de SQL (consulta
  [El editor de SQL](#el-editor-de-sql)).
- `EscapeString(s)` duplica las comillas simples, pero es mejor usar
  parámetros.
- Eventos: `OnConnect`, `OnQueryDone`, `OnDisconnect`, `OnError (Message)`; y,
  de una consulta en segundo plano, `OnRows (Count)` y `OnDone (Rows, Ms)`.
<!-- legal -->
- En un navegador, `Connect "app.db"` abre el `app.db` del proyecto (incluido en
  la página) en memoria durante la sesión; los cambios no se conservan cuando la
  página se cierra.

## Parámetros: sin inyección de SQL

Los valores que van después del SQL, y los valores en cola con `AddParam`, se
enlazan en orden a sus marcadores de posición `?` (`ClearParams` vacía la cola;
un argumento que es un arreglo (array) se despliega). Un valor enlazado es dato,
nunca SQL, así que el texto de un usuario no puede cambiar la instrucción. La
cantidad de valores tiene que coincidir con la de marcadores de posición, o la
consulta es un error y no se ejecuta.

## MySQL

`RMySQL` (nombre anterior `QMYSQL`), con sus miembros:

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

`Connect(host, user, password [, database])` (o las propiedades `Host`, `User`,
`Password`, `DB` y `Port`; puerto 3306 de forma predeterminada), `Connected`,
`SelectDB`, `DB(i)` / `DBCount` (las bases de datos del servidor),
`EscapeString`, y los mismos resultados, parámetros y eventos que SQLite.

<!-- legal -->
Guarda la contraseña en el llavero con [RSecrets](secrets.md), no en el
programa: `my.Connect("db.example.com", "shop", Secrets.Read("MySQL", "shop"))`.

### Conexiones cifradas

<!-- legal -->
RapidR cifra una conexión con MySQL o MariaDB con el TLS del sistema operativo
(Security.framework en macOS, SChannel en Windows, OpenSSL 3 en Linux).
`SSLMode` dice cómo, con los mismos modos que el cliente propio de MySQL.
Establécelo antes de `Connect`:

| SSLMode | La conexión |
|---|---|
| `Preferred` (el valor predeterminado) | Cifrada cuando el servidor puede, y si no, sin cifrar. El certificado del servidor no se comprueba. |
| `Disabled` | Nunca cifrada. |
| `Required` | Cifrada, o no hay conexión. El certificado no se comprueba. |
| `VerifyCA` | Cifrada, y el certificado del servidor está firmado por `SSLCA`, o por una autoridad en la que confía el sistema. |
| `VerifyFull` | Como `VerifyCA`, y el certificado nombra a `Host`. |

<!-- legal -->
**Solo `VerifyCA` y `VerifyFull` comprueban que sea tu servidor.** `Preferred` y
`Required` cifran, pero no comprueban quién responde: en una red en la que no
confías (el Wi-Fi de una cafetería), alguien en el medio puede hacerse pasar por
el servidor y leer la contraseña y los datos. Para un servidor en otra
computadora, usa `VerifyFull`.

```basic
DIM my AS RMySQL
my.SSLMode = "VerifyFull"
IF my.Connect("db.example.com", "shop", Secrets.Read("MySQL", "shop")) THEN
    PRINT "Cifrada: "; my.Encrypted
ELSE
    PRINT "Sin conexión."     ' OnError dice por qué
END IF
```

<!-- legal -->
- `Encrypted` es 1 mientras la conexión está cifrada, y 0 cuando es simple o no
  hay ninguna.
<!-- legal -->
- `SSLCA` es el archivo de una autoridad de certificación, `.pem` o `.der` (como
  el `ca.pem` del servidor), para `VerifyCA` y `VerifyFull`. Cuando está vacío,
  se usan las autoridades de confianza del sistema.
<!-- legal -->
- Un servidor sin TLS, con `Required` o algo más estricto, da en `OnError` "the
  server doesn't offer encrypted connections (TLS), and SSLMode asks for one".
<!-- legal -->
- Un certificado rechazado da "the server's certificate wasn't accepted". El
  certificado que un servidor crea por su cuenta es uno en el que el sistema no
  confía: da su `ca.pem` como `SSLCA`.
<!-- legal -->
- macOS rechaza cualquier certificado de servidor válido por más de 825 días,
  incluso con el `SSLCA` correcto. Los certificados que MySQL crea por sí mismo
  son válidos por diez años, así que en una Mac `VerifyCA` y `VerifyFull`
  necesitan un certificado de servidor propio; `Required` sigue cifrando.
<!-- legal -->
- Un socket Unix o una canalización con nombre (el `UnixSock` de `RealConnect`)
  nunca lleva TLS. `Preferred` se conecta sin cifrar a través de él; `Required`
  y los modos más estrictos lo rechazan.
<!-- legal -->
- Un servidor en esta misma computadora (`127.0.0.1`, `localhost`): `Preferred`
  puede pasar por el socket local propio del servidor, que nunca sale de la
  computadora, y `Encrypted` entonces dice 0. `Required` y los modos más
  estrictos siempre usan TLS.
<!-- legal -->
- La marca `CLIENT_SSL` (2048) de `RealConnect` pide `Required`.

<!-- legal -->
Antes, la conexión nunca se cifraba. Con `Preferred`, un programa que funcionaba
antes sigue conectándose a los mismos servidores. Ahora se cifra dondequiera que
el servidor tenga TLS.
y los mismos resultados, parámetros y eventos que SQLite. La conexión no está
cifrada (el cliente se compila sin TLS).

## Consultas en segundo plano

`QueryAsync` ejecuta una consulta en segundo plano: el programa y sus ventanas
siguen respondiendo mientras llegan las filas, incluso un millón de ellas.
Recibe el SQL y sus valores como `Query`.

DIM DB AS RSQLite
DB.OnRows = GotRows
DB.OnDone = Done
DB.OnError = Failed
DB.Connect "shop.db"
DB.QueryAsync "SELECT * FROM orders WHERE total > ?", 100

SUB GotRows (Count AS INTEGER)
    Status.Caption = STR$(Count) + " filas hasta ahora..."
END SUB

SUB Done (Rows AS INTEGER, Ms AS INTEGER)
    Status.Caption = STR$(Rows) + " filas en " + STR$(Ms) + " ms"
END SUB

SUB Failed (Message AS STRING)
    Status.Caption = "Detenido: " + Message     ' "cancelled" después de Cancel
END SUB

- Las filas llegan por tandas: las primeras decenas de una vez (se muestran
  lo antes posible), y después tandas más grandes. `OnRows (Count)` se dispara a
  medida que llegan, con las filas hasta ahora, y `RowCount` las sigue. `FetchRow`
  y `Row(i)` recorren las filas que ya llegaron.
- `OnDone (Rows, Ms)` dice cuántas filas hubo y cuántos milisegundos tardó la
  consulta. `OnError (Message)` dice por qué se detuvo.
- `Cancel` la detiene: `OnError` recibe `"cancelled"`. Si la consulta acababa de
  terminar, llega `OnDone` en su lugar. SQLite se interrumpe; a MySQL se le pide
  que detenga la instrucción (`KILL QUERY`, enviado por una segunda conexión).
- `Busy` es 1 mientras se ejecuta una consulta en segundo plano. Una consulta a
  la vez: mientras se ejecuta, `Query`, `QueryScalar`, `Schema` y otro
  `QueryAsync` se rechazan de inmediato con `OnError ("busy: …")`. Nunca
  esperan, así que el programa nunca se congela. Cancela la consulta que se
  ejecuta, o espera a `OnDone`.
- En el escritorio cada conexión ejecuta sus consultas en un hilo de trabajo
  propio. En un navegador, donde una página no tiene hilos, la consulta se
  ejecuta en el hilo de la página en porciones de unos pocos milisegundos, y la
  página se dibuja y atiende los clics entre ellas. Una instrucción que tiene que
  ordenar todo antes de su primera fila (`ORDER BY` sin un índice) acapara la
  página durante ese único paso.
- Un programa de consola oye los eventos mediante `DOEVENTS`:
  `DO: DOEVENTS: LOOP UNTIL Finished`.
- Con RMySQL, las celdas de una consulta en segundo plano son texto; para datos
  binarios (`RowBlob`, `SaveBlob`) usa `Query`.

Qué tan rápido, medido con
[million_rows_async.rr](../../../examples/data/million_rows_async.rr)
(`SELECT * FROM big`, cinco columnas, en un M4 Max, compilación de lanzamiento):
un millón de filas en el escritorio mostraron sus primeras filas después de 15 a
25 ms, y todas estaban dentro después de unos 200 ms. En Chromium el mismo
ejemplo con 100 000 filas mostró sus primeras filas después de 1 a 25 ms. Un
botón Ping al que se hizo clic mientras las filas llegaban en flujo se atendió de
inmediato en los dos.

![Un millón de filas que llegan en flujo a un RDBGrid en segundo plano; la línea de estado dice cuánto tardó](../images/databases/million-rows-async.png)

## Las filas de una consulta en una cuadrícula

Un `RDBGrid` cuyo `DataSource` es un `RSQLite` o un `RMySQL` muestra la última
consulta de ese componente: las filas de una consulta en segundo plano a medida
que llegan (su barra de desplazamiento crece), o las filas de un `Query`. Solo
dibuja las filas que están en pantalla, así que un millón de filas se desplazan
con fluidez. Las flechas, Page Up y Page Down, Home y End mueven su fila actual;
Ctrl+C (Cmd+C en una Mac) la copia, con sus celdas separadas por tabuladores.

CREATE Grid AS RDBGrid
    Align = alClient
    DataSource = DB
    OnTitleClick = SortBy
END CREATE

Un clic en un encabezado no ordena las filas de una base de datos en la cuadrícula:
tu programa las ordena ejecutando la consulta de nuevo con `ORDER BY`. La cuadrícula
muestra la flecha y dice en qué se hizo clic, en `SortColumn` (desde 0; -1:
ninguna) y `SortDescending`. Un segundo clic pide el otro sentido, un tercero
ningún orden. Cuando tu programa ejecuta una consulta propia, establece
`Grid.SortColumn = -1` para que la flecha desaparezca. Las filas de una consulta
nueva se muestran desde arriba, con su primera fila como la actual.

SUB SortBy (Col AS INTEGER)
    IF Grid.SortColumn < 0 THEN
        DB.QueryAsync LastSQL
    ELSE
        DB.QueryAsync "SELECT * FROM (" + LastSQL + ") ORDER BY " + STR$(Grid.SortColumn + 1) + IIF(Grid.SortDescending, " DESC", "")
    END IF
END SUB

Filtrar funciona de la misma manera: ejecuta la consulta de nuevo con un `WHERE`.

La cuadrícula conserva su orden mientras llegan las filas de tu consulta, y cuando
vuelves a establecer `DataSource` en el componente que ya muestra (enlazar antes
de cada consulta está bien): el siguiente clic en el mismo encabezado continúa
desde donde estaba, ascendente, luego descendente, luego ninguno. Establecer otro
`DataSource` empieza sin orden. Una cuadrícula enlazada a datos propios (un
`RDataFrame`, un `RCSV`) ordena su vista por sí misma, y las filas añadidas a
esos datos se ordenan al entrar.

## ¿Hay una transacción abierta?

`InTransaction` dice si hay una transacción abierta en la conexión: 1 desde
`BEGIN` hasta `COMMIT` o `ROLLBACK`, y si no, 0. Es lo que dice la propia base de
datos, establecido después de cada instrucción que ejecuta el componente, así que
leerlo no ejecuta ninguna consulta y las filas de la cuadrícula se conservan. Cerrar
la conexión revierte una transacción abierta: pregunta primero.

```basic
SUB FormClose (Action AS INTEGER)
    IF DB.InTransaction = 0 THEN EXIT SUB
    IF MESSAGEDLG("Hay una transacción abierta: al cerrar se descartan sus filas." + CHR$(10) + "¿Cerrar de todos modos?", mtWarning, mbYes OR mbNo, 0, mrNo) = mrYes THEN
        DB.Query "ROLLBACK"
    ELSE
        Action = caNone
    END IF
END SUB
```

- SQLite: `BEGIN` (o un `SAVEPOINT` fuera de una transacción) abre una;
  `COMMIT`, `END` y `ROLLBACK` la terminan. `Query`, `QueryScalar` y una consulta
  en segundo plano (antes de su `OnDone`) la establecen todas, también una
  instrucción que falló.
- MySQL y MariaDB: el servidor lo dice en su respuesta a cada instrucción que lo
  cambia (MySQL 5.7 y posteriores, MariaDB 10.3 y posteriores): `BEGIN` /
  `START TRANSACTION`, `COMMIT`, `ROLLBACK`, el primer cambio mientras
  `Autocommit` es 0, la confirmación que MySQL hace después de `CREATE`, `ALTER`
  o `DROP`, y `SET autocommit = 1`. `Autocommit` (1 o 0) es la configuración de
  MySQL, seguida de la misma manera.
- Con MySQL `InTransaction` es -1 cuando las respuestas del servidor no pudieron
  decirlo: después de una consulta de varias instrucciones cuya última da filas
  (`BEGIN; SELECT …`), después de una lectura mientras `Autocommit` es 0 (puede
  haber iniciado una), y con un servidor más antiguo. -1 cuenta como verdadero en
  un `IF`, así que una pregunta al cerrar sigue haciéndose. La respuesta de la
  siguiente instrucción lo dice de nuevo.

![El ejemplo de transacciones: una transacción iniciada y dos filas añadidas, la línea de estado dice que hay una abierta](../images/databases/transactions.png)

[transactions.rr](../../../examples/data/transactions.rr) lo muestra: Begin,
Add a row, Commit y Rollback, el estado a partir de `InTransaction`, y una
pregunta al cerrar cuya respuesta predeterminada es No (consulta los
[cuadros de mensaje](components.md#cuadros-de-mensaje-la-respuesta-segura-con-enter)).

## Nombres y tipos de columnas

`ColName(i)` y `ColType(i)` describen la columna `i` de la última consulta, desde
0, a partir de los metadatos del propio resultado: no se vuelve a ejecutar nada,
así que también funcionan en una conexión de solo lectura.

DB.Query "SELECT id, name AS who, COUNT(*) FROM people GROUP BY id"
FOR i = 0 TO DB.ColCount - 1
    PRINT DB.ColName(i); " "; DB.ColType(i)
NEXT

id INTEGER
who VARCHAR(40)
COUNT(*)

- SQLite da el tipo declarado de una columna, tal como lo declara la tabla
  (`INTEGER`, `VARCHAR(40)`, `DATE`), y "" para una expresión.
- MySQL da el tipo de la columna como lo nombra una tabla: `INT`,
  `BIGINT UNSIGNED`, `DECIMAL`, `VARCHAR`, `VARBINARY`, `TEXT`, `BLOB`,
  `DATETIME`, `ENUM`, `JSON` y así sucesivamente.
- Un nombre que aparece dos veces (`SELECT a.id, b.id`) se queda como está. Un
  `RDBGrid` lo hace único en su encabezado (`id_2`).

## El editor de SQL

Un `RCodeEditor` con `Language = "sql"` colorea el SQL, completa tus tablas y
columnas, pliega cada instrucción y ejecuta la instrucción que está en el cursor
con Ctrl+Enter. `Schema` le da las tablas de la base de datos:

Editor.Language = "sql"
Editor.SqlDialect = "sqlite"
Editor.SetSchema DB.Schema

SUB RunStatement (SQL AS STRING)        ' Editor.OnRunStatement
    DB.QueryAsync SQL
END SUB

`Schema` es JSON: `{"tables": [{"name": "people", "kind": "table",
"columns": [{"name": "id", "type": "INTEGER"}, …]}, …]}`. Para MySQL enumera las
tablas de la base de datos seleccionada, a partir de `information_schema`. Consulta
[El editor de código en tus programas](code-editor.md#sql).
