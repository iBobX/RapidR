' QMIDI, QWAVE, QVIDEO, QCDAUDIO with no device (docs/io-media-plan.md
' §4-§7): their defaults, Open's failures with MCI's own texts (checked in
' the Windows VM), the libraries' rules for Volume, CurrentFrame, Bits /
' Frequence / Mode, New and Save, a CD drive with no disc. A console
' program: their Timers never tick here.
$APPTYPE CONSOLE
$INCLUDE "QMidi.inc"
$INCLUDE "QWave.inc"
$INCLUDE "QVideo.inc"
$INCLUDE "Qcdaudio.inc"
DIM m AS QMIDI
DIM w AS QWAVE
DIM v AS QVIDEO
DIM c AS QCDAUDIO
PRINT "midi "; m.State; " "; m.FileOpen; " "; m.Lenght; " "; m.Timer.Interval; " "; m.Timer.Enabled; " "; m.Volume
PRINT m.Open("no_such_file.mid"); " "; m.Error; " "; m.State
OPEN "media_not.mid" FOR OUTPUT AS #1
PRINT #1, "not a song"
CLOSE #1
PRINT m.Open("media_not.mid"); " "; m.Error
KILL "media_not.mid"
m.Volume = 50
m.Volume = 150
PRINT m.Volume
m.CurrentFrame = 10
PRINT m.CurrentFrame
m.Play
PRINT m.State; " "; m.Timer.Enabled
w.New
PRINT "wave "; w.State; " "; w.FileOpen; " "; w.Bits; " "; w.Frequence; " "; w.Mode; " "; w.Lenght
w.Frequence = 22050
PRINT w.Frequence
w.Frequence = 44100
w.Bits = 16
w.Mode = 2
w.Bits = 12
PRINT w.Frequence; " "; w.Bits; " "; w.Mode
PRINT w.Save("media_empty.wav")
w.Close
PRINT w.State; " "; w.FileOpen; " "; w.Lenght
PRINT w.Open("media_empty.wav"); " "; w.Lenght; " "; w.Bits; " "; w.Frequence; " "; w.Mode
PRINT w.Open("media_empty.wav"); " "; w.Error; " "; w.FileOpen
KILL "media_empty.wav"
PRINT "video "; v.Open("no_such_film.avi"); " "; v.Error; " "; v.State; " "; v.Width
PRINT "cd "; c.Open; " ["; c.Error; "] "; c.Present; " "; c.State; " "; c.AudioOpen; " "; c.TrackNumber; " ["; c.Time; "]"
c.Play
c.CurrentTrack = 2
PRINT c.State; " "; c.CurrentTrack; " "; c.Timer.Enabled
c.Eject
PRINT c.Present
