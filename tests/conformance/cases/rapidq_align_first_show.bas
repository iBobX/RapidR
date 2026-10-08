' Aligned controls made before their form shows, placed as RapidQ places
' them at that first Show — the .expected is RC.EXE's output: in creation
' order (tool buttons alLeft in a panel, a splitter before its alLeft
' panel, alTop panels), an alBottom / alRight one nearer the edge only when
' it reaches further (the higher alBottom panel at the bottom; alRight
' buttons of one width: the first at the edge); one aligned after the Show
' goes first among equals. (RapidQ's QToolbar example; RapidR keeps the
' layout current before the Show too, in that same order.)
CREATE F AS QFORM
  Width = 400: Height = 300
  CREATE P AS QPANEL
    Align = 1
    CREATE B1 AS QCOOLBTN
      Align = 3
      Width = 25
    END CREATE
    CREATE B2 AS QCOOLBTN
      Align = 3
      Width = 25
    END CREATE
    CREATE B3 AS QCOOLBTN
      Align = 3
      Width = 30
    END CREATE
    CREATE B4 AS QBUTTON
      Width = 40
      Align = 3
    END CREATE
  END CREATE
  CREATE S AS QSPLITTER
    Align = 3
  END CREATE
  CREATE T AS QPANEL
    Align = 3
    Width = 100
  END CREATE
  CREATE L1 AS QPANEL
    Align = 2
    Height = 20
  END CREATE
  CREATE L2 AS QPANEL
    Align = 2
    Height = 30
  END CREATE
  CREATE U1 AS QPANEL
    Height = 20
    Align = 1
  END CREATE
END CREATE
DIM D1 AS QBUTTON
DIM D2 AS QBUTTON
D1.Parent = F: D1.Align = 4: D1.Width = 20
D2.Parent = F: D2.Align = 4: D2.Width = 20
F.Show
PRINT B1.Left; " "; B2.Left; " "; B3.Left; " "; B4.Left
PRINT S.Left; " "; T.Left; " "; L1.Top - L2.Top; " "; P.Top; " "; U1.Top; " "; D1.Left - D2.Left
DIM X AS QPANEL
X.Parent = F
X.Align = 3
X.Width = 10
PRINT S.Left; " "; T.Left; " "; X.Left
F.Close
