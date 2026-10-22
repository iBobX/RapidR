DIM x AS INTEGER
x = 1
NoSuchRoutine x
GOTO Nowhere
'! diagnostic main.bas:3:1 "Unknown SUB or FUNCTION 'NoSuchRoutine' — no file of the program has a SUB or FUNCTION of that name, and no builtin has it: write it (SUB … END SUB, in the main file or a file it $INCLUDEs), or call one that exists"
'! diagnostic main.bas:4:1 "Label 'Nowhere' not found in the main program"
