' `List2.Handle = List1.Handle` gives a QIMAGELIST the other's pictures, as
' RapidQ does — the .expected is RC.EXE's output (RapidQ's QToolbar example
' hands its TYPE's image list the program's this way).
DIM B AS QBITMAP
B.Width = 16: B.Height = 16
B.FillRect(0, 0, 16, 16, &HFF)
B.SaveToFile "il_case.bmp"
DIM I1 AS QIMAGELIST
I1.AddBMPFile "il_case.bmp", 0
I1.AddBMPFile "il_case.bmp", 0
DIM I2 AS QIMAGELIST
PRINT I1.Count; " "; I2.Count; " "; (I1.Handle <> 0)
I2.Handle = I1.Handle
PRINT I2.Count; " "; I2.Width; " "; I2.Height
CREATE F AS QFORM
  CREATE Img AS QIMAGE
    Width = 50: Height = 30
  END CREATE
END CREATE
Img.FillRect(0, 0, 50, 30, &HC0C0C0)
Img.Draw(4, 4, I2.GetBMP(0))
PRINT HEX$(Img.Pixel(10, 10)); " "; HEX$(Img.Pixel(1, 1))
TYPE TB EXTENDS QPANEL
  L AS QIMAGELIST
  H AS INTEGER
  SUB Go
    TB.L.Handle = TB.H
    PRINT TB.L.Count
  END SUB
END TYPE
DIM T AS TB
T.H = I1.Handle
T.Go
