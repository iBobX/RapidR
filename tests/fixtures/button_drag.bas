' A button dragged, RapidQ's two ways (RC.EXE in the Windows VM, the real
' mouse): OnStartDrag bound makes Drag a drag source — the press fires
' OnStartDrag instead of OnMouseDown, the release OnEndDrag instead of
' OnMouseUp and OnClick; Mover's OnMouseDown calls StartDrag — it moves
' with the mouse until the release, when StartDrag returns (Left / Top
' moved by the mouse's way: 150,100 went to 185,125 for a 35,25 drag), and
' no OnMouseUp or OnClick follows. A cool button moves the same way.
DECLARE SUB Started
DECLARE SUB Ended
DECLARE SUB DragDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB DragUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB DragClick
DECLARE SUB MoverDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB MoverUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB MoverClick
DECLARE SUB CoolDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DIM Log AS STRING
CREATE Form AS QFORM
  Caption = "button drag"
  Width = 360
  Height = 260
  CREATE Drag AS QBUTTON
    Caption = "drag me"
    Left = 10
    Top = 10
    OnStartDrag = Started
    OnEndDrag = Ended
    OnMouseDown = DragDown
    OnMouseUp = DragUp
    OnClick = DragClick
  END CREATE
  CREATE Mover AS QBUTTON
    Caption = "move me"
    Left = 150
    Top = 100
    OnMouseDown = MoverDown
    OnMouseUp = MoverUp
    OnClick = MoverClick
  END CREATE
  CREATE Cool AS QCOOLBTN
    Caption = "C"
    Left = 10
    Top = 60
    OnMouseDown = CoolDown
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 5
    Top = 200
    Width = 340
    Caption = "-"
  END CREATE
END CREATE

SUB Note(S AS STRING)
  Log = Log + " " + S
  Lbl.Caption = Log
END SUB

SUB Started
  Note "start"
END SUB

SUB Ended
  Note "end"
END SUB

SUB DragDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "ddown"
END SUB

SUB DragUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "dup"
END SUB

SUB DragClick
  Note "dclick"
END SUB

SUB MoverDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "m" + STR$(Mover.Left) + "," + STR$(Mover.Top)
  Mover.StartDrag
  Note "moved" + STR$(Mover.Left) + "," + STR$(Mover.Top)
END SUB

SUB MoverUp (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Note "mup"
END SUB

SUB MoverClick
  Note "mclick"
END SUB

SUB CoolDown (Button AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
  Cool.StartDrag
  Note "cool" + STR$(Cool.Left) + "," + STR$(Cool.Top)
END SUB

Form.ShowModal
