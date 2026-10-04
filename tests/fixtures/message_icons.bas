' MESSAGEBOX's MB_ICONxxx icon (Windows' MessageBox) and MESSAGEDLG's
' type icon and caption (Delphi's MessageDlg): MB_YESNO OR MB_ICONQUESTION
' asked from the main program. Under the desktop's test hooks the dialog
' shows and waits (nobody answers: the label keeps "asked"); in the
' browser it is the page's own dialog, whose caption, icon and buttons the
' web check reads.
CREATE Form AS QFORM
  Caption = "message icons"
  Width = 300
  Height = 150
  CREATE Lbl AS QLABEL
    Left = 10 : Top = 10 : Width = 280 : Caption = "start"
  END CREATE
END CREATE
Form.Show
Lbl.Caption = "asked"
DIM Answer AS INTEGER
Answer = MESSAGEBOX("Delete the file?", "Files", 4 OR 32)
Lbl.Caption = "answered " + STR$(Answer)
Form.ShowModal
