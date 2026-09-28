' QSTRINGLIST: Add, Insert, Delete, Item, Count, IndexOf, Sort, Text, Clear.
DIM L AS QSTRINGLIST
L.Add("pear")
L.Add("apple")
L.Add("fig")
PRINT L.Count; " "; L.Item(0); " "; L.Item(2)
L.Insert(1, "kiwi")
PRINT L.Count; " "; L.Item(1); " "; L.Item(2)
PRINT L.IndexOf("fig"); " "; L.IndexOf("nope")
L.Delete(0)
PRINT L.Count; " "; L.Item(0)
L.Item(0) = "KIWI"
L.Sort
PRINT L.Item(0); " "; L.Item(1); " "; L.Item(2)
PRINT LEN(L.Text)
PRINT "[" + L.Item(9) + "]"
L.Clear
PRINT L.Count
