' QBITMAP's PixelFormat as RapidQ's compiler (RC.EXE) reads it: pfDevice
' (0) new or sized, the file's after a BMP is loaded (8-bit 3, 24-bit 6),
' what the program sets. RapidQ's direct3d/RQ_3DTerrain.bas and
' Lights_terrain.bas check it before reading a height map. (The expected
' output is RC.EXE's with the two files beside the program; its
' EXTRACTRESOURCE of them stops with EReadError, so RC.EXE can't run this
' copy, which carries them for the test runners.)
$APPTYPE CONSOLE
$RESOURCE P8 AS "picture_files/rr_8bit.bmp"
$RESOURCE P24 AS "picture_files/rr_24bit.bmp"
EXTRACTRESOURCE P8, "rr_pf_8bit.bmp"
EXTRACTRESOURCE P24, "rr_pf_24bit.bmp"
DIM B AS QBITMAP
PRINT "new "; B.PixelFormat
B.Width = 10: B.Height = 10
PRINT "sized "; B.PixelFormat
B.LoadFromFile "rr_pf_8bit.bmp"
PRINT "8-bit "; B.PixelFormat; " "; B.Width
B.LoadFromFile "rr_pf_24bit.bmp"
PRINT "24-bit "; B.PixelFormat
B.PixelFormat = 3
PRINT "set "; B.PixelFormat
DIM C AS QBITMAP
C.BMP = "rr_pf_8bit.bmp"
PRINT "bmp "; C.PixelFormat
KILL "rr_pf_8bit.bmp"
KILL "rr_pf_24bit.bmp"
