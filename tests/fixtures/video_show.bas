' QVIDEO shown without playing (docs/io-media-plan.md §6): MCI keeps the
' window it makes hidden after Open — Show (or Play) shows it, with the
' frame it's at. The Cinepak clip of video_player.bas (key frames 0 and 4)
' opened on the form, moved and zoomed to 200 % as there, sought to frame
' 5 (MCIAVI lands on its key frame, 4) and shown: the window shows frame
' 4, nothing playing — as RC.EXE's QVideo.inc does in the Windows VM
' (Open alone: nothing; Show: frame 0; Show then CurrentFrame = 12: frame
' 12; Play then Stop: frame 0). Checked natively and interpreted by
' tests/native_gui_events.mjs (and what the window shows), in the browser
' by tests/web_gui_parity.mjs.
$INCLUDE "RAPIDQ.INC"
$INCLUDE "QVideo.inc"
$RESOURCE CLIP AS "video/cinepak.avi"
DECLARE SUB Go
DIM V AS QVIDEO
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
  CREATE lbl AS QLABEL
    Left = 10
    Top = 40
    Width = 300
    Caption = "-"
  END CREATE
END CREATE
V.Parent = Form.Handle

SUB Go
  EXTRACTRESOURCE CLIP, "video_show.tmp.avi"
  lbl.Caption = STR$(V.Open("video_show.tmp.avi"))
  V.Top = 70
  V.Left = 20
  V.Width = V.ImgWidth * 2
  V.Height = V.ImgHeight * 2
  V.CurrentFrame = 5
  V.Show
  lbl.Caption = lbl.Caption + " " + STR$(V.State) + " " + STR$(V.CurrentFrame) + " " + STR$(V.Timer.Enabled)
  KILL "video_show.tmp.avi"
END SUB

Form.ShowModal
