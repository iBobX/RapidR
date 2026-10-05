' QREGISTRY on Windows' own registry: registry_check.ps1 runs this with and
' without RAPIDR_REGISTRY - the per-user store and Windows' registry must
' print the same up to the "machine" line. Everything it writes is under
' HKEY_CURRENT_USER\Software\RapidR-Test (the script seeds Seed there with
' kinds of value QREGISTRY can't write, and deletes the key afterwards).
Const HKEY_CLASSES_ROOT = &H80000000
Const HKEY_CURRENT_USER = &H80000001
Const HKEY_LOCAL_MACHINE = &H80000002
DIM R AS QRegistry
DIM Name AS STRING
DefInt I, N
DefDbl T

R.RootKey = HKEY_CURRENT_USER
Print "start"; R.KeyExists("\Software\RapidR-Test\Work"); R.KeyExists("\Software\RapidR-Test\Seed"); R.KeyExists("")
Print "open"; R.OpenKey("\Software\RapidR-Test\Work", 1); " "; R.CurrentPath; " "; R.CurrentKey <> 0
R.WriteString("Str", "hello, world")
R.WriteString("", "the default")
R.WriteString("Lines", "one" + Chr$(13) + Chr$(10) + "two")
R.WriteString("Empty", "")
R.WriteInteger("Int", -5)
R.WriteFloat("Float", 3.25)
DefByte B(4) = {7, 0, 255, 9, 1}
R.WriteBinary("Bin", B(), 4)
R.WriteBinary("FromText", "abc", 99)
Print "values"; R.ValueItemCount
For I = 0 To R.ValueItemCount - 1
  Name = R.ValueItem(I)
  Print "  ["; Name; "] type"; R.GetDataType(Name); " size"; R.GetDataSize(Name)
Next I
Print "read ["; R.ReadString("STR"); "] ["; R.ReadString(""); "]"; R.ReadInteger("int"); R.ReadFloat("Float"); Len(R.ReadString("Lines")); " ["; R.ReadString("Int"); "]"
Print "binary";
For I = -2 To 4
  Print R.ReadBinary("Bin", I);
Next I
Print R.ReadBinary("FromText", -1); R.ReadBinary("Str", -1); R.ReadBinary("Int", -1)
Print "missing"; R.GetDataType("Nope"); R.GetDataSize("Nope"); R.ValueExists("Nope"); R.ReadInteger("Nope"); " ["; R.ReadString("Nope"); "]"
R.WriteInteger("STR", 1)
Print "rewritten "; R.ValueItem(0); R.GetDataType("Str"); R.ReadInteger("Str")
R.RenameValue("Str", "Renamed")
Print "renamed"; R.ValueExists("Str"); R.ReadInteger("Renamed"); " "; R.ValueItem(R.ValueItemCount - 1)
R.RenameValue("Renamed", "Int")
Print "not over"; R.ReadInteger("Renamed"); R.ReadInteger("Int")
Print "deleted"; R.DeleteValue("renamed"); R.DeleteValue("renamed"); R.ValueItemCount

Print "keys"; R.CreateKey("b"); R.CreateKey("A2"); R.CreateKey("c\deep"); R.HasSubKeys; R.KeyItemCount
For I = 0 To R.KeyItemCount
  Print "  ["; R.KeyItem(I); "]"
Next I
Print "delete b"; R.DeleteKey("B"); " "; R.KeyItem(1); R.KeyItemCount; R.KeyExists("C\Deep"); R.KeyExists("b")

