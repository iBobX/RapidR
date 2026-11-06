' RFormEx (rapidr_value::formex, docs/rformex-design.md): files dropped one by
' one (`main.__drop_X_Y`: RAPIDR_TEST_DROP's files at (X, Y) of the client
' area) — anywhere while DragZone is empty, then only inside it, DropPoint
' set first; the form's own tray icon (`main.__tray_N`: mouse message N from
' it) as OnTrayClick (the right button's release), OnTrayDblClick and
' OnTrayMouseUp; OnMouseWheel as an RForm's, OnMinimise once the window is
' minimised (`__minimise`: WindowState reads wsMinimized in it); a second
' form's OnMessage hearing the program's own RNotifyIconData icon
' (`other.__tray_N`). Lbl2: what the members read before the program sets
' them. Lbl3: OnGetFocus / OnLostFocus (`__activate` / `__deactivate`), once
' for each change (a window shown active already hears no second one).
CONST NIM_ADD = 0
CONST NIF_MESSAGE = 1
DECLARE FUNCTION Shell_NotifyIcon LIB "shell32" ALIAS "Shell_NotifyIconA" (BYVAL dwMessage AS LONG, NIData AS QNotifyIconData) AS LONG
DECLARE SUB Dropped (File AS STRING)
DECLARE SUB TrayRight
DECLARE SUB TrayDouble
DECLARE SUB TrayUp (Button AS INTEGER)
DECLARE SUB Minimised
DECLARE SUB GotFocus
DECLARE SUB LostFocus
DECLARE SUB Wheel (Delta AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
DECLARE SUB SetZone
DECLARE SUB Heard (Hwnd AS LONG, Msg AS LONG, wParam AS LONG, lParam AS LONG)
DIM NI AS QNotifyIconData

CREATE Main AS RFormEx
    Caption = "RFormEx"
    Width = 360
    Height = 200
    DragEnable = 1
    OnDrag = Dropped
    OnTrayClick = TrayRight
    OnTrayDblClick = TrayDouble
    OnTrayMouseUp = TrayUp
    OnMinimise = Minimised
    OnGetFocus = GotFocus
    OnLostFocus = LostFocus
    OnMouseWheel = Wheel
    CREATE Zone AS RButton
        Left = 250: Top = 120: Width = 90
        Caption = "Zone"
        OnClick = SetZone
    END CREATE
    CREATE Lbl AS RLabel
        Left = 8: Top = 8: Width = 340
        Caption = ""
    END CREATE
    CREATE Lbl2 AS RLabel
        Left = 8: Top = 30: Width = 340
        Caption = ""
    END CREATE
    CREATE Lbl3 AS RLabel
        Left = 8: Top = 52: Width = 340
        Caption = ""
    END CREATE
END CREATE

CREATE Other AS RFormEx
    Caption = "Other"
    Width = 200: Height = 100
    OnMessage = Heard
END CREATE

SUB Dropped (File AS STRING)
    Lbl.Caption = Lbl.Caption + "drag:" + File + "@" + STR$(Main.DropPoint.X) + "," + STR$(Main.DropPoint.Y) + " "
END SUB
SUB SetZone
    Main.DragZone.Left = 100: Main.DragZone.Top = 0
    Main.DragZone.Right = 200: Main.DragZone.Bottom = 50
END SUB
SUB TrayRight
    Lbl.Caption = Lbl.Caption + "click "
END SUB
SUB TrayDouble
    Lbl.Caption = Lbl.Caption + "dbl "
END SUB
SUB TrayUp (Button AS INTEGER)
    Lbl.Caption = Lbl.Caption + "up" + STR$(Button) + " "
END SUB
SUB Minimised
    Lbl.Caption = Lbl.Caption + "min" + STR$(Main.WindowState) + " "
END SUB
SUB GotFocus
    Lbl3.Caption = Lbl3.Caption + "get "
END SUB
SUB LostFocus
    Lbl3.Caption = Lbl3.Caption + "lost "
END SUB
SUB Wheel (Delta AS INTEGER, X AS INTEGER, Y AS INTEGER, Shift AS INTEGER)
    Lbl.Caption = Lbl.Caption + "wheel" + STR$(Delta) + "," + STR$(X) + "," + STR$(Y) + "," + STR$(Shift) + " "
END SUB
SUB Heard (Hwnd AS LONG, Msg AS LONG, wParam AS LONG, lParam AS LONG)
    Lbl2.Caption = Lbl2.Caption + " msg" + STR$(Msg - &H400) + "," + STR$(wParam) + ":" + HEX$(lParam) + IIF(Hwnd = Other.Handle, "o", "?")
END SUB

Lbl2.Caption = "t" + STR$(Main.Transparent) + " u" + STR$(Main.TrayIconUpdate) + " s" + STR$(Main.TrayIconShown) + " f" + STR$(Main.FormStyle) + " d" + STR$(Main.DragEnable) + " c" + STR$(Main.TrayICO.Count)
Main.TrayIconHint = "RFormEx test"
Main.AddTrayIcon
Lbl2.Caption = Lbl2.Caption + " s" + STR$(Main.TrayIconShown)
Main.Transparent = 300
Lbl2.Caption = Lbl2.Caption + " t" + STR$(Main.Transparent)
NI.hWnd = Other.Handle
NI.uID = 9
NI.uFlags = NIF_MESSAGE
NI.uCallbackMessage = &H405
Shell_NotifyIcon(NIM_ADD, NI)
Other.Show
Main.ShowModal
