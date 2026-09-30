' RapidQ's dotted TYPE fields (CommCtrl.inc, qdataBaseSQL.inc): a record inside the record.
TYPE NMHDR2
  hdr.hwndFrom AS LONG
  hdr.code AS LONG
  Table.Name(3) AS STRING
  Plain AS INTEGER
END TYPE
DIM N AS NMHDR2
N.hdr.hwndFrom = 5
N.hdr.code = 7
N.Table.Name(2) = "two"
N.Plain = 1
PRINT N.hdr.hwndFrom + N.hdr.code; " "; N.Table.Name(2); " "; N.Plain
TYPE Deep
  a.b.c AS INTEGER
  a.b.d AS STRING
  a.e AS DOUBLE
END TYPE
DIM D AS Deep
D.a.b.c = 3
D.a.b.d = "x"
D.a.e = 1.5
PRINT D.a.b.c; D.a.b.d; D.a.e
DIM M(2) AS NMHDR2
M(1).hdr.code = 9
PRINT M(1).hdr.code; M(0).hdr.code
