$INCLUDE |"util.inc"
DIM n AS INTEGER
n = Add|One(1)
PRINT AddOne(n) + Lim|it
|
'! 1 definition util.inc:1:1
'! 2 definition util.inc:2:10
'! 2 references util.inc:2:10 util.inc:3:5 main.bas:3:5 main.bas:4:7
'! 2 hover has "FUNCTION AddOne(x AS INTEGER) AS INTEGER" "util.inc"
'! 3 definition util.inc:1:7
'! 4 completion has AddOne Limit n
'! diagnostics none
