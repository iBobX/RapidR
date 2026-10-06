' QDOCKFORM (RAPIDQ2.INC's dockable form, the manual's Appendix A) as
' RapidR has it built in, with the manual example's members: a docked
' panel with a title (DockStyle 1, a close button), its components on its
' Client, an alternative place (Style 1, AltPanel.Parent = Form), a
' toolbar-style one (DockStyle 6: a grip); Dock(2) docks it at the other
' side, Dock(0) floats it in its own window, Dock(1) brings it home, Close
' hides it (OnClose) — OnDock tells each move.
DECLARE SUB DockRight
DECLARE SUB Float
DECLARE SUB Home
DECLARE SUB CloseIt
DECLARE SUB Docked (D AS INTEGER, A AS INTEGER)
DECLARE SUB Closed
CREATE Form AS QFORM
  Caption = "dock form"
  ClientWidth = 478
  ClientHeight = 289
  CREATE Bar AS QDOCKFORM
    Align = 1
    Height = 31
    DockStyle = 6
    Caption = "Toolbar"
    CREATE BLbl AS QLABEL
      Parent = Bar.Client
      Caption = "Tools"
      Top = 5
    END CREATE
  END CREATE
  CREATE P AS QDOCKFORM
    AltPanel.Parent = Form
    Style = 1
    DockStyle = 1
    UnDockStyle = 1
    Caption = "Project"
    CanClose = 1
    UndockedWidth = 160
    UndockedHeight = 200
    OnDock = Docked
    OnClose = Closed
    CREATE CP AS QPANEL
      Parent = P.Client
      Align = 5
      BevelOuter = 0
      CREATE B1 AS QBUTTON
        Left = 10 : Top = 10 : Width = 70
        Caption = "Right"
        OnClick = DockRight
      END CREATE
      CREATE B2 AS QBUTTON
        Left = 10 : Top = 40 : Width = 70
        Caption = "Float"
        OnClick = Float
      END CREATE
      CREATE B3 AS QBUTTON
        Left = 10 : Top = 70 : Width = 70
        Caption = "Home"
        OnClick = Home
      END CREATE
      CREATE B4 AS QBUTTON
        Left = 10 : Top = 100 : Width = 70
        Caption = "Close"
        OnClick = CloseIt
      END CREATE
    END CREATE
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 160 : Top = 200 : Width = 140 : Height = 60
    WordWrap = 1
    Caption = "-"
  END CREATE
END CREATE
SUB DockRight
  P.Dock(2)
  Lbl.Caption = Lbl.Caption + " r" + STR$(P.Docked) + STR$(P.AltDock)
END SUB
SUB Float
  P.Dock(0)
  Lbl.Caption = Lbl.Caption + " f" + STR$(P.Docked) + STR$(P.AltDock)
END SUB
SUB Home
  P.Dock(1)
  Lbl.Caption = Lbl.Caption + " h" + STR$(P.Docked) + STR$(P.AltDock)
END SUB
SUB CloseIt
  P.Close
  Lbl.Caption = Lbl.Caption + " c" + STR$(P.Closed)
END SUB
SUB Docked (D AS INTEGER, A AS INTEGER)
  Lbl.Caption = Lbl.Caption + " d" + STR$(D) + STR$(A)
END SUB
SUB Closed
  Lbl.Caption = Lbl.Caption + " closed"
END SUB
Form.ShowModal
