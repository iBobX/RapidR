' QVIDEO (docs/io-media-plan.md §6) with AVIs RapidR decodes (made by
' tools/make_avi_fixtures.py), as D. Glodt's QVideo.inc reads them from
' MCI — checked against MCI in the Windows VM: Lenght (frames),
' LenghtTime (MCI's milliseconds / 1000 stored in a LONG), ImgWidth /
' ImgHeight, a popup window's size (a 1-pixel border) and an overlapped
' one's (its inside at least 120 wide), its caption (the file's name),
' CurrentFrame kept within 0 … Lenght, AudioOff never stored, Open while
' open (MCI's alias in use: the open one stays open), Close's resets, a
' file that isn't an AVI, Error and Caption with MCI's NUL. The expected
' output is RC.EXE's running QVideo.inc in the Windows VM, but for the MJPEG
' line: Windows 11 has no MJPEG codec, RapidR plays it. A console program:
' nothing is shown, no Timer ticks.
$APPTYPE CONSOLE
$INCLUDE "QVideo.inc"
$RESOURCE CINEPAK AS "video_files/video_cinepak.avi"
$RESOURCE RLE8 AS "video_files/video_rle8.avi"
$RESOURCE MJPEG AS "video_files/video_mjpeg.avi"
$RESOURCE SOUND AS "video_files/video_audio.avi"
$RESOURCE NOTE AS "resource_files/hello.txt"
DIM v AS QVIDEO
EXTRACTRESOURCE CINEPAK, "video_c.tmp.avi"
EXTRACTRESOURCE RLE8, "video_r.tmp.avi"
EXTRACTRESOURCE MJPEG, "video_m.tmp.avi"
EXTRACTRESOURCE SOUND, "video_a.tmp.avi"
EXTRACTRESOURCE NOTE, "video_n.tmp.avi"
PRINT "closed "; v.State; " "; v.FileOpen; " "; v.Lenght; " "; v.Handle; " "; v.BorderStyle; " "; v.Timer.Interval
PRINT v.Open("video_c.tmp.avi"); " "; v.Lenght; " "; v.LenghtTime; " "; v.ImgWidth; "x"; v.ImgHeight; " "; v.Width; "x"; v.Height; " "; v.State; " "; v.FileOpen; " ["; v.Caption; "] "; v.Handle <> 0; " "; v.Left; " "; v.Top
v.CurrentFrame = 3
PRINT v.CurrentFrame;
v.CurrentFrame = 99
PRINT " "; v.CurrentFrame;
v.CurrentFrame = -5
PRINT " "; v.CurrentFrame
v.Left = 5
v.Width = 64
PRINT v.Left; " "; v.Width; " "; v.Height
v.AudioOff = 1
v.Volume = 40
PRINT v.AudioOff; " "; v.Volume
PRINT v.Open("video_c.tmp.avi"); " "; v.Error; " "; v.FileOpen; " "; v.State
v.Error = ""
v.BorderStyle = 1
v.Caption = "Clip"
PRINT v.Open("video_c.tmp.avi"); " "; v.Width; "x"; v.Height; " ["; v.Caption; "]"
v.Close
PRINT "closed "; v.State; " "; v.Lenght; " "; v.LenghtTime; " "; v.Width; " "; v.ImgWidth; " "; v.CurrentFrame; " "; v.BorderStyle; " ["; v.Caption; "]"
v.BorderStyle = 0
v.Caption = ""
PRINT "rle8 "; v.Open("video_r.tmp.avi"); " "; v.Lenght; " "; v.ImgWidth; "x"; v.ImgHeight; " "; v.Width; "x"; v.Height; " ["; v.Caption; "]"
v.Close
PRINT "mjpeg "; v.Open("video_m.tmp.avi"); " "; v.Lenght; " "; v.ImgWidth; "x"; v.ImgHeight
v.Close
PRINT "sound "; v.Open("video_a.tmp.avi"); " "; v.Lenght; " "; v.LenghtTime
v.Close
PRINT "text "; v.Open("video_n.tmp.avi"); " "; v.Error; " "; v.State
KILL "video_c.tmp.avi"
KILL "video_r.tmp.avi"
KILL "video_m.tmp.avi"
KILL "video_a.tmp.avi"
KILL "video_n.tmp.avi"
