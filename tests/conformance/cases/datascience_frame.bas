' RDATAFRAME (one implementation on every runtime: rapidr-frame, polars —
' linked on the desktop, a module of its own on the web): CSV text and
' files, cells, filter / sort / group-by / describe / value counts, a
' join, building a frame row by row, the printed table.
DIM df AS RDATAFRAME, dept AS RDATAFRAME, t AS RDATAFRAME
NL$ = CHR$(10)
csv$ = "Name,Department,Salary,Years" + NL$ + "Alice,Engineering,85000,5" + NL$ + "Bob,Sales,62000,3" + NL$
csv$ = csv$ + "Charlie,Engineering,92000,8" + NL$ + "Diana,Sales,58000,2" + NL$ + "Fiona,Marketing,71000,4" + NL$
df.LoadFromCsv(csv$)
PRINT df.RowCount; "x"; df.ColCount; " "; df.Columns; " "; df.Shape
PRINT "cell(0,0) "; df.Cell(0, 0); " salary of row 2: "; df.CellByName(2, "salary")
PRINT "dtypes: "; df.DTypes
df.Filter("Salary", ">", 60000)
df.Sort("Salary", 0)
PRINT "over 60000, highest first: "; df.Column("Name")
df.LoadFromCsv(csv$)
df.Filter("department", "=", "Sales")
PRINT "sales: "; df.Column("Name"); " empty "; df.Empty
df.LoadFromCsv(csv$)
df.Filter("Name", "contains", "an")
PRINT "names with 'an': "; df.Column("Name")
df.LoadFromCsv(csv$)
df.SaveToCsv("tests/conformance/.work/ds_staff.csv")
t.LoadFromCsv("tests/conformance/.work/ds_staff.csv")
PRINT "saved and read back: "; t.Shape; " "; t.Cell(4, 0)
df.Select("Department,Salary")
df.GroupBy("Department", "mean")
PRINT "mean salary by department:"
PRINT df.ToString
df.LoadFromCsv(csv$)
df.Describe
PRINT "describe salary: mean "; df.CellByName(1, "Salary"); " std "; df.CellByName(2, "Salary"); " median "; df.CellByName(5, "Salary")
df.LoadFromCsv(csv$)
df.Value_Counts("Department")
PRINT "counts: "; df.Column("Department"); " = "; df.Column("count")
dept.LoadFromCsv("Department,Floor" + NL$ + "Engineering,3" + NL$ + "Sales,1" + NL$)
df.LoadFromCsv(csv$)
df.Merge("dept", "Department", "left")
PRINT "joined: "; df.Columns; " floors "; df.Column("Floor")
t.Create
t.AddColumn("x", "1,2,3")
t.AddColumn("label", "a,b,c")
t.AddRow(4, "d")
t.SetCell(0, 1, "first")
PRINT "built: "; t.Shape; " "; t.Column("label"); " sum of x "; t.Column("x")
t.Info
PRINT "nunique "; t.NUnique("x"); " corr "; FORMAT$("%.3f", t.Corr("x", "x"))
t.Head(2)
t.Show
