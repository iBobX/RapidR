' vbport.inc: constants as a VB port writes them (the WeakType
' example's library; its own, a few lines). The option is ON around the
' CONSTs, so the names RAPIDQ.INC has can be written again, and OFF after.
$IFNDEF __VBPORT

$DEFINE __VBPORT

$OPTION WEAKTYPE ON

Public Const ANSI_CHARSET = 0
Public Const SND_SYNC = &H0           ' play synchronously
Public Const SND_ASYNC = &H1          ' play asynchronously
Public Const SND_NODEFAULT = &H2      ' silence, not the default sound
Public Const WM_USER = &H400

$OPTION WEAKTYPE OFF

$ENDIF ' __VBPORT
