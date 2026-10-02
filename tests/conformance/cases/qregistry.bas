' QREGISTRY as RapidQ's manual has it (its example 2, by Jacques Philippe):
' keys and values in a per-user store; ReadBinary's index starts at -1 (a
' RapidQ bug programs count on); then paths, the default value, KeyItem,
' MoveKey, GetDataType / GetDataSize.
Const HKEY_CLASSES_ROOT = &H80000000
Const HKEY_CURRENT_USER = &H80000001
DIM Registry AS QRegistry
With Registry
   .RootKey = HKEY_CURRENT_USER
   .DeleteKey ("TestToDelete")
   .OpenKey("TestToDelete", 1)
        .WriteInteger("MyInteger", 1234567890)
        .WriteString ("Mystring", "1234567890")
        DefByte MyBinary(10) = {1,2,3,4,5,6,7,8,9,0}
        .WriteBinary ("MyBinary", MyBinary(), 10)
        .WriteFloat ("MyFloat", 3.25)
   .CloseKey
   .OpenKey("TestToDelete", 1)
        Print "MyInteger=";.ReadInteger("MyInteger")
        Print " Mystring=";.ReadString ("Mystring")
        DefInt N
        For N = -1 To 8
            Print "MyBinary(";N;")=";.ReadBinary ("MyBinary", N)
        Next N
        Print "MyFloat="; .ReadFloat("MyFloat")
        Print "types"; .GetDataType("Mystring"); .GetDataType("MyInteger"); .GetDataType("MyBinary"); .GetDataType("Nope")
        Print "sizes"; .GetDataSize("Mystring"); .GetDataSize("MyInteger"); .GetDataSize("MyBinary"); .GetDataSize("Nope")
        Print "values"; .ValueItemCount; " "; .ValueItem(0); " "; .ValueExists("mystring"); .ValueExists("Nope")
        Print "path "; .CurrentPath; " open"; .CurrentKey <> 0
   .CloseKey
   Print "closed"; .CurrentKey; " ["; .CurrentPath; "]"
   .OpenKey("\TestToDelete\Shell\Open\Command", 1)
   .WriteString("", "app.exe %1")
   .OpenKey("\\TestToDelete\\Shell", 0)
   Print "sub "; .CurrentPath; .KeyItemCount; " "; .KeyItem(0); .HasSubKeys
   .CreateKey("Edit")
   Print "keys"; .KeyItemCount; " "; .KeyItem(0); ","; .KeyItem(1)
   Print "move"; .MoveKey("\TestToDelete\Shell", "\TestToDelete\Verbs", 1); .KeyExists("\TestToDelete\Shell"); .KeyExists("\TestToDelete\Verbs\Open\Command")
   .OpenKey("\TestToDelete\Verbs\Open\Command", 0)
   Print "default ["; .ReadString(""); "]"
   .RenameValue("", "Cmd")
   Print "renamed ["; .ReadString("Cmd"); "]"; .ValueExists("")
   .CloseKey
   Print "deleted"; .DeleteKey("TestToDelete"); .KeyExists("TestToDelete")
   .RootKey = HKEY_CLASSES_ROOT
   Print "root"; .RootKey = HKEY_CLASSES_ROOT; .KeyExists("TestToDelete")
End With
