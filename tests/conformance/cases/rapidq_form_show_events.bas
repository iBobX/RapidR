' The events a form gets as it shows and changes size, in RapidQ's order —
' the .expected is RC.EXE's output: its first Show fires OnResize, OnShow,
' OnResize; OnPaint comes when the program next lets its windows work
' (DOEVENTS here), not inside the Show; a size the program sets on a shown
' form fires OnResize at once (Height, ClientWidth), its OnPaint later; a
' new position fires neither. Programs that lay out in OnResize (RapidQ's
' SPYINFO3A: its "Lock to" check box) rely on it.
$APPTYPE CONSOLE
DIM F AS QFORM
DIM L AS STRING
SUB R
  L = L + "Resize "
END SUB
SUB S
  L = L + "Show "
END SUB
SUB P
  L = L + "Paint "
END SUB
F.Width = 300: F.Height = 200
F.OnResize = R: F.OnShow = S: F.OnPaint = P
F.Show
L = L + "| "
DOEVENTS
L = L + "| "
F.Height = F.Height + 20
L = L + "height | "
F.ClientWidth = 200
L = L + "clientwidth | "
F.Left = 20
L = L + "left | "
F.Width = F.Width
L = L + "same | "
PRINT L
F.Close
