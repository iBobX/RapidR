' $RESOURCE and QIMAGE: a BMP built into the program shown with BMPHandle
' (AutoSize, Transparent), drawing on it and on an image without a picture,
' Pixel, RESOURCECOUNT and OnClick. Checked natively and interpreted by
' tests/native_gui_events.mjs.
$RESOURCE PIC_BMP AS "picture_res.bmp"
DECLARE SUB Clicked
CREATE Form AS QFORM
  Caption = "Picture": Width = 300: Height = 200
  CREATE Img AS QIMAGE
    AutoSize = True
    Transparent = True
    BMPHandle = PIC_BMP
    OnClick = Clicked
  END CREATE
  CREATE Pad AS QIMAGE
    Left = 100: Width = 40: Height = 30
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 120: Width = 280
  END CREATE
  CREATE Summary AS QLABEL
    Top = 145: Width = 280
  END CREATE
END CREATE
' Outlined red, filled green.
Img.Circle(2, 2, 12, 8, &HFF0000, &H00FF00)
Pad.Line(0, 5, 39, 5, &HFF)
Summary.Caption = STR$(RESOURCECOUNT) + "|" + STR$(Img.Width) + "x" + STR$(Img.Height) + "|" + HEX$(Img.Pixel(7, 5)) + "|" + HEX$(Img.Pixel(2, 5)) + "|" + HEX$(Img.Pixel(19, 0)) + "|" + STR$(Pad.Width) + "|" + HEX$(Pad.Pixel(10, 5)) + "|" + HEX$(Pad.Pixel(30, 6)) + "|" + STR$(PIC_BMP = RESOURCE(0))
Form.ShowModal

SUB Clicked
  Lbl.Caption = Lbl.Caption + "click;"
END SUB
