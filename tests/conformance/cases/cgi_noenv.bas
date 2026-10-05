' RapidQ's own answers: RC.EXE running QCGI.INC 1.6 (docs/io-media-plan.md)
$APPTYPE CONSOLE
$INCLUDE "qcgi.inc"

DIM CGI AS QCGI
DIM v AS STRING
DIM n AS INTEGER
PRINT "port "; CGI.ServerPort; " len "; CGI.ContentLength; " m ["; CGI.RequestMethod; "]"
v = "unset"
n = CGI.Get("x", v)
PRINT "x "; n; " ["; v; "]"
