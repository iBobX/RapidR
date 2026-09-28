' CREATE name(dims) AS Type … END CREATE makes an array of components (as DIM).
CREATE bm(0 TO 2, 0 TO 1) AS QBITMAP
END CREATE
FOR r = 0 TO 1
    FOR c = 0 TO 2
        bm(c, r).Width = 4 + c
        bm(c, r).Height = 2 + r
    NEXT c
NEXT r
PRINT bm(2, 1).Width; "x"; bm(2, 1).Height; " "; bm(0, 0).Width
