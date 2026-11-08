' weaktype.bas: a CONST written again, as older programs do.
'
' A VB constants file ported to a classic program starts with
' $OPTION WEAKTYPE ON, ends with $OPTION WEAKTYPE OFF, and writes some of
' the names RAPIDQ.INC already has (SND_SYNC, ANSI_CHARSET …) again. With
' the option ON a CONST can be written again: no error, the last value
' wins, a line between two CONSTs reads the value of the one before it,
' and the new value may be another number or text. With it OFF (the
' default) a second CONST of a name is an error: "CONST variable
' ANSI_CHARSET already defined" — in the same file or another, and `Public
' Const` too. The option is for CONSTs only: a name DIMmed twice is still an
' error. The include files must come in this order: the port after
' RAPIDQ.INC; the other way round stops at RAPIDQ.INC's first repeated name.
' Written for RapidR in RapidQ's dialect, with no RapidR extension.
'
'   rapidr run rapidq/weaktype/weaktype.bas
'
' Files: weaktype.bas, lib/vbport.inc (the port).
$APPTYPE CONSOLE
$INCLUDE "RAPIDQ.INC"
$INCLUDE "Lib\VBPort.inc"

PRINT "RAPIDQ.INC and the port write these names; each reads the last value:"
PRINT "SND_SYNC = " + LTRIM$(STR$(SND_SYNC))
PRINT "SND_NODEFAULT = " + LTRIM$(STR$(SND_NODEFAULT))
PRINT "The port's own name:"
PRINT "WM_USER = " + LTRIM$(STR$(WM_USER))

' The option on, in the program itself: a CONST written again
$OPTION WEAKTYPE ON
CONST Version = 1
PRINT "Version = " + LTRIM$(STR$(Version))
CONST Version = 2
PRINT "Version = " + LTRIM$(STR$(Version))
CONST Version = "two point five"
PRINT "Version = " + Version
$OPTION WEAKTYPE OFF
' (from here a second CONST of a name is an error again)
'
' Try it: delete the $OPTION WEAKTYPE ON line above the first CONST Version
' and run again: the second one is the error "CONST variable Version already
' defined".
