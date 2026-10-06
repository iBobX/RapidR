' RapidQ's implicit scope (RC.EXE): a name never declared is a variable
' made by its first use. A name the main program used first (above the
' SUB) is that global inside a SUB; a name the SUB uses first is the SUB's
' own, kept between calls, and the main program's same name further down
' is another variable.
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
'! 1 references main.bas:6:1 main.bas:8:5 main.bas:8:13 main.bas:14:7
'! 2 hover has "variable of `Up`" "kept between calls"
'! 2 references main.bas:9:5 main.bas:11:11
'! 3 references main.bas:14:14
'! 3 hover has "global variable"
