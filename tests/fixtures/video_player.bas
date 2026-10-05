' QVIDEO (docs/io-media-plan.md §6) the way D. Glodt's video.bas drives
' it: on a form (Parent = Form.Handle), its Timer's OnChange following
' the play. A Cinepak clip (video/cinepak.avi, made by
' tools/make_avi_fixtures.py: 32x24, 8 frames at 10 fps, key frames 0
' and 4) opened — and a missing file, MCI's error text —, a frame sought
' (MCIAVI lands on the key frame at or before it), moved and zoomed to
' 200 % (MoveWindow: the picture stretched), played to its end; then a
' QVIDEO with no Parent opened in a window of its own (a popup's and an
' overlapped window's sizes, its caption the file's name in the library's
' 128-character buffer, as in RC.EXE), and
' the first left showing frame 5's key frame, 4. Checked natively and
' interpreted by tests/native_gui_events.mjs (and what the window shows),
' in the browser by tests/web_gui_parity.mjs.
$INCLUDE "RAPIDQ.INC"
$INCLUDE "QVideo.inc"
$RESOURCE CLIP AS "video/cinepak.avi"
DECLARE SUB Go
DECLARE SUB Pad
DECLARE SUB Changed(Position AS LONG, TimePos AS LONG)
DIM log AS STRING
DIM ticks AS INTEGER
DIM stage AS INTEGER
DIM V AS QVIDEO
DIM P AS QVIDEO
V.OnChange = Changed
V.Timer.Interval = 50
CREATE Form AS QFORM
  Caption = "video"
  Width = 320
  Height = 200
  CREATE b1 AS QBUTTON
    Left = 10
    Top = 10
    Caption = "Go"
    OnClick = Go
  END CREATE
  CREATE b2 AS QBUTTON
    Left = 90
    Top = 10
    Caption = "Wait"
    OnClick = Pad
  END CREATE
  CREATE lbl AS QLABEL
    Left = 10
    Top = 40
    Width = 300
    Caption = "-"
  END CREATE
END CREATE
V.Parent = Form.Handle

SUB Pad
END SUB

SUB Go
  EXTRACTRESOURCE CLIP, "video_clip.tmp.avi"
  log = STR$(V.Open("no_such_film.avi")) + " " + LEFT$(V.Error, 26) + " | "
  log = log + STR$(V.Open("video_clip.tmp.avi")) + " " + STR$(V.Lenght) + " " + STR$(V.LenghtTime) + " " + STR$(V.ImgWidth) + "x" + STR$(V.ImgHeight)
  log = log + " " + STR$(V.Width) + "x" + STR$(V.Height) + " " + STR$(V.Handle <> 0) + " " + STR$(V.State) + " "
  V.CurrentFrame = 3
  log = log + STR$(V.CurrentFrame) + " "
  V.Top = 70
  V.Left = 20
  V.Width = V.ImgWidth * 2
  V.Height = V.ImgHeight * 2
  V.Play
  log = log + STR$(V.State) + " " + STR$(V.Timer.Enabled) + " "
END SUB

SUB Changed(Position AS LONG, TimePos AS LONG)
  ticks = ticks + 1
  IF V.State = VD_STOP AND stage = 0 THEN
    stage = 1
    log = log + "end " + STR$(Position) + " " + STR$(TimePos) + " " + STR$(ticks > 3) + " " + STR$(V.Timer.Enabled) + " | "
    P.BorderStyle = 0
    log = log + STR$(P.Open("video_clip.tmp.avi")) + " " + STR$(P.Width) + "x" + STR$(P.Height) + " " + LEFT$(P.Caption, INSTR(P.Caption, CHR$(0)) - 1) + " " + STR$(LEN(P.Caption)) + " "
    P.Close
    P.BorderStyle = 1
    P.Caption = "Clip"
    P.Open("video_clip.tmp.avi")
    log = log + STR$(P.Width) + "x" + STR$(P.Height) + " " + P.Caption + " "
    P.Close
    V.CurrentFrame = 5
    log = log + STR$(V.CurrentFrame) + " " + STR$(V.State)
    KILL "video_clip.tmp.avi"
    lbl.Caption = log
  END IF
END SUB

Form.ShowModal
