' RDBGRID and data-aware controls (I7, rapidr_value::datascience::link):
' an RCSV's rows in a grid drawn by the UI kernel, the current row shared
' with a bound REdit and RLabel; a row clicked, a heading clicked (the
' view sorted), a name typed back. The same pixels on the desktop and the
' web.
DIM NL AS STRING
NL = CHR$(10)

CREATE Form AS RFORM
    Caption = "DBGrid"
    Width = 420: Height = 300
    CREATE People AS RCSV
        Text = "name,age,city,score" + NL + "bob,30,NYC,1.5" + NL + "amy,25,LA,2" + NL + "Lee,41,NYC,3.25" + NL + "cat,,LA,0.5" + NL + "dan,35,SF,4" + NL
    END CREATE
    CREATE Grid AS RDBGRID
        Left = 8: Top = 8: Width = 396: Height = 170
        DataSource = People
    END CREATE
    CREATE NameEdit AS REDIT
        Left = 8: Top = 190: Width = 160
        DataSource = People: DataField = "name"
    END CREATE
    CREATE AgeLbl AS RLABEL
        Left = 180: Top = 194: Width = 100
        DataSource = People: DataField = "age"
    END CREATE
    CREATE Info AS RLABEL
        Left = 8: Top = 230: Width = 396
    END CREATE
END CREATE

SUB Clicked(Row AS INTEGER, Col AS INTEGER)
    Info.Caption = "clicked " + STR$(Row) + "," + STR$(Col) + " of " + STR$(Grid.RowCount)
END SUB
Grid.OnCellClick = Clicked

Form.ShowModal
