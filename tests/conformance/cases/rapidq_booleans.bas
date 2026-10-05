' A component's Boolean property reads 1 when true, as RapidQ's (Delphi's)
' do — RAPIDQ.INC's True is 1 and programs test `.Checked = True` — while a
' comparison is -1. The .expected is RC.EXE's output
' (docs/rapidq-ground-truth.md). Checked keeps 0 / 1 whatever is stored;
' Enabled keeps the number stored. FILEEXISTS and DIREXISTS give 1.
DIM c AS QCHECKBOX
c.Checked = -1
PRINT "a "; c.Checked;
c.Checked = 5
PRINT " "; c.Checked;
c.Checked = 0
PRINT " "; c.Checked
DIM f AS QFORM
PRINT "b "; f.Visible; " "; f.Enabled; " "; c.Enabled; " "; c.Visible
DIM e AS QEDIT
PRINT "c "; e.ReadOnly; " "; e.Enabled
PRINT "d "; (f.Enabled = 1); " "; (f.Enabled = -1); " "; NOT f.Enabled
f.Enabled = -1
PRINT "e "; f.Enabled
PRINT "f "; FILEEXISTS("rapidq_booleans.nope"); " "; DIREXISTS("."); " "; (1 = 1)
