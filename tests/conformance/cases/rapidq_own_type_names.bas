' A program's own TYPE named like a RapidR-only component (RapidQ's
' QToolbar example: TYPE QToolBar EXTENDS QPANEL) is the program's TYPE, as
' in RapidQ — the .expected is RC.EXE's output.
TYPE QToolBar EXTENDS QPANEL
  Hint(5) AS STRING
  Buttons AS INTEGER
  SUB Load
    QToolBar.Buttons = 3
    PRINT "load "; QToolBar.Hint(1); " "; QToolBar.Buttons
  END SUB
END TYPE
CREATE F AS QFORM
  CREATE T AS QToolBar
    Hint(0) = "a"
    Hint(1) = "b"
    Load
  END CREATE
END CREATE
PRINT T.Hint(0); " "; T.Buttons; " "; T.Width
