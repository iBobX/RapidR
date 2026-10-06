' RPLOT's charts drawn by the one renderer every runtime uses (the UI
' kernel's ops through rapidr-ui-render): the same picture — pixel for
' pixel — in native builds, the interpreter and the browser. A checksum of
' a chart's pixels (Image.LoadFromPlot, then Pixel), and its size.
DIM m AS RNUM, quito AS RNUM, lima AS RNUM, sales AS RNUM
DIM p AS RPLOT, q AS RPLOT
DIM img AS QIMAGE
img.AutoSize = 1

FUNCTION Checksum (w AS INTEGER, h AS INTEGER) AS DOUBLE
    DIM x AS INTEGER, y AS INTEGER, s AS DOUBLE
    FOR y = 0 TO h - 1 STEP 3
        FOR x = 0 TO w - 1 STEP 3
            s = s + img.Pixel(x, y) MOD 997 * (x + 1)
        NEXT
    NEXT
    Checksum = s
END FUNCTION

m.Arange(1, 7, 1)
lima.FromList("23.1,24.0,23.5,21.9,19.8,18.2")
quito.FromList("14.2,14.3,14.1,14.4,14.6,14.5")
p.Width = 240: p.Height = 160
p.Title = "Mean temperature": p.XLabel = "month"
p.Plot(m, lima, "Lima")
p.Plot(m, quito, "Quito", "", "--")
p.Legend
img.LoadFromPlot(p)
PRINT "lines: "; img.Width; "x"; img.Height; " "; Checksum(240, 160)

sales.FromList("12,15,9,18")
q.Width = 200: q.Height = 150
q.Title = "Sales"
q.Bar("Q1,Q2,Q3,Q4", sales, "units", "steelblue")
img.LoadFromPlot(q)
PRINT "bars: "; Checksum(200, 150)

q.Clear
q.Pie("45,25,20,10", "North,South,East,West")
img.LoadFromPlot(q)
PRINT "pie: "; Checksum(200, 150)
