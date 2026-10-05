' QCOMPORT on the tests' scripted ports (RAPIDR_TEST_COMPORT: COM2 echoes, COM3 answers OK, COM4 is in use) — never a real device
$APPTYPE CONSOLE
ENVIRON "RAPIDR_TEST_COMPORT=COM2:echo;COM3:reply:OK\r\n;COM4:busy"
DECLARE SUB ComError(s AS STRING)
DECLARE SUB Opened
DECLARE SUB Closed
DECLARE SUB Written
DECLARE SUB Got
DIM c AS QCOMPORT
DIM m AS QCOMPORT
DIM b AS QCOMPORT
DIM s AS QMEMORYSTREAM
SUB ComError(s AS STRING)
  PRINT "error ["; LEFT$(s, LEN(s) - 2); "]"
END SUB
SUB Opened
  PRINT "opened"
END SUB
SUB Closed
  PRINT "closed"
END SUB
SUB Written
  PRINT "written"
END SUB
SUB Got
  PRINT "read"
END SUB
c.OnComError = ComError
c.OnOpen = Opened
c.OnClose = Closed
c.OnWriteString = Written
c.OnReadString = Got
c.Port = 2
PRINT "port "; c.Port
c.Open
PRINT "connected "; c.Connected; " handle>0 "; (c.Handle > 0)
c.WriteString("hello", 0)
PRINT "notread "; c.BytesNotRead; " inque "; c.InQue
PRINT "["; c.ReadString(3, 0); "] notread "; c.BytesNotRead
c.PurgeIn
PRINT "purged inque "; c.InQue; " notread "; c.BytesNotRead
s.WriteStr("ABCDEF")
c.Write(s, 4, 0)
c.Read(s, 10, 0)
s.Position = 0
PRINT "stream ["; s.ReadStr(s.Size); "]"
c.Open
c.Close
PRINT "connected "; c.Connected
m.Port = "com3"
m.BaudRate = 6
m.Open
m.WriteString("ATZ" + CHR$(13) + CHR$(10), 0)
PRINT "modem ["; LEFT$(m.ReadString(100, 0), 2); "]"
b.OnComError = ComError
b.Port = "COM4"
b.Open
PRINT "busy connected "; b.Connected; " handle "; b.Handle
b.Port = "COM3"
b.Parity = 3
b.Open
PRINT "mark connected "; b.Connected
