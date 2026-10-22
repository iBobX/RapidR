# The code editor in your programs

`RCodeEditor` is RapidR Studio's own code editor, as a component for your
programs: syntax colours, line numbers, folding, several carets, find and
replace, a minimap, markers and completion. It looks and works the same on
the desktop and in the browser, and screen readers read it as they read
Studio's editor. Every property, method and event is in the
[members reference](reference/members.md#rcodeeditor); this page shows the
common ones and the SQL mode.

```basic
CREATE Editor AS RCodeEditor
    Align = alClient
    Language = "json"                    ' or "rapidr-basic", "sql", "html" …
    Text = "{" + CHR$(34) + "a" + CHR$(34) + ": 1}"
    OnChange = Edited
END CREATE
```

- `Language` picks the colours, brackets, indentation and folding: an id
  (`rapidr-basic`, `sql`, `json`, `html`, `css`, `javascript`, `markdown`,
  `toml`, `rust`, `csv`, `plaintext`), a file name whose extension says, or
  a language definition file of your own (`.toml`).
- `LoadFromFile` and `SaveToFile` keep a file's line breaks and encoding.
- `CaretLine`, `CaretColumn`, `GotoLineColumn`, `Find`, `ReplaceAll`,
  `Fold`, `AddMarker` and the rest count lines and columns from 1.
- For a language without RapidR's language service, completion, hovers and
  signature help are yours: answer `OnCompletionRequest` with
  `ShowCompletion`, `OnHoverRequest` with `ShowHover`.

## SQL

With `Language = "sql"` the editor knows SQL by itself:

```basic
CREATE Editor AS RCodeEditor
    Align = alTop: Height = 120
    Language = "sql"
    SqlDialect = "sqlite"                ' or "mysql"; "" for both
    OnRunStatement = RunStatement
END CREATE

Editor.SetSchema DB.Schema               ' the database's tables and columns

SUB RunStatement (SQL AS STRING)
    DB.QueryAsync SQL                    ' in the background: see Databases
END SUB
```

- **Colours.** `SqlDialect` picks whose words are coloured and completed:
  `"sqlite"` (`PRAGMA`, `GLOB`, `JULIANDAY` …), `"mysql"` for MySQL and
  MariaDB (`SHOW`, `AUTO_INCREMENT`, `DATE_FORMAT` …), or `""` for both.
  `Language` still reads `"sql"`.
- **Completion.** As a word starts, the editor offers the columns of the
  tables the statement names (every table's when it names none), then the
  tables, then the dialect's keywords and functions. After `name.` it
  offers that table's columns, also through an alias (`FROM people p` …
  `p.`). After `FROM`, `JOIN`, `INTO`, `UPDATE` and `TABLE` the tables come
  first. Tab or Enter accepts.
- **Your tables.** `SetSchema(json)` gives completion your tables, views
  and columns. `DB.Schema` (an RSQLite's or RMySQL's) is ready-made JSON for
  it. You can also write it yourself: `{"tables": [{"name": "people",
  "kind": "table", "columns": [{"name": "id", "type": "INTEGER"}, …]}]}`,
  a list of such tables, or the short `{"people": ["id", "name"]}`.
  (BASIC strings have no `""` escape: build the quotes with `CHR$(34)`, or
  use `RJson`.) `SetSchema ""` clears it.
- **Run.** Ctrl+Enter (Cmd+Enter on a Mac) fires `OnRunStatement (SQL)` with
  the selection, or the statement the caret is in, up to its `;` (a `;` in a
  string or a comment doesn't count). `CurrentStatement` is the same text,
  for a Run button: `DB.QueryAsync Editor.CurrentStatement`.
- **Folding.** A statement over several lines folds, from its first line
  to its `;`, and so does a bracketed list over several lines (a
  `CREATE TABLE`'s columns, a subquery).
- **Your own completion.** With `LanguageService = 0` the editor asks your
  program instead (`OnCompletionRequest`, answered with `ShowCompletion`).

![The SQL editor completing "select c": the table's columns city and customer first, then SQLite's words](images/databases/sql-completion.png)

The example [million_rows_async.rr](../../examples/data/million_rows_async.rr)
puts it together: an SQL editor, background queries and a grid of a
million rows. See also [Databases](databases.md).
