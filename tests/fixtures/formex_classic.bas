' QFormEx, written as a RapidQ program that includes Rapidq2.inc writes it:
' the include's TYPE (here a stand-in with the same public members,
' tests/fixtures/QFormEx.inc) is RapidR's RFormEx, and a form written
' QFormEx behaves as the include's (rapidr_ast::formex): its empty DragZone
' takes no drop (a zone set: the files inside it), Transparent reads 0 until
' set, OnMouseWheel gets 1 / -1, the mouse on the screen and Windows' key
' flags (here less the form's Left and Top), OnMinimise comes before the window minimises on Windows (WindowState
' still wsNormal in it; elsewhere the system tells RapidR after), OnShow set
' by the program and its InheritOnShow, OnMessage bound in the CREATE.
$INCLUDE "QFormEx.inc"
DECLARE SUB Dropped (File AS STRING)
DECLARE SUB Minimised
DECLARE SUB Wheel (Rotation AS INTEGER, X AS LONG, Y AS LONG, Keys AS INTEGER)
DECLARE SUB Shown
DECLARE SUB SetZone
DECLARE SUB Heard (Hwnd AS LONG, Msg AS LONG, wParam AS LONG, lParam AS LONG)

CREATE Main AS QFORMEX
    Caption = "QFormEx"
    Left = 100: Top = 50
    Width = 360: Height = 200
    DragEnable = 1
    OnDrag = Dropped
    OnMinimise = Minimised
    OnMouseWheel = Wheel
    OnShow = Shown
    OnMessage = Heard
    CREATE Zone AS QBUTTON
        Left = 250: Top = 120: Width = 90
        Caption = "Zone"
        OnClick = SetZone
    END CREATE
    CREATE Lbl AS QLABEL
        Left = 8: Top = 8: Width = 340
        Caption = ""
    END CREATE
    CREATE Lbl2 AS QLABEL
        Left = 8: Top = 30: Width = 340
        Caption = ""
    END CREATE
END CREATE

SUB Shown
    Main.InheritOnShow
    Lbl2.Caption = "shown t" + STR$(Main.Transparent) + " u" + STR$(Main.TrayIconUpdate) + " s" + STR$(Main.TrayIconShown)
END SUB
SUB Dropped (File AS STRING)
    Lbl.Caption = Lbl.Caption + "drag:" + File + "@" + STR$(Main.DropPoint.X) + "," + STR$(Main.DropPoint.Y) + " "
END SUB
SUB SetZone
    Main.DragZone.Left = 100: Main.DragZone.Top = 0
    Main.DragZone.Right = 200: Main.DragZone.Bottom = 50
END SUB
SUB Minimised
    Lbl.Caption = Lbl.Caption + "min" + STR$(Main.WindowState) + " "
END SUB
SUB Wheel (Rotation AS INTEGER, X AS LONG, Y AS LONG, Keys AS INTEGER)
    Lbl.Caption = Lbl.Caption + "wheel" + STR$(Rotation) + "," + STR$(X - Main.Left) + "," + STR$(Y - Main.Top) + "," + STR$(Keys) + " "
END SUB
SUB Heard (Hwnd AS LONG, Msg AS LONG, wParam AS LONG, lParam AS LONG)
END SUB

Main.ShowModal
