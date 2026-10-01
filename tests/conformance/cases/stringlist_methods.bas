' QSTRINGLIST: AddList, Sort, Text, Item, IndexOf, Parse, Build (a whole
' list and a range), Exchange; Duplicates in a sorted list.
DIM Fruit AS QStringList
DIM Veg AS QStringList
Fruit.AddItems "pear", "fig", "apple"
Veg.AddItems "kale", "leek"
Fruit.AddList(Veg)
Fruit.Sort
PRINT Fruit.Text
PRINT Fruit.Item(1)
PRINT Fruit.IndexOf("leek")
PRINT Fruit.IndexOf("plum")
DIM Ways AS QSTRINGLIST
PRINT Ways.Parse("north|east|south|west|up", "|")
PRINT Ways.Build(0, Ways.ItemCount - 1, "+")
Ways.Exchange(1, 4)
PRINT Ways.Build(1, 3, "/")
DIM D AS QSTRINGLIST
D.Sorted = 1
D.AddItems "b", "a", "b"
PRINT D.ItemCount;
D.Duplicates = 1
D.AddItems "a"
PRINT D.ItemCount
