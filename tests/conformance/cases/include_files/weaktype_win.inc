'-- declarations like a VB port's: WEAKTYPE ON around Public Const lines,
'-- some of them names the file above has
$IFNDEF __WEAKTYPE_WIN

$DEFINE __WEAKTYPE_WIN

$OPTION WEAKTYPE ON
$OPTION VBDLL    ON

Public Const ANSI_CHARSET = 0
Public Const SND_SYNC = &H0         '  play synchronously
Public Const SND_ASYNC = &H1
Public Const SND_NODEFAULT = &H2
Public Const WM_USER = &H400

$OPTION WEAKTYPE OFF
$OPTION VBDLL    OFF
$ENDIF '__WEAKTYPE_WIN
