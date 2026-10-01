' Templates (manual 10.8): TYPE Holder<DataType, Size> made for each set of arguments used.
TYPE Holder<DataType, Size> EXTENDS QOBJECT
  N AS DataType
  Items(Size) AS DataType
  SUB Show
    PRINT This.N; UBOUND(This.Items)
  END SUB
END TYPE
DIM A AS Holder<INTEGER, 3>
DIM B AS Holder<STRING, 5>
A.N = 7.6
B.N = "hi"
A.Show
B.Show
FUNCTION Total(H AS Holder<INTEGER, 3>) AS INTEGER
  Total = H.N * 2
END FUNCTION
PRINT Total(A)
