' QLISTVIEW as RapidQ has it: SmallImages / LargeImages, CheckBoxes,
' SortType stText, a click, a check box's click, the arrows and Space,
' a header button (OnColumnClick), vsIcon's icons, MultiSelect — each
' OnChange (Index, Change), OnClick and OnColumnClick logged in order.
$INCLUDE "RAPIDQ.INC"
$RESOURCE STAR AS "rr_star.svg"
$RESOURCE DISC AS "rr_disc.ico"
DECLARE SUB Changed (Index AS INTEGER, Change AS BYTE)
DECLARE SUB Clicked
DECLARE SUB Headed (Column AS INTEGER)
DECLARE SUB ToIcons
DECLARE SUB Report
DIM Log AS STRING
DIM Small AS QIMAGELIST
Small.Width = 16 : Small.Height = 16
Small.AddICOHandle DISC
DIM Large AS QIMAGELIST
Large.Width = 32 : Large.Height = 32
Large.AddICOHandle STAR
CREATE Form AS QFORM
  Caption = "ListView views": Width = 420: Height = 300
  CREATE LV AS QLISTVIEW
    Width = 400: Height = 200
    ViewStyle = vsReport
    SmallImages = Small
    LargeImages = Large
    CheckBoxes = True
    SortType = stText
    AddColumns "Name", "Kind"
    Column(0).Width = 120
    AddItems "pear", "Apple", "fig"
    OnChange = Changed
    OnClick = Clicked
    OnColumnClick = Headed
  END CREATE
  CREATE Btn2 AS QBUTTON
    Top = 210 : Caption = "Icons"
    OnClick = ToIcons
  END CREATE
  CREATE Btn AS QBUTTON
    Top = 210 : Left = 100 : Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 240: Width = 400
  END CREATE
END CREATE

SUB Changed (Index AS INTEGER, Change AS BYTE)
  Log = Log + "c" + STR$(Index) + ":" + STR$(Change) + " "
END SUB

SUB Clicked
  Log = Log + "k "
END SUB

SUB Headed (Column AS INTEGER)
  Log = Log + "h" + STR$(Column) + " "
END SUB

SUB ToIcons
  LV.ViewStyle = vsIcon
END SUB

SUB Report
  DIM Picked AS STRING
  Picked = STR$(LV.ItemIndex) + " " + STR$(LV.Item(1).Checked) + " " + LV.Item(0).Caption + " " + STR$(LV.ViewStyle)
  LV.MultiSelect = True
  LV.Item(0).Selected = True
  Lbl.Caption = Log + "| " + Picked + " " + STR$(LV.SelCount) + " " + STR$(LV.ItemIndex)
END SUB

Form.ShowModal