' kinds QREGISTRY can't write (REG_MULTI_SZ, REG_QWORD, REG_EXPAND_SZ)
Print "seed"; R.OpenKey("\Software\RapidR-Test\Seed", 0); R.ValueItemCount
Print "  types"; R.GetDataType("Multi"); R.GetDataType("Qword"); R.GetDataType("Path")
Print "  sizes"; R.GetDataSize("Multi"); R.GetDataSize("Qword"); R.GetDataSize("Path")
Print "  ["; R.ReadString("Path"); "] ["; R.ReadString("Multi"); "]"; R.ReadBinary("Qword", -1); R.ReadInteger("Qword")
R.CloseKey
Print "closed"; R.CurrentKey; " ["; R.CurrentPath; "]"
Print "move"; R.MoveKey("\Software\RapidR-Test\Seed", "\Software\RapidR-Test\Work\Seed", 1); R.KeyExists("\Software\RapidR-Test\Seed")
Print "  into itself"; R.MoveKey("\Software\RapidR-Test\Work", "\Software\RapidR-Test\work\Inside", 1)
Print "  nothing"; R.MoveKey("\Software\RapidR-Test\Nope", "\Software\RapidR-Test\Elsewhere", 1); R.KeyExists("\Software\RapidR-Test\Elsewhere")
Print "  copy"; R.MoveKey("\Software\RapidR-Test\Work", "\Software\RapidR-Test\Final", 0); R.KeyExists("\Software\RapidR-Test\Work")
R.OpenKey("\Software\RapidR-Test\Final\Seed", 0)
Print "  kept"; R.GetDataType("Multi"); R.GetDataSize("Multi"); R.GetDataSize("Qword"); " "; R.ReadString("Path")

' the open key deleted meanwhile: reads find nothing, a write makes it again
DIM Other AS QRegistry
R.OpenKey("\Software\RapidR-Test\Work", 0)
Print "gone"; Other.DeleteKey("\Software\RapidR-Test\Work\Seed"); Other.DeleteKey("\Software\RapidR-Test\Work\c")
Print "  "; R.KeyItemCount; R.ValueItemCount; R.ReadString("")
Print "deleted under"; Other.DeleteKey("\Software\RapidR-Test\Work"); R.ValueItemCount; R.GetDataSize("Lines")
R.WriteInteger("Again", 3)
Print "again"; Other.KeyExists("\Software\RapidR-Test\Work"); R.ValueItemCount
Print "delete above"; R.DeleteKey("\Software\RapidR-Test\Work"); R.CurrentKey; " ["; R.CurrentPath; "]"; R.KeyExists("\Software\RapidR-Test\Work")

' left for reg query: Final (Work's copy) and these
R.OpenKey("\Software\RapidR-Test\Final", 1)
R.WriteString("Text", "x=1;y=2")
R.WriteInteger("Number", 305419896)
Print "final"; R.ValueItemCount; R.KeyItemCount
R.CloseKey

' the machine's own keys: only in Windows' registry
R.RootKey = HKEY_LOCAL_MACHINE
Print "machine"
IF R.KeyExists("\SOFTWARE\Microsoft\Windows NT\CurrentVersion") THEN
  Print "  open"; R.OpenKey("\SOFTWARE\Microsoft\Windows NT\CurrentVersion", 0); R.GetDataType("ProductName"); " "; R.ReadString("ProductName")
  Print "  open to create"; R.OpenKey("\SOFTWARE\Microsoft\Windows NT\CurrentVersion", 1); " "; R.CurrentPath
  R.CloseKey
  Print "  no writes"; R.CreateKey("\SOFTWARE\RapidR-Test"); R.OpenKey("\SOFTWARE\RapidR-Test", 1); R.KeyExists("\SOFTWARE\RapidR-Test"); R.CurrentKey
  R.RootKey = HKEY_CURRENT_USER
  R.OpenKey("\Control Panel\Colors", 0)
  Print "  colors "; R.ReadString("ButtonFace")
  R.RootKey = HKEY_CLASSES_ROOT
  Print "  classes"; R.OpenKey("\.txt", 0); R.GetDataType(""); " "; R.ReadString("")
  R.CloseKey
  T = TIMER
  N = 0
  For I = 0 To R.KeyItemCount - 1
    IF LEFT$(R.KeyItem(I), 1) = "." THEN N = N + 1
  Next I
  Print "  walk"; R.KeyItemCount; " keys,"; N; " extensions, in"; INT(TIMER - T); " s"
ELSE
  Print "  none (the per-user store)"
END IF
