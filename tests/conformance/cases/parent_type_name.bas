' A component type's name used as a value is the object of that type created
' last (RapidQ's examples: a toolbar or main menu in an include file sets
' `Parent = QFORM` to land on the program's form); "" while there's none.
' Inside a TYPE … EXTENDS constructor the type's own name is the instance.
PRINT "[" + QFORM + "]"
TYPE QCoolForm EXTENDS QFORM
  B AS QBUTTON
  CONSTRUCTOR
    Caption = "cool"
    B.Parent = QCoolForm
  END CONSTRUCTOR
END TYPE
PRINT "[" + QCoolForm + "]"
CREATE First AS QFORM
  Caption = "first"
END CREATE
CREATE Second AS QFORM
  Caption = "second"
END CREATE
CREATE Panel1 AS QPANEL
  Parent = QFORM
END CREATE
PRINT Panel1.Parent
DIM Btn AS QBUTTON
Btn.Parent = QForm
PRINT Btn.Parent
PRINT QPANEL; " "; QBUTTON
DIM Cool AS QCoolForm
PRINT Cool.B.Parent
PRINT QFORM
CREATE Third AS QFORM
END CREATE
Btn.Parent = QFORM
PRINT Btn.Parent; " "; QCoolForm
