' QSTRINGLIST's LoadFromStream (the stream's text from its position, lines
' ending CR LF, LF or CR) and SaveToStream, which RC.EXE makes read the
' list from the stream too (writing nothing); AddList. Expected output:
' RC.EXE's.
$APPTYPE CONSOLE
$INCLUDE "RAPIDQ.INC"
DIM L AS QSTRINGLIST
DIM K AS QSTRINGLIST
DIM M AS QMEMORYSTREAM
DIM E AS QMEMORYSTREAM
L.AddItems "one", "two", "three"
K.AddItems "four", ""
L.AddList(K)
PRINT "addlist "; L.ItemCount
FOR I = 0 TO L.ItemCount - 1: PRINT "["; L.Item(I); "]";: NEXT: PRINT
M.WriteStr("pre" + CHR$(13) + CHR$(10) + "post", 9)
M.Position = 0
L.SaveToStream(M)
PRINT "save0 size "; M.Size; " pos "; M.Position; " count "; L.ItemCount
FOR I = 0 TO L.ItemCount - 1: PRINT "["; L.Item(I); "]";: NEXT: PRINT
L.Clear
L.AddItems "one", "two", "three"
M.Position = 5
L.SaveToStream(M)
PRINT "save5 size "; M.Size; " pos "; M.Position; " count "; L.ItemCount
FOR I = 0 TO L.ItemCount - 1: PRINT "["; L.Item(I); "]";: NEXT: PRINT
E.WriteStr("a" + CHR$(13) + CHR$(10) + "b" + CHR$(10) + "c" + CHR$(13) + "d", 8)
E.Position = 2
K.AddItems "x"
K.LoadFromStream(E)
PRINT "e loaded "; K.ItemCount; " pos "; E.Position
FOR I = 0 TO K.ItemCount - 1: PRINT "["; K.Item(I); "]";: NEXT: PRINT
E.Position = 0
K.LoadFromStream(E)
PRINT "e0 loaded "; K.ItemCount; " pos "; E.Position
FOR I = 0 TO K.ItemCount - 1: PRINT "["; K.Item(I); "]";: NEXT: PRINT
E.Position = 8
K.LoadFromStream(E)
PRINT "e8 loaded "; K.ItemCount; " pos "; E.Position
P$ = "tests/conformance/.work/stringlist_streams.txt"
DIM F AS QFILESTREAM
F.Open(P$, fmCreate)
F.WriteStr("f1" + CHR$(13) + CHR$(10) + "f2" + CHR$(13) + CHR$(10), 8)
F.Position = 0
K.LoadFromStream(F)
PRINT "f loaded "; K.ItemCount; " pos "; F.Position
FOR I = 0 TO K.ItemCount - 1: PRINT "["; K.Item(I); "]";: NEXT: PRINT
F.Close
KILL P$
