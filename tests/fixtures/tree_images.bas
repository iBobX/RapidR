' QTREEVIEW's images: Images and StateImages (a state image beside the
' node's own, index 0 none), OnGetImageIndex / OnGetSelectedIndex setting a
' node's image as it is drawn, GetItemAt(X, Y), HideSelection.
$RESOURCE STAR AS "rr_star.svg"
$RESOURCE DISC AS "rr_disc.ico"
DECLARE SUB GetImage (Index AS INTEGER, Sender AS QTREEVIEW)
DECLARE SUB GetSelected (Index AS INTEGER, Sender AS QTREEVIEW)
DECLARE SUB Report
DIM Asked AS INTEGER, AskedSelected AS INTEGER
DIM Pics AS QIMAGELIST
Pics.Width = 16 : Pics.Height = 16
Pics.AddBMPHandle STAR, 0
Pics.AddBMPHandle DISC, 0
DIM States AS QIMAGELIST
States.Width = 16 : States.Height = 16
States.AddBMPHandle DISC, 0
States.AddBMPHandle STAR, 0
CREATE Form AS QFORM
  Caption = "tree images"
  Width = 360
  Height = 260
  CREATE Tv AS QTREEVIEW
    Left = 5 : Top = 5 : Width = 200 : Height = 200
    Images = Pics
    StateImages = States
    HideSelection = 1
    AddItems "One", "Two", "Three"
    OnGetImageIndex = GetImage
    OnGetSelectedIndex = GetSelected
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 220 : Top = 5
    Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5 : Top = 215 : Width = 340
    Caption = "-"
  END CREATE
END CREATE
Tv.Item(0).StateIndex = 1
Tv.ItemIndex = 2

SUB GetImage (Index AS INTEGER, Sender AS QTREEVIEW)
  Asked = Asked + 1
  Sender.Item(Index).ImageIndex = Index MOD 2
END SUB

SUB GetSelected (Index AS INTEGER, Sender AS QTREEVIEW)
  AskedSelected = AskedSelected + 1
  Sender.Item(Index).SelectedIndex = 1
END SUB

SUB Report
  Lbl.Caption = STR$(Asked > 0) + " " + STR$(AskedSelected > 0) + " " + STR$(Tv.Item(1).ImageIndex) + " " + STR$(Tv.Item(2).SelectedIndex) + " " + STR$(Tv.GetItemAt(5, 8)) + " " + STR$(Tv.GetItemAt(5, 30)) + " " + STR$(Tv.GetItemAt(5, 190)) + " " + STR$(Tv.HideSelection)
END SUB

Form.ShowModal
