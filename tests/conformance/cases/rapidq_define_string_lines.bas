' A $DEFINE is replaced on lines that name a string function too
' (`STR$(TEN)`, RapidQ's zlib examples' `If gFile < Z_OK Then … str$(gFile)`):
' they were left as undeclared names, 0. The .expected is RC.EXE's output.
$APPTYPE CONSOLE
$DEFINE TEN 10
PRINT STR$(TEN) + "!"
PRINT LEFT$("abcdefghijklm", TEN); TEN
IF 5 < TEN THEN PRINT "lt "; CHR$(65)
