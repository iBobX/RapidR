' w_cm_bitmap: what each CopyMode does to a QBITMAP's CopyRect and Draw of a
' solid source onto a solid destination, read back. Dest pixel &H0F3CC5,
' source pixel &H55AA33 (BGR). Writes "mode copyrect=.. draw=.." to w_cm_bitmap.txt.
$INCLUDE "RAPIDQ.INC"
DIM Rep AS QFILESTREAM
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
  Rep.WriteLine Name + " copyrect=" + HEX$(A) + " draw=" + HEX$(A2) + " stretchdraw=" + HEX$(A3)
END SUB

Rep.Open("w_cm_bitmap.txt", fmCreate)
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
Rep.WriteLine "default CopyMode=" + HEX$(B.CopyMode)
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
Rep.Close
