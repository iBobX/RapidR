' xfail: web — a page has no folders to list (DIR$)
' DIR$ as RapidQ's (checked against RC.EXE): faDirectory (&H10) lists the
' folders too, "." and ".." first, then everything in name order (any
' case); attribute 0 lists files only; `?` and `*` match any case. FileRec
' tells about the file found last: its name, size, date (month-day-year)
' and time (h:mm), FileTime.
$APPTYPE CONSOLE
MKDIR "dirprobe"
MKDIR "dirprobe/Sub1"
DIM S AS QFILESTREAM
S.Open("dirprobe/b.txt", 65535)
S.WriteLine(STRING$(3000, "x"))
S.Close
S.Open("dirprobe/A.bas", 65535)
S.WriteLine("x")
S.Close
F$ = DIR$("dirprobe/*.*", &H10)
WHILE F$ <> ""
  PRINT F$; " ";
  F$ = DIR$
WEND
PRINT
F$ = DIR$("dirprobe/*.*", 0)
WHILE F$ <> ""
  PRINT F$; " ";
  F$ = DIR$
WEND
PRINT
F$ = DIR$("dirprobe/?.TXT", 0)
PRINT F$; " "; FileRec.FileName; " "; FileRec.Size > 3000; " "; TALLY(FileRec.Date, "-"); " "; INSTR(FileRec.Time, ":") > 0; " "; FileRec.FileTime > 0
PRINT "["; DIR$("dirprobe/*.none", 0); "]"
KILL "dirprobe/b.txt"
KILL "dirprobe/A.bas"
RMDIR "dirprobe/Sub1"
RMDIR "dirprobe"
