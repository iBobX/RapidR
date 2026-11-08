' RC.EXE (the Windows VM, 2026-11-07): ERROR: CONST variable SND_SYNC already defined — the other
' order: the RAPIDQ.INC-like file comes after the port's option is OFF again, so its first
' repeated name is the error.
$INCLUDE "include_files/weaktype_win.inc"
$INCLUDE "include_files/weaktype_rq.inc"
PRINT SND_SYNC
