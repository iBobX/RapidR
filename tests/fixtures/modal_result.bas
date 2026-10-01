' ShowModal returns the form's ModalResult (RapidQ / Delphi): a button's
' ModalResult closes its modal form with that result, Kind = bkOK gives a
' button its caption and mrOk; without KeyPreview the form doesn't hear the
' keys typed in its controls.
DECLARE SUB EdKey (Key AS WORD, Shift AS INTEGER)
DECLARE SUB DlgKey (Key AS WORD, Shift AS INTEGER)
DIM Keys AS STRING
CREATE Main AS QFORM
  Caption = "main"
  CREATE Lbl AS QLABEL
    Width = 300
  END CREATE
END CREATE
CREATE Dlg AS QFORM
  Caption = "dialog"
  OnKeyDown = DlgKey
  CREATE OkBtn AS QBUTTON
    Kind = 1
  END CREATE
  CREATE NoBtn AS QBUTTON
    Top = 40
    Caption = "No"
    ModalResult = 7
  END CREATE
  CREATE Ed AS QEDIT
    Top = 80
    OnKeyDown = EdKey
  END CREATE
END CREATE
Main.Show
R1 = Dlg.ShowModal
R2 = Dlg.ShowModal
Lbl.Caption = STR$(R1) + STR$(R2) + OkBtn.Caption + Keys
Main.ShowModal

SUB EdKey (Key AS WORD, Shift AS INTEGER)
  Keys = Keys + "e"
END SUB

SUB DlgKey (Key AS WORD, Shift AS INTEGER)
  Keys = Keys + "f"
END SUB
