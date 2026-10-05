' QGLASSFRAME as RapidQ's compiler (RC.EXE) has it, inside a form: 105 x
' 105, Transparency 60 (a byte: -5 is 251, 256 is 0, 33.7 is 33),
' TransparentColor 0, Moveable 1 (5 stays 5), Align 0, Enabled / Visible 1,
' ShowHint / Cursor 0, ClientWidth its Width (setting it sets Width);
' DIMmed and given a Parent too (checked against RC.EXE).
$APPTYPE CONSOLE
CREATE F AS QFORM
  CREATE G AS QGLASSFRAME
  END CREATE
END CREATE
PRINT G.Left; " "; G.Top; " "; G.Width; " "; G.Height; " "; G.ClientWidth; " "; G.ClientHeight
PRINT G.Transparency; " "; G.TransparentColor; " "; G.Moveable
PRINT G.Align; " "; G.Enabled; " "; G.Visible; " "; G.ShowHint; " "; G.Cursor
PRINT "["; G.Hint; "]"
G.Transparency = 150
PRINT G.Transparency
G.Transparency = -5
PRINT G.Transparency
G.Transparency = 33.7
PRINT G.Transparency
G.Transparency = 256
PRINT G.Transparency
G.Width = 50: G.Height = 40: G.Left = 7
PRINT G.Left; " "; G.Width; " "; G.Height; " "; G.ClientWidth; " "; G.ClientHeight
G.Moveable = 0
PRINT G.Moveable
G.Moveable = 5
PRINT G.Moveable
G.TransparentColor = &HFF
PRINT G.TransparentColor
G.ClientWidth = 30
PRINT G.Width; " "; G.ClientWidth
G.Visible = 0
PRINT G.Visible
PRINT G.Handle <> 0
DIM H AS QGLASSFRAME
H.Parent = F
PRINT H.Transparency; " "; H.Width
DIM A(3) AS QGLASSFRAME
