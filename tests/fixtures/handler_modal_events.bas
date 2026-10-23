' A form shown with ShowModal from a menu item's handler (a program's
' "close without saving?" question), driven by the test's events: the
' first time its Cancel button's click answers it; the second time the
' test ends with it still open — the dump and the captures are taken
' then, the question's window among them. A native build waits inside the
' handler, its loop stepping the test (as the interpreter's VM does).
' Checked natively and interpreted by tests/native_gui_events.mjs.
DECLARE SUB AskClose
DECLARE SUB Cancelled
CREATE Form AS QFORM
  Caption = "Notes": Width = 320: Height = 140
  CREATE Menu AS QMAINMENU
    CREATE FileMenu AS QMENUITEM
      Caption = "&File"
      CREATE CloseItem AS QMENUITEM
        Caption = "&Close": OnClick = AskClose
      END CREATE
    END CREATE
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 10: Width = 300
  END CREATE
END CREATE
CREATE TxForm AS QFORM
  Caption = "Close?": Width = 260: Height = 120
  CREATE TxMsg AS QLABEL
    Left = 10: Top = 10: Width = 240: Caption = "The changes aren't saved."
  END CREATE
  CREATE TxCancel AS QBUTTON
    Left = 40: Top = 50: Caption = "Cancel": ModalResult = 2: OnClick = Cancelled
  END CREATE
  CREATE TxClose AS QBUTTON
    Left = 130: Top = 50: Caption = "Close": ModalResult = 1
  END CREATE
END CREATE

SUB Cancelled
  Lbl.Caption = Lbl.Caption + "cancel;"
END SUB

SUB AskClose
  DIM R AS INTEGER
  R = TxForm.ShowModal
  Lbl.Caption = Lbl.Caption + "asked" + STR$(R) + ";"
END SUB

Form.ShowModal
