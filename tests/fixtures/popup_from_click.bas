' A pop-up menu RapidR draws (SystemMenu = 0) opened with PopupAt from the
' handler of the gesture the person is making: a button's OnClick (the
' "..." button under which a menu drops), a label's OnMouseDown (the menu
' opens while the button is still down) and OnMouseUp, a right click's
' OnContextMenu. The gesture that opened it never closes it or picks from
' it: the click's own release, even a few pixels off over the first item,
' leaves the menu open; the next click on an item picks it (a letter picks
' too). Shift+F10 and the Menu key ask the focused label for the same
' OnContextMenu (Key 1), its menu opening with the first item chosen. The
' log says what was picked, in order.
DIM log AS STRING

SUB Picked (Which AS STRING)
  log = log + " " + Which
  Said.Caption = log
END SUB

SUB PickAlpha
  Picked "A"
END SUB

SUB PickBeta
  Picked "B"
END SUB

SUB DotsClick (Sender AS RBUTTON)
  Pop.PopupAt Sender, 0, Sender.Height
END SUB

SUB OpenOnDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER, Sender AS RLABEL)
  Pop.PopupAt Sender, X, Y
END SUB

SUB OpenOnUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER, Sender AS RLABEL)
  Pop.PopupAt Sender, X, Y
END SUB

SUB OpenOnContext (X AS INTEGER, Y AS INTEGER, Key AS INTEGER, Sender AS RLABEL)
  Pop.PopupAt Sender, X, Y
END SUB

CREATE Pop AS RPOPUPMENU
  SystemMenu = 0
  CREATE AlphaItem AS RMENUITEM
    Caption = "&Alpha"
    OnClick = PickAlpha
  END CREATE
  CREATE BetaItem AS RMENUITEM
    Caption = "&Beta"
    OnClick = PickBeta
  END CREATE
END CREATE

CREATE Form AS QFORM
  Caption = "popup from a click"
  Width = 300
  Height = 260
  CREATE Dots AS RBUTTON
    Left = 10
    Top = 10
    Width = 60
    Height = 26
    Caption = "..."
    OnClick = DotsClick
  END CREATE
  CREATE DownLbl AS RLABEL
    Left = 10
    Top = 60
    Width = 140
    Height = 24
    AutoSize = 0
    Caption = "Press (OnMouseDown)"
    OnMouseDown = OpenOnDown
  END CREATE
  CREATE UpLbl AS RLABEL
    Left = 10
    Top = 100
    Width = 140
    Height = 24
    AutoSize = 0
    Caption = "Release (OnMouseUp)"
    OnMouseUp = OpenOnUp
  END CREATE
  CREATE CtxLbl AS RLABEL
    Left = 10
    Top = 140
    Width = 140
    Height = 24
    AutoSize = 0
    Caption = "Right-click (OnContextMenu)"
    TabStop = 1
    OnContextMenu = OpenOnContext
  END CREATE
  CREATE Said AS RLABEL
    Left = 10
    Top = 190
    Width = 260
    Height = 24
    AutoSize = 0
    Caption = ""
  END CREATE
END CREATE
Form.ShowModal
