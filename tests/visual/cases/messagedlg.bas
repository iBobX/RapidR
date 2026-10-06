' MESSAGEDLG with a warning icon and Yes / No / Cancel buttons.
$INCLUDE "RAPIDQ.INC"
DIM r AS INTEGER
r = MESSAGEDLG("Save the changes to Untitled?", mtWarning, mbYes OR mbNo OR mbCancel, 0)
