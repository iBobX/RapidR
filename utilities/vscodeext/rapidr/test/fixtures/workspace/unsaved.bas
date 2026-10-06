' unsaved.bas: calls a FUNCTION that unsaved.inc only has once it's edited
' (the test edits it in the editor and never saves it).
$INCLUDE "unsaved.inc"
PRINT Triple(2)
