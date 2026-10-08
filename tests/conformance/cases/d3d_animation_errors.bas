' QD3DANIMATION has no member but Parent, and QD3DMESH's MaxY can't be
' set (RC.EXE's words).
DIM A AS QD3DANIMATION
DIM M AS QD3DMESH
A.SetTime 1
PRINT A.Tag
M.MaxY = 5
