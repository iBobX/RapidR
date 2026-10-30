' hints.bas: parameter names, colours, highlights and Help (test/suite/features.test.js).
$APPTYPE GUI
$INCLUDE "helpers.inc"

SUB DrawDot (x AS INTEGER, y AS INTEGER, r AS INTEGER)
END SUB

DIM total AS INTEGER
DIM x AS INTEGER

DrawDot x, 40, 12

CREATE Form AS RForm
    Caption = "Hints"
    Color = RGB(255, 130, 20)
    CREATE GoButton AS RButton
        Caption = "Go"
        Color = &H0000FF
        FontColor = clRed
        OnClick = Go
    END CREATE
END CREATE

SUB Go
    total = total + 1
    GoButton.Caption = "Went " + Shout(STR$(total))
END SUB

Form.ShowModal
