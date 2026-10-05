' RapidQ's own answers: RC.EXE running QCGI.INC 1.6 (docs/io-media-plan.md)
$APPTYPE CONSOLE
$INCLUDE "qcgi.inc"

ENVIRON "REQUEST_METHOD=GET"
ENVIRON "QUERY_STRING=b=2&a=1&c=3&B=4&aa=5&A=6"
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
n = CGI.Get("aa", v)
PRINT "aa "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("A", v)
PRINT "A "; n; " ["; v; "]"
