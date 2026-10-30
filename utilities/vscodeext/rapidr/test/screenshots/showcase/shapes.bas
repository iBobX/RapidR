' Shapes: parameter names and colours, as VS Code shows them
$APPTYPE GUI

SUB DrawDot (x AS INTEGER, y AS INTEGER, r AS INTEGER)
END SUB

DIM dots AS INTEGER

DrawDot 10, 40, 12

CREATE Form AS RForm
    Caption = "Shapes"
    Color = RGB(255, 130, 20)
    CREATE GoButton AS RButton
        Caption = "Go"
        Color = &H0000FF
        FontColor = clBlue
        OnClick = Go
    END CREATE
END CREATE

SUB Go
    dots = dots + 1
    GoButton.Caption = "Dots: " + STR$(dots)
END SUB

Form.ShowModal
