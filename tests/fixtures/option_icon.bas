' $OPTION ICON "file.ico": the program's icon, built in — the
' application's, so every form without its own shows it.
$OPTION ICON "rr_disc.ico"
$OPTION INKEY$ TRAPALL
$OPTION WEAKTYPE ON
CREATE Form AS QFORM
  Caption = "option icon"
  CREATE Lbl AS QLABEL
    Width = 200
  END CREATE
END CREATE
Lbl.Caption = STR$(Application.IcoHandle <> 0)
Form.ShowModal
