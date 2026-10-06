' SaveUDTArray / LoadUDTArray: RC.EXE takes one argument, a TYPE field,
' and the stream is left as it was (nothing written, nothing read).
' Expected output: RC.EXE's.
TYPE TP
  X AS INTEGER
  Y AS SHORT
END TYPE
DIM M AS QMEMORYSTREAM
DIM U(1 TO 3) AS TP
DIM V(1 TO 3) AS TP
U(1).X = 1: U(1).Y = 2: U(2).X = 3: U(2).Y = 4
M.SaveUDTArray(U(1).X)
PRINT "size "; M.Size
M.WriteNum(7, 4)
M.Position = 0
M.LoadUDTArray(V(1).X)
PRINT V(1).X; V(1).Y; V(2).X; V(2).Y; " pos "; M.Position
M.SaveUDTArray(U(2).Y)
M.LoadUDTArray(V(2).Y)
PRINT "again "; M.Size; M.Position; V(2).Y
