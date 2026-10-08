' handlers/declares.bas: a form whose SUBs are declared before it (the
' designer adds a DECLARE after the last one and the SUB at the end).
DECLARE SUB ShowIt

DIM Count AS INTEGER

CREATE Form AS QFORM
    Caption = "Handlers"
    Width = 320: Height = 240
    CREATE OkButton AS QBUTTON
        Caption = "OK"
        Left = 8: Top = 8
        OnClick = ShowIt
    END CREATE
    CREATE NameEdit AS QEDIT
        Left = 8: Top = 40
    END CREATE
    CREATE Grid AS QSTRINGGRID
        Left = 8: Top = 72: Width = 200: Height = 100
    END CREATE
END CREATE

Form.ShowModal

SUB ShowIt
    Count = Count + 1
END SUB
