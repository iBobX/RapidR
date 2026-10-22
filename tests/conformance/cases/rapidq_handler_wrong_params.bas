' An OnClick handler whose parameter isn't the Sender's type (RC.EXE,
' 2026-10-22, docs/rapidq-ground-truth.md): RapidQ compiles it — in a
' CREATE block and as `Obj.OnClick = H` — and runs it when the event fires:
' an object parameter is the Sender, a STRING one reads "" and a number one
' 0 (rapidr_ast::event_params; on the VM, native builds and the web alike).
' (RapidQ's parameters keep their place between calls: after a direct call
' `HS "direct"` its next event finds "direct" there, where RapidR gives ""
' every time — not done here.) The .expected is RC.EXE's output.
SUB H(N AS INTEGER)
  PRINT "H called, N is 0: "; N = 0
END SUB
SUB HS(T AS STRING)
  PRINT "HS called ["; T; "] "; LEN(T)
END SUB
SUB HD(D AS DOUBLE, S AS STRING)
  PRINT "HD called "; D = 0; " ["; S; "]"
END SUB
SUB HO(Sender AS QCHECKBOX)
  PRINT "HO called, Checked "; Sender.Checked
END SUB
CREATE Form AS QFORM
  CREATE CK AS QCHECKBOX
    OnClick = H
  END CREATE
  CREATE CK2 AS QCHECKBOX
  END CREATE
  CREATE CK3 AS QCHECKBOX
    OnClick = HO
  END CREATE
  CREATE CK4 AS QCHECKBOX
    OnClick = HD
  END CREATE
END CREATE
CK2.OnClick = HS
CK.Checked = 1
CK2.Checked = 1
CK3.Checked = 1
CK4.Checked = 1
HS "direct"
PRINT "done"
