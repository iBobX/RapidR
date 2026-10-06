# Data science

RapidR's own components for numbers, tables, charts and JSON — not in
RapidQ, and usable from any RapidR program, console or GUI. Every member
name, with its aliases and where it works: [reference/data-science.md](reference/data-science.md).

| Component | Backed by (desktop) | |
|---|---|---|
| `RNUM` | ndarray | a one-dimensional array of numbers, in the manner of NumPy |
| `RDATAFRAME` | polars | a table, in the manner of pandas |
| `RPLOT` | plotters | charts saved as PNG or shown in a QIMAGE, in the manner of Matplotlib |
| `RJSON` | serde_json | JSON documents |

On the desktop (native and interpreted) these use the Rust libraries
above; in the browser they have their own, smaller implementation, so a
few members are desktop-only or web-only (the reference marks them).

## RNUM

```basic
DIM a AS RNum
a.arange 0, 10, 1                ' 0 … 9   (also linspace, zeros, ones, full, fromlist)
PRINT a.sum; " "; a.mean; " "; a.max      ' 45 4.500000000 9
a.multiply 2                     ' element-wise, in place
PRINT a.tolist                   ' 0,2,4,6,8,10,12,14,16,18

DIM b AS RNum
b.fromlist "1,2,3,4,5"
DIM c AS RNum
c.linspace 0, 1, 5
c.square
PRINT c.dot("b")                 ' another array is named by its component's name
```

Methods: creation (`arange`, `linspace`, `zeros`, `ones`, `full`,
`fromlist`), aggregates (`sum`, `mean`, `min`, `max`, `std`, `var`,
`median`, `argmin`, `argmax`, `count`, `ptp`), element-wise math in place
(`sin`, `sqrt`, `log`, `abs`, `round`, …), arithmetic with a number or
another array (`add`, `subtract`, `multiply`, `divide`, `power`, `mod`,
`clip`), ordering (`sort`, `reverse`, `unique`, `shuffle`, `append`,
`slice`), `cumsum` / `cumprod` / `diff`, `dot` / `norm` / `normalize`,
`any` / `all` / `where` / `searchsorted`, random numbers (`rand`, `randn`,
`uniform`, `randint`, `choice`), `tolist`. Properties: `Size` (`Length`),
`Data` (the values as `"1,2,3"`, settable), `NDim`, `DType`.

## RDATAFRAME

```basic
DIM df AS RDataFrame
df.loadfromcsv "people.csv"            ' name,age,city
PRINT df.rowcount; "x"; df.colcount; " "; df.columns   ' 4x3 name,age,city
df.filter "age", ">", "30"             ' >, <, >=, <=, =, !=, contains
df.sort "name", 1                      ' 1 ascending
PRINT df.cell(0, 1)                    ' row 0, column 1
df.savetocsv "older.csv"
df.togrid "Grid1"                      ' fill a QSTRINGGRID with it (headers and cells)
```

Methods change the frame in place: I/O (`loadfromcsv`, `savetocsv`,
`loadfromjson`, `savetojson`), selection (`head`, `tail`, `select`,
`cell(row, col)`, `cellbyname(row, name)`, `setcell`, `iloc`), `sort`,
`filter`, `query`, `groupby(column, function)` (`mean`, `sum`, `count`,
`min`, `max`, `first`, `last` of every other column), columns (`drop`,
`rename`, `addcolumn`), missing data (`fillna`, `dropna`), statistics
(`describe` — the frame becomes the summary — `value_counts`, `nunique`,
`corr`), sampling (`sample`, `nlargest`, `nsmallest`), joins (`merge`,
`concat`), transforms (`transpose`, `apply`, `replace`), `togrid`.
Properties: `RowCount`, `ColCount`, `Columns`, `Shape`, `Empty`.

Known issues in 2.117.0 on the desktop: `ToString` / `Print` show only the
frame's shape, and `Cell` returns a text cell with double quotes around it
(`"bob"`).

## RPLOT

A chart's data are RNUM components, named by their names:

```basic
DIM x AS RNum
DIM y AS RNum
x.linspace 0, 6.28, 50
y.linspace 0, 6.28, 50
y.sin

DIM plt AS RPlot
plt.title = "Sine"
plt.xlabel = "x"
plt.ylabel = "sin(x)"
plt.grid = 1
plt.plot "x", "y", "sin(x)", "steelblue"   ' x array, y array, label, colour
plt.legend
plt.savefig "sine.png"
```

Series: `plot`, `bar`, `barh`, `scatter`, `step`, `area` (x, y, label,
colour), `hist(data, bins, label, colour)`, `pie(data, label, colours)`;
`hline` / `vline`, `annotate(text, x, y, colour)`, `legend`, `xlim` /
`ylim`, `figsize(w, h)`, `savefig(file)`, `clear`. Properties: `Title`,
`XLabel`, `YLabel`, `Grid`, `Width`, `Height`, `DPI`. Colours are names
(`red`, `steelblue`, `coral`, … — 30 or so) or `#RRGGBB`; series without one
take the next colour of a palette. Chart text is drawn in the built-in
Liberation Sans, the same on every system.

To show a chart in a window: `Image1.LoadFromPlot plt` (a QIMAGE), drawn
from memory with no file. On the web, RPLOT draws on the page.

## RJSON

```basic
DIM j AS RJson
j.Parse "{" + CHR$(34) + "a" + CHR$(34) + ": {" + CHR$(34) + "b" + CHR$(34) + ": [1, 2, 3]}}"
PRINT j.Get("a.b.1")             ' 2  (dotted paths, array indexes from 0)
j.Set "count", 5
PRINT j.Has("count")             ' 1
PRINT j.Stringify                ' {"a":{"b":[1,2,3]},"count":5}
```

`Parse`, `Get(path)`, `Set path, value`, `Has`, `Remove`, `Keys`,
`Stringify`, `Prettify`, `LoadFile`, `SaveFile`. (Remember that `""`
inside a string isn't a quote in RapidQ's BASIC: write `CHR$(34)`, or use
`$ESCAPECHARS ON` and `\"`.)
