' Window icons: Application.IcoHandle (an SVG $RESOURCE) for every form
' without its own, Form.IcoHandle (an .ICO) for one form; an icon is also a
' picture (QIMAGE), soft edges kept.
$RESOURCE STAR AS "rr_star.svg"
$RESOURCE DISC AS "rr_disc.ico"
Application.IcoHandle = STAR
CREATE Form AS QFORM
  Caption = "icons"
  Width = 300
  Height = 160
  IcoHandle = DISC
  CREATE Img AS QIMAGE
    Left = 10 : Top = 10
    BMPHandle = DISC
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 10 : Top = 60 : Width = 280
  END CREATE
END CREATE
CREATE Other AS QFORM
  Caption = "other"
  Left = 320
END CREATE
Lbl.Caption = STR$(Img.Width) + "x" + STR$(Img.Height) + " " + HEX$(Img.Pixel(8, 8))
Other.Show
Form.ShowModal
