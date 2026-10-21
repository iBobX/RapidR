' TRN$ (RapidR's): the plural form each language's Plural-Forms picks,
' {n} the count; English's rule for the program's own text.
' (A UTF-8 file with a non-ASCII character, “like this”, prints Unicode;
' a .bas file of plain ASCII prints in RapidQ's console code page, CP437.)
Application.Language = "es"
FOR i = 0 TO 2
    PRINT TRN$("{n} file", "{n} files", i)
NEXT
PRINT TRN$("{n} file in {folder}", "{n} files in {folder}", 5, "folder", "Docs")

' Chinese: one form for every count
Application.Language = "zh-Hans"
PRINT TRN$("{n} file", "{n} files", 1)
PRINT TRN$("{n} file", "{n} files", 7)

' Polish: three forms
Application.Language = "pl"
DIM counts(5) AS INTEGER
counts(1) = 1: counts(2) = 2: counts(3) = 5: counts(4) = 12: counts(5) = 22
FOR i = 1 TO 5
    PRINT TRN$("{n} file", "{n} files", counts(i))
NEXT

' no catalog: English's rule
Application.Language = "pt-BR"
PRINT TRN$("{n} file", "{n} files", 1)
PRINT TRN$("{n} file", "{n} files", 3)
PRINT TRN$("{n} item of {total}", "{n} items of {total}", 2, "total", 9)
