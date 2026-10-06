' QCANVAS.Rotate turns the canvas as QBITMAP.Rotate does; QCANVAS.Get and
' .Put (in RC.EXE's member table, not in RapidQ's manual) take nothing, draw
' and keep nothing and give 0: a Put after a Get doesn't bring the pixels
' back. The expected output is RC.EXE's (RapidQ 2006, in the Windows VM).
CREATE Form AS QFORM
  Width = 300: Height = 200
  CREATE C AS QCANVAS
    Left = 0: Top = 0: Width = 40: Height = 30
  END CREATE
END CREATE
Form.Show
C.FillRect(0, 0, 40, 30, &HFF)
PRINT "pixel "; HEX$(C.Pixel(5, 5))
X = C.Get
PRINT "get="; X
C.FillRect(0, 0, 40, 30, &HFF0000)
Y = C.Put
PRINT "put="; Y
PRINT "pixel "; HEX$(C.Pixel(5, 5))
C.FillRect(0, 0, 40, 30, &HFF0000)
C.Pset(0, 0, &H123456)
C.Line(10, 10, 18, 10, &HFF)
C.Rotate(10, 10, 90)
PRINT "rot "; HEX$(C.Pixel(10, 5)); " "; HEX$(C.Pixel(15, 10)); " "; HEX$(C.Pixel(39, 29)); " "; HEX$(C.Pixel(0, 0)); " "; HEX$(C.Pixel(30, 5))
C.Rotate(10, 10, -90)
PRINT "back "; HEX$(C.Pixel(15, 10)); " "; HEX$(C.Pixel(10, 5))
Form.Close
