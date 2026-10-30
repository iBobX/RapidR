' C's length letters aren't RapidQ's: `%ld` stops the program with EConvertError
' "Format '%l' invalid or incompatible with argument." (the specifier up to
' the letter it can't read). The .expected is RC.EXE's output
' (docs/rapidq-ground-truth.md, "FORMAT$").
$APPTYPE CONSOLE
PRINT "before"
PRINT FORMAT$("%d items, %ld bytes", 3, 4096)
PRINT "after"
