' QDXSCREEN (DirectX 2D, docs/directx-plan.md): set up when its form is
' first shown (OnInitialize, then OnInitializeSurface); drawing goes to the
' back buffer and shows only after Flip; Pixel reads the back buffer; Fill
' takes DirectDraw's &HRRGGBB, the rest RapidQ's &HBBGGRR; a QDXIMAGELIST's
' pictures from a DelphiX image library ($RESOURCE dx_sprites.dxg, made by
' tools/make_dx_fixture.py: a transparent sprite, a strip of two patterns)
' drawn on its Parent screen; a QDXTIMER with Interval 0; the screen's
' OnMouseDown (Button, X, Y). Checked natively and interpreted by
' tests/native_gui_events.mjs, in the browser by tests/web_gui_parity.mjs.
$INCLUDE "RAPIDQ.INC"
$RESOURCE SPRITES_DXG AS "dx_sprites.dxg"
DECLARE SUB Started
DECLARE SUB Surface
DECLARE SUB Tick
DECLARE SUB DrawIt
DECLARE SUB Down (Button AS INTEGER, X AS INTEGER, Y AS INTEGER)
DIM ticks AS INTEGER
DIM log AS STRING
DIM DXTimer AS QDXTIMER
    DXTimer.Interval = 0
    DXTimer.OnTimer = Tick
CREATE Form AS QFORM
  Caption = "dx screen"
  ClientWidth = 200
  ClientHeight = 180
  CREATE DX AS QDXSCREEN
    Left = 10
    Top = 10
    Width = 160
    Height = 100
    Init(160, 100)
    OnInitialize = Started
    OnInitializeSurface = Surface
    OnMouseDown = Down
    CREATE Sprites AS QDXIMAGELIST
      LoadFromResource(SPRITES_DXG)
    END CREATE
  END CREATE
  CREATE btn AS QBUTTON
    Left = 10
    Top = 115
    Caption = "Draw"
    OnClick = DrawIt
  END CREATE
  CREATE lbl AS QLABEL
    Left = 10
    Top = 145
    Width = 190
    Caption = "-"
  END CREATE
END CREATE

SUB Started
  log = log + "init "
  ' (drawn before the surface is set up: there's nothing to draw on yet)
  DXTimer.Enabled = True
END SUB

SUB Surface
  log = log + "surface "
END SUB

SUB Tick
  ticks = ticks + 1
  IF ticks = 3 THEN
    DXTimer.Enabled = False
    log = log + "t3 "
  END IF
END SUB

SUB Down (Button AS INTEGER, X AS INTEGER, Y AS INTEGER)
  log = log + "down" + STR$(Button) + STR$(X) + STR$(Y) + " "
END SUB

SUB DrawIt
  DX.Fill(&HFF)
  DX.FillRect(0, 0, 20, 20, &HFF)
  DX.Pixel(30, 5) = &H00FF00
  DX.Line(40, 0, 59, 19, &HFFFFFF)
  DX.Circle(70, 0, 90, 20, &H00FFFF, &H00FFFF)
  DX.TextOut(5, 75, "Hi", &HFFFFFF, -1)
  Sprites.Draw(0, 100, 0, 0)
  Sprites.Draw(1, 120, 30, 1)
  Sprites.Draw(1, 128, 30, 0)
  DX.Flip
  lbl.Caption = log + "|" + STR$(DX.Pixel(150, 90)) + "," + STR$(DX.Pixel(5, 5)) + "," + STR$(DX.Pixel(30, 5)) + "," + STR$(DX.Pixel(80, 10)) + "|" + STR$(DX.Pixel(101, 1)) + "," + STR$(DX.Pixel(108, 8)) + "," + STR$(DX.Pixel(121, 31)) + "," + STR$(DX.Pixel(129, 31)) + "|" + STR$(DX.TextWidth("Hi")) + "," + STR$(DX.Width) + "x" + STR$(DX.Height)
END SUB

Form.ShowModal
