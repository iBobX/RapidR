' RC.EXE (the Windows VM, 2026-11-07): ERROR: Option WEAKTYPE expected ON|OFF but got MAYBE; a word
' after ON: Expected end-of-line but got JUNK; nothing after it.
$OPTION WEAKTYPE maybe
$OPTION WEAKTYPE ON junk
$OPTION WEAKTYPE
PRINT 1
