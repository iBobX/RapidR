' A component moved to another parent leaves its old parent's scroll bars
' as if it had never been there (RUNTIME-BUGS 2026-11-05: a scroll box kept
' the horizontal bar a far-away button had needed after the button went to
' the form: ClientHeight 103, the bar's 17 pixels less, for ever).
$APPTYPE CONSOLE
CREATE F AS RForm
  Width = 400
  Height = 300
  CREATE Box AS RScrollBox
    Left = 0
    Top = 0
    Width = 200
    Height = 120
    BorderStyle = 0
    CREATE Far AS RButton
      Left = 500
      Top = 10
    END CREATE
  END CREATE
END CREATE
F.Show
PRINT "box"; Box.ClientWidth; Box.ClientHeight
Far.Parent = F
PRINT "moved"; Box.ClientWidth; Box.ClientHeight
Far.Parent = Box
PRINT "back"; Box.ClientWidth; Box.ClientHeight
F.Close
