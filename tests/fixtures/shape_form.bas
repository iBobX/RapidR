' Form.ShapeForm: the window's outline is the bitmap's pixels that aren't
' the transparent colour (white here: an oval with a square hole) — the
' bitmap's pixel (x, y) the window's (x, y) from its top left corner, its
' frame included (none here: bsNone, a skin's usual form). What's outside
' is cut away; Width, Height, ClientWidth and BorderStyle don't change
' (RC.EXE's probe read them back, and the window's region). The same
' bitmap shows on the form as its skin; a click on it moves nothing.
$RESOURCE SHAPE_BMP AS "shape_form.bmp"
DECLARE SUB Shape
DIM Log AS STRING
CREATE Form AS QFORM
  Caption = "shape form"
  BorderStyle = 0
  Width = 160
  Height = 120
  CREATE Skin AS QIMAGE
    BMPHandle = SHAPE_BMP
    AutoSize = True
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 40
    Top = 22
    Width = 80
    Caption = "-"
    Transparent = True
  END CREATE
  CREATE Btn AS QBUTTON
    Caption = "shape"
    Left = 42
    Top = 85
    OnClick = Shape
  END CREATE
END CREATE

SUB Shape
  Form.ShapeForm(SHAPE_BMP, &HFFFFFF)
  Lbl.Caption = STR$(Form.Width) + "x" + STR$(Form.Height) + " " + STR$(Form.ClientWidth) + " " + STR$(Form.BorderStyle)
END SUB

Form.ShowModal
