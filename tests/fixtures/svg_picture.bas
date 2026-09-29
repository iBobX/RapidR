' SVG pictures (RapidR's addition to RapidQ): a QIMAGE showing an SVG
' resource, a QIMAGELIST holding it, and drawing it on a canvas.
$RESOURCE STAR AS "rr_star.svg"
DECLARE SUB PaintIt
DECLARE SUB Report
CREATE IL AS QIMAGELIST
  Width = 48
  Height = 48
END CREATE
CREATE Form AS QFORM
  Caption = "svg picture"
  Width = 260
  Height = 160
  CREATE Img AS QIMAGE
    Left = 10 : Top = 10 : Width = 48 : Height = 48
    AutoSize = 1
  END CREATE
  CREATE Cv AS QCANVAS
    Left = 80 : Top = 10 : Width = 60 : Height = 60
    OnPaint = PaintIt
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 10 : Top = 80
    Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 100 : Top = 84 : Width = 150
    Caption = "-"
  END CREATE
END CREATE
Img.BMPHandle = STAR
IL.AddBMPHandle STAR, 0
SUB PaintIt
  Cv.FillRect 0, 0, 60, 60, &HFFFFFF
  Cv.Draw 6, 6, IL.GetBMP(0)
END SUB
SUB Report
  Lbl.Caption = STR$(Img.Width) + " " + STR$(IL.Count) + " " + HEX$(Cv.Pixel(30, 30)) + " " + HEX$(Cv.Pixel(8, 8))
END SUB
Form.ShowModal
