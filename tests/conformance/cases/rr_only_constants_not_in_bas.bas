' ctText, ctImage and ctState (an RListView's OnChange kinds) are RapidR's own,
' built into a .rr program only (rr_builtin_constants.rr): in a RapidQ program
' (.bas) they are names nobody declared — variables that are 0, as RapidQ reads them.
PRINT ctText; " "; ctImage; " "; ctState
