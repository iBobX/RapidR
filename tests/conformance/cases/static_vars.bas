' STATIC (RapidQ manual example): one variable shared by every call and recursion.
SUB Count (N AS INTEGER)
   DIM B AS LONG
   B = N
   IF N = 10 THEN EXIT SUB
   Count(N+1)
   PRINT B;
END SUB
Count(1)
PRINT
SUB StaticCount (N AS INTEGER)
   STATIC B AS LONG
   B = N
   IF N = 10 THEN EXIT SUB
   StaticCount(N+1)
   PRINT B;
END SUB
StaticCount(1)
PRINT
FUNCTION NextId AS INTEGER
   STATIC Counter AS INTEGER, Hist(3) AS STRING
   Counter++
   Hist(Counter MOD 4) = "#" + STR$(Counter)
   NextId = Counter
END FUNCTION
FUNCTION Other AS INTEGER
   STATIC Counter AS INTEGER
   Counter += 10
   Other = Counter
END FUNCTION
PRINT NextId; NextId; Other; NextId; Other
DIM s AS STRING
