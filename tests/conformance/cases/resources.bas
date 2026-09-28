' $RESOURCE (RapidQ manual, chapter 8): files built into the program. The
' name is the resource's handle; RESOURCE(n), RESOURCECOUNT,
' EXTRACTRESOURCE and QMEMORYSTREAM.ExtractRes read them.
$RESOURCE HELLO_TXT AS "resource_files/hello.txt"
$RESOURCE TWO AS "resource_files\two.bin"
PRINT RESOURCECOUNT; " "; HELLO_TXT = RESOURCE(0); " "; TWO = RESOURCE(1); " "; RESOURCE(2); " "; RESOURCE(-1)
DIM M AS QMEMORYSTREAM
M.ExtractRes(Resource(0))
M.ExtractRes(TWO)
M.Position = 0
PRINT M.Size; " "; M.ReadStr(M.Size)
EXTRACTRESOURCE Resource(1), "rapidr_resource_test.bin"
DIM F AS QFILESTREAM
F.Open("rapidr_resource_test.bin", 0)
PRINT F.ReadStr(F.Size)
F.Close
KILL "rapidr_resource_test.bin"
