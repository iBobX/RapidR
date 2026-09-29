' Keywords as a TYPE's member names (Step, Open, Alias fields; Create,
' Close methods, a Data field, select / case parameters) and `END PROPERTY SET`,
' as RapidQ's include libraries write.
TYPE TDb EXTENDS QOBJECT
  Step AS DOUBLE
  Open AS INTEGER
  Alias AS STRING
  FUNCTION Create(N AS STRING) AS INTEGER
    This.Open = 1
    This.Alias = N
    Result = LEN(N)
  END FUNCTION
  SUB Close
    WITH This
      .Step = .Step + 0.5
    END WITH
  END SUB
END TYPE
DIM D AS TDb
PRINT D.Create("abc"); " "; D.Open; " "; D.Alias
D.Close
D.Close
PRINT D.Step
TYPE TP EXTENDS QOBJECT
  V AS LONG PROPERTY SET SetV
  PROPERTY SET SetV(N AS LONG)
    This.V = N + 1
  END PROPERTY SET
END TYPE
DIM T AS TP
T.V = 4
PRINT T.V
TYPE TD2 EXTENDS QOBJECT
  Data AS STRING
  Multi AS INTEGER
  PROPERTY SET SetMulti(select AS INTEGER)
    This.Multi = select * 2
  END PROPERTY
  FUNCTION Find(start AS LONG, case AS INTEGER) AS LONG
    IF case THEN Result = start + 1 ELSE Result = start
  END FUNCTION
END TYPE
DIM T2 AS TD2
T2.Data = "x"
T2.SetMulti(4)
PRINT T2.Data; " "; T2.Multi; " "; T2.Find(5, 1); " "; T2.Find(5, 0)
DATA 7
READ A
PRINT A
