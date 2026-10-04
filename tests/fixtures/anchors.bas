' Anchors and Constraints (RapidR extensions, from Delphi; RapidQ had
' Align only), checked natively, interpreted (tests/native_gui_events.mjs,
' which resizes the form as a user would, RAPIDR_TEST_RESIZE) and in the
' browser (tests/web_gui_parity.mjs). The form is 400x300: its inside
' 398x269.
DECLARE SUB Go
DECLARE FUNCTION State$ AS STRING
CREATE Form AS QFORM
  Caption = "Anchors": Width = 400: Height = 300
  Constraints.MinWidth = 300: MinHeight = 200
  ' Right + bottom: keeps its distances to those edges (23, 14).
  CREATE Ok AS QBUTTON
    Caption = "OK": Left = 300: Top = 230: Width = 75: Height = 25
    Anchors = akRight + akBottom
    OnClick = Go
  END CREATE
  ' Left + right: stretches.
  CREATE Ed AS QEDIT
    Left = 10: Top = 10: Width = 200
    Anchors = akLeft + akTop + akRight
  END CREATE
  ' Stretches, but never below its MinWidth; its own child follows it.
  CREATE Pn AS QPANEL
    Left = 10: Top = 50: Width = 300: Height = 100
    Anchors = akLeft OR akTop OR akRight
    Constraints.MinWidth = 250
    CREATE Inner AS QBUTTON
      Caption = "in": Left = 200: Top = 10: Width = 60: Height = 25
      Anchors = akTop + akRight
    END CREATE
  END CREATE
  ' Neither left nor right: keeps its centre proportionally.
  CREATE Mid AS QLABEL
    Caption = "mid": Left = 150: Top = 180: Width = 100: Height = 20
    Anchors = akTop
  END CREATE
  ' The default anchors (left + top): stays.
  CREATE Info AS QLABEL
    Left = 10: Top = 160: Width = 100: Height = 20
  END CREATE
  CREATE A AS QLABEL
    Left = 10: Top = 210: Width = 50
  END CREATE
  CREATE B AS QLABEL
    Left = 60: Top = 210: Width = 50
  END CREATE
  CREATE C AS QLABEL
    Left = 110: Top = 210: Width = 50
  END CREATE
END CREATE

FUNCTION State$ AS STRING
  State$ = STR$(Ok.Left) + "," + STR$(Ok.Top) + "|" + STR$(Ed.Width) + "|" + STR$(Pn.Width) + "|" + STR$(Inner.Left) + "|" + STR$(Mid.Left) + "|" + STR$(Form.Width) + "x" + STR$(Form.Height)
END FUNCTION

' Before the form is shown.
A.Caption = State$ + "|" + STR$(Ok.Anchors) + "|" + STR$(Info.Anchors) + "|" + STR$(Info.MinWidth) + "|" + STR$(Form.Constraints.MinWidth) + "|" + STR$(Form.MinHeight) + "|" + STR$(akLeft + akTop + akRight + akBottom)

' After the user made the form 250x180 (it can't be less than 300x200).
SUB Go
  B.Caption = State$
  Form.Width = 500: Form.Height = 400
  C.Caption = State$
  ' The program places Ed: its distances are taken again.
  Ed.Width = 250: Form.Width = 400
  C.Caption = C.Caption + " " + State$
  Pn.Width = 100
  C.Caption = C.Caption + " " + STR$(Pn.Width) + "," + STR$(Inner.Left)
  ' A maximum below the minimum lowers the minimum (as in Delphi).
  Pn.Constraints.MaxWidth = 200
  C.Caption = C.Caption + " " + STR$(Pn.Width) + "," + STR$(Inner.Left) + "," + STR$(Pn.MinWidth)
  Form.Width = 100
  C.Caption = C.Caption + " " + State$ + "|" + STR$(Info.Left)
END SUB

Form.ShowModal
