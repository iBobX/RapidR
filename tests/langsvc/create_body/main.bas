$APPTYPE GUI
CREATE Form AS QFORM
    Caption = "Hello"
    Width = 320
    CREATE Btn AS QBUTTON
        Cap|tion = "Go"
        |
    END CREATE
END CREATE
Form.|
Btn.Cap|tion = "Again"
Form.Show|Modal
'! 1 hover has "QButton.Caption" "property of QButton"
'! 2 completion has Caption Left OnClick CREATE
'! 2 completion lacks PRINT Form MID$
'! 3 completion has Caption ShowModal OnClose Width
'! 3 completion lacks Btn MID$ PRINT
'! 4 hover has "QButton.Caption"
'! 5 hover has "QForm.ShowModal" "method of QForm"
