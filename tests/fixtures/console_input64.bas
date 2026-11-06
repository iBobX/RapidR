' INPUT into INT64 / UINT64 on the web (tests/web_bundle_console.mjs): the
' digits typed are read whole.
DIM u AS UINT64
DIM i AS INT64
INPUT "A UINT64? ", u
INPUT "An INT64? ", i
PRINT "u = "; u; ", u - 1 = "; u - 1
PRINT "i = "; i; ", i + 1 = "; i + 1
