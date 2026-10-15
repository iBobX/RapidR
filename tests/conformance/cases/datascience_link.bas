' Linked data (rapidr_value::datascience::link, I7): an RCSV as a data
' source, an RDataFrame taking its rows (Source), an RDataSource's current
' row, RJSON records as a source — the same on native builds, the
' interpreter and the web.
DIM NL AS STRING
DIM Q AS STRING
NL = CHR$(10)
Q = CHR$(34)
CREATE People AS RCSV
    Text = "name,age,city" + NL + "bob,30,NYC" + NL + "amy,25,LA" + NL + "Lee,41,NYC" + NL
END CREATE
CREATE DF AS RDATAFRAME
    Source = People
END CREATE
CREATE Nav AS RDATASOURCE
    DataSet = DF
END CREATE
PRINT "rows: "; DF.RowCount; " "; People.RowCount; " "; Nav.RecordCount; " cols: "; People.Columns; " "; People.ColCount
PRINT "state: "; Nav.State; " row "; Nav.Row; " eof "; Nav.EOF; " bof "; Nav.BOF

' --- walking the rows ---
Nav.First
WHILE NOT Nav.EOF
    PRINT Nav.Row; ": "; Nav.Field("name"); " "; Nav.Field("age")
    Nav.Next
WEND
PRINT "eof "; Nav.EOF; " row "; Nav.Row
Nav.Last
PRINT "last "; Nav.Field("name"); " eof "; Nav.EOF
Nav.Prior
PRINT "prior "; Nav.Field("name"); " bof "; Nav.BOF
Nav.MoveBy -5
PRINT "moveby "; Nav.Row; " bof "; Nav.BOF

' --- the frame changes: the current row stays inside ---
Nav.Last
DF.Filter "age", ">", 26
PRINT "filtered: "; Nav.RecordCount; " "; Nav.Row; " "; Nav.Field("name")
DF.Refresh
PRINT "refreshed: "; DF.RowCount

' --- editing the current row ---
Nav.Row = 1
Nav.SetField "city", "Paris"
PRINT DF.Cell(1, "city"); " "; DF.Cell(1, "name")

' --- the source changes: the frame follows ---
People.Text = "name,age" + NL + "zed,9" + NL
PRINT "after source change: "; DF.RowCount; " "; Nav.Field("name"); " "; DF.Columns

' --- RJSON's records as a source ---
CREATE J AS RJSON
END CREATE
J.Parse "[{" + Q + "a" + Q + ":1," + Q + "b" + Q + ":" + Q + "x" + Q + "},{" + Q + "a" + Q + ":2," + Q + "b" + Q + ":" + Q + "y" + Q + "}]"
DF.Source = J
PRINT "json: "; DF.RowCount; " "; DF.Columns; " "; DF.Cell(1, "b"); " "; DF.dtypes
J.Parse "[{" + Q + "a" + Q + ":7}]"
PRINT "json changed: "; DF.RowCount; " "; DF.Columns; " "; DF.Cell(0, "a")

' --- delimiters and headers ---
CREATE Semi AS RCSV
    Delimiter = ";"
    Text = "x;y" + NL + "1;2" + NL + "3;4" + NL
END CREATE
PRINT "semi: "; Semi.Columns; " "; Semi.RowCount; " "; Semi.Delimiter
CREATE Bare AS RCSV
    HasHeader = 0
    Text = "1,2" + NL + "3,4" + NL
END CREATE
PRINT "no header: "; Bare.Columns; " "; Bare.RowCount; " "; Bare.HasHeader

' --- a data source with no data set ---
CREATE Empty AS RDATASOURCE
END CREATE
PRINT "empty: "; Empty.State; " "; Empty.RecordCount; " "; Empty.EOF; " ["; Empty.Field("x"); "]"
