' A TYPE named QFormEx that isn't Rapidq2.inc's (an older include's, or a
' program's own class of that name): RapidR doesn't guess — it stops at the
' TYPE and says what differs from the QFormEx it knows (rapidr_ast::formex).
DECLARE SUB OneArg (X AS INTEGER)
DECLARE SUB TwoArgs (X AS INTEGER, Y AS INTEGER)

TYPE QFormEx EXTENDS QFORM
    DragEnable AS INTEGER
    Blinking AS INTEGER
    OnDrag AS EVENT(TwoArgs)
    OnTrayRightClick AS EVENT(OneArg)
    SUB Blink
        QFormEx.Blinking = 1
    END SUB
END TYPE

CREATE F AS QFormEx
    Caption = "mine"
END CREATE
PRINT "never"
