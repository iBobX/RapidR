$|
$APP|TYPE CONSOLE
DIM s AS STRING
s = MI|D$("hello", 2, |3)
PRINT LEFT$(s, 1); "a.|b"
' Form.|
start:
GOTO |
'! 1 completion has $INCLUDE $APPTYPE $DEFINE
'! 2 hover has "$APPTYPE"
'! 3 hover has "MID$"
'! 4 signature "MID$(str, start, [length])" active 2
'! 5 completion lacks PRINT
'! 6 completion lacks PRINT
'! 7 completion has start
