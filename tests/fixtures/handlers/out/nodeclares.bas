' handlers/nodeclares.bas: a form whose SUBs come before it (the designer
' puts a new SUB just before the form's CREATE).
DIM Count AS INTEGER

SUB ShowIt
    Count = Count + 1
END SUB

SUB NameEditChange
    
END SUB

SUB FormClose (BYREF Action AS INTEGER)
    
END SUB

SUB FormMouseDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
    
END SUB

SUB GridDrawCell (Col AS INTEGER, Row AS INTEGER, State AS INTEGER, R AS QRECT)
    
END SUB

SUB OkButtonKeyPress (Key AS BYTE)
    
END SUB

CREATE Form AS QFORM
    Caption = "Handlers"
    Width = 320: Height = 240
    OnClose = FormClose
    OnMouseDown = FormMouseDown
    CREATE OkButton AS QBUTTON
        Caption = "OK"
        Left = 8: Top = 8
        OnClick = ShowIt
        OnKeyPress = OkButtonKeyPress
    END CREATE
    CREATE NameEdit AS QEDIT
        Left = 8: Top = 40
        OnChange = NameEditChange
    END CREATE
    CREATE Grid AS QSTRINGGRID
        Left = 8: Top = 72: Width = 200: Height = 100
        OnDrawCell = GridDrawCell
    END CREATE
END CREATE

Form.ShowModal
