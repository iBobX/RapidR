' RDATAFRAME on the one data-science model (rapidr_value::datascience):
' every member gives the same result in native builds, the interpreter and
' the browser — cells as plain text, a frame printed once as a table.
DIM df AS RDataFrame
DIM other AS RDataFrame
DIM NL AS STRING
DIM Q AS STRING
NL = CHR$(10)
Q = CHR$(34)

SUB LoadPeople
    df.LoadFromCSV "name,age,city,score" + NL + "bob,30,NYC,1.5" + NL + "amy,25,LA,2" + NL + Q + "Lee, Jo" + Q + ",41,NYC,3.25" + NL + "cat,,LA,0.5" + NL
END SUB

' --- I/O and shape ---
LoadPeople
PRINT df.RowCount; "x"; df.ColCount; " "; df.Columns; " "; df.Shape; " "; df.Empty
PRINT df.dtypes
PRINT df.ToString
df.Print
df.info

' --- Selection / Indexing ---
PRINT df.Cell(0, 0); "|"; df.Cell(2, 0); "|"; df.Cell(3, 1); "|"; df.Cell(1, "city"); "|"; df.CellByName(1, "CITY"); "|"; df.at(2, "age")
df.SetCell 3, 1, "28"
PRINT df.Cell(3, 1)
df.head 2
PRINT df.Columns; " "; df.RowCount; " "; df.Cell(1, 0)
LoadPeople
df.tail 1
PRINT df.Cell(0, 0)
LoadPeople
df.iloc 1, 3
PRINT df.Cell(0, 0); " "; df.Cell(1, 0); " "; df.rows
LoadPeople
df.select "city,name"
PRINT df.Columns; " "; df.Cell(0, 0)

' --- Sorting ---
LoadPeople
df.sort "age", 0
PRINT df.Cell(0, 0); " "; df.Cell(1, 0); " "; df.Cell(2, 0); " "; df.Cell(3, 0)
df.sort_values "city,name"
PRINT df.Cell(0, 0); " "; df.Cell(1, 0); " "; df.Cell(2, 0); " "; df.Cell(3, 0)

' --- Filtering ---
LoadPeople
df.filter "age", ">", 26
PRINT df.RowCount; " "; df.Cell(0, 0); " "; df.Cell(1, 0)
LoadPeople
df.filter "city", "=", "LA"
PRINT df.RowCount; " "; df.Cell(0, 0)
LoadPeople
df.filter "name", "contains", "o"
PRINT df.RowCount
LoadPeople
df.query "score <= 1.5"
PRINT df.RowCount; " "; df.Cell(1, 0)

' --- Grouping / Aggregation ---
LoadPeople
df.groupby "city", "mean"
df.Print
LoadPeople
df.groupby "city", "count"
PRINT df.Cell(0, 0); " "; df.Cell(0, 2); " "; df.Cell(1, 2)
LoadPeople
df.group_by "city", "sum"
PRINT df.Cell(1, 2); " "; df.Cell(1, 3)

' --- Column operations ---
LoadPeople
df.drop "score"
df.rename "city", "town"
df.addcolumn "id", "1,2,3,4"
PRINT df.Columns; " "; df.Cell(3, 3)

' --- Missing data ---
LoadPeople
df.dropna
PRINT df.RowCount
LoadPeople
df.fillna "0"
PRINT df.Cell(3, 1)

' --- Statistics ---
LoadPeople
PRINT df.nunique("city"); " "; df.corr("age", "score")
df.value_counts "city"
df.Print
LoadPeople
df.describe
df.Print

' --- Sampling ---
LoadPeople
df.nlargest "score", 2
PRINT df.Cell(0, 0); " "; df.Cell(1, 0)
LoadPeople
df.nsmallest "score", 1
PRINT df.Cell(0, 0)
LoadPeople
df.sample 3
PRINT df.RowCount; " "; df.ColCount

' --- Merge / Join ---
other.create
other.addcolumn "city", "NYC,LA,SF"
other.addcolumn "state", "NY,CA,CA"
LoadPeople
df.merge "other", "city", "inner"
PRINT df.Columns; " "; df.RowCount; " "; df.Cell(1, 4)
LoadPeople
df.select "name,city"
df.merge "other", "city", "outer"
PRINT df.RowCount; " ["; df.Cell(4, 0); "] "; df.Cell(4, 1)
LoadPeople
df.concat "other"
PRINT df.RowCount; " "; df.ColCount; " "; df.Cell(4, 2); " "; df.Cell(4, 4)

' --- Transform ---
LoadPeople
df.apply "name", "upper"
df.apply "score", "round", 0
df.replace "city", "LA", "Los Angeles"
PRINT df.Cell(0, 0); " "; df.Cell(1, 2); " "; df.Cell(2, 3)
df.select "name,age"
df.head 2
df.transpose
df.Print

' --- Building a frame ---
df.create
df.addrow "x", "1"
df.addrow "y,2"
df.SetCell 2, 0, "z"
df.rename "column_1", "key"
PRINT df.Columns; " "; df.RowCount; " ["; df.Cell(2, 1); "]"
df.clear
PRINT df.RowCount; " "; df.ColCount; " "; df.Empty

' --- Files ---
LoadPeople
df.SaveToCSV "tests/conformance/.work/ds_people.csv"
df.clear
df.LoadFromCSV "tests/conformance/.work/ds_people.csv"
PRINT df.Cell(2, 0); " "; df.Cell(3, 1); " "; df.RowCount
df.SaveToJSON "tests/conformance/.work/ds_people.json"
df.clear
df.LoadFromJSON "tests/conformance/.work/ds_people.json"
PRINT df.Columns; " "; df.dtypes; " "; df.Cell(2, 3)
df.LoadFromJSON "[{" + Q + "a" + Q + ": 1, " + Q + "b" + Q + ": " + Q + "x" + Q + "}, {" + Q + "a" + Q + ": 2, " + Q + "b" + Q + ": null}]"
PRINT df.Columns; " "; df.Cell(1, 0); " ["; df.Cell(1, 1); "]"

' --- Populate a QSTRINGGRID ---
DIM Grid AS QSTRINGGRID
LoadPeople
df.ToGrid "Grid"
PRINT Grid.ColCount; " "; Grid.RowCount; " "; Grid.Cell(0, 0); " "; Grid.Cell(0, 3); " "; Grid.Cell(3, 2); " ["; Grid.Cell(1, 4); "]"
