' RapidQ's own answers: RC.EXE running QCGI.INC 1.6 (docs/io-media-plan.md)
$APPTYPE CONSOLE
$INCLUDE "qcgi.inc"

ENVIRON "REQUEST_METHOD=GET"
ENVIRON "QUERY_STRING==x&=y&k=1"
DIM CGI AS QCGI
DIM v AS STRING
DIM n AS INTEGER
v = "unset"
n = CGI.Get("", v)
PRINT " "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("k", v)
PRINT "k "; n; " ["; v; "]"
