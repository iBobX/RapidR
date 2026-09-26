' CREATE of a TYPE (setup, then the block's settings and calls, a PROPERTY
' SET among them); an array field of objects (one instance per element);
' a field of the TYPE's own type starts empty (a link, `Link AS TItem`);
' `DIM a(n) AS TType` makes one instance per element; objects inside SUBs.
TYPE TPart
    Name AS STRING
    Weight AS INTEGER
END TYPE
TYPE TItem EXTENDS QObject
    Label AS STRING
    Qty AS INTEGER PROPERTY SET SetQty
    Parts(2) AS TPart
    Link AS TItem
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
Box.Parts(1).Name = "lid"
Box.Parts(1).Weight = 2
PRINT Box.Parts(1).Name; Box.Parts(1).Weight; " ["; Box.Parts(0).Name; "] "; Box.Parts(2)
PRINT "next empty:"; Box.Link = 0

DIM Many(1 TO 3) AS TItem
Many(2).Label = "second"
Many(2).Qty = 9
Many(2).Describe
Many(3).Describe
PRINT Many(1)
Box.Link = Many(2)
Box.Link.Describe
Many(2).Qty = 1
PRINT "shared:"; Box.Link.Qty

SUB MakeLocal
    DIM Tmp AS TItem
    Tmp.Qty = 7
    Tmp.Describe
END SUB
MakeLocal
