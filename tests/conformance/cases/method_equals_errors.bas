' `Obj.Member = a, b` of a property, and `Obj.Method = a` of a method that
' takes nothing, are errors (RC.EXE's words).
DIM L AS QSTRINGLIST
L.Sorted = 1, 2
L.Clear = 1
