' xfail: codegen — native builds don't compile OOP TYPEs yet (a clear compile error; in progress)
' RapidQ manual ch. 10: PROPERTY SET setters, the type name standing for the
' instance (TCounter.Focus, WITH TCounter), EXTENDS QObject, RESULT, obj.Func.
TYPE TCounter EXTENDS QObject
    Focus AS LONG PROPERTY SET Set_Focus
    Log AS STRING

    PROPERTY SET Set_Focus (Handle AS LONG)
        WITH TCounter
            .Focus = Handle * 2
            .Log = .Log + "set" + STR$(Handle) + ";"
        END WITH
    END PROPERTY

    FUNCTION TextSize AS INTEGER
        Result = LEN(TCounter.Log)
    END FUNCTION

    SUB Twice
        TCounter.Focus = TCounter.Focus + 1
        Focus = 100
    END SUB
END TYPE

DIM C AS TCounter
C.Focus = 5
PRINT C.Focus; " "; C.Log
C.Twice
PRINT C.Focus; " "; C.Log; " "; C.TextSize
