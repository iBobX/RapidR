' RPLOT (one implementation on every runtime: rapidr-value's chart model,
' drawn by the UI kernel's renderer): properties, series from arrays, lists
' and category names, the size in inches, SaveFig's PNG at 1x and 2x.
DIM x AS RNUM, y AS RNUM, p AS RPLOT
x.Linspace(0, 6.28, 20)
y.Linspace(0, 6.28, 20)
y.Sin
p.Title = "Sine": p.XLabel = "x": p.YLabel = "sin(x)": p.Grid = 1
p.Width = 320: p.Height = 200
p.Plot(x, y, "sin")
p.Scatter(x, y, "points", "coral")
p.HLine(0, "gray")
p.Legend
PRINT p.Title; " "; p.XLabel; " "; p.YLabel; " "; p.Width; "x"; p.Height; " grid "; p.Grid; " series "; p.Count
p.SaveFig("tests/conformance/.work/ds_plot.png")
n = FILELEN("tests/conformance/.work/ds_plot.png")
PRINT "png saved: "; IIF(n > 2000, "yes", "no")
p.SaveFig("tests/conformance/.work/ds_plot2x.png", 2)
PRINT "2x is larger: "; IIF(FILELEN("tests/conformance/.work/ds_plot2x.png") > n, "yes", "no")
p.Clear
PRINT "cleared: series "; p.Count; ", size kept "; p.Width; "x"; p.Height
p.Bar("North,South,East", "3,5,2", "units", "steelblue")
p.Hist("1,2,2,3,3,3,4", 3)
p.Pie("45,25,20,10", "N,S,E,W")
PRINT "series "; p.Count
p.FigSize(4, 3)
PRINT "figsize 4x3 at "; p.DPI; " dpi: "; p.Width; "x"; p.Height
p.SaveFig("tests/conformance/.work/ds_pie.png")
PRINT "pie saved: "; IIF(FILELEN("tests/conformance/.work/ds_pie.png") > 2000, "yes", "no")
