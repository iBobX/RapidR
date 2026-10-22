' RINSTR with two arguments — the text and what to find — on variables,
' expressions and a FUNCTION's result, as programs use it (RapidR Database
' Studio's file names and paths): the last place, 0 when it isn't there.
' Natively too (both forms compile and agree). The .expected is RC.EXE's
' output.
FUNCTION PathOf (P AS STRING) AS STRING
  DIM i AS INTEGER
  i = RINSTR(P, "\")
  IF i = 0 THEN
    PathOf = ""
  ELSE
    PathOf = LEFT$(P, i)
  END IF
END FUNCTION
DIM Text AS STRING
DIM Find AS STRING
Text = "C:\data\shop\orders.csv"
Find = "\"
PRINT RINSTR(Text, Find); " "; RINSTR(Text, "."); " "; RINSTR(Text, "zip")
PRINT PathOf(Text); "|"; PathOf("orders.csv"); "|"
t$ = "a,b,,c"
f$ = ","
PRINT RINSTR(t$, f$); " "; RINSTR(t$ + ",", f$); " "; RINSTR(UCASE$(t$), "B")
PRINT MID$(Text, RINSTR(Text, Find) + 1)
PRINT RINSTR(3, Text, Find); " "; RINSTR(10, Text, Find)
