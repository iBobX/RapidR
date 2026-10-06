' main.bas: the program the extension's integration tests open
' (test/suite/extension.test.js finds what it asks about by its text).
$INCLUDE "helpers.inc"

DECLARE SUB Greet

DIM counter AS INTEGER

CREATE Form AS QFORM
    Caption = "Fixture"
    Width = 320
    Height = 200
    CREATE GreetButton AS QBUTTON
        Caption = "&Greet"
        Left = 16: Top = 16
        OnClick = Greet
    END CREATE
END CREATE

SUB Greet
    counter = counter + 1

    Form.Caption = MID$("Hello, world", 1, 5) + Shout(STR$(counter))
END SUB

Form.ShowModal
