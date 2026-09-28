SUB ArrayInsert (Array() AS LONG, Element AS LONG, Value AS LONG)
    DIM M AS QMEMORYSTREAM
    M.Position = 0
    M.SaveArray(Array(LBOUND(Array)), Element-1)
    M.Write(Value)
    M.SaveArray(Array(Element), UBOUND(Array) - Element)
    M.Position = 0
    M.LoadArray(Array(LBOUND(Array)), UBOUND(Array)+1)
END SUB

DIM A(1 TO 10) AS LONG
FOR I = 1 TO 6: A(I) = I * 10: NEXT
ArrayInsert(A, 3, 99)
FOR I = 1 TO 7: PRINT A(I);: NEXT: PRINT

DIM F(9) AS SHORT
DIM D(2, 2) AS DOUBLE
DIM B(3) AS BYTE
DIM S AS QMEMORYSTREAM
FOR I = 0 TO 9: F(I) = -I: NEXT
S.SaveArray(F(0), 10)
PRINT "short bytes:"; S.Size
D(1, 0) = 1.5: D(1, 1) = -2.25: D(1, 2) = 3
S.SaveArray(D(1, 0), 3)
PRINT "with doubles:"; S.Size
FOR I = 0 TO 9: F(I) = 0: NEXT
S.Position = 0
S.LoadArray(F(2), 5)
FOR I = 0 TO 9: PRINT F(I);: NEXT: PRINT
S.Position = 20
S.LoadArray(D(0, 0), 3)
PRINT D(0, 0); D(0, 1); D(0, 2)
S.Position = 0
S.LoadArray(B(0), 4)
PRINT B(0); B(1); B(2); B(3)
DIM sh AS SHORT, by AS BYTE, sg AS SINGLE, lg AS LONG, db AS DOUBLE
DIM T AS QMEMORYSTREAM
sh = -3: by = 200: sg = 0.5: lg = 70000: db = 2.5
T.Write(sh): T.Write(by): T.Write(sg): T.Write(lg): T.Write(db)
PRINT "typed bytes:"; T.Size
T.Position = 0
sh = 0: by = 0: sg = 0: lg = 0: db = 0
T.Read(sh): T.Read(by): T.Read(sg): T.Read(lg): T.Read(db)
PRINT sh; by; sg; lg; db
