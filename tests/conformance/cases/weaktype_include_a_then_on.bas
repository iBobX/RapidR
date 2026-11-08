' skip-on: web — it reads include files, which the web runner is not handed
' RC.EXE (the Windows VM, 2026-11-07): an include file that turns the option on and writes the
' name again lets it through.
$INCLUDE "include_files/weaktype_a.inc"
$INCLUDE "include_files/weaktype_c.inc"
PRINT A
