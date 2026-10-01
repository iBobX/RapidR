' QSTRINGLIST as RapidQ's manual has it (its own examples): AddList, Sort,
' Text, IndexOf, Parse, Build, Exchange; Duplicates in a sorted list.
DIM StringList1 AS QStringList
DIM StringList2 AS QStringList
StringList1.AddItems "Oranges","Apples"
StringList2.AddItems "Byte","Word","Integer"
StringList1.AddList(StringList2)
StringList1.Sort
PRINT StringList1.Text
PRINT StringList1.Item(3)
PRINT StringList1.IndexOf("Word")
PRINT StringList1.IndexOf("hi")
DIM StringList AS QSTRINGLIST
PRINT StringList.Parse("James:Brown:555-3454:House",":")
PRINT StringList.Build(0,StringList.ItemCount-1,",")
StringList.Exchange(0, 3)
PRINT StringList.Build(0, 3, "|")
DIM D AS QSTRINGLIST
D.Sorted = 1
D.AddItems "b", "a", "b"
PRINT D.ItemCount;
D.Duplicates = 1
D.AddItems "a"
PRINT D.ItemCount
