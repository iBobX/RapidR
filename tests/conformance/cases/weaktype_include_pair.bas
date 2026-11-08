' skip-on: web — it reads include files, which the web runner is not handed
' RC.EXE (the Windows VM, 2026-11-07): a RAPIDQ.INC-like file, then a VB port's that
' turns the option on around the constants the first one has: it compiles, the later value wins
' (&H0 and 0 are one value), and the port's own names are there.
$INCLUDE "include_files/weaktype_rq.inc"
$INCLUDE "include_files/weaktype_win.inc"
PRINT SND_SYNC; " "; SND_ASYNC; " "; SND_NODEFAULT; " "; ANSI_CHARSET; " "; WM_USER
