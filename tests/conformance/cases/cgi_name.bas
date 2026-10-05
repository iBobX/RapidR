' RapidQ's own answers: RC.EXE running QCGI.INC 1.6 (docs/io-media-plan.md)
$APPTYPE CONSOLE
$INCLUDE "qcgi.inc"

ENVIRON "REQUEST_METHOD=GET"
ENVIRON "QUERY_STRING=my+name=1&f%41o=2&x%3Dy=3"
DIM CGI AS QCGI
DIM v AS STRING
DIM n AS INTEGER
v = "unset"
n = CGI.Get("my name", v)
PRINT "my name "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("MY NAME", v)
PRINT "MY NAME "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("fAo", v)
PRINT "fAo "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("x=y", v)
PRINT "x=y "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("x", v)
PRINT "x "; n; " ["; v; "]"
