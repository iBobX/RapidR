' RapidR Studio's form designer surface (RDesignSurface on the designer
' model, docs/ide-plan.md I4): a dialog being designed. Click a component
' to select it (Shift+click adds), drag it — it snaps to the grid and to
' smart guides (edges, centres, baselines, margins, equal spacing; hold Alt
' for none) — or drag its handles. The round pins around the selected
' component are its Anchors: click one to anchor that side. Drag the
' form's bottom-right corner: the resize preview puts every component where
' the running program would (the runtimes' own layout code), and lets go.
DECLARE SUB Selected(Index AS INTEGER)
DECLARE SUB Moved(Index AS INTEGER, X AS INTEGER, Y AS INTEGER, W AS INTEGER, H AS INTEGER)
DECLARE SUB UndoClick
DECLARE SUB RedoClick
DECLARE SUB PreviewClick

CREATE Main AS RForm
  Caption = "Form designer": Width = 760: Height = 520
  CREATE DS AS RDesignSurface
    Left = 8: Top = 8: Width = 600: Height = 400
    FormCaption = "Dialog1"
    OnSelect = Selected
    OnMove = Moved
  END CREATE
  CREATE BUndo AS RButton
    Caption = "Undo": Left = 624: Top = 8: Width = 112
    OnClick = UndoClick
  END CREATE
  CREATE BRedo AS RButton
    Caption = "Redo": Left = 624: Top = 40: Width = 112
    OnClick = RedoClick
  END CREATE
  CREATE BPreview AS RButton
    Caption = "Preview 440 x 300": Left = 624: Top = 72: Width = 112
    OnClick = PreviewClick
  END CREATE
  CREATE Info AS RLabel
    Left = 8: Top = 420: Width = 728: Height = 40
    Caption = "Select a component; drag it, its handles, its pins or the form's corner."
  END CREATE
END CREATE

DS.AddComponent("RLABEL", "NameLbl", 16, 20, 48, 16)
DS.SetProp(0, "Caption", "Name:")
DS.AddComponent("REDIT", "NameEd", 72, 16, 300, 24)
DS.SetProp(1, "Anchors", "akLeft + akTop + akRight")
DS.AddComponent("RRICHEDIT", "Notes", 16, 56, 356, 280)
DS.SetProp(2, "Anchors", "akLeft + akTop + akRight + akBottom")
DS.AddComponent("RPANEL", "Side", 392, 16, 192, 320)
DS.SetProp(3, "Anchors", "akTop + akRight + akBottom")
DS.AddComponent("RBUTTON", "Ok", 424, 352, 75, 25)
DS.SetProp(4, "Caption", "OK")
DS.SetProp(4, "Anchors", "akRight + akBottom")
DS.AddComponent("RBUTTON", "Cancel", 504, 352, 75, 25)
DS.SetProp(5, "Anchors", "akRight + akBottom")
DS.SelectComp(2)

SUB Selected(Index AS INTEGER)
  Info.Caption = DS.GetName(Index) + " (" + DS.GetType(Index) + ")  Anchors = " + DS.GetProp(Index, "Anchors")
END SUB

SUB Moved(Index AS INTEGER, X AS INTEGER, Y AS INTEGER, W AS INTEGER, H AS INTEGER)
  Info.Caption = DS.GetName(Index) + " at " + STR$(X) + ", " + STR$(Y) + "  " + STR$(W) + " x " + STR$(H)
END SUB

SUB UndoClick
  DS.Undo
END SUB

SUB RedoClick
  DS.Redo
END SUB

SUB PreviewClick
  IF DS.PreviewWidth = 0 THEN
    DS.PreviewWidth = 440: DS.PreviewHeight = 300
  ELSE
    DS.PreviewWidth = 0
  END IF
END SUB

Main.ShowModal
