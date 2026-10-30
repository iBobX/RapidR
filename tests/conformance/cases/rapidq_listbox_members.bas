' QLISTBOX has no TabStop (no QEDIT, QBUTTON or QCOMBOBOX either: RC.EXE has
' none on any component but QSTRINGGRID's TabStops): `ERROR: Member TABSTOP not
' part of class L`. Keys typed in a list reach its form's OnKeyDown only with
' KeyPreview (tests/fixtures/list_keys.bas).
CREATE Form AS QFORM
  CREATE L AS QLISTBOX
    AddItems "a", "b"
  END CREATE
END CREATE
L.TabStop = 0
PRINT "ok"
