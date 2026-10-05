' RapidQ's own answers: RC.EXE running QCGI.INC 1.6 (docs/io-media-plan.md)
$APPTYPE CONSOLE
$INCLUDE "qcgi.inc"

ENVIRON "REQUEST_METHOD=GET"
ENVIRON "QUERY_STRING=a=+&b=++&c=+ x&d=x+&e=+++"
DIM CGI AS QCGI
DIM v AS STRING
DIM n AS INTEGER
v = "unset"
n = CGI.Get("a", v)
PRINT "a "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("b", v)
PRINT "b "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("c", v)
PRINT "c "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("d", v)
PRINT "d "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("e", v)
PRINT "e "; n; " ["; v; "]"
