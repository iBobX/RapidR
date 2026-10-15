' A RapidQ program's file names as Windows reads them, on every system:
' `\` between folders and a name in any case find the file (RapidQ's
' programs were written on Windows: `OPEN "data\Notes.txt"`, a picture
' named OPEN.BMP beside open.bmp). QFILESTREAM, QSTRINGLIST, FILEEXISTS,
' FILELEN and KILL alike (and RapidR's OPEN); on Windows the names are the
' system's own.
$INCLUDE "RAPIDQ.INC"
F$ = "tests\conformance\.work\Paths_Case.TXT"
DIM File AS QFILESTREAM
File.Open(F$, fmCreate)
File.WriteLine("first")
File.WriteLine("second")
File.Close
PRINT "exists "; FILEEXISTS("TESTS\Conformance\.WORK\paths_case.txt")
PRINT "len "; FILELEN("tests/conformance/.work/PATHS_CASE.txt")
File.Open("Tests\Conformance\.work\PATHS_case.TXT", fmOpenRead)
PRINT File.ReadLine
File.Close
DIM L AS QSTRINGLIST
L.LoadFromFile("tests\CONFORMANCE\.work\paths_CASE.txt")
PRINT L.ItemCount; " "; L.Item(1)
KILL "TESTS\CONFORMANCE\.WORK\PATHS_CASE.TXT"
PRINT "after kill "; FILEEXISTS(F$)
