' The drawing members RapidQ has and RapidR answers since the registry's
' "missing" list: TextRect (text clipped to a rectangle that a background
' fills) on a form, QCANVAS, QIMAGE, QHEADER, an owner-drawn QLISTBOX and
' QSTRINGGRID; Rotate (degrees, anticlockwise) on a QCANVAS and a QIMAGE;
' RoundRect, CopyRect and StretchDraw in OnDrawItem / OnDrawCell; TextWidth
' on lists and grids; QIMAGELIST.Draw onto a QIMAGE and a QCANVAS; QCANVAS
' Get / Put (0). Checked natively, interpreted and in the browser
' (tests/gui_parity_cases.mjs: drawing_members).
$INCLUDE "RAPIDQ.INC"
DECLARE SUB DrawItem (Index AS INTEGER, State AS INTEGER, Rect AS QRECT)
DECLARE SUB DrawCell (Col AS INTEGER, Row AS INTEGER, State AS INTEGER, Rect AS QRECT)
DECLARE SUB DrawSection (Index AS INTEGER, Pressed AS INTEGER, Rect AS QRECT, Sender AS QHEADER)
DECLARE SUB DrawAll
DIM R AS QRECT
DIM Chip AS QBITMAP
Chip.Width = 8: Chip.Height = 8
Chip.FillRect(0, 0, 8, 8, &HFF0000)
Chip.FillRect(0, 0, 4, 4, &H00FFFF)
DIM Icons AS QIMAGELIST
Icons.Width = 8: Icons.Height = 8
Chip.SaveToFile "drawing_members.tmp.bmp"
Icons.AddBMPFile "drawing_members.tmp.bmp", &H123456
KILL "drawing_members.tmp.bmp"

CREATE Form AS QFORM
  Caption = "Drawing members"
  Width = 520: Height = 330
  CREATE Cv AS QCANVAS
    Left = 10: Top = 10: Width = 120: Height = 80
  END CREATE
  CREATE Img AS QIMAGE
    Left = 140: Top = 10: Width = 120: Height = 80
  END CREATE
  CREATE Lst AS QLISTBOX
    Left = 270: Top = 10: Width = 140: Height = 80
    Style = lbOwnerDrawFixed
    ItemHeight = 24
    OnDrawItem = DrawItem
    AddItems "Alpha with a long text", "Beta", "Gamma"
    ItemIndex = 1
  END CREATE
  CREATE Grid AS QSTRINGGRID
    Left = 10: Top = 100: Width = 260: Height = 100
    ColCount = 3: RowCount = 3
    OnDrawCell = DrawCell
  END CREATE
  CREATE Head AS QHEADER
    Left = 280: Top = 100: Width = 220: Height = 24
    AddSections "Plain", "Owner"
    Sections(0).Width = 80
    Sections(1).Width = 130
    Sections(1).Style = hsOwnerDraw
    OnDrawSection = DrawSection
  END CREATE
  CREATE Cb AS QCOMBOBOX
    Left = 280: Top = 140: Width = 120
    AddItems "One", "Two"
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 420: Top = 140: Width = 80
    Caption = "Draw"
    OnClick = DrawAll
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 10: Top = 270: Width = 490
    Caption = "-"
  END CREATE
END CREATE
Grid.Cell(1, 1) = "cell"

SUB DrawItem (Index AS INTEGER, State AS INTEGER, Rect AS QRECT)
  DIM Bg AS LONG
  IF State = 0 THEN Bg = &HFFCC99 ELSE Bg = &HFFFFFF
  ' (the text clipped 30 pixels before the item's right)
  DIM Clip AS QRECT
  Clip.Left = Rect.Left: Clip.Top = Rect.Top: Clip.Right = Rect.Right - 30: Clip.Bottom = Rect.Bottom
  Lst.TextRect(Clip, Rect.Left + 20, Rect.Top + 4, Lst.Item(Index), 0, Bg)
  Lst.RoundRect(Rect.Left + 3, Rect.Top + 5, Rect.Left + 17, Rect.Top + 19, 6, 6, &H0080FF)
  DIM Spot AS QRECT
  Spot.Left = Rect.Right - 26: Spot.Top = Rect.Top + 4: Spot.Right = Rect.Right - 6: Spot.Bottom = Rect.Top + 20
  Lst.StretchDraw(Spot, Chip.BMP)
END SUB

SUB DrawCell (Col AS INTEGER, Row AS INTEGER, State AS INTEGER, Rect AS QRECT)
  IF Col = 1 AND Row = 1 THEN
    Grid.TextRect(Rect, Rect.Left + 2, Rect.Top + 4, "clipped in the cell", &HFFFFFF, &H800000)
  END IF
  IF Col = 2 AND Row = 1 THEN
    Grid.RoundRect(Rect.Left + 4, Rect.Top + 3, Rect.Right - 4, Rect.Bottom - 3, 10, 10, &H00C000)
  END IF
  IF Col = 1 AND Row = 2 THEN
    DIM S AS QRECT, D AS QRECT
    S.Left = 0: S.Top = 0: S.Right = 4: S.Bottom = 4
    D.Left = Rect.Left + 2: D.Top = Rect.Top + 2: D.Right = Rect.Left + 22: D.Bottom = Rect.Top + 22
    Grid.CopyRect(D, Chip, S)
  END IF
END SUB

SUB DrawSection (Index AS INTEGER, Pressed AS INTEGER, Rect AS QRECT, Sender AS QHEADER)
  DIM Clip AS QRECT
  Clip.Left = Rect.Left + 4: Clip.Top = Rect.Top + 3: Clip.Right = Rect.Right - 40: Clip.Bottom = Rect.Bottom - 3
  Sender.TextRect(Clip, Rect.Left + 6, Rect.Top + 4, "Owner drawn header", &HFFFFFF, &H008000)
END SUB

SUB DrawAll
  ' A canvas: a bar turned a quarter (it points up), text clipped.
  Cv.FillRect(0, 0, 120, 80, &HFFFFFF)
  Cv.FillRect(60, 36, 110, 44, &H0000FF)
  Cv.Rotate(60, 40, 90)
  R.Left = 4: R.Top = 58: R.Right = 100: R.Bottom = 76
  Cv.TextRect(R, 8, 60, "Canvas text runs past", 0, &H00FFFF)
  Icons.Draw Cv, 100, 4, 0
  ' An image: a square turned 30 degrees about its centre, an icon.
  Img.FillRect(0, 0, 120, 80, &HE0E0E0)
  Img.FillRect(40, 20, 80, 60, &H008000)
  Img.Rotate(60, 40, 30)
  Icons.Draw Img, 4, 4, 0
  R.Left = 4: R.Top = 62: R.Right = 116: R.Bottom = 78
  Img.TextRect(R, 6, 63, "Image", &HFFFFFF, -1)
  ' The form itself.
  R.Left = 280: R.Top = 180: R.Right = 380: R.Bottom = 200
  Form.TextRect(R, 284, 182, "Form TextRect clipped", &H000080, &HCCFFFF)
  Lbl.Caption = "w" + STR$(Lst.TextWidth("Hello")) + " h" + STR$(Lst.TextHeight("Hello")) + " cb" + STR$(Cb.TextWidth("Hello")) + " g" + STR$(Grid.TextWidth("Hello")) + " get" + STR$(Cv.Get) + " put" + STR$(Cv.Put) + " " + HEX$(Cv.Pixel(60, 20)) + " " + HEX$(Cv.Pixel(90, 40))
END SUB

Form.ShowModal
