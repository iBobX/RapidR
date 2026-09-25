' DATA / READ / RESTORE (manual: unquoted items are text unless numeric,
' RESTORE label restarts at that label's DATA), $ESCAPECHARS ON escapes,
' `_ ' comment` continuations and dotted DECLARE names.
DECLARE SUB SLEEP.ms LIB "kernel32" ALIAS "Sleep" (dwMilliseconds AS DWORD)
DIM Name AS STRING, Age AS INTEGER, Score AS DOUBLE
FOR i = 1 TO 2
  READ Name, Age, Score
  PRINT Name; "|"; Age; "|"; Score
NEXT
RESTORE Second
READ Name
PRINT "again: "; Name
RESTORE
READ Name
PRINT "first: "; Name
SUB Show (a AS INTEGER, _   ' the first number
          b AS INTEGER)
  PRINT a + b
END SUB
Show 2, _
     3
DATA my dog ate my homework, 12, 34.5
Second:
DATA "oh, boy!", 7, -1.25   ' a comment
$ESCAPECHARS ON
PRINT "tab:\tA\x42\67 quote:\" backslash:\\"
