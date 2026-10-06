# Data science

RapidR's own components for numbers, tables, charts and JSON — not in
RapidQ, and usable from any RapidR program, console or GUI. Every member
name, with its aliases and where it works: [reference/data-science.md](reference/data-science.md).

| Component | |
|---|---|
| `RNUM` | a one-dimensional array of numbers, in the manner of NumPy |
| `RDATAFRAME` | a table, in the manner of pandas |
| `RPLOT` | charts saved as PNG or shown in a QIMAGE, in the manner of Matplotlib |
| `RJSON` | JSON documents |

RNUM, RDATAFRAME and RPLOT are one implementation (RapidR's own, in pure
Rust, with no data library underneath) that native builds, interpreted
programs and the browser all run, so every member gives the same result
everywhere. Only drawing a chart is each runtime's own: a PNG on the
desktop, a canvas on the web page.

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
`uniform`, `randint` — both ends included —, `choice`), one element
(`get(i)`, `set i, value` — past the end the array grows —, `push`),
`tolist`, `print`. Properties: `Size` (`Length`, `Count`), `Data` (the
values as `"1,2,3"`, settable), `Shape` (`(3,)`), `NDim`, `DType`, and the
aggregates `Sum`, `Mean`, `Min`, `Max`, `Std`. Numbers print as RapidR
prints them (`0.1 + 0.2` is `0.3`).

## RDATAFRAME

```basic
DIM df AS RDataFrame
df.loadfromcsv "people.csv"            ' name,age,city
PRINT df.rowcount; "x"; df.colcount; " "; df.columns   ' 4x3 name,age,city
df.filter "age", ">", 30              ' =, <>, >, <, >=, <=, contains, startswith, endswith
df.sort "name", 1                      ' 1 ascending, 0 descending
PRINT df.cell(0, 1)                    ' row 0, column 1 (or its name): the text, "30"
df.print                               ' the frame as a table, below
df.savetocsv "older.csv"
df.togrid "Grid1"                      ' fill a QSTRINGGRID with it (headers and cells)
```

`Print` (and `PRINT df.ToString`) shows the frame as a plain-text table, the
same on every runtime — numbers right-aligned, a missing value as `null`,
the first and last ten rows of a longer frame:

```text
name  age  city
----  ---  ----
bob    30  NYC
amy    25  LA
[2 rows x 3 columns]
```

A cell is its text as read (`cell` and `cellbyname` return it as is); an
empty CSV field, `NA` or JSON's `null` is a missing value. A column's type
(`dtypes`: `i64`, `f64`, `bool`, `str`) is what its cells are, and decides
how it sorts and compares: numbers as numbers, text as text.

Methods change the frame in place: I/O (`loadfromcsv` and `loadfromjson`
take a file — or the data itself, `"a,b" + CHR$(10) + "1,2"` —, `savetocsv`,
`savetojson`: an array of records), selection (`head`, `tail`, `select`,
`cell(row, col)`, `cellbyname(row, name)`, `setcell row, col, value` — past
the end the frame grows —, `iloc`), `sort` / `sort_values` (several columns:
`"city,name"`), `filter`, `query "age > 30"`, `groupby(column, function)`
(the column first, then `mean`, `sum`, `count`, `min`, `max`, `first`,
`last`, `median` or `std` of every other column, a row a group), columns
(`drop`, `rename`, `addcolumn name, "v1,v2,…"`), missing data (`fillna`,
`dropna`), statistics (`describe` — the frame becomes the summary of its
numeric columns —, `value_counts`, `nunique`, `corr`), sampling (`sample`,
`nlargest`, `nsmallest`), joins (`merge other, on, how` — `inner`, `left`,
`right`, `outer`, `cross` —, `concat`), transforms (`transpose`, `apply`
— `upper`, `lower`, `trim`, `abs`, `round`, `sqrt`, `log`, `exp` —,
`replace`), building one (`create`, `addrow`), `info`, `togrid`.
Properties: `RowCount`, `ColCount`, `Columns`, `Shape`, `Empty`, `DTypes`.

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
`addseries(label, y, x, colour)` (a line from numbers written in place),
`hline` / `vline`, `annotate(text, x, y, colour)`, `legend`, `xlim` /
`ylim`, `xscale` / `yscale` (kept with the chart; drawn linear),
`figsize(w, h)` (inches), `savefig(file)`, `show`, `clear`. Data are RNUM
components or numbers written in place (`"35,25,40"`). Properties: `Title`,
`XLabel`, `YLabel`, `Grid`, `Width`, `Height` (pixels; under 100, inches at
the chart's `DPI`), `DPI`. Colours are names
(`red`, `steelblue`, `coral`, … — 30 or so) or `#RRGGBB`; series without one
take the next colour of a palette. On the desktop, chart text is drawn in
the built-in Liberation Sans, the same on every system.

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
