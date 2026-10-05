' QDXSCREEN, more (docs/directx-plan.md, stage D1b): the screen's text in
' MS Sans Serif 8 until a QFONT is given; Rotate (256ths of a turn,
' clockwise); View.SetFront / GetFront / GetBack / Clear; Cursor = crNone; a
' screen put on a form already shown (set up once the handler returns); a
' hidden form's screen set up when the form is first shown; FullScreen (the
' form covers the screen, the surface keeps Init's size); a QDXTIMER with
' ActiveOnly left True (the program is active under the tests). Checked
' natively and interpreted by tests/native_gui_events.mjs, in the browser by
' tests/web_gui_parity.mjs.
$INCLUDE "RAPIDQ.INC"
DECLARE SUB Go
DECLARE SUB AddLate
DECLARE SUB ShowSecond
DECLARE SUB ShowFull
DECLARE SUB Report
DECLARE SUB LateInit
DECLARE SUB SecondInit
DECLARE SUB FullInit
DECLARE SUB Tick
DIM log AS STRING
DIM ticks AS INTEGER
DIM widths AS STRING
DIM Big AS QFONT
    Big.Name = "Arial"
    Big.Size = 14
DIM Late AS QDXSCREEN
DIM DXTimer AS QDXTIMER
    DXTimer.Interval = 0
    DXTimer.OnTimer = Tick
    DXTimer.Enabled = True
CREATE Form AS QFORM
  Caption = "dx more"
  ClientWidth = 240
  ClientHeight = 200
  CREATE DX AS QDXSCREEN
    Left = 10
    Top = 10
    Width = 100
    Height = 60
    Init(100, 60)
    Cursor = crNone
  END CREATE
  CREATE b1 AS QBUTTON
    Left = 10
    Top = 80
    Caption = "Go"
    OnClick = Go
  END CREATE
  CREATE b2 AS QBUTTON
    Left = 90
    Top = 80
    Caption = "Late"
    OnClick = AddLate
  END CREATE
  CREATE b3 AS QBUTTON
    Left = 10
    Top = 110
    Caption = "Second"
    OnClick = ShowSecond
  END CREATE
  CREATE b4 AS QBUTTON
    Left = 90
    Top = 110
    Caption = "Full"
    OnClick = ShowFull
  END CREATE
  CREATE b5 AS QBUTTON
    Left = 170
    Top = 110
    Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE lbl AS QLABEL
    Left = 10
    Top = 145
    Width = 230
    Caption = "-"
  END CREATE
END CREATE
CREATE Form2 AS QFORM
  Caption = "second"
  ClientWidth = 120
  ClientHeight = 80
  CREATE DX2 AS QDXSCREEN
    Align = alClient
    OnInitialize = SecondInit
  END CREATE
END CREATE
CREATE Form3 AS QFORM
  Caption = "full"
  BorderStyle = bsNone
  CREATE DX3 AS QDXSCREEN
    Init(64, 48)
    Align = alClient
    FullScreen = True
    OnInitialize = FullInit
  END CREATE
END CREATE

SUB Tick
  ticks = ticks + 1
  IF ticks = 2 THEN
    log = log + "tick "
    DXTimer.Enabled = False
  END IF
END SUB

SUB Go
  widths = STR$(DX.TextWidth("Hi")) + "x" + STR$(DX.TextHeight("Hi"))
  DX.Font = Big
  widths = widths + "," + STR$(DX.TextWidth("Hi"))
  DX.Line(20, 30, 39, 30, &HFF)
  DX.Rotate(20, 30, 64)
  DX.View.SetFront(10.5)
  log = log + "rot" + STR$(DX.Pixel(20, 40)) + "," + STR$(DX.Pixel(30, 30)) + " view" + STR$(DX.View.GetFront) + "," + STR$(DX.View.GetBack) + " "
  DX.Flip
END SUB

SUB AddLate
  Late.Left = 130
  Late.Top = 10
  Late.Width = 50
  Late.Height = 40
  Late.OnInitialize = LateInit
  Late.Parent = Form
  log = log + "parented "
END SUB

SUB LateInit
  log = log + "late "
  Late.Fill(&HFF)
  Late.Flip
END SUB

SUB ShowSecond
  Form2.Show
END SUB

SUB SecondInit
  log = log + "second" + STR$(DX2.Width) + " "
END SUB

SUB ShowFull
  Form3.Show
END SUB

SUB FullInit
  log = log + "full "
END SUB

SUB Report
  DX.View.Clear
  lbl.Caption = log + "|" + widths + "|" + STR$(DX.Pixel(20, 40)) + "|" + IIF(DX3.Width > 400, "wide", "narrow") + STR$(DX3.Pixel(10, 5)) + "," + STR$(DX3.Pixel(70, 5))
END SUB

Form.ShowModal
