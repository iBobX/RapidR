' A form's HideTitleBar / ShowTitleBar, QFORM's MDI members and a button's StartDrag, as RC.EXE's programs do them (frame sizes are the system's: what's checked is what changes)
$APPTYPE CONSOLE
DIM Form AS QFORM
DIM B AS QBUTTON
B.Parent = Form
SUB Started
  PRINT "onstartdrag"
END SUB
SUB Ended
  PRINT "onenddrag"
END SUB
Form.Width = 300
Form.Height = 200
DIM h AS INTEGER, c AS INTEGER, w AS INTEGER
h = Form.Height
c = Form.ClientHeight
w = Form.ClientWidth
' (the title bar goes: the client area keeps its size, the form is shorter)
Form.HideTitleBar
PRINT "hidden: client "; (Form.ClientHeight = c); (Form.ClientWidth = w); " shorter "; (Form.Height < h); " width "; Form.Width
Form.HideTitleBar
PRINT "again: "; (Form.ClientHeight = c); (Form.Height < h)
Form.ShowTitleBar
PRINT "shown: "; (Form.Height = h); (Form.ClientHeight = c)
Form.ShowTitleBar
PRINT "again: "; (Form.Height = h); (Form.ClientHeight = c)
' (a QFORM has no MDI children)
PRINT "mdi "; Form.MdiChildCount; " tile "; Form.TileMode
Form.Cascade
Form.Tile
Form.Next
Form.Previous
Form.ArrangeIcons
Form.TileMode = 1
PRINT "tile "; Form.TileMode; " mdi "; Form.MdiChildCount
' (no mouse button held: StartDrag moves nothing; OnStartDrag only hears a press)
B.OnStartDrag = Started
B.OnEndDrag = Ended
PRINT "before "; B.Left; B.Top
B.StartDrag
PRINT "after "; B.Left; B.Top
