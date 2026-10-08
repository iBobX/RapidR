' QCOMPORT's RapidR extras on the tests' scripted ports (never a real
' device): the ports listed with their USB IDs, DTR / RTS set and read back
' (looped to DSR / CTS), whole lines (LineEnd, ReadLine, HasLine), a break,
' and the scripted ESP32's boot log after a reset through DTR / RTS
$APPTYPE CONSOLE
ENVIRON "RAPIDR_TEST_COMPORT=COM2:echo;COM5:esp32"
DECLARE SUB ComError(s AS STRING)
DIM c AS QCOMPORT
DIM e AS QCOMPORT
DIM i AS INTEGER
SUB ComError(s AS STRING)
  PRINT "error ["; LEFT$(s, LEN(s) - 2); "]"
END SUB
c.OnComError = ComError
PRINT "ports "; c.ListPorts; " count "; c.PortCount
FOR i = 0 TO c.PortCount - 1
  PRINT i; " "; c.PortName(i); " ["; c.PortDescription(i); "] "; HEX$(c.PortVendorID(i)); ":"; HEX$(c.PortProductID(i)); " "; c.PortManufacturer(i); " "; c.PortSerialNumber(i)
NEXT
PRINT "past the end ["; c.PortName(5); "] "; c.PortVendorID(5)
PRINT "closed: dtr "; c.DTR; " rts "; c.RTS; " cts "; c.CTS; " dsr "; c.DSR
c.ReadLine(0)
c.Port = "COM2"
c.RTS = 0
c.Open
PRINT "open: dtr "; c.DTR; " rts "; c.RTS; " cts "; c.CTS; " dsr "; c.DSR; " cd "; c.CD; " ri "; c.RI
c.RTS = 1
c.DTR = 0
PRINT "set: dtr "; c.DTR; " rts "; c.RTS; " cts "; c.CTS; " dsr "; c.DSR
c.WriteString("one" + CHR$(13) + CHR$(10) + "two" + CHR$(10) + "thr", 0)
PRINT "hasline "; c.HasLine
PRINT "["; c.ReadLine(0); "] ["; c.ReadLine; "] ["; c.ReadLine(0); "] inque "; c.InQue; " hasline "; c.HasLine
c.LineEnd = ";"
c.WriteString("ee;four;", 0)
PRINT "["; c.ReadLine(0); "] ["; c.ReadLine(0); "]"
c.SendBreak(10)
c.Close
e.Port = "COM5"
e.BaudRate = 115200
e.Open
PRINT "esp32 inque "; e.InQue
e.DTR = 0
e.RTS = 1
PRINT "in reset inque "; e.InQue
e.RTS = 0
DO WHILE e.HasLine
  PRINT "| "; e.ReadLine(0)
LOOP
e.Close
