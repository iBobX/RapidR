' RapidQ syntax from the example corpus: a keyword as a declared variable
' (`DEFSTR return`, qcgi.inc), an array field without bounds (`Hint() AS
' STRING`, QtoolBar.inc), and a component's own Panel(i) inside its CREATE
' while the program has a Panel array of its own.
FUNCTION Get$ (n AS INTEGER) AS STRING
  DEFSTR return
  return = "v" + STR$(n)
  Get$ = return
END FUNCTION
PRINT Get$(3)
TYPE TB
  Hint() AS STRING
  N AS INTEGER
END TYPE
DIM T AS TB
T.Hint(2) = "x"
PRINT T.Hint(2); UBOUND(T.Hint)
DIM Panel(3) AS INTEGER
Panel(1) = 9
CREATE Z AS QSTATUSBAR
  AddPanels "a", "b", "c"
  Panel(0).Width = 75 : Panel(1).Width = 76 : _
    Panel(2).Width = 77
END CREATE
PRINT Z.Panel(2).Width; Panel(1)
