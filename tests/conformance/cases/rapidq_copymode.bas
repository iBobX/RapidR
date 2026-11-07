' CopyMode is how a picture combines with what it lands on (RC.EXE, Windows
' 11, 2026-11-07; the .expected is RC.EXE's output): the 15 cmXXX raster
' operations of Windows apply to CopyRect, Draw and StretchDraw alike, on a
' QBITMAP, a QIMAGE and a QCANVAS (the same results from the w_cm_* probes
' that ran on those two in a window). A destination pixel &H0F3CC5 and a
' source pixel &H55AA33 give each mode's colour; the modes that use a
' pattern (cmMergeCopy, cmPatCopy, cmPatInvert, cmPatPaint ...) use the
' brush, and a canvas's brush is the colour last used to fill: FillRect,
' Rectangle and RoundRect with a colour, a Circle's fill, a TextOut's
' background. Line, Pset and Paint leave it alone; a fresh one is white.
$INCLUDE "RAPIDQ.INC"
DIM S AS QBITMAP
DIM B AS QBITMAP
DIM RD AS QRECT
DIM RS AS QRECT

SUB Try (Name AS STRING, Mode AS INTEGER)
  DIM A AS INTEGER
  DIM A2 AS INTEGER
  DIM A3 AS INTEGER
  B.FillRect 0, 0, 4, 4, &H0F3CC5
  B.CopyMode = Mode
  B.CopyRect RD, S, RS
  A = B.Pixel(0, 0)
  B.FillRect 0, 0, 4, 4, &H0F3CC5
  B.CopyMode = Mode
  B.Draw 0, 0, S.BMP
  A2 = B.Pixel(0, 0)
  B.FillRect 0, 0, 4, 4, &H0F3CC5
  B.CopyMode = Mode
  B.StretchDraw RD, S.BMP
  A3 = B.Pixel(0, 0)
  PRINT Name; " copyrect="; HEX$(A); " draw="; HEX$(A2); " stretchdraw="; HEX$(A3)
END SUB

SUB Reset
  B.FillRect 0, 0, 4, 4, &H0F3CC5
END SUB

SUB Check (Name AS STRING)
  DIM A AS INTEGER
  B.CopyMode = cmPatCopy
  B.CopyRect RD, S, RS
  A = B.Pixel(0, 0)
  B.CopyMode = cmSrcCopy
  PRINT Name; " brush="; HEX$(A)
END SUB

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
PRINT "default CopyMode="; HEX$(B.CopyMode)
Try "cmBlackness", cmBlackness
Try "cmDstInvert", cmDstInvert
Try "cmMergeCopy", cmMergeCopy
Try "cmMergePaint", cmMergePaint
Try "cmNotSrcCopy", cmNotSrcCopy
Try "cmNotSrcErase", cmNotSrcErase
Try "cmPatCopy", cmPatCopy
Try "cmPatInvert", cmPatInvert
Try "cmPatPaint", cmPatPaint
Try "cmSrcAnd", cmSrcAnd
Try "cmSrcCopy", cmSrcCopy
Try "cmSrcErase", cmSrcErase
Try "cmSrcInvert", cmSrcInvert
Try "cmSrcPaint", cmSrcPaint
Try "cmWhiteness", cmWhiteness

' (a fresh bitmap's brush, and what moves it)
DIM F AS QBITMAP
F.Width = 4
F.Height = 4
F.CopyMode = cmPatCopy
F.CopyRect RD, S, RS
PRINT "fresh brush="; HEX$(F.Pixel(0, 0))
Reset
Check "after FillRect 0F3CC5"
Reset
B.Circle 0, 0, 3, 3, &H112233, &H445566
Check "after Circle c=112233 fill=445566"
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
