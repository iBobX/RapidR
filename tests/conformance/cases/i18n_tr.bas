' TR$ and Application.Language (RapidR's): catalogs in lang/ beside the
' program (the runner copies cases/lang there), fallback es-AR -> es -> the
' program's own text, contexts, placeholders.
' (A UTF-8 file with a non-ASCII character, “like this”, prints Unicode;
' a .bas file of plain ASCII prints in RapidQ's console code page, CP437.)
Application.Language = "es"
PRINT Application.Language
PRINT TR$("Save file")
PRINT TR$("Open", "menu")
PRINT TR$("Open")
PRINT TR$("Hello, {name}!", "name", "Ana")
PRINT TR$("A long text on two lines")
PRINT TR$("Draft")
PRINT TR$("Not in the file")
name$ = "Luis"
PRINT TR$("Hello, {name}!", "NAME", name$)
PRINT TR$("Use {{braces}} and {missing}", "other", 1)

' no catalog for Portuguese: the program's own text
Application.Language = "pt"
PRINT TR$("Save file")
PRINT TR$("Hello, {name}!", "name", 42)

' Argentina's Spanish: its own file first, then es.po
Application.Language = "es_ar"
PRINT Application.Language
PRINT TR$("Open", "menu")
PRINT TR$("Save file")

' Chinese: by script, and by region alone
Application.Language = "zh-Hans"
PRINT TR$("Save file")
Application.Language = "zh_CN"
PRINT Application.Language
PRINT TR$("Save file")
