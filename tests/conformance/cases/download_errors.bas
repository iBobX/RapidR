' QDOWNLOAD (Qdownload.inc v2): its defaults, the library's checks and
' messages (errors 1 to 4), a refused connection (5, on this machine:
' nothing listens on port 1), Percent and Speed — no network reached
$APPTYPE CONSOLE
$INCLUDE "qdownload.inc"
DIM d AS QDOWNLOAD
PRINT "port "; d.Port; " out "; d.OutDevice; " state dev "; d.StateDevice; " state "; d.State; " size "; d.Size; " err "; d.LastError
PRINT "gauge "; d.StateGauge.Width; "x"; d.StateGauge.Height; " check "; d.Check
PRINT d.LeechFile; " "; d.LastError; " "; d.LastStringError
d.Server = "127.0.0.1"
d.Port = 0
PRINT d.LeechFile; " "; d.LastError; " "; d.LastStringError
d.Port = 1
PRINT d.LeechFile; " "; d.LastError; " "; d.LastStringError
d.File = "nothing.txt"
d.OutDevice = 2
PRINT d.LeechFile; " "; d.LastError; " "; d.LastStringError
d.OutDevice = 1
IF d.LeechFile THEN PRINT "fetched" ELSE PRINT "no: "; d.LastError; " "; d.LastStringError
PRINT d.Percent(1, 3); " "; d.Percent(50, 200); " "; d.Speed("10:00:10", "10:00:00", 2500)
