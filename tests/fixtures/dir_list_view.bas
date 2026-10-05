' QDIRLISTVIEW (RapidQ's QDirListView.inc, the manual's Appendix A) as
' RapidR has it built in: a list view of a folder's folders and files —
' Name, Size, Type, Date Modified — with ShowRoot's "..", a Mask, the
' library's events: its own OnChange keeps FileName (a program's OnChange
' calls InheritOnChange first, as the manual's example), a double click or
' Enter goes into a folder or fires OnFileSelect, Backspace goes up. The
' program makes the folder it lists.
DECLARE SUB Changed
DECLARE SUB Picked (Tag AS INTEGER)
DECLARE SUB Report
MKDIR "dirlv"
MKDIR "dirlv/Inner"
DIM S AS QFILESTREAM
S.Open("dirlv/notes.txt", 65535)
S.WriteLine(STRING$(2100, "x"))
S.Close
S.Open("dirlv/Inner/deep.bas", 65535)
S.WriteLine("x")
S.Close
CREATE Form AS QFORM
  Caption = "dir list view"
  Width = 500
  Height = 260
  CREATE DirList AS QDIRLISTVIEW
    Left = 0 : Top = 0 : Width = 490 : Height = 150
    ShowRoot = 1
    ViewStyle = 3
    OnChange = Changed
    OnFileSelect = Picked
  END CREATE
  CREATE Btn AS QBUTTON
    Left = 10 : Top = 160
    Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 10 : Top = 195 : Width = 480
    Caption = "-"
  END CREATE
END CREATE
SUB Changed
  DirList.InheritOnChange
END SUB
SUB Picked (Tag AS INTEGER)
  Lbl.Caption = Lbl.Caption + " pick:" + MID$(DirList.FileName, RINSTR(DirList.FileName, "dirlv"))
END SUB
SUB Report
  DIM i AS INTEGER
  Lbl.Caption = Lbl.Caption + " [" + STR$(DirList.ItemIndex) + ">" + MID$(DirList.FileName, RINSTR(DirList.FileName, "dirlv") + 5) + STR$(DirList.ItemCount)
  FOR i = 0 TO DirList.ItemCount - 1
    Lbl.Caption = Lbl.Caption + " " + DirList.Item(i).Caption + "/" + STR$(DirList.Item(i).ImageIndex)
  NEXT i
  Lbl.Caption = Lbl.Caption + " " + DirList.SubItem(2, 0) + "|" + DirList.SubItem(2, 1) + "]"
END SUB
DirList.Directory = CURDIR$ + "/dirlv"
Form.ShowModal
KILL "dirlv/notes.txt"
KILL "dirlv/Inner/deep.bas"
RMDIR "dirlv/Inner"
RMDIR "dirlv"
