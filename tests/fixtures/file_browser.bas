' QDIRTREE and QFILELISTBOX together, as RapidQ's picture viewer uses them:
' the tree's OnChange shows the chosen directory's files. Checked natively
' and interpreted by tests/native_gui_events.mjs (run from the repo root).
DECLARE SUB DirChange
DIM Changes AS INTEGER
CREATE Form AS QFORM
  Caption = "Files": Width = 420: Height = 260
  CREATE DirTree AS QDIRTREE
    Width = 200: Height = 180
    OnChange = DirChange
  END CREATE
  CREATE Files AS QFILELISTBOX
    Left = 210: Width = 200: Height = 180
    Mask = "*.txt;*.bin"
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 190: Width = 400
  END CREATE
END CREATE
DirTree.InitialDir = CURDIR$ + "/tests/conformance/cases/resource_files"
Form.ShowModal

SUB DirChange
  Changes = Changes + 1
  Files.Directory = DirTree.Directory
  Lbl.Caption = RIGHT$(DirTree.Directory, 14) + "|" + STR$(Files.ItemCount) + "|" + Files.Item(0) + "|" + STR$(Changes)
END SUB
