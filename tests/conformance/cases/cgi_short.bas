' RapidQ's own answers: RC.EXE running QCGI.INC 1.6 (docs/io-media-plan.md)
$APPTYPE CONSOLE
$INCLUDE "qcgi.inc"

ENVIRON "REQUEST_METHOD=GET"
ENVIRON "QUERY_STRING=x=%41+%42&y=12345678901234"
DIM CGI AS QCGI
DIM v AS STRING
DIM n AS INTEGER
CGI.MaxInput = 10
CGI.AutoConvert = 7
PRINT "max "; CGI.MaxInput; " auto "; CGI.AutoConvert
CGI.MaxInput = 0
PRINT "max0 "; CGI.MaxInput
CGI.MaxInput = -5
PRINT "maxneg "; CGI.MaxInput
v = "unset"
n = CGI.Get("x", v)
PRINT "x "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("y", v)
PRINT "y "; n; " ["; v; "]"
