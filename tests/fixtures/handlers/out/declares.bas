' handlers/declares.bas: a form whose SUBs are declared before it (the
' designer adds a DECLARE after the last one and the SUB at the end).
DECLARE SUB ShowIt
DECLARE SUB NameEditKeyDown (Key AS WORD, Shift AS INTEGER)
DECLARE SUB FormClose (BYREF Action AS INTEGER)
DECLARE SUB FormMouseDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB GridDrawCell (Col AS INTEGER, Row AS INTEGER, State AS INTEGER, R AS QRECT)
DECLARE SUB OkButtonKeyPress (Key AS BYTE)

DIM Count AS INTEGER

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
        OnKeyDown = NameEditKeyDown
    END CREATE
    CREATE Grid AS QSTRINGGRID
        Left = 8: Top = 72: Width = 200: Height = 100
        OnDrawCell = GridDrawCell
    END CREATE
END CREATE

Form.ShowModal

SUB ShowIt
    Count = Count + 1
END SUB

SUB NameEditKeyDown (Key AS WORD, Shift AS INTEGER)
    
END SUB

SUB FormClose (BYREF Action AS INTEGER)
    
END SUB

SUB FormMouseDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
    
END SUB

SUB GridDrawCell (Col AS INTEGER, Row AS INTEGER, State AS INTEGER, R AS QRECT)
    
END SUB

SUB OkButtonKeyPress (Key AS BYTE)
    
END SUB
