' RapidR Studio's designer and the running program lay a form out the
' same way (docs/ide-plan.md I4, anchoring): crates/rapidr-designer/tests/
' anchors.rs reads this CREATE block into the designer, resizes its preview
' to 600 x 450, and must get the rectangles tests/gui_parity_cases.mjs's
' "designer_anchors" case reads from this program resized by its user to
' the same size — native, interpreted and in the browser. Anchors and
' Constraints are RapidR's (RC.EXE: "Member ANCHORS not part of class");
' Align and AutoSize are RapidQ's.
$INCLUDE "RAPIDQ.INC"
CREATE Form AS QFORM
  Caption = "Designer anchors": Width = 420: Height = 320
  Constraints.MinWidth = 260: MinHeight = 200
  CREATE Bar AS QPANEL
    Height = 32: Align = alTop
  END CREATE
  CREATE Status AS QPANEL
    Height = 22: Align = alBottom
  END CREATE
  ' An AutoSize label, then the edit it names: stretches.
  CREATE NameLbl AS QLABEL
    Caption = "Name:": Left = 12: Top = 48
  END CREATE
  CREATE NameEd AS QEDIT
    Left = 64: Top = 44: Width = 240
    Anchors = akLeft + akTop + akRight
  END CREATE
  ' Grows both ways.
  CREATE Notes AS QRICHEDIT
    Left = 12: Top = 80: Width = 292: Height = 150
    Anchors = akLeft + akTop + akRight + akBottom
  END CREATE
  ' Keeps to the right, grows down; its button keeps to its bottom.
  CREATE Side AS QPANEL
    Left = 316: Top = 44: Width = 92: Height = 186
    Anchors = akTop + akRight + akBottom
    CREATE Pick AS QBUTTON
      Caption = "Pick": Left = 8: Top = 150: Width = 75
      Anchors = akLeft + akBottom
    END CREATE
  END CREATE
  CREATE Ok AS QBUTTON
    Caption = "OK": Left = 248: Top = 236: Width = 75: Height = 25
    Anchors = akRight + akBottom
  END CREATE
  CREATE Cancel AS QBUTTON
    Caption = "Cancel": Left = 332: Top = 236: Width = 75: Height = 25
    Anchors = akRight + akBottom
  END CREATE
  ' Neither left nor right: its centre keeps its place proportionally.
  CREATE Mid AS QLABEL
    Caption = "centred": Left = 150: Top = 240
    Anchors = akBottom
  END CREATE
END CREATE

Form.ShowModal
