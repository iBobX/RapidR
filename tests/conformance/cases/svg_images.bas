' SVG images wherever a bitmap file goes (RapidR's addition): QBITMAP's
' LoadFromFile, QIMAGELIST's AddBMPFile, Draw with soft edges blended.
DIM F AS STRING
F = "rr_test_icon.svg"
OPEN F FOR OUTPUT AS #1
PRINT #1, "<svg xmlns='http://www.w3.org/2000/svg' width='8' height='6'>"
PRINT #1, "<rect x='0' y='0' width='4' height='6' fill='#0000ff'/>"
PRINT #1, "<rect x='4' y='0' width='4' height='6' fill='#00ff00' fill-opacity='0.5'/>"
PRINT #1, "</svg>"
CLOSE #1
DIM B AS QBITMAP
B.LoadFromFile F
PRINT B.Width; " "; B.Height; " "; HEX$(B.Pixel(1, 1))
DIM IL AS QIMAGELIST
IL.Width = 8
IL.Height = 6
IL.AddBMPFile F, 0
PRINT IL.Count
DIM D AS QBITMAP
D.Width = 10
D.Height = 10
D.FillRect 0, 0, 10, 10, &HFFFFFF
D.Draw 0, 0, B.BMP
PRINT HEX$(D.Pixel(1, 1)); " "; HEX$(D.Pixel(6, 1)); " "; HEX$(D.Pixel(9, 9))
KILL F
