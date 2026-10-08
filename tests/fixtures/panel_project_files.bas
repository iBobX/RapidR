' RPROJECTTREE reading its project through the program's files (the disk
' on the desktop, the page's own files on the web): an .rrproj, and a
' source file as an implicit project with what it $INCLUDEs (RAPIDQ.INC
' isn't there: skipped). A form's components come from its file; Reveal
' opens the nodes above one; the keyboard walks the tree (Down, Right,
' End, Left, type-ahead) and Enter opens a file.
DIM log AS STRING

SUB Opened (Path AS STRING)
  log = log + " O:" + Path
END SUB

SUB Selected (Path AS STRING)
  log = log + " S:" + Path
END SUB

SUB Report
  DIM i AS INTEGER
  DIM s AS STRING
  s = T1.ProjectName + STR$(T1.FileCount) + ":"
  FOR i = 0 TO T1.FileCount - 1
    s = s + " " + T1.File(i) + "/" + T1.FileKind(T1.File(i))
  NEXT i
  s = s + " | " + T2.ProjectName + STR$(T2.FileCount) + ":"
  FOR i = 0 TO T2.FileCount - 1
    s = s + " " + T2.File(i) + "/" + T2.FileKind(T2.File(i))
  NEXT i
  Info.Caption = s
  T2.Reveal "panel_project_form.rr#Button1"
  Lbl.Caption = log + " | " + T1.Selected + " | " + T2.Selected + " " + STR$(T2.Modified)
END SUB

CREATE Form AS QFORM
  Caption = "Project files"
  Width = 640: Height = 420
  CREATE T1 AS RPROJECTTREE
    Left = 8: Top = 8: Width = 300: Height = 240
    OnOpen = Opened
    OnSelect = Selected
  END CREATE
  CREATE T2 AS RPROJECTTREE
    Left = 320: Top = 8: Width = 300: Height = 240
  END CREATE
  CREATE BReport AS QBUTTON
    Left = 8: Top = 256: Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 8: Top = 290: Width = 620
  END CREATE
  CREATE Info AS QLABEL
    Left = 8: Top = 340: Width = 620
  END CREATE
END CREATE

DIM dir AS STRING
IF FILEEXISTS("tests/fixtures/panel_project_demo.rrproj") THEN dir = "tests/fixtures/"
T1.Project = dir + "panel_project_demo.rrproj"
T2.Project = dir + "panel_project_main.rr"
Form.ShowModal
