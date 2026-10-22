' An OnClick handler whose parameter isn't the Sender's type (RC.EXE,
' 2026-10-22, docs/rapidq-ground-truth.md): RapidQ compiles it — in a
' CREATE block and as `Obj.OnClick = H` — and runs it when the event fires;
' a number parameter reads 0 there, an object one is the Sender. (A STRING
' one: RapidQ leaves it as the SUB's last call left it, "" before any — its
' parameters keep their place between calls; RapidR gives the Sender's
' name. Not printed here.) The .expected is RC.EXE's output.
SUB H(N AS INTEGER)
  PRINT "H called, N is 0: "; N = 0
END SUB
SUB HS(T AS STRING)
  PRINT "HS called"
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
END CREATE
CK2.OnClick = HS
CK.Checked = 1
CK2.Checked = 1
CK3.Checked = 1
PRINT "done"
