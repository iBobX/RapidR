' A format RapidQ's Format refuses stops the program with its own message —
' `%+d` (no flags: a + or a space in a specifier): EConvertError "Format '%+'
' invalid or incompatible with argument." The .expected is RC.EXE's output
' (docs/rapidq-ground-truth.md, "FORMAT$"); the same for `% d` (a lone % in
' "50% off" is `% o`), `%#x`, `%ld` and `%.*f` — rapidq_format_ld,
' rapidq_format_starprec.
$APPTYPE CONSOLE
PRINT "before"
PRINT FORMAT$("%d|%+d", 1, 2)
PRINT "after"
