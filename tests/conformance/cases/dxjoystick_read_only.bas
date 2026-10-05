' QDXJOYSTICK's state can't be set: RC.EXE's "J.ISLEFT is a read-only
' value." (checked against RC.EXE; RapidR's additions the same).
DIM J AS QDXJOYSTICK
J.Update
J.IsLeft = 1
J.X = 5
