' INPUT (RapidQ manual ch. 6.4): the prompt is printed as written, a whole
' line is read, and it's stored as text or a number for the variable: by
' its DIM type, else its suffix, else a number when the line is one.
DIM age AS INTEGER
DIM nick AS STRING
DIM arr(3)
INPUT "Name? ", n$
PRINT
PRINT "Hi "; n$
INPUT "Age? "; age
PRINT
PRINT age + 1
INPUT "Nick: ", nick
PRINT
PRINT "[" + nick + "]"
INPUT x
PRINT x * 2
INPUT "Item: ", arr(2)
PRINT
PRINT arr(2)
INPUT "Height: ", h#
PRINT
PRINT h# * 2
