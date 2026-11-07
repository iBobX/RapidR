' (RapidR's) RDBGRID's current column, Col: a click on a cell sets it
' (OnCellClick says the same column), Left and Right move it (kept inside
' the data's columns), Up and Down move the row, and the program sets it.
' The current cell has the focus box and is the cell a screen reader is
' on: "name, Di, row 4 of 4, column 1 of 3" at the end.
DIM NL AS STRING
NL = CHR$(10)
DIM log AS STRING

SUB Clicked(Row AS INTEGER, Col AS INTEGER)
    log = log + "c" + STR$(Row) + "," + STR$(Col) + "/" + STR$(Grid.Col) + " "
    Info.Caption = log
END SUB

SUB ShowClick
    log = log + "=" + STR$(Grid.Row) + "," + STR$(Grid.Col) + " "
    Grid.Col = 0
    log = log + "set" + STR$(Grid.Col) + " "
    Grid.Col = 9
    log = log + "far" + STR$(Grid.Col) + " "
    Grid.Col = 0
    Info.Caption = log
END SUB

CREATE Form AS RFORM
    Caption = "Current column"
    Width = 420: Height = 260
    CREATE People AS RCSV
        Text = "name,age,city" + NL + "Ada,36,London" + NL + "Bob,41,Paris" + NL + "Cy,29,Rome" + NL + "Di,52,Oslo" + NL
    END CREATE
    CREATE Grid AS RDBGRID
        Left = 8: Top = 8: Width = 396: Height = 150
        DataSource = People
        OnCellClick = Clicked
    END CREATE
    CREATE Show AS RBUTTON
        Caption = "Show"
        Left = 8: Top = 170: Width = 90
        OnClick = ShowClick
    END CREATE
    CREATE Info AS RLABEL
        Left = 8: Top = 205: Width = 396
    END CREATE
END CREATE

Form.ShowModal
