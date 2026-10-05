' RapidQ's own answers: RC.EXE running QCGI.INC 1.6 (docs/io-media-plan.md)
$APPTYPE CONSOLE
$INCLUDE "qcgi.inc"

ENVIRON "REQUEST_METHOD=POST"
ENVIRON "CONTENT_LENGTH=11"
ENVIRON "QUERY_STRING=q=fromquery"
DIM CGI AS QCGI
DIM v AS STRING
DIM n AS INTEGER
PRINT "len "; CGI.ContentLength; " qs ["; CGI.QueryString; "]"
v = "unset"
n = CGI.Get("a", v)
PRINT "a "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("b", v)
PRINT "b "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("q", v)
PRINT "q "; n; " ["; v; "]"
