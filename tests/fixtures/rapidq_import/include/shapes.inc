' shapes.inc: a helper the program includes from its include folder.
DIM ShapeFont AS QFONT

FUNCTION Shape$ (n AS INTEGER) AS STRING
    IF n > 5 THEN Shape$ = "a long name" ELSE Shape$ = "a short name"
END FUNCTION
