' RapidR's canvas names — Rect, SetPixel, Ellipse, DrawText — wherever a
' program draws, on every runtime (the language registry once had them as
' the desktop's only): on a QBITMAP (shown by a QCANVAS's OnPaint), on a
' QDXSCREEN's back buffer, in an owner-drawn QLISTBOX's OnDrawItem (with
' RapidQ's Line, FillRect, Circle) and a QSTRINGGRID's OnDrawCell; Paint
' (a flood fill) in a cell, a list that isn't owner-drawn and a QIMAGE's
' Clear draw nothing. Checked natively and interpreted by
' tests/native_gui_events.mjs; in the browser by tests/web_gui_parity.mjs,
' whose captures must be byte-identical to the desktop's (1x and 2x).
$INCLUDE "RAPIDQ.INC"
DECLARE SUB PaintCv
DECLARE SUB DrawItem (Index AS INTEGER, State AS BYTE, Rect AS QRECT)
DECLARE SUB DrawCell (Col%, Row%, State%, Rect AS QRECT, Sender AS QSTRINGGRID)
DECLARE SUB DrawIt
DIM Bmp AS QBITMAP
Bmp.Width = 80
Bmp.Height = 50
Bmp.FillRect(0, 0, 80, 50, &HFFFFFF)
Bmp.Rect(2, 2, 30, 24, &HFF)
Bmp.SetPixel(40, 5, &H00FF00)
Bmp.Ellipse(44, 2, 76, 24, &HFF0000, &HFFFF00)
Bmp.DrawText("Bmp", 4, 28, &H800000)
CREATE Form AS QFORM
  Caption = "drawing members"
  ClientWidth = 480
  ClientHeight = 250
  CREATE Cv AS QCANVAS
    Left = 10
    Top = 10
    Width = 80
    Height = 50
    OnPaint = PaintCv
  END CREATE
  CREATE DX AS QDXSCREEN
    Left = 100
    Top = 10
    Width = 120
    Height = 60
    Init(120, 60)
  END CREATE
  CREATE Lst AS QLISTBOX
    Left = 230
    Top = 10
    Width = 120
    Height = 80
    Style = lbOwnerDrawFixed
    ItemHeight = 24
    OnDrawItem = DrawItem
    AddItems "Rect", "Ellipse", "Text"
  END CREATE
  CREATE Plain AS QLISTBOX
    Left = 230
    Top = 100
    Width = 120
    Height = 50
    AddItems "plain"
  END CREATE
  CREATE Grid AS QSTRINGGRID
    Left = 10
    Top = 80
    Width = 210
    Height = 90
    ColCount = 3
    RowCount = 3
    OnDrawCell = DrawCell
  END CREATE
  CREATE Img AS QIMAGE
    Left = 230
    Top = 160
    Width = 40
    Height = 40
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 10
    Top = 180
    Caption = "Draw"
    OnClick = DrawIt
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 10
    Top = 220
    Width = 460
    Caption = "-"
  END CREATE
END CREATE

SUB PaintCv
  Cv.Draw(0, 0, Bmp.BMP)
END SUB

SUB DrawItem (Index AS INTEGER, State AS BYTE, Rect AS QRECT)
  Lst.FillRect(Rect.Left, Rect.Top, Rect.Right, Rect.Bottom, &HFFFFFF)
  SELECT CASE Index
  CASE 0
    Lst.Rect(Rect.Left + 2, Rect.Top + 2, Rect.Left + 22, Rect.Bottom - 2, &HFF)
    Lst.Line(Rect.Left + 30, Rect.Top + 2, Rect.Left + 60, Rect.Bottom - 2, &H008000)
    Lst.SetPixel(Rect.Left + 70, Rect.Top + 10, &HFF00FF)
  CASE 1
    Lst.Ellipse(Rect.Left + 2, Rect.Top + 2, Rect.Left + 40, Rect.Bottom - 2, &HFF0000, &HFFFF00)
    Lst.Circle(Rect.Left + 50, Rect.Top + 2, Rect.Left + 70, Rect.Bottom - 2, &H0000FF, &H00FFFF)
  CASE 2
    Lst.DrawText("drawn", Rect.Left + 4, Rect.Top + 4, &H800000)
  END SELECT
END SUB

SUB DrawCell (Col%, Row%, State%, Rect AS QRECT, Sender AS QSTRINGGRID)
  IF Col% = 1 AND Row% = 1 THEN
    Sender.Rect(Rect.Left + 2, Rect.Top + 2, Rect.Right - 2, Rect.Bottom - 2, &HFF)
    Sender.SetPixel(Rect.Left + 6, Rect.Top + 6, &H00FF00)
    Sender.Ellipse(Rect.Left + 10, Rect.Top + 4, Rect.Left + 30, Rect.Bottom - 4, &HFF0000, &HFFFF00)
    Sender.DrawText("g", Rect.Left + 40, Rect.Top + 4, &H800000)
  END IF
  IF Col% = 2 AND Row% = 1 THEN Sender.Paint(Rect.Left + 5, Rect.Top + 5, &HFF, &H0)
END SUB

SUB DrawIt
  DX.Fill(&H202020)
  DX.Rect(2, 2, 40, 30, &HFF)
  DX.SetPixel(50, 10, &H00FF00)
  DX.Ellipse(60, 2, 110, 40, &HFFFF00, &HFF0000)
  DX.DrawText("DX", 5, 38, &HFFFFFF)
  DX.Flip
  Img.FillRect(0, 0, 40, 40, &HFF)
  Img.Clear
  Plain.FillRect(0, 0, 120, 50, &HFF)
  Plain.Rect(0, 0, 120, 50, &HFF)
  Plain.Ellipse(0, 0, 50, 20, &HFF)
  Plain.DrawText("x", 2, 2)
  Lbl.Caption = "bmp" + STR$(Bmp.Pixel(2, 10)) + "," + STR$(Bmp.Pixel(40, 5)) + "," + STR$(Bmp.Pixel(60, 13)) + "," + STR$(Bmp.Pixel(60, 2)) + " dx" + STR$(DX.Pixel(2, 10)) + "," + STR$(DX.Pixel(50, 10)) + "," + STR$(DX.Pixel(85, 21)) + " img" + STR$(Img.Pixel(5, 5))
END SUB

Form.ShowModal
