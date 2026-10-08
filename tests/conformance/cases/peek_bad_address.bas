' An address that is neither a console page's (0 to 3999) nor inside the
' program's memory (VARPTR) stops the program with a clear error, never
' a crash (RapidQ reads whatever lies there)
PRINT "start"
PRINT PEEK(5000)
PRINT "not reached"
