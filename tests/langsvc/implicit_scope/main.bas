' RapidQ's implicit scope: a name never declared is a global made by its
' first use; in a SUB, assigning an undeclared name sets that global.
cou|nt = 5
SUB Up
    count = count + 1
    tmp = 1
    DIM loc AS INTEGER
    loc = tm|p
END SUB
Up
PRINT count; t|mp
'! 1 hover has "implicit"
'! 1 references main.bas:3:1 main.bas:5:5 main.bas:5:13 main.bas:11:7
'! 2 hover has "global variable"
'! 2 references main.bas:6:5 main.bas:8:11 main.bas:11:14
'! 3 definition none
