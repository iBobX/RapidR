' CREATE of a TYPE (setup, then the block's settings and calls, a PROPERTY
' SET among them), an array field of objects holding `<id>.field(i)` ids,
' a plain `DIM a(n) AS TType` array, and objects inside SUBs.
TYPE TItem EXTENDS QObject
    Label AS STRING
    Qty AS INTEGER PROPERTY SET SetQty
    Parts(2) AS TItem
    PROPERTY SET SetQty (n AS INTEGER)
        IF n < 0 THEN n = 0
        TItem.Qty = n
    END PROPERTY
    SUB Describe
        PRINT Label; " x"; Qty
    END SUB
    CONSTRUCTOR
        Label = "item"
    END CONSTRUCTOR
END TYPE

CREATE Box AS TItem
    Label = "box"
    Qty = -5
    Describe
END CREATE
Box.Qty = 3
Box.Describe
PRINT Box.Parts(0); "|"; Box.Parts(2)

DIM Many(2) AS TItem
PRINT "plain array:"; Many(1) = 0

SUB MakeLocal
    DIM Tmp AS TItem
    Tmp.Qty = 7
    Tmp.Describe
END SUB
MakeLocal
