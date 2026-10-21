' A program's own FUNCTION TR$ and an array named trn keep working beside
' RapidR's TR$ / TRN$ (programs written before them).
FUNCTION TR$(s AS STRING) AS STRING
    TR$ = "<" + s + ">"
END FUNCTION
DIM trn(3) AS INTEGER
trn(2) = 7
PRINT TR$("x")
PRINT trn(2)
