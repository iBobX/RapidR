' A binary operator with nothing after it: RC.EXE drops it, the value is what came before (checked against RC.EXE; RapidQ's Qcdaudio.inc relies on it)
DECLARE FUNCTION F(a AS STRING, b AS INTEGER) AS STRING
FUNCTION F(a AS STRING, b AS INTEGER) AS STRING
  Result = "[" + a + "]" + STR$(b)
END FUNCTION
PRINT F("ab"+, 5)
PRINT F("ab"+"c"+, 6)
PRINT LEN("abc"+)
x = 2 +
PRINT x
