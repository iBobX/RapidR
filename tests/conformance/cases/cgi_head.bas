' RapidQ's own answers: RC.EXE running QCGI.INC 1.6 (docs/io-media-plan.md)
$APPTYPE CONSOLE
$INCLUDE "qcgi.inc"

ENVIRON "REQUEST_METHOD=HEAD"
ENVIRON "QUERY_STRING=x=1"
DIM CGI AS QCGI
DIM v AS STRING
DIM n AS INTEGER
v = "unset"
n = CGI.Get("x", v)
PRINT "x "; n; " ["; v; "]"
