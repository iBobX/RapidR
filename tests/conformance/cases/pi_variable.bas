' A program may use PI as a variable of its own (as 3dcube.bas does).
pi = 3.14
PRINT pi
SUB Circle (r AS DOUBLE)
    pi = 3
    PRINT pi * r
END SUB
Circle 2
PRINT pi
