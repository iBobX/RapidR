' QDXSOUND (DirectSound, docs/directx-plan.md, stage D2): a WAV file
' (dx_beep.wav, made by tools/make_dx_fixture.py: 4000 bytes of 8-bit mono
' at 8000 Hz, extracted from a $RESOURCE) — Size and Frequency from the file,
' Volume, Pan, Looped, Play, Playing and Position while it plays, Stop
' keeping the place, Position set, Frequency changing the speed, the end of
' a sound played once (stopped, back at 0). The GUI tests make no sound:
' Playing and Position follow the clock. Checked natively and interpreted by
' tests/native_gui_events.mjs, in the browser by tests/web_gui_parity.mjs.
$INCLUDE "RAPIDQ.INC"
$RESOURCE BEEP_WAV AS "dx_beep.wav"
DECLARE SUB Go
DECLARE SUB Check
DECLARE SUB Report
DIM log AS STRING
DIM kept AS INTEGER
DIM t AS DOUBLE
CREATE Form AS QFORM
  Caption = "dx sound"
  CREATE Snd AS QDXSOUND
  END CREATE
  CREATE b1 AS QBUTTON
    Left = 10
    Top = 10
    Caption = "Go"
    OnClick = Go
  END CREATE
  CREATE b2 AS QBUTTON
    Left = 90
    Top = 10
    Caption = "Check"
    OnClick = Check
  END CREATE
  CREATE b3 AS QBUTTON
    Left = 170
    Top = 10
    Caption = "Report"
    OnClick = Report
  END CREATE
  CREATE lbl AS QLABEL
    Left = 10
    Top = 50
    Width = 300
    Caption = "-"
  END CREATE
END CREATE

SUB Go
  EXTRACTRESOURCE BEEP_WAV, "dx_beep.wav"
  Snd.FileName = "dx_beep.wav"
  log = STR$(Snd.Size) + "," + STR$(Snd.Frequency) + "," + STR$(Snd.Volume) + "," + STR$(Snd.Pan) + "," + STR$(Snd.Playing) + " "
  Snd.Looped = True
  Snd.Volume = 80
  Snd.Pan = -30
  Snd.Play
  log = log + "play" + STR$(Snd.Playing) + " "
END SUB

SUB Check
  log = log + "still" + STR$(Snd.Playing) + IIF(Snd.Position >= 0 AND Snd.Position < Snd.Size, " in ", " out ")
  Snd.Stop
  kept = Snd.Position
  t = TIMER
  WHILE TIMER - t < 0.1
  WEND
  log = log + "stop" + STR$(Snd.Playing) + IIF(Snd.Position = kept, " kept ", " moved ")
  Snd.Position = 1000
  Snd.Looped = False
  Snd.Frequency = 16000
  log = log + STR$(Snd.Position) + "," + STR$(Snd.Frequency) + " "
  Snd.Play
  t = TIMER
END SUB

SUB Report
  ' (3000 bytes left at 16000 bytes a second: over in 0.19 s)
  WHILE TIMER - t < 0.4
  WEND
  log = log + "end" + STR$(Snd.Playing) + "," + STR$(Snd.Position) + " "
  lbl.Caption = log + "|" + STR$(Snd.Volume) + "," + STR$(Snd.Pan) + "," + STR$(Snd.Looped) + "," + Snd.FileName
  KILL "dx_beep.wav"
END SUB

Form.ShowModal
