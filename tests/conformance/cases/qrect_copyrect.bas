' Canvas / bitmap CopyRect(D, Image, S) with DIMmed QRECTs; a QRECT passed to a SUB.
DIM A AS QBITMAP
DIM B AS QBITMAP
DIM D AS QRECT
DIM Sr AS QRECT
A.Width = 20: A.Height = 20
A.FillRect(0, 0, 20, 20, &HFF)
B.Width = 30: B.Height = 30
D.Left = 5: D.Top = 5: D.Right = 15: D.Bottom = 15
Sr.Left = 0: Sr.Top = 0: Sr.Right = 10: Sr.Bottom = 10
B.CopyRect(D, A, Sr)
PRINT HEX$(B.Pixel(10, 10)); " "; HEX$(B.Pixel(2, 2))
SUB Show (R AS QRECT)
  PRINT R.Left; R.Top; R.Right; R.Bottom
END SUB
Show D
DIM E AS QRECT
PRINT E.Right
