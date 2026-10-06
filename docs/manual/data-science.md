# Data science

RapidR's own components for numbers, tables, charts and JSON — not in
RapidQ, and usable from any RapidR program, console or GUI. Every member
name, with its aliases and where it works: [reference/data-science.md](reference/data-science.md).

| Component | Built on | |
|---|---|---|
| `RNUM` | RapidR's own | a one-dimensional array of numbers, in the manner of NumPy |
| `RDATAFRAME` | polars | a table, in the manner of pandas |
| `RPLOT` | the UI kernel's drawing | charts shown in a QIMAGE or saved as PNG, in the manner of Matplotlib |
| `RJSON` | serde_json | JSON documents |

Each is **one implementation every runtime uses** — native builds,
interpreted programs and the browser — so a program's numbers, tables and
charts come out the same everywhere. In the browser, polars is a module of
its own (about 1.5 MB compressed) that a page loads only when its program
uses `RDATAFRAME`; a bundle (`rapidr bundle-bc`, the web IDE's Build)
carries it then.

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
`uniform`, `randint`, `choice` — the program's own random numbers, so
`RANDOMIZE n` repeats them), `tolist`, and one element at a time (`create
n`, `get i`, `set i, value`, `push value`). Properties: `Size` (`Length`,
`Count`), `Data` (the values as `"1,2,3"`, settable), `Shape` (`(5,)`),
`NDim`, `DType`, `Sum`, `Mean`, `Min`, `Max`, `Std`.

## RDATAFRAME

```basic
DIM df AS RDataFrame
df.loadfromcsv "people.csv"            ' name,age,city
PRINT df.rowcount; "x"; df.colcount; " "; df.columns   ' 4x3 name,age,city
df.filter "age", ">", "30"             ' >, <, >=, <=, =, <>, contains
df.sort "name", 1                      ' 1 ascending
PRINT df.cell(0, 1)                    ' row 0, column 1
PRINT df.column("name")                ' a column as "ann,bob,…"
df.savetocsv "older.csv"
df.togrid "Grid1"                      ' fill a QSTRINGGRID with it (headers and cells)
PRINT df.tostring                      ' the table, as polars prints it
```

Methods change the frame in place: I/O (`loadfromcsv` — a file, or CSV
text itself when it has a line break —, `savetocsv`, `loadfromjson`,
`savetojson`), building (`create`, `addcolumn(name, "v1,v2,…")`,
`addrow(v1, v2, …)`), selection (`head`, `tail`, `select`, `cell(row,
col)`, `cellbyname(row, name)`, `setcell(row, col, value)`, `column(name)`,
`iloc`), `sort`, `filter`, `query`, `groupby(column, function)` (`mean`,
`sum`, `count`, `min`, `max`, `median`, `std`, `first`, `last` of every
other column; the groups in order), columns (`drop`, `rename`), missing
data (`fillna`, `dropna`), statistics (`describe` — the frame becomes
pandas' summary — `value_counts`, `nunique`, `corr`), sampling (`sample` —
`RANDOMIZE` repeats it —, `nlargest`, `nsmallest`), joins (`merge`,
`concat`), transforms (`transpose`, `apply`, `replace`), output
(`tostring`, `print` / `show`, `info`, `togrid`). Column names match
without regard to case; a cell reads as text, `""` where a value is
missing. Properties: `RowCount`, `ColCount`, `Columns`, `Shape`, `Empty`.

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

Series: `plot` (x, y, label, colour, style: `-`, `--`, `:`, `o`, `o-`),
`bar`, `barh`, `scatter`, `step`, `area` (x, y, label, colour),
`hist(data, bins, label, colour)`, `pie(data, labels, colours)`; `hline` /
`vline`, `annotate(text, x, y, colour)`, `legend`, `grid`, `xlim` / `ylim`,
`xscale` / `yscale` (`"log"`), `xticks(names [, positions])`,
`figsize(w, h)` (inches at `DPI`), `savefig(file [, scale])`, `clear`.
Properties: `Title`, `XLabel`, `YLabel`, `Grid`, `Width`, `Height`, `DPI`,
`Legend`, `Count` (the series).

An x (or y) is an RNUM, a list of numbers (`"1,2,3"`), or **names**: bars
over their categories —

```basic
plt.bar "North,South,East", sales, "units", "steelblue"
```

Several bar series share each slot side by side. Colours are the CSS names
(`steelblue`, `royalblue`, `coral`, … all 148), `#RGB` / `#RRGGBB`, `C0` …
`C9` (the palette's) or a RapidQ colour number (`RGB(255, 0, 0)`); series
without one take the next colour of a ten-colour palette made for charts.
The look: ticks at round steps, light horizontal gridlines (`Grid = 1`:
both ways, `Grid = 0`: none), the legend where it hides the fewest points,
percentages on pie slices with their names beside, and the current theme's
colours (`$THEME dark` draws dark charts).

To show a chart in a window: `Image1.LoadFromPlot plt` (a QIMAGE). The
chart is drawn by the UI kernel, as the windows are, in its fonts: crisp at
any screen scale (a 2× screen gets a 2× chart), while the picture's
`Pixel`s stay the chart's own size. `savefig "chart.png", 2` writes a PNG
at twice the chart's size (on the web, among the page's files).

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
