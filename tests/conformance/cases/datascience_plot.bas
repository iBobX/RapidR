' RPLOT on the one data-science model (rapidr_value::datascience): its
' members and properties are the same in native builds, the interpreter and
' the browser; only the drawing is each runtime's (a PNG on the desktop, a
' canvas on the web), so this case checks the model, not the pixels.
DIM x AS RNum
DIM y AS RNum
DIM p AS RPlot

PRINT p.Title; "|"; p.Width; " "; p.Height; " "; p.DPI; " "; p.Grid
p.Title = "Sine"
p.XLabel = "x"
p.YLabel = "sin(x)"
p.Grid = 1
PRINT p.Title; " "; p.XLabel; " "; p.YLabel; " "; p.Grid
p.settitle "Waves"
p.set_xlabel "angle"
p.ylabel "value"
PRINT p.Title; " "; p.XLabel; " "; p.YLabel
p.DPI = 80
p.figsize 5, 4
PRINT p.Width; " "; p.Height
p.Width = 7
p.Height = 300
PRINT p.Width; " "; p.Height; " "; p.DPI

x.linspace 0, 6.28, 20
y.linspace 0, 6.28, 20
y.sin
p.plot x, y, "sin(x)", "steelblue"
p.plot "x", "y", "dashed", "red", "--"
p.scatter x, y, "points"
p.bar "1,2,3", "4,5,6", "bars", "#336699"
p.barh "1,2,3", "4,5,6"
p.step x, y
p.area x, y, "area", "coral"
p.fill_between x, y
p.hist y, 5, "histogram"
p.histogram "1,2,2,3,3,3", 3
p.hline 0.5, "gray"
p.axhline 0
p.vline 3.14
p.axvline 1
p.annotate "peak", 1.57, 1, "black"
p.legend
p.xlim 0, 7
p.ylim -1.5, 1.5
p.xscale "linear"
p.yscale "linear"
p.addseries "added", "1,4,9"
p.grid 0
PRINT p.Grid
p.savefig "tests/conformance/.work/ds_plot.png"
p.render
p.clear
PRINT p.Title; "|"; p.Width

p.pie "35,25,40", "A,B,C", "red,green,blue"
p.save "tests/conformance/.work/ds_pie.png"
p.show
PRINT "done"
