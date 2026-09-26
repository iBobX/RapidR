' Object-oriented TYPEs: same results natively (codegen objects.rs) and in the interpreter.
' Composition (manual 10.5): a TYPE field of TYPE type is its own object per
' instance; nested stores, method calls and FUNCTION methods through it.
TYPE TEngine EXTENDS QObject
    Power AS INTEGER
    SUB Boost (n AS INTEGER)
        This.Power = This.Power + n
    END SUB
    FUNCTION Describe AS STRING
        Result = "power " + STR$(TEngine.Power)
    END FUNCTION
END TYPE
TYPE TCar EXTENDS QObject
    Name AS STRING
    Engine AS TEngine
    CONSTRUCTOR
        Engine.Power = 100
        Name = "car"
    END CONSTRUCTOR
    SUB Tune
        .Engine.Boost 5
        Engine.Boost 1
    END SUB
END TYPE
DIM A AS TCar
DIM B AS TCar
A.Engine.Power = A.Engine.Power + 50
A.Tune
PRINT A.Name; " "; A.Engine.Power; " "; B.Engine.Power
PRINT A.Engine.Describe; " / "; B.Engine.Describe()
B.Engine.Boost 7
PRINT B.Engine.Power
