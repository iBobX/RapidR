' RapidQ's console shows characters 128 to 255 in the DOS code page 437:
' CHR$(201) is a corner of a double box, the byte 218 a single one. This
' file is Windows-1252, not UTF-8 (as RapidQ's editors saved programs), so
' RapidR shows it so too; a UTF-8 file (RapidR's own) shows Unicode as it is.
' The .expected is RC.EXE's output (its bytes) read in code page 437.
$APPTYPE CONSOLE
PRINT CHR$(201); STRING$(3, 205); CHR$(187)
PRINT CHR$(186); "   "; CHR$(186)
PRINT CHR$(200); STRING$(3, 205); CHR$(188)
PRINT "зд© ╟╠╡ ш"
PRINT LEN("зд©"); " "; ASC("з"); " "; ASC(CHR$(201))
