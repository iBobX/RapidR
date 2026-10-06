' RAPIDQ.INC's constants (supplied on the $INCLUDE line) are not the file's.
$INCLUDE "RAPIDQ.INC"
CONST Rate = 2
DIM total AS DOUBLE
TYPE TItem
    Name AS STRING
    FUNCTION Label AS STRING
        Label = Name
    END FUNCTION
END TYPE
CREATE Form AS QFORM
    CREATE Btn AS QBUTTON
        Caption = "Go"
    END CREATE
END CREATE
SUB Go
    total = total * Rate + clRed
END SUB
FUNCTION Twice(n AS INTEGER) AS INTEGER
    Twice = n * 2
END FUNCTION
PRINT cl|Red
'! outline Rate total TItem Name Label Form Btn Go Twice
'! 1 hover has "RAPIDQ.INC"
'! diagnostics none
