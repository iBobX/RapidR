' WITH on a field of This in a TYPE's own code (`WITH TF.Bar`): its
' members are the field's object's (RC.EXE's output; RapidQ's
' forms/newform/newform.bas draws its title bar so — RapidR set the form's
' Width instead, and the form shrank to nothing).
$APPTYPE CONSOLE
TYPE TF EXTENDS QOBJECT
  Bar AS QSTRINGLIST
  W AS INTEGER
  SUB Fix
    WITH TF.Bar
      .Sorted = 1
      .AddItems "b", "a"
    END WITH
  END SUB
END TYPE
DIM F AS TF
F.W = 5
F.Fix
PRINT F.Bar.Sorted; " "; F.Bar.Item(0); " "; F.W
