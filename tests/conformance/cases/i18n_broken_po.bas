' A broken catalog (lang/xx.po: garbage lines, unterminated strings, an
' entry without a translation) still gives its good entries; nothing fails.
' (A UTF-8 file with a non-ASCII character, “like this”, prints Unicode;
' a .bas file of plain ASCII prints in RapidQ's console code page, CP437.)
Application.Language = "xx"
PRINT TR$("Good")
PRINT TR$("Fine")
PRINT TR$("Also")
PRINT TR$("Bad")
PRINT TR$("No translation")
PRINT TR$("Missing")
PRINT "done"
