' Object-oriented TYPEs: same results natively (codegen objects.rs) and in the interpreter.
' RapidQ manual ch. 10: PROPERTY SET setters, the type name standing for the
' instance (TCounter.Focus, WITH TCounter), EXTENDS QObject, RESULT, obj.Func.
' Inside the TYPE's own code a store into its property field is just the
' store — the setter runs for stores from outside (RC.EXE prints this).
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
        WITH TCounter
            .Focus = .Focus + 100
        END WITH
    END SUB
END TYPE

DIM C AS TCounter
C.Focus = 5
PRINT C.Focus; " "; C.Log
C.Twice
PRINT C.Focus; " "; C.Log; " "; C.TextSize
WITH C
    .Focus = 7
END WITH
PRINT C.Focus; " "; C.Log
