' A TYPE's instance CREATEd in a component's block inside another's (a
' QFolderTree on a panel on a form, RapidQ's FolderTree demo) is made where it
' stands, with the panel as its Parent, and both blocks go on after it
' (the deeper form of the GUI case nested_typed_create).
$APPTYPE CONSOLE
TYPE TNote EXTENDS QLABEL
  Tag2 AS INTEGER
  CONSTRUCTOR
    Caption = "note"
    Tag2 = 7
  END CONSTRUCTOR
END TYPE
CREATE Form AS QFORM
  Caption = "before"
  CREATE P AS QPANEL
    Left = 5
    CREATE N AS TNote
      Left = 10
    END CREATE
    Width = 200
  END CREATE
  Caption = "after"
END CREATE
PRINT "form ["; Form.Caption; "] note ["; N.Caption; "] "; N.Tag2; " "; N.Left; " "; P.Width; " "; P.Left
