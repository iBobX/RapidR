' QFILELISTBOX: a list box of a directory's files — Directory, Mask (several
' with ;), AddFileTypes / DelFileTypes (directories in brackets), FileName,
' OnChange when the directory changes.
DECLARE SUB Changed
DIM Log AS STRING
DIM Files AS QFILELISTBOX
Files.OnChange = Changed
Files.Directory = CURDIR$ + "/tests/conformance/cases/resource_files"
PRINT Files.ItemCount; " "; Files.Item(0); " "; Files.Item(1)
Files.Mask = "*.TXT"
PRINT Files.ItemCount; " "; Files.Item(0)
Files.ItemIndex = 0
PRINT RIGHT$(Files.FileName, 25)
Files.Mask = "*.bin;*.txt"
Files.AddFileTypes(4)
Files.DelFileTypes(6)
PRINT Files.ItemCount; " "; Files.Item(0)
Files.DelFileTypes(4)
Files.AddFileTypes(6)
PRINT Files.ItemCount; " "; Files.Item(0) - "." ; " "; Log
SUB Changed
  Log = Log + "changed;"
END SUB
