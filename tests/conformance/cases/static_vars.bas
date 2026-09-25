' STATIC: one variable shared by every call and recursion (a DIM is each
' call's own).
SUB Depth (N AS INTEGER)
   DIM Mark AS INTEGER
   Mark = N * N
   IF N < 6 THEN Depth(N + 1)
   PRINT Mark; " ";
END SUB
Depth(2)
PRINT
SUB SharedDepth (N AS INTEGER)
   STATIC Mark AS INTEGER
   Mark = N * N
   IF N < 6 THEN SharedDepth(N + 1)
   PRINT Mark; " ";
END SUB
SharedDepth(2)
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
