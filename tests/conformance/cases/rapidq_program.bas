' A RapidQ-style console program: RAPIDQ.INC constants, $TYPECHECK, `?`, Q-prefixed types.
$APPTYPE CONSOLE
$TYPECHECK ON
$INCLUDE "RAPIDQ.INC"

DECLARE SUB Report (Label AS STRING, Value AS INTEGER)

DIM Names AS QSTRINGLIST
Names.Add("Ada")
Names.Add("Grace")

Report "blue", clBlue
Report "mrOk", mrOk
Report "MB_YESNO", MB_YESNO
Report "VK_RETURN", VK_RETURN
? "second name: " + Names.Item(1)

SUB Report (Label AS STRING, Value AS INTEGER)
  ? Label + "=" + STR$(Value)
END SUB
