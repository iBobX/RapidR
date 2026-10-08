' An object's name used as a type (RC.EXE, probes 2026-10-08): `DIM x AS
' Lst` after `DIM Lst AS QSTRINGLIST` makes a new QSTRINGLIST, a SUB's
' parameter `p AS Lst` takes one (RapidQ's Choosecolor.bas: `Sender AS
' BUTTON` after `CREATE Button AS QBUTTON`). RC.EXE's output.
$APPTYPE CONSOLE
DIM Lst AS QSTRINGLIST
SUB S(p AS LST)
  p.AddItems "a"
  PRINT p.ItemCount
END SUB
S Lst
S Lst
DIM x AS Lst
x.AddItems "b"
PRINT x.ItemCount; Lst.ItemCount
