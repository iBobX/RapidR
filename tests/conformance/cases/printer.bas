' The global PRINTER (A4 at 300 dpi, Helvetica metrics) and PLAYWAV "" (stop).
' Aborts the document, so running the test never prints anything.
DIM F AS QFONT
F.Size = 20
PRINT Printer.PageWidth; "x"; Printer.PageHeight
Printer.Font = F
PRINT Printer.TextWidth("Hello"); " "; Printer.TextHeight("Hello")
Printer.Font.Size = 10
PRINT Printer.TextWidth("Hello")
Printer.BeginDoc
Printer.TextOut(100, 100, "one", 0, -1)
Printer.NewPage
Printer.Line(0, 0, 100, 100, 0)
PRINT Printer.PageNumber; " "; Printer.Printing
Printer.Abort
PRINT Printer.Printing; " "; Printer.Aborted
Printer.Orientation = 1
PRINT Printer.PageWidth
PLAYWAV "", 1
