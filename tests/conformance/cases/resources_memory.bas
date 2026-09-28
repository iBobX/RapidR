' $RESOURCE without touching the disk: RESOURCE(n), RESOURCECOUNT and
' QMEMORYSTREAM.ExtractRes (the web IDE takes the files from its assets).
$RESOURCE HELLO_TXT AS "resource_files/hello.txt"
$RESOURCE TWO AS "resource_files\two.bin"
PRINT RESOURCECOUNT; " "; HELLO_TXT = RESOURCE(0); " "; TWO = RESOURCE(1); " "; RESOURCE(2); " "; RESOURCE(-1)
DIM M AS QMEMORYSTREAM
M.ExtractRes(Resource(0))
M.ExtractRes(TWO)
M.Position = 0
PRINT M.Size; " "; M.ReadStr(M.Size)
