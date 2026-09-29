' A SUB bound to an event takes its event arguments by reference (the
' runtime reads OnClose's Action back), but called from the program it takes
' them by value, as any SUB: directly, with CALLFUNC, or with @ (by reference).
DECLARE SUB FormClose (Action AS INTEGER)
DECLARE SUB Touch (BYREF N AS INTEGER, Other AS INTEGER)
DECLARE FUNCTION Twice (N AS INTEGER) AS INTEGER
CREATE Win AS QFORM
    OnClose = FormClose
    CREATE Btn AS QBUTTON
        OnClick = Touch
        OnKeyPress = Twice
    END CREATE
END CREATE
DIM A AS INTEGER, B AS INTEGER
A = 5
FormClose A
PRINT "direct "; A
FormClose(A)
PRINT "parens "; A
DIM P AS INTEGER
BIND P TO FormClose
CALLFUNC(P, A)
PRINT "callfunc "; A
FormClose @A
PRINT "ref "; A
A = 1: B = 2
Touch A, B
PRINT "byref "; A; " "; B
PRINT "function "; Twice(A); " "; A
SUB FormClose (Action AS INTEGER)
    Action = 0
END SUB
SUB Touch (BYREF N AS INTEGER, Other AS INTEGER)
    N = N + 10
    Other = 0
END SUB
FUNCTION Twice (N AS INTEGER) AS INTEGER
    N = N * 2
    Twice = N
END FUNCTION
