' w_cm_brush: the pattern of the raster operations that use one (cmPatCopy ...)
' is the canvas's brush. Which drawing statements set it? After each one
' (colour &H112233, a second colour &H445566 where it takes two), a CopyRect
' with cmPatCopy puts the brush colour in the destination: read back.
$INCLUDE "RAPIDQ.INC"
DIM Rep AS QFILESTREAM
DIM S AS QBITMAP
DIM B AS QBITMAP
DIM RD AS QRECT
DIM RS AS QRECT

SUB Reset
  B.FillRect 0, 0, 4, 4, &H0F3CC5
END SUB

SUB Check (Name AS STRING)
  DIM A AS INTEGER
  B.CopyMode = cmPatCopy
  B.CopyRect RD, S, RS
  A = B.Pixel(0, 0)
  B.CopyMode = cmSrcCopy
  Rep.WriteLine Name + " brush=" + HEX$(A)
END SUB

Rep.Open("w_cm_brush.txt", fmCreate)
S.Width = 4
S.Height = 4
S.FillRect 0, 0, 4, 4, &H55AA33
B.Width = 4
B.Height = 4
RD.Left = 0
RD.Top = 0
RD.Right = 2
RD.Bottom = 2
RS.Left = 0
RS.Top = 0
RS.Right = 2
RS.Bottom = 2
Check "fresh"
Reset
Check "after FillRect 0F3CC5"
Reset
B.Circle 0, 0, 3, 3, &H112233, &H445566
Check "after Circle c=112233 fill=445566"
Reset
B.Circle 0, 0, 3, 3, &H112233, -1
Check "after Circle c=112233 fill=-1"
Reset
B.Rectangle 0, 0, 3, 3, &H112233
Check "after Rectangle 112233"
Reset
B.Line 0, 0, 3, 3, &H112233
Check "after Line 112233"
Reset
B.Pset 1, 1, &H112233
Check "after Pset 112233"
Reset
B.RoundRect 0, 0, 4, 4, 2, 2, &H112233
Check "after RoundRect 112233"
Reset
B.Paint 1, 1, &H112233, &H0
Check "after Paint 112233"
Reset
B.TextOut 0, 0, "x", &H112233, &H445566
Check "after TextOut fc=112233 bc=445566"
Rep.Close
