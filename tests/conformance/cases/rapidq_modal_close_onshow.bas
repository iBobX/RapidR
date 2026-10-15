' A Close in a form's OnShow under ShowModal is lost — VCL's ShowModal sets
' ModalResult to 0 once the form has shown —: the form stays and gets no
' OnClose until the program closes it later. The .expected is RC.EXE's
' output (RapidQ's OpenGL/GL_test1.bas closes its form in OnShow when a
' texture file is missing, and under RapidQ its window stays).
$APPTYPE CONSOLE
DECLARE SUB FShow
DECLARE SUB FClose (Action AS INTEGER)
DECLARE SUB Tick
CREATE F AS QFORM
  Caption = "probe F"
  OnShow = FShow
  OnClose = FClose
END CREATE
CREATE T AS QTIMER
  Interval = 300
  Enabled = 0
  OnTimer = Tick
END CREATE
SUB FShow
  PRINT "f onshow"
  F.Close
  PRINT "f after close visible="; F.Visible
  T.Enabled = 1
END SUB
SUB FClose (Action AS INTEGER)
  PRINT "f onclose"
END SUB
SUB Tick
  T.Enabled = 0
  PRINT "tick visible="; F.Visible
  F.ModalResult = 1
END SUB
DIM r AS INTEGER
r = F.ShowModal
PRINT "f showmodal="; r
