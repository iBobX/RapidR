' A timer keeps ticking after a modal dialog closes, as under RapidQ
' (RC.EXE, 2026-10-16: a timer that showed a dialog ticks on once it is
' closed). The end of any ShowModal used to stop every timer of the
' program — RapidR Studio's own after its Settings dialog. Checked natively,
' interpreted and in the browser by tests/gui_parity_cases.mjs.
DECLARE SUB Tick
DECLARE SUB OpenDlg
DECLARE SUB CloseDlg
DIM n AS INTEGER, closedAt AS INTEGER
CREATE Form AS QFORM
  Caption = "Main": Width = 300: Height = 150
  CREATE Btn AS QBUTTON
    Caption = "Open": OnClick = OpenDlg
  END CREATE
  CREATE Lbl AS QLABEL
    Top = 40: Width = 280: Caption = "waiting"
  END CREATE
END CREATE
CREATE Dlg AS QFORM
  Caption = "Dialog": Width = 200: Height = 100
END CREATE
CREATE T AS QTIMER
  Interval = 50: Enabled = 1: OnTimer = Tick
END CREATE
CREATE T2 AS QTIMER
  Interval = 150: Enabled = 0: OnTimer = CloseDlg
END CREATE

SUB Tick
  n = n + 1
  IF closedAt > 0 AND n >= closedAt + 5 THEN Lbl.Caption = "ticking after the dialog"
END SUB

SUB OpenDlg
  T2.Enabled = 1
  Dlg.ShowModal
END SUB

SUB CloseDlg
  T2.Enabled = 0
  closedAt = n + 1
  Dlg.Close
END SUB

Form.ShowModal
