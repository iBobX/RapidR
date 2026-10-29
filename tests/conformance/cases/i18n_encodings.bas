' A catalog is read as UTF-8 (a byte-order mark dropped) or, when it is not
' UTF-8, as Windows-1252, the ANSI text older programs and an earlier Studio
' wrote: no character is lost on the way in, none becomes U+FFFD.
' (A UTF-8 file with a non-ASCII character prints Unicode: é — and 中文.)
Application.Language = "ansi"
PRINT TR$("Playing")
PRINT TR$("Language")
Application.Language = "bom"
PRINT TR$("Playing")
' no such catalog: the program's own text
Application.Language = "xx-none"
PRINT TR$("Playing")
