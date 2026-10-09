' A form's OnResize as RapidQ fires it — the .expected is RC.EXE's output
' (docs/rapidq-ground-truth.md): a size set before the form shows fires
' none; its first Show (or ShowModal) fires OnResize, OnShow, OnResize;
' shown again after it was hidden, only OnShow. (RapidQ's examples size
' their canvases in OnResize: CoolGauge, credits.)
DIM F AS QFORM
DIM C AS QCANVAS
DIM n AS INTEGER
F.Width = 300: F.Height = 200
C.Parent = F
SUB R
  n = n + 1
  PRINT "resize "; n
  C.Width = F.ClientWidth
END SUB
SUB Sh
  PRINT "show "; n
END SUB
F.OnResize = R
F.OnShow = Sh
PRINT "before "; C.Width; " "; C.Height
F.Width = 320
PRINT "after width "; n
F.Show
PRINT "shown "; n; " "; C.Width = F.ClientWidth
DOEVENTS
PRINT "events "; n
F.Visible = 0
F.Show
PRINT "again "; n
F.Close
PRINT "end "; n
