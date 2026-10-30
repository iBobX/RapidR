' An event whose handler fires it again before it ends runs inside itself
' (a program's own set fires its event at once, RapidQ's way): one that
' never stops — an edit's OnChange giving it a new Text each time, a Repaint
' inside a canvas's own OnPaint — stops with a run-time error naming the
' event, 200 handlers deep, instead of the process running out of stack.
$APPTYPE CONSOLE
DECLARE SUB EChange

DIM n AS INTEGER

CREATE Form AS QFORM
  CREATE E AS QEDIT
    OnChange = EChange
  END CREATE
END CREATE

SUB EChange
  n = n + 1
  E.Text = "text " + STR$(n)
END SUB

PRINT "start"
E.Text = "go"
PRINT "not reached"
