' QIMAGELIST.Draw(Target, X, Y, Index): image Index drawn at (X, Y) on a
' QBITMAP, a QCANVAS or a QIMAGE (RC.EXE also takes a QDXSCREEN, not a form);
' an Index out of range draws nothing; a mask colour's pixels are left
' out. The expected output is RC.EXE's (RapidQ 2006, in the Windows VM).
DIM B AS QBITMAP
B.Width = 16: B.Height = 16
B.FillRect(0, 0, 16, 16, &HFF)
B.FillRect(4, 4, 12, 12, &HFF00)
B.SaveToFile "imagelist_draw.bmp"
DIM L AS QIMAGELIST
L.AddBMPFile "imagelist_draw.bmp", &H123456
L.AddBMPFile "imagelist_draw.bmp", &HFF
PRINT "count"; L.Count

DIM T AS QBITMAP
T.Width = 40: T.Height = 30
T.FillRect(0, 0, 40, 30, &HFF0000)
L.Draw T, 5, 6, 0
PRINT HEX$(T.Pixel(5, 6)); " "; HEX$(T.Pixel(10, 12)); " "; HEX$(T.Pixel(4, 6)); " "; HEX$(T.Pixel(20, 21)); " "; HEX$(T.Pixel(21, 22))
L.Draw T, 30, 0, 5
L.Draw T, 30, 0, -1
PRINT HEX$(T.Pixel(31, 1))
T.FillRect(0, 0, 40, 30, &HFF0000)
L.Draw T, 20, 10, 1
PRINT HEX$(T.Pixel(20, 10)); " "; HEX$(T.Pixel(25, 15)); " "; HEX$(T.Pixel(35, 25))

CREATE Form AS QFORM
  Width = 300: Height = 200
  CREATE C AS QCANVAS
    Left = 0: Top = 0: Width = 40: Height = 30
  END CREATE
  CREATE I AS QIMAGE
    Left = 50: Top = 0: Width = 40: Height = 30
  END CREATE
END CREATE
Form.Show
C.FillRect(0, 0, 40, 30, &HFF0000)
L.Draw C, 2, 3, 0
PRINT "canvas "; HEX$(C.Pixel(2, 3)); " "; HEX$(C.Pixel(8, 9)); " "; HEX$(C.Pixel(1, 3))
L.Draw I, 2, 3, 0
PRINT "image "; HEX$(I.Pixel(2, 3)); " "; HEX$(I.Pixel(8, 9))
Form.Close
KILL "imagelist_draw.bmp"
