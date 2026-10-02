' QREGISTRY: values of each kind written to a key of a per-user store and
' read back — ReadBinary's index starts at -1 (a RapidQ bug programs count
' on) — then paths, the default value, KeyItem, MoveKey, GetDataType /
' GetDataSize.
Const HKEY_CLASSES_ROOT = &H80000000
Const HKEY_CURRENT_USER = &H80000001
DIM Reg AS QRegistry
DefByte Blob(6) = {10, 20, 30, 40, 50, 60}
DefInt K
With Reg
   .RootKey = HKEY_CURRENT_USER
   ' (a key left by an earlier run goes first)
   .DeleteKey("RapidRProbe")
   .OpenKey("RapidRProbe", 1)
   .WriteInteger("Count", -42)
   .WriteString("Label", "hello reg")
   .WriteBinary("Bytes", Blob(), 6)
   .WriteFloat("Ratio", 0.5)
   .CloseKey
   .OpenKey("RapidRProbe", 0)
        Print "Count"; .ReadInteger("Count"); " Label "; .ReadString("Label")
        For K = 4 To -1 Step -1
            Print K; ":"; .ReadBinary("Bytes", K); " ";
        Next K
        Print
        Print "Ratio "; .ReadFloat("Ratio")
        Print "types"; .GetDataType("Label"); .GetDataType("Count"); .GetDataType("Bytes"); .GetDataType("Nope")
        Print "sizes"; .GetDataSize("Label"); .GetDataSize("Count"); .GetDataSize("Bytes"); .GetDataSize("Nope")
        Print "values"; .ValueItemCount; " "; .ValueItem(0); " "; .ValueExists("label"); .ValueExists("Nope")
        Print "path "; .CurrentPath; " open"; .CurrentKey <> 0
   .CloseKey
   Print "closed"; .CurrentKey; " ["; .CurrentPath; "]"
   .OpenKey("\RapidRProbe\Shell\Open\Command", 1)
   .WriteString("", "app.exe %1")
   .OpenKey("\\RapidRProbe\\Shell", 0)
   Print "sub "; .CurrentPath; .KeyItemCount; " "; .KeyItem(0); .HasSubKeys
   .CreateKey("Edit")
   Print "keys"; .KeyItemCount; " "; .KeyItem(0); ","; .KeyItem(1)
   Print "move"; .MoveKey("\RapidRProbe\Shell", "\RapidRProbe\Verbs", 1); .KeyExists("\RapidRProbe\Shell"); .KeyExists("\RapidRProbe\Verbs\Open\Command")
   .OpenKey("\RapidRProbe\Verbs\Open\Command", 0)
   Print "default ["; .ReadString(""); "]"
   .RenameValue("", "Cmd")
   Print "renamed ["; .ReadString("Cmd"); "]"; .ValueExists("")
   .CloseKey
   Print "deleted"; .DeleteKey("RapidRProbe"); .KeyExists("RapidRProbe")
   .RootKey = HKEY_CLASSES_ROOT
   Print "root"; .RootKey = HKEY_CLASSES_ROOT; .KeyExists("RapidRProbe")
End With
