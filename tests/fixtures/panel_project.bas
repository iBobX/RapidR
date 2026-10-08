' RPROJECTTREE (RapidR Studio's project explorer, docs/ide-components.md
' 3.5): a project given as text (LoadText: an .rrproj; SetFileText: a
' form's source, its components listed from it). The user opens Form1.rr
' (its chevron), double-clicks Button1 (OnSelect, OnOpen), renames
' Utils.rr with F2, typing "tools" and Enter (OnRename), deletes
' Report.rr (Delete, Enter on the strip's Remove: OnDelete), drags
' tools.rr above Main.rr (OnMove) and About.rr out of its folder onto the
' Forms group; the program renames Main.rr to Summary (its OnRename
' cancels that), makes a new form named by typing "dialog" (OnNewFile),
' and lists the project's files in their order.
DIM log AS STRING
DIM NL AS STRING
NL = CHR$(10)

SUB TreeSelect (Path AS STRING)
  log = log + " S:" + Path
END SUB

SUB TreeOpen (Path AS STRING)
  log = log + " O:" + Path
END SUB

SUB TreeRename (OldPath AS STRING, NewPath AS STRING, Cancel AS INTEGER)
  log = log + " R:" + OldPath + ">" + NewPath
  IF NewPath = "Summary.rr" THEN Cancel = 1
END SUB

SUB TreeDelete (Path AS STRING, Cancel AS INTEGER)
  log = log + " D:" + Path
END SUB

SUB TreeMove (Path AS STRING, NewPath AS STRING, Index AS INTEGER)
  log = log + " M:" + Path + ">" + NewPath + "@" + STR$(Index)
END SUB

SUB TreeNewFile (Path AS STRING, Kind AS STRING)
  log = log + " N:" + Path + "/" + Kind
END SUB

SUB RenameMain
  Tree.Rename "Main.rr", "Summary"
END SUB

SUB NewForm
  log = log + " new:" + Tree.NewFile("form")
END SUB

SUB Report
  DIM i AS INTEGER
  DIM files AS STRING
  FOR i = 0 TO Tree.FileCount - 1
    files = files + " " + Tree.File(i)
  NEXT i
  Lbl.Caption = log
  Info.Caption = Tree.ProjectName + ":" + files + " |" + STR$(Tree.Modified) + STR$(INSTR(Tree.ProjectText, "tools.rr") > 0) + " " + Tree.Selected + " " + Tree.FileKind("About.rr")
END SUB

CREATE Form AS QFORM
  Caption = "Project"
  Width = 520: Height = 560
  CREATE Tree AS RPROJECTTREE
    Left = 8: Top = 8: Width = 260: Height = 480
    OnSelect = TreeSelect
    OnOpen = TreeOpen
    OnRename = TreeRename
    OnDelete = TreeDelete
    OnMove = TreeMove
    OnNewFile = TreeNewFile
  END CREATE
  CREATE BRename AS QBUTTON
    Left = 280: Top = 8: Width = 100: Caption = "Rename Main"
    OnClick = RenameMain
  END CREATE
  CREATE BNew AS QBUTTON
    Left = 280: Top = 40: Width = 100: Caption = "New form"
    OnClick = NewForm
  END CREATE
  CREATE BReport AS QBUTTON
    Left = 280: Top = 72: Width = 100: Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 280: Top = 110: Width = 230
  END CREATE
  CREATE Info AS QLABEL
    Left = 280: Top = 140: Width = 230
  END CREATE
END CREATE

DIM P AS STRING
P = "format = 2" + NL + "name = 'Inventory'" + NL + "main = 'Main.rr'" + NL
P = P + "[[files]]" + NL + "path = 'Main.rr'" + NL + "kind = 'module'" + NL
P = P + "[[files]]" + NL + "path = 'Form1.rr'" + NL + "kind = 'form'" + NL
P = P + "[[files]]" + NL + "path = 'Utils.rr'" + NL + "kind = 'module'" + NL
P = P + "[[files]]" + NL + "path = 'forms/About.rr'" + NL + "kind = 'form'" + NL
P = P + "[[files]]" + NL + "path = 'Report.rr'" + NL + "kind = 'module'" + NL
P = P + "[[files]]" + NL + "path = 'lib/strings.inc'" + NL
P = P + "[[files]]" + NL + "path = 'data/stock.csv'" + NL
Tree.LoadText P, "Inventory.rrproj"
DIM F AS STRING
F = "CREATE Form1 AS QFORM" + NL + "  CREATE Panel1 AS QPANEL" + NL + "    CREATE Button1 AS QBUTTON: END CREATE" + NL
F = F + "    CREATE Edit1 AS QEDIT ' CREATE Ghost AS QLABEL" + NL + "    END CREATE" + NL + "  END CREATE" + NL
F = F + "  CREATE Label1 AS QLABEL" + NL + "  END CREATE" + NL + "END CREATE" + NL
Tree.SetFileText "Form1.rr", F
Tree.SetFileText "forms/About.rr", "CREATE About AS QFORM" + NL + "END CREATE" + NL
Form.ShowModal
