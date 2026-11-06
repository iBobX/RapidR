' xfail: web — a browser can't listen on a port (no RServerSocket there)
' A server and a client on this computer only (127.0.0.1, a free port):
' RServerSocket's Stop then Start on the same port works at once (the port
' free when Stop returns — RAPIDR-ISSUES #8); RSocket's Connect(Host, Port)
' makes Connected 1 (#13), IsServerReady answers at once (nothing sent: 0);
' Disconnect really closes the connection — the server sees it go (#14) —
' and Connected is 0 again; a Read with the program's Timeout gives up
' after it ("" when nothing came).
$APPTYPE CONSOLE
CREATE S AS RServerSocket
  Host = "127.0.0.1"
  Port = 0
END CREATE
DIM c AS RSocket
DIM rv AS VARIANT
DIM t AS DOUBLE
DIM i AS INTEGER
DIM p AS INTEGER
PRINT "start"; S.Start
p = S.Port
S.Stop
PRINT "start again"; S.Start; S.Port = p
c.Timeout = 1000
rv = c.Connect("127.0.0.1", S.Port)
PRINT "connect"; rv > 0; c.Connected
t = TIMER
PRINT "ready"; c.IsServerReady(c.MySocket); TIMER - t < 0.5
FOR i = 1 TO 200
  S.Poll
  IF S.ClientCount = 1 THEN EXIT FOR
  SLEEP 0.01
NEXT
PRINT "server's clients"; S.ClientCount
c.Disconnect
PRINT "disconnected"; c.Connected
FOR i = 1 TO 200
  S.Poll
  IF S.ClientCount = 0 THEN EXIT FOR
  SLEEP 0.01
NEXT
PRINT "server's clients"; S.ClientCount
rv = c.Connect("127.0.0.1", S.Port)
t = TIMER
PRINT "read"; LEN(c.Read(c.MySocket, 10)); TIMER - t > 0.9; TIMER - t < 3
' the server ends the connection: Connected follows
FOR i = 1 TO 200
  S.Poll
  IF S.ClientCount = 1 THEN EXIT FOR
  SLEEP 0.01
NEXT
S.Stop
FOR i = 1 TO 200
  IF c.Connected = 0 THEN EXIT FOR
  SLEEP 0.01
NEXT
PRINT "server stopped"; c.Connected
PRINT "done"
