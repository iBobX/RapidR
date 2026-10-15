' shapes.bas: a RapidQ program in several files. The main program brings
' in its library and its window from subfolders with $INCLUDE, written as
' on Windows (backslashes, any case): RapidR finds them on every system,
' where RapidQ did (the program's folder first, then the include folders).
' Written for RapidR in RapidQ's dialect, with no RapidR extension.
'
'   rapidr run rapidq/shapes/shapes.bas
'   rapidr import-rapidq rapidq/shapes/shapes.bas
'       (a copy with RapidR's names: its include files are copied with it,
'       and the report lists each one and where it was found)
'
' Files: shapes.bas, lib/geometry.inc (areas; it includes lib/units.inc),
' forms/shapesform.inc (the window).
$TYPECHECK ON
$APPTYPE GUI
$INCLUDE "RAPIDQ.INC"
$INCLUDE "Lib\Geometry.INC"
$INCLUDE "Forms\ShapesForm.inc"

ShapesForm.Center
ShapesForm.ShowModal
