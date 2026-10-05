' QDOWNLOAD (docs/io-media-plan.md §3): LeechFile from the tests' own HTTP
' server (tests/http_test_server.mjs: ENVIRON$("RAPIDR_TEST_HTTP") is its
' host:port — on the desktop the environment, in the browser the page's
' RAPIDR_TEST_ENV), which sends the file slowly: while LeechFile waits the
' program's QTIMER keeps ticking. OutVar with State, then OutFile with the
' StateGauge, then a file the server doesn't know (error 11). Checked
' natively and interpreted by tests/native_gui_events.mjs, in the browser by
' tests/web_gui_parity.mjs.
$INCLUDE "RAPIDQ.INC"
$INCLUDE "qdownload.inc"
DECLARE SUB Fetch
DECLARE SUB Tick
DIM ticks AS INTEGER
DIM d AS QDOWNLOAD
CREATE Form AS QFORM
  Caption = "download"
  Width = 320
  CREATE b1 AS QBUTTON
    Left = 10
    Top = 10
    Caption = "Fetch"
    OnClick = Fetch
  END CREATE
  CREATE lbl AS QLABEL
    Left = 10
    Top = 80
    Width = 300
  END CREATE
  CREATE tm AS QTIMER
    Interval = 30
    Enabled = 0
    OnTimer = Tick
  END CREATE
END CREATE
d.StateGauge.Parent = Form
d.StateGauge.Top = 45

SUB Tick
  ticks = ticks + 1
END SUB

SUB Fetch
  DIM r AS STRING, host AS STRING
  host = ENVIRON$("RAPIDR_TEST_HTTP")
  d.Server = LEFT$(host, INSTR(host, ":") - 1)
  d.Port = VAL(MID$(host, INSTR(host, ":") + 1))
  d.File = "tests/fixtures/download_data.txt"
  tm.Enabled = 1
  r = STR$(d.LeechFile) + " " + STR$(d.Size) + " " + STR$(d.State) + " [" + LEFT$(d.OutVar, 5) + "]" + STR$(LEN(d.OutVar))
  r = r + " ticked " + STR$(ticks > 0)
  d.OutDevice = 2
  d.StateDevice = 2
  d.OutFile = "download_out.tmp"
  r = r + " | " + STR$(d.LeechFile) + " " + STR$(d.StateGauge.Position) + " " + STR$(FILELEN("download_out.tmp"))
  KILL "download_out.tmp"
  d.File = "tests/fixtures/no_such_file.txt"
  r = r + " | " + STR$(d.LeechFile) + " " + STR$(d.LastError) + " " + d.LastStringError
  tm.Enabled = 0
  lbl.Caption = r
END SUB

Form.ShowModal
