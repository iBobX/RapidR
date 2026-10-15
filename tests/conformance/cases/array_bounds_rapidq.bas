' RapidQ doesn't check array limits ("There is no checking for limits on
' arrays", manual: DIM): an index past one dimension is the next row's
' element (arrays are stored by the last subscript); past the whole array a
' write reads back (RC.EXE's output, this program's .expected; RapidR keeps
' it beside the array, memory-safely — under RapidQ it overwrote whatever
' followed) and a place never written reads the type's zero.
DIM G(1 TO 2, 1 TO 3) AS INTEGER
G(1, 4) = 7
PRINT G(2, 1)
DIM A(3) AS INTEGER
A(5) = 9
PRINT A(5); A(3)
DIM S(2) AS STRING
PRINT "["; S(9); "]"
