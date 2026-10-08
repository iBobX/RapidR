' handlers/nodeclares.bas: a form whose SUBs come before it (the designer
' puts a new SUB just before the form's CREATE).
DIM Count AS INTEGER

SUB ShowIt
    Count = Count + 1
END SUB

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
