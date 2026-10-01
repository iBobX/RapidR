' QFORM AddBorderIcons / DelBorderIcons (SUBI: any number of biSystemMenu
' 0, biMinimize 1, biMaximize 2, biHelp 3): the web greys out the title
' bar's buttons; the desktop's title bar is the system's (RapidQ's manual:
' an icon may stay, greyed out).
CREATE Form AS QFORM
  Caption = "icons"
  DelBorderIcons(2)
END CREATE
Form.AddBorderIcons(3)
Form.ShowModal
