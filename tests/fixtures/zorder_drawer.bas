' (RapidR's) BringToFront / SendToBack: a drawer panel, made first (so
' under the "Under" button where they overlap), brought to the front covers
' the button and takes its clicks; sent back, the button is over it again.
' A click at the button's place says who heard it: U (the button), D (the
' drawer), I (the drawer's own button, which came to the front with it).
' The last BringToFront leaves the drawer on top for the capture.
DIM log AS STRING

SUB Heard (Who AS STRING)
  log = log + Who
  Said.Caption = log
END SUB

SUB UnderClick
  Heard "U"
END SUB

SUB DrawerClick
  Heard "D"
END SUB

SUB InsideClick
  Heard "I"
END SUB

SUB FrontClick
  Drawer.BringToFront
  Heard "+"
END SUB

SUB BackClick
  Drawer.SendToBack
  Heard "-"
END SUB

CREATE Form AS RFORM
  Caption = "Drawer"
  ClientWidth = 320
  ClientHeight = 200
  CREATE Drawer AS RPANEL
    Left = 0
    Top = 0
    Width = 160
    Height = 200
    Color = &H996633
    BevelOuter = 0
    OnClick = DrawerClick
    CREATE Inside AS RBUTTON
      Caption = "Inside"
      Left = 10
      Top = 150
      Width = 100
      Height = 30
      OnClick = InsideClick
    END CREATE
  END CREATE
  CREATE Under AS RBUTTON
    Caption = "Under"
    Left = 20
    Top = 20
    Width = 100
    Height = 30
    OnClick = UnderClick
  END CREATE
  CREATE Front AS RBUTTON
    Caption = "Front"
    Left = 200
    Top = 20
    Width = 100
    Height = 30
    OnClick = FrontClick
  END CREATE
  CREATE Back AS RBUTTON
    Caption = "Back"
    Left = 200
    Top = 60
    Width = 100
    Height = 30
    OnClick = BackClick
  END CREATE
  CREATE Said AS RLABEL
    Caption = ""
    Left = 200
    Top = 110
    Width = 110
    Height = 20
  END CREATE
END CREATE

Form.ShowModal
