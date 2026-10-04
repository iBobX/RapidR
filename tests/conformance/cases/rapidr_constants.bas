' RapidR's own constants (Anchors' akLeft … akBottom) are there without an
' include; a program's own constant or variable of the same name wins (an
' assigned name is a variable everywhere, so akTop reads 0 before it's set).
CONST akBottom = 100
PRINT akLeft; " "; akTop; " "; akRight; " "; akBottom
PRINT akLeft OR akTop OR akRight
akTop = 7
PRINT akTop
SUB Show
  PRINT akRight + akTop
END SUB
Show
