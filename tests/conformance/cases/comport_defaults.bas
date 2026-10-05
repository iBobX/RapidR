' RapidQ's own answers: RC.EXE running RAPIDQ2.INC's COMPORT (its QCOMPORT), no port opened (docs/io-media-plan.md)
$APPTYPE CONSOLE
DECLARE SUB ComError(s AS STRING)
DECLARE SUB Opened
DECLARE SUB Closed
DIM c AS QCOMPORT
SUB ComError(s AS STRING)
  PRINT "error ["; s; "] len "; LEN(s)
END SUB
SUB Opened
  PRINT "opened"
END SUB
SUB Closed
  PRINT "closed"
END SUB
c.OnComError = ComError
c.OnOpen = Opened
c.OnClose = Closed
PRINT "port ["; c.Port; "] baud "; c.BaudRate; " bits "; c.DataBits; " parity "; c.Parity; " stop "; c.StopBits
PRINT "rbuf "; c.ReadBufSize; " wbuf "; c.WriteBufSize; " flags "; c.DCBflags
PRINT "connected "; c.Connected; " handle "; c.Handle; " notread "; c.BytesNotRead; " notwritten "; c.BytesNotWritten; " inque "; c.InQue
c.Close
PRINT "after close connected "; c.Connected
c.Port = "NOSUCHPORT"
c.Open
PRINT "after open connected "; c.Connected; " handle "; c.Handle
c.WriteString("x", 0)
PRINT "end"
