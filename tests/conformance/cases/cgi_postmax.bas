' RapidQ's own answers: RC.EXE running QCGI.INC 1.6 (docs/io-media-plan.md)
$APPTYPE CONSOLE
$INCLUDE "qcgi.inc"

ENVIRON "REQUEST_METHOD=post"
ENVIRON "CONTENT_LENGTH=20"
DIM CGI AS QCGI
DIM v AS STRING
DIM n AS INTEGER
CGI.MaxInput = 4
v = "unset"
n = CGI.Get("c", v)
PRINT "c "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("d", v)
PRINT "d "; n; " ["; v; "]"
