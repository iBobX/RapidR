' RapidQ's global objects and QRECT: Clipboard's text and methods,
' Application's properties, Screen.Cursor; a QRECT's fields start at 0.
DIM R AS QRECT
R.Left = 3 : R.Right = 10
PRINT R.Left; " "; R.Top; " "; R.Right; " "; R.Bottom
Clipboard.Open
Clipboard.Text = "hello"
PRINT Clipboard.Text; " "; Clipboard.HasFormat(1); " "; Clipboard.FormatCount
Clipboard.SetAsText "abcdef"
PRINT Clipboard.GetAsText(3)
Clipboard.Clear
PRINT LEN(Clipboard.Text); " "; Clipboard.HasFormat(1)
Clipboard.Close
PRINT Application.Title <> ""
Application.Title = "Mine"
PRINT Application.Title
Application.HelpFile = "x.hlp"
PRINT Application.HelpFile; " "; Application.ShowHint; " "; Application.HintPause
PRINT Screen.Cursor; " "; Screen.Width >= 0
