' A QBITMAP's own font takes Font.AddStyles / DelStyles as a component's
' does (`= n` and the method) — the .expected is RC.EXE's output
' (docs/rapidq-ground-truth.md); RapidQ's Sokoban draws its bold labels
' so (`plaat.font.addstyles=0`).
DIM B AS QBITMAP
B.Width = 100: B.Height = 30
PRINT B.Font.Bold; " "; B.Font.Name; " "; B.Font.Size
B.Font.Name = "Arial": B.Font.AddStyles = 0: B.Font.Size = 8
PRINT B.Font.Bold; " "; B.Font.Name; " "; B.Font.Size
W1 = B.TextWidth("bestuur magazijnmedewerker")
B.Font.DelStyles = 0
W2 = B.TextWidth("bestuur magazijnmedewerker")
PRINT B.Font.Bold; " "; W1 > W2 + 10
B.Font.AddStyles(0, 1)
PRINT B.Font.Bold; " "; B.Font.Italic
B.Font.DelStyles(1)
PRINT B.Font.Bold; " "; B.Font.Italic
DIM C AS QCANVAS
C.Font.AddStyles = 0
PRINT C.Font.Bold
