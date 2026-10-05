' RapidQ's own answers: RC.EXE running QCGI.INC 1.6 (docs/io-media-plan.md)
$APPTYPE CONSOLE
$INCLUDE "qcgi.inc"

ENVIRON "REQUEST_METHOD=GET"
ENVIRON "QUERY_STRING=name=John+Smith&Age=42&x=%41%42%7e&y=a++b&z=%0Aq&w=%4&e=&noeq&name=Jane&p=%2B%20%zz&t=a=b=c&=lone&&last=%41"
ENVIRON "SERVER_PORT=8080"
ENVIRON "CONTENT_LENGTH=12abc"
ENVIRON "HTTP_USER_AGENT=probe/1.0"
ENVIRON "HTTP_ACCEPT=text/html"
ENVIRON "SCRIPT_NAME=/cgi-bin/t.exe"
ENVIRON "SERVER_PROTOCOL=HTTP/1.1"
ENVIRON "REMOTE_ADDR=10.0.0.1"
DIM CGI AS QCGI
DIM v AS STRING
DIM n AS INTEGER
PRINT "port "; CGI.ServerPort; " len "; CGI.ContentLength; " ua ["; CGI.UserAgent; "] m ["; CGI.RequestMethod; "]"
PRINT "max "; CGI.MaxInput; " auto "; CGI.AutoConvert
PRINT "qs ["; CGI.QueryString; "] accept ["; CGI.Accept; "] cookie ["; CGI.Cookie; "]"
PRINT "script ["; CGI.ScriptName; "] proto ["; CGI.ServerProtocol; "] remote ["; CGI.RemoteAddr; "]"
v = "unset"
n = CGI.Get("name", v)
PRINT "name "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("NAME", v)
PRINT "NAME "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("Age", v)
PRINT "Age "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("x", v)
PRINT "x "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("y", v)
PRINT "y "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("z", v)
PRINT "z "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("w", v)
PRINT "w "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("e", v)
PRINT "e "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("noeq", v)
PRINT "noeq "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("p", v)
PRINT "p "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("t", v)
PRINT "t "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("", v)
PRINT " "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("last", v)
PRINT "last "; n; " ["; v; "]"
v = "unset"
n = CGI.Get("missing", v)
PRINT "missing "; n; " ["; v; "]"
