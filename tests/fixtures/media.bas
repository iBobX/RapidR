' QMIDI and QWAVE (docs/io-media-plan.md §4-§5), the way RapidQ programs
' drive them: through their Timer's OnChange. A song (media_song.mid,
' made by tools/make_media_fixture.py: two quarter notes at 120 bpm) opened
' — and a missing file, MCI's error text — played to its end (the tests
' have no MIDI output: RAPIDR_TEST_MIDI; the object plays by the clock);
' then a new wave recorded for 300 ms from the tests' scripted input
' (RAPIDR_TEST_WAVE_IN=tone:440, never a microphone), saved, opened again
' and played to its end. Checked natively and interpreted by
' tests/native_gui_events.mjs, in the browser by tests/web_gui_parity.mjs.
$INCLUDE "RAPIDQ.INC"
$INCLUDE "QMidi.inc"
$INCLUDE "QWave.inc"
$RESOURCE SONG AS "media_song.mid"
DECLARE SUB Go
DECLARE SUB Pad
DECLARE SUB SongChanged(Position AS LONG)
DECLARE SUB WaveChanged(Position AS LONG)
DIM log AS STRING
DIM songTicks AS INTEGER
DIM waveTicks AS INTEGER
DIM stage AS INTEGER
DIM Midi AS QMIDI
DIM Wave AS QWAVE
Midi.OnChange = SongChanged
Midi.Timer.Interval = 50
Wave.OnChange = WaveChanged
Wave.Timer.Interval = 50
CREATE Form AS QFORM
  Caption = "media"
  Width = 320
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
    Top = 50
    Width = 300
    Caption = "-"
  END CREATE
END CREATE

SUB Pad
END SUB

SUB Go
  EXTRACTRESOURCE SONG, "media_song.tmp.mid"
  log = STR$(Midi.Open("no_such_song.mid")) + " " + LEFT$(Midi.Error, 26) + " " + STR$(Midi.State) + " | "
  log = log + STR$(Midi.Open("media_song.tmp.mid")) + " " + STR$(Midi.Lenght) + " " + STR$(Midi.State) + " " + STR$(Midi.Timer.Interval) + " "
  Midi.Play
  log = log + STR$(Midi.State) + " " + STR$(Midi.Timer.Enabled) + " "
END SUB

SUB SongChanged(Position AS LONG)
  songTicks = songTicks + 1
  IF Midi.State = MD_STOP AND stage = 0 THEN
    stage = 1
    log = log + "end " + STR$(Position) + " " + STR$(songTicks > 3) + " " + STR$(Midi.Timer.Enabled) + " | "
    Midi.Close
    KILL "media_song.tmp.mid"
    Wave.New
    log = log + STR$(Wave.Bits) + " " + STR$(Wave.Frequence) + " " + STR$(Wave.Mode) + " " + STR$(Wave.Lenght) + " "
    Wave.Lenght = 300
    Wave.Record
    log = log + STR$(Wave.State) + " "
  END IF
END SUB

SUB WaveChanged(Position AS LONG)
  waveTicks = waveTicks + 1
  IF Wave.State = WV_STOP AND stage = 1 THEN
    stage = 2
    log = log + "rec " + STR$(Wave.Lenght) + " " + STR$(Wave.Save("media_rec.tmp.wav")) + " "
    Wave.Close
    log = log + STR$(Wave.Open("media_rec.tmp.wav")) + " " + STR$(Wave.Lenght) + " " + STR$(Wave.Bits) + " "
    Wave.CurrentPos = 100
    Wave.Play
  ELSEIF Wave.State = WV_STOP AND stage = 2 THEN
    stage = 3
    log = log + "played " + STR$(Position) + " " + STR$(waveTicks > 3)
    Wave.Close
    KILL "media_rec.tmp.wav"
    lbl.Caption = log
  END IF
END SUB

Form.ShowModal
