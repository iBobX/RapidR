' QSOCKET's GetHostIP / GetHostName read without parentheses call the
' method, as in RapidQ (its sockets/getMyIp.bas: `Showmessage Sock.GetHostIP`)
$APPTYPE CONSOLE
DIM Sock AS QSOCKET
DIM ip AS STRING, host AS STRING
ip = Sock.GetHostIP
host = Sock.GetHostName
IF LEN(ip) >= 7 THEN PRINT "ip ok"
IF LEN(host) > 0 THEN PRINT "host ok"
PRINT TALLY(ip, ".")
IF ip = Sock.GetHostIP() THEN PRINT "same"
