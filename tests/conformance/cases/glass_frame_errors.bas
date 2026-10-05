' QGLASSFRAME's members are RC.EXE's own: no Caption, Tag, Font, AutoSize,
' OnPaint or Repaint; Handle is read-only (RC.EXE's messages).
CREATE F AS QFORM
  CREATE G AS QGLASSFRAME
  END CREATE
END CREATE
G.Caption = "x"
G.Tag = 1
G.Font.Name = "Arial"
G.AutoSize = 1
G.Repaint
G.Handle = 5
