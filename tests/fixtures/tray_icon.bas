' The system tray as RapidQ programs use it (examples/forms/titlebtn.bas,
' examples/reminder): a QNOTIFYICONDATA given to Windows' Shell_NotifyIcon
' (shell32), the icon's clicks heard in the form's WndProc as the message
' the program chose — wParam the icon's uID, lParam the mouse message.
' RapidR shows the icon in the system's tray on every platform
' (rapidr_value::tray); the test hook `form.__tray_N` sends message N.
CONST NIM_ADD = 0
CONST NIM_MODIFY = 1
CONST NIM_DELETE = 2
CONST NIF_MESSAGE = 1
CONST NIF_ICON = 2
CONST NIF_TIP = 4
CONST WM_USER = &H400
CONST WM_TRAYICON = WM_USER + 400
CONST WM_LBUTTONDOWN = &H201
CONST WM_LBUTTONUP = &H202
DECLARE FUNCTION Shell_NotifyIcon LIB "shell32" ALIAS "Shell_NotifyIconA" (ByVal dwMessage AS LONG, NIData AS QNotifyIconData) AS LONG
DECLARE SUB FormProc (hWnd AS LONG, uMsg AS LONG, wParam AS LONG, lParam AS LONG)
DECLARE SUB ToTray
DIM NI AS QNotifyIconData
CREATE Form AS QFORM
  Caption = "tray icon"
  Width = 360
  Height = 160
  WndProc = FormProc
  CREATE Btn AS QBUTTON
    Left = 10 : Top = 10
    Caption = "To tray"
    OnClick = ToTray
  END CREATE
  CREATE Lbl AS QLABEL
    Left = 10 : Top = 50 : Width = 340
    Caption = "-"
  END CREATE
END CREATE
SUB ToTray
  NI.hWnd = Form.Handle
  NI.uID = 7
  NI.uFlags = NIF_MESSAGE OR NIF_ICON OR NIF_TIP
  NI.hIcon = Application.Icon
  NI.uCallbackMessage = WM_TRAYICON
  NI.szTip = "Click to show the window"
  Lbl.Caption = "add" + STR$(Shell_NotifyIcon(NIM_ADD, NI)) + STR$(Shell_NotifyIcon(NIM_ADD, NI))
  NI.szTip = "RapidR tray"
  Lbl.Caption = Lbl.Caption + " mod" + STR$(Shell_NotifyIcon(NIM_MODIFY, NI))
  Form.Visible = 0
END SUB
SUB FormProc (hWnd AS LONG, uMsg AS LONG, wParam AS LONG, lParam AS LONG)
  IF uMsg = WM_TRAYICON THEN
    Lbl.Caption = Lbl.Caption + " " + STR$(wParam) + ":" + HEX$(lParam AND &HFFFF) + IIF(hWnd = Form.Handle, "f", "?")
    IF (lParam AND &HFFFF) = WM_LBUTTONUP THEN
      Lbl.Caption = Lbl.Caption + " del" + STR$(Shell_NotifyIcon(NIM_DELETE, NI)) + STR$(Shell_NotifyIcon(NIM_DELETE, NI))
      Form.Visible = 1
    END IF
  END IF
END SUB
Form.ShowModal
