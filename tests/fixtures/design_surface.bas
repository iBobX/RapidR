' RDESIGNSURFACE (RapidR's form designer, the IDE's): the designed
' components and their answers (one shared model: AddComponent, GetName,
' GetType, GetProp / SetProp, GetCompX … , SetCompBounds, SetName,
' SelectComp, RemoveComponent, ClearAll, Count, CompCount, FormCaption,
' made before the form shows), and the mouse through the test hooks: a
' press selects (OnSelect), a drag moves on the 8-pixel grid (OnMove), the
' selected one's corner handle resizes it, the background clears the
' selection (OnBgClick), a second press soon after is a double click
' (OnDblClick). No OnMouseDown / OnClick for it, as FLTK's.
DECLARE SUB Sel(Index AS INTEGER)
DECLARE SUB Moved(Index AS INTEGER, X AS INTEGER, Y AS INTEGER, W AS INTEGER, H AS INTEGER)
DECLARE SUB Dbl(Index AS INTEGER)
DECLARE SUB Bg(X AS INTEGER, Y AS INTEGER)
DECLARE SUB Down(Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB Report
CREATE Form AS QFORM
  Caption = "designer"
  Width = 460: Height = 360
  CREATE DS AS RDESIGNSURFACE
    Left = 8: Top = 8: Width = 300: Height = 240
    FormCaption = "Main"
    OnSelect = Sel
    OnMove = Moved
    OnDblClick = Dbl
    OnBgClick = Bg
    OnMouseDown = Down
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 256: Width = 440
  END CREATE
  CREATE Log AS QLABEL
    Top = 280: Width = 440
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 320: Top = 8
    OnClick = Report
  END CREATE
END CREATE
DS.AddComponent("RBUTTON", "Button1", 16, 16, 80, 24)
DS.AddComponent("RLABEL", "Label1", 16, 56, 64, 16)
DS.AddComponent("RCHECKBOX", "Check1", 120, 16, 96, 24)
DS.SetProp(2, "checked", "1")
DS.AddComponent("RTIMER", "Timer1", 200, 160, 48, 48)
DS.SetProp(0, "Color", "&H00FFFF")
Lbl.Caption = STR$(DS.CompCount) + "|" + DS.FormCaption + "|" + DS.GetName(1) + "|" + DS.GetType(2) + "|" + DS.GetProp(0, "caption") + "|" + DS.GetProp(9, "x")
Form.ShowModal

SUB Sel(Index AS INTEGER)
  Log.Caption = Log.Caption + "s" + STR$(Index) + "/"
END SUB

SUB Moved(Index AS INTEGER, X AS INTEGER, Y AS INTEGER, W AS INTEGER, H AS INTEGER)
  Log.Caption = Log.Caption + "m" + STR$(Index) + ":" + STR$(X) + "," + STR$(Y) + "," + STR$(W) + "," + STR$(H) + "/"
END SUB

SUB Dbl(Index AS INTEGER)
  Log.Caption = Log.Caption + "d" + STR$(Index) + "/"
END SUB

SUB Bg(X AS INTEGER, Y AS INTEGER)
  Log.Caption = Log.Caption + "b" + STR$(X) + "," + STR$(Y) + "/"
END SUB

SUB Down(Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Log.Caption = Log.Caption + "DOWN/"
END SUB

SUB Report
  DS.SetCompBounds(3, 208, 168, 40, 40)
  DS.SetName(3, "Tick")
  DS.RemoveComponent(2)
  DS.SelectComp(1)
  DS.FormCaption = "Other"
  Lbl.Caption = Lbl.Caption + "|" + STR$(DS.Count) + "|" + DS.GetName(2) + "|" + STR$(DS.GetCompX(0)) + "," + STR$(DS.GetCompY(0)) + "," + STR$(DS.GetCompW(0)) + "," + STR$(DS.GetCompH(0)) + "|" + STR$(DS.GetCompX(2)) + "|" + DS.GetProp(0, "color") + "|" + DS.GetProp(1, "Caption") + "|" + DS.FormCaption + "|" + STR$(DS.Width)
END SUB
