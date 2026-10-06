' Components start with RapidQ's sizes (RC.EXE reads them for components created without one), the same on every runtime.
CREATE F AS QFORM
  CREATE A1 AS QBUTTON: END CREATE
  CREATE A2 AS QLABEL: END CREATE
  CREATE A3 AS QEDIT: END CREATE
  CREATE A4 AS QRICHEDIT: END CREATE
  CREATE A5 AS QCOOLBTN: END CREATE
  CREATE A6 AS QOVALBTN: END CREATE
  CREATE A7 AS QCHECKBOX: END CREATE
  CREATE A8 AS QRADIOBUTTON: END CREATE
  CREATE A9 AS QPANEL: END CREATE
  CREATE B1 AS QTABCONTROL: END CREATE
  CREATE B2 AS QCOMBOBOX: END CREATE
  CREATE B3 AS QLISTBOX: END CREATE
  CREATE B4 AS QGROUPBOX: END CREATE
  CREATE B5 AS QSCROLLBOX: END CREATE
  CREATE B6 AS QSCROLLBAR: END CREATE
  CREATE B7 AS QSTRINGGRID: END CREATE
  CREATE B8 AS QLISTVIEW: END CREATE
  CREATE B9 AS QDIRTREE: END CREATE
  CREATE C1 AS QFILELISTBOX: END CREATE
  CREATE C2 AS QTREEVIEW: END CREATE
  CREATE C3 AS QTRACKBAR: END CREATE
  CREATE C4 AS QCANVAS: END CREATE
  CREATE C5 AS QIMAGE: END CREATE
  CREATE C6 AS QPROGRESSBAR: END CREATE
END CREATE
PRINT "form"; F.Width; "x"; F.Height
PRINT "button"; A1.Width; "x"; A1.Height; " label"; A2.Width; "x"; A2.Height; " edit"; A3.Width; "x"; A3.Height
PRINT "richedit"; A4.Width; "x"; A4.Height; " coolbtn"; A5.Width; "x"; A5.Height; " ovalbtn"; A6.Width; "x"; A6.Height
PRINT "checkbox"; A7.Width; "x"; A7.Height; " radio"; A8.Width; "x"; A8.Height; " panel"; A9.Width; "x"; A9.Height
PRINT "tabcontrol"; B1.Width; "x"; B1.Height; " combobox"; B2.Width; "x"; B2.Height; " listbox"; B3.Width; "x"; B3.Height
PRINT "groupbox"; B4.Width; "x"; B4.Height; " scrollbox"; B5.Width; "x"; B5.Height; " scrollbar"; B6.Width; "x"; B6.Height
PRINT "stringgrid"; B7.Width; "x"; B7.Height; " listview"; B8.Width; "x"; B8.Height; " dirtree"; B9.Width; "x"; B9.Height
PRINT "filelistbox"; C1.Width; "x"; C1.Height; " treeview"; C2.Width; "x"; C2.Height; " trackbar"; C3.Width; "x"; C3.Height
PRINT "canvas"; C4.Width; "x"; C4.Height; " image"; C5.Width; "x"; C5.Height; " progressbar"; C6.Width; "x"; C6.Height
