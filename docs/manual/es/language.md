<!-- translated-from: language.md 734f79e172fd3b75 -->
# El lenguaje

El lenguaje de RapidR es un BASIC moderno, y los programas anteriores se
ejecutan tal como están. Este capítulo es un resumen de trabajo, con las reglas
donde RapidR se diferencia de otros BASIC —esas son las que sorprenden a la
gente—. Las compilaciones nativas, el intérprete y la web dan la misma salida
para cada ejemplo de aquí.

RapidR también acepta algo de sintaxis que el lenguaje clásico no tiene (el
`#If` de VB, `ON ERROR`, `i++`, …): mira
[Diferencias y extensiones](differences.md).

## Archivos de código fuente

- Las extensiones `.bas` y `.rr` son el mismo lenguaje. Las extensiones
  anteriores también se aceptan: `.rqw`, `.rqb` y `.rq` (se ejecutan, se
  compilan, se abren en RapidR Studio y se incluyen con `$INCLUDE` como un
  `.bas`). Un archivo se lee como UTF-8; uno que no es UTF-8 válido se lee como
  Windows-1252 (ANSI), como suele ser el caso de los programas anteriores.
- **Las mayúsculas y minúsculas no importan** en las palabras clave, los
  nombres, los componentes y las funciones integradas.
- `:` separa instrucciones en una línea; `_` al final de una línea la continúa
  (una cadena de texto también puede continuar).
- Comentarios: `'` o `REM`.
- `?` es la forma abreviada de `PRINT`.
- Etiquetas: `Name:` en una línea propia, o un número de línea.
- Una primera línea `#!/usr/bin/env rapidr` convierte el archivo en un script en
  macOS y Linux (`chmod +x prog.bas; ./prog.bas`); la línea se ignora.

## Clases de programa: `$APPTYPE`

`$APPTYPE CONSOLE`, `$APPTYPE GUI`, `$APPTYPE CGI` (los clásicos) y `$APPTYPE
WEB` (el de RapidR). Sin ninguno, un programa que crea componentes es un
programa de interfaz gráfica (GUI) y cualquier otro es un programa de consola.
La clase decide, por ejemplo, si un ejecutable de Windows abre una ventana de
consola, y si un programa al que se le hace doble clic en macOS o Linux obtiene
una terminal.

## Variables y tipos

| Tipo | Sufijo | Contiene |
|---|---|---|
| `BYTE` | `?` | 0 … 255 (guardar un valor mayor da la vuelta: `b? = 300` contiene 44) |
| `WORD` | `??` | 0 … 65 535 |
| `SHORT` | `%` | -32 768 … 32 767 (`q% = 40000` contiene -25 536) |
| `INTEGER`, `LONG` | `&` | 32 bits con signo |
| `DWORD` | `???` | 32 bits (con signo) |
| `SINGLE` | `!` | punto flotante de 32 bits |
| `DOUBLE` | `#` | punto flotante de 64 bits |
| `STRING` | `$` | texto; `STRING * n` siempre tiene n caracteres |
| `VARIANT` | | lo que se guarde |

```basic
DIM count AS INTEGER, title AS STRING
DIM x AS INTEGER = 5
DIM code AS STRING * 3        ' guardar "abcdef" da "abc"; "a" da "a  "
DIM (a, b, c)(5) AS LONG      ' tres arreglos
DIM m                         ' sin AS: un DOUBLE
DIM p, q AS LONG              ' solo q es un LONG; p es un VARIANT
CONST MaxItems = 3
DEFINT k(1 TO 3) = {7, 8, 9}  ' DEFINT, DEFSTR, DEFLNG, DEFDBL, …
```

- No hace falta declarar las variables; un nombre no declarado es un `VARIANT`.
  Un nombre con sufijo (`n%`, `s$`) tiene el tipo del sufijo.
- **Guardar un valor en una variable entera lo trunca** (`i = 2.7` da 2); más
  allá de 32 bits da -2 147 483 648. En cambio, un parámetro `BYVAL` redondea
  los casos intermedios al entero par.
- Dentro de una `SUB` o una `FUNCTION`, `DIM` crea una variable local; `STATIC`
  conserva su valor entre llamadas. Una variable no declarada que una `SUB` usa
  antes que el programa principal es propia de la `SUB` (se conserva entre
  llamadas): la regla de una sola pasada.
- `TRUE` y `FALSE`: una comparación da -1 o 0 (`PRINT 1 = 1` imprime -1). En un
  programa de RapidR (`.rr`), `True` es -1 y `False` es 0. En un programa `.bas`
  vienen de `$INCLUDE "RAPIDQ.INC"`, donde `True` es 1; sin el archivo incluido
  son nombres no declarados que valen 0 (así que `IF True THEN` no se ejecuta), y
  el editor de código avisa.

## Constantes: `clRed`, `mbYes`, `VK_ESCAPE`…

Las constantes son nombres para números fijos, como colores, botones de cuadros
de mensaje y códigos de tecla. La lista completa está en
[la referencia de constantes](../reference/constants.md).

**En un programa de RapidR (`.rr`)** están integradas. Simplemente úsalas, sin
línea de inclusión:

```basic
Form.Color = clRed
IF MESSAGEDLG("¿Guardar?", mtConfirmation, mbYes OR mbNo, 0) = mrYes THEN SaveIt
PRINT QBColor(12)          ' los 16 colores de QBasic, QBColor(0) … QBColor(15)
```

Un nombre propio gana: `DIM mbYes AS INTEGER = 5`, `CONST clRed = &HFF`, una
variable a la que le asignas un valor o el parámetro de una `SUB` es tuyo donde
lo declaras, y la constante integrada sigue estando en todos los demás lugares.
Solo se compilan las constantes que usa tu programa, y el depurador no las
enumera como variables. `True` sigue siendo -1, como lo da una comparación (el
`Checked` de una casilla es 1: comprueba `IF CheckBox1.Checked THEN`, no
`= True`).

**En un programa `.bas`** nada cambia: las constantes vienen con una línea al
comienzo:

```basic
$INCLUDE "RAPIDQ.INC"
```

<!-- legal -->
RapidR tiene ese archivo incluido integrado, así que la línea no necesita ningún
archivo: RapidR no distribuye ninguna copia de él. Sin la línea, `clRed` —y
también `True` y `False`— es un nombre no declarado y vale 0, y el editor de
código te avisa
([la advertencia de RAPIDQ.INC](studio-editor.md#the-rapidqinc-warning)). Los
programas anteriores se comportan igual. La línea en un programa `.rr` es
inofensiva: las constantes son entonces las del archivo incluido.

Las constantes propias de RapidR, como `akLeft` … `akBottom` para los anclajes,
no necesitan ninguna inclusión en ningún programa. Una variable o `CONST` propia
con el mismo nombre siempre gana.

Tus propias constantes:

```basic
CONST MaxPlayers = 4
CONST Title = "Mi juego"
```

**Lo que viene (después de Preview 2):**

- En los programas de RapidR (`.rr`), todas las constantes estarán ahí sin
  ninguna línea `$INCLUDE`. Solo se compilan las que usa tu programa, y tus
  propios nombres siguen ganando.
- Los programas anteriores (`.bas`) conservan la regla de hoy.
- Las constantes también se agruparán por tema, así que escribir `Color.` en el
  editor enumerará todos los colores. Los nombres anteriores siguen funcionando.

## Arreglos

```basic
DIM a(1 TO 3) AS INTEGER      ' límites 1 … 3
DIM grid(2, 2) AS STRING      ' 0 … 2 en cada dimensión
REDIM a(1 TO 5) AS INTEGER    ' cambia de tamaño, conserva sus valores
PRINT LBOUND(a), UBOUND(a)
SWAP a(1), a(3)
QUICKSORT a(1), a(5), 0       ' ordena a(1) … a(5); 0 ascendente
```

Se pueden pasar arreglos (array) completos (`Fill M()`,
`SUB Fill (list() AS STRING)`), y los `TYPE` pueden contener arreglos
(`Names(2) AS STRING`, `vertex(9, 2) AS SINGLE`).

## Operadores

De la precedencia más baja a la más alta: `OR`, `XOR` · `AND` · `NOT` ·
comparaciones (`=`, `<>`, `<`, `>`, `<=`, `>=`, `NOT=`) · `+`, `-`, `&` · `*`,
`/`, `\`, `MOD`, `SHL`, `SHR`, `INV` · `^` · `-` unario.

- `AND`, `OR`, `XOR`, `NOT` son **bit a bit** sobre enteros de 32 bits (`7 AND 3`
  es 3, `NOT 0` es -1). `NOT` tiene menor precedencia que las comparaciones.
- `/` divide como reales (`7 / 2` es 3.5); `\` es la división entera, con sus
  operandos redondeados primero; los operandos de `MOD` redondean los casos
  intermedios al entero par (`7.5 MOD 2` es 0).
- `\` y `MOD` entre cero detienen el programa con “Division by zero”; `/` entre
  cero da un infinito.
- `a INV m` es el inverso modular (`3 INV 26` es 9).
- Cadenas de texto: `+` o `&` las unen; `-` quita todas las apariciones
  (`"jello" - "l"` es `"jeo"`); `s$[i]` es el i-ésimo carácter.
- `i++`, `i--`, `x += y` (y `-=`, `*=`, `/=`, `&=`); `INC x [, n]`,
  `DEC x [, n]`.
- `@var` pasa una variable por referencia.

## Imprimir números

RapidR imprime los números a la manera clásica (el conjunto de pruebas de
compatibilidad lo cubre):

```basic
PRINT 1 / 3         ' 0.333333333
PRINT 3.5           ' 3.500000000   (fracciones: 9 decimales)
PRINT 10 / 5        ' 2             (números enteros: como enteros)
PRINT 1E10          ' -2147483648   (un número entero más allá de 32 bits)
PRINT STR$(1 / 3)   ' 0.333333333   (STR$: 9 dígitos significativos)
```

**La coma de `PRINT` es un punto y coma**: `PRINT 1, 2` imprime `12`, sin zonas
de tabulación. Usa `;` con espacios explícitos, `TAB(n)`, `SPACE$(n)`,
`FORMAT$` o `STRF$` para disponer las columnas:

```basic
PRINT FORMAT$("%5d|%.2f|%s", 42, 3.14159, "x")    '    42|3.14|x
PRINT STRF$(1234.5678, 2, 8, 2)                    ' 1234.57 (ffFixed)
```

Otras reglas numéricas: `INT` y `FIX` truncan hacia cero; `ROUND`, `CINT` y
`CLNG` son `INT(x + 0.5)` (2.5 → 3, -2.5 → -2); `VAL` lee el número que está al
comienzo del texto (`VAL("12abc")` es 12); `HEX$` tiene 8 dígitos.

## Control de flujo

```basic
IF x > 10 THEN
    PRINT "grande"
ELSEIF x > 5 THEN
    PRINT "mediano"
ELSE
    PRINT "pequeño"
END IF
IF ok THEN PRINT "sí" ELSE PRINT "no"

FOR i = 1 TO 10 STEP 2 : PRINT i : NEXT i
WHILE k < 5 : k = k + 1 : WEND
DO
    k = k + 1
LOOP UNTIL k >= 3                 ' también DO WHILE / DO UNTIL / LOOP WHILE

SELECT CASE n
    CASE 1 TO 10
        PRINT "pequeño"
    CASE 11, 12
        PRINT "once o doce"
    CASE IS > 12
        PRINT "grande"
    CASE ELSE
        PRINT "?"
END SELECT

GOSUB Shout                       ' … Shout: PRINT "!" : RETURN
GOTO Done
EXIT FOR / EXIT DO / EXIT WHILE / EXIT SUB / EXIT FUNCTION
END
```

`DATA`, `READ` y `RESTORE [label]` funcionan como en otros BASIC.

### `FOR i AS INTEGER`: un contador declarado en su ciclo

Muchos BASIC (y los modelos de IA que los aprendieron) escriben el tipo del
contador en el propio ciclo. RapidR también lo acepta:

```basic
FOR i AS INTEGER = 1 TO 3 : PRINT i : NEXT      ' imprime 1, 2, 3

SUB CountDown
    FOR k AS LONG = 10 TO 0 STEP -5             ' k es propia de la SUB
        PRINT k
    NEXT
END SUB
```

- El contador se declara donde todavía no lo está, como lo haría un `DIM`: en una
  `SUB` o una `FUNCTION` es una variable local propia; en el programa principal,
  una global.
- Un contador ya declarado con el **mismo tipo** es esa variable (dos ciclos
  sobre `i`, o primero `DIM i AS INTEGER`). Uno declarado con **otro tipo** es un
  error que lo dice:
  `FOR name AS INTEGER: name is already declared AS STRING (in main.rr:3)`.
- El tipo es un tipo numérico: `INTEGER`, `LONG`, `DOUBLE`, `SINGLE`, `BYTE` …
- `FOR i = 1 TO 3` no cambia. Funciona igual en el intérprete, en las
  compilaciones nativas y en la web.

### `END` y el código de salida

`END` termina el programa de inmediato; los archivos que escribió se conservan.
El `END n` de RapidR lo termina con el código de salida `n`, para que una
herramienta de consola pueda decirle al sistema, o al script que la ejecutó, si
funcionó. 0 significa éxito, como en todas partes:

```basic
IF COMMANDCOUNT = 0 THEN
    PRINT "uso: check ARCHIVO"
    END 2
END IF
IF FILEEXISTS(COMMAND$(1)) = 0 THEN END 1
PRINT "bien"                      ' llegar al final también da el código de salida 0
```

`n` es cualquier número o expresión numérica (`END failures + 1`), guardado como
se guarda un `LONG`, así que `END 2.9` termina con 2. Un nombre suelto tiene que
ser uno que declaraste (`DIM code AS INTEGER = 5` y luego `END code`; también
sirve una `CONST`, un parámetro o una variable local): un error de escritura en
el final de un bloque, `END SUBB` o `END FUNCTON`, y un nombre que nunca se
declaró siguen siendo un error de sintaxis, con la sugerencia “did you mean END
SUB?” cuando la palabra se parece a uno, en lugar de terminar el programa allí
sin avisar. Un `END` simple termina con 0, como siempre, y `END IF`, `END SUB` y
los demás finales de bloque son lo que eran. Después de una ejecución, lee el
código en la terminal: `echo $?` en macOS y Linux, `echo %ERRORLEVEL%` en el
Símbolo del sistema de Windows. Es igual para `rapidr run`, para una aplicación
compilada con `rapidr build` (nativa o `--interp`) y para un programa con
ventanas. macOS y Linux conservan los 8 bits bajos del número (de 0 a 255);
Windows conserva los 32. Un programa que se ejecuta en una página web no tiene
código de salida: simplemente termina, y el número se ignora.

## `SUB` y `FUNCTION`

```basic
DECLARE FUNCTION Square (x AS DOUBLE) AS DOUBLE   ' opcional

SUB Greet (Who AS STRING)
    PRINT "Hola, "; Who
END SUB

FUNCTION Square (x AS DOUBLE) AS DOUBLE
    Square = x * x            ' o: Result = x * x
END FUNCTION

FUNCTION Twice% (n%)          ' un sufijo da el tipo del resultado
    Twice% = n% * 2
END FUNCTION

SUB Change (BYREF v AS INTEGER)
    v = 99
END SUB

Greet "RapidR"                ' una SUB se llama sin paréntesis
PRINT Square(1.5)             ' 2.250000000
```

- Los parámetros son por valor a menos que sean `BYREF` (o se pasen como
  `@var`).
- `SUBI` / `FUNCTIONI` aceptan cualquier cantidad de argumentos, que se leen con
  `ParamVal(i)`, `ParamValCount`, `ParamStr$(i)`, `ParamStrCount`.
- Punteros a funciones: `BIND p TO MySub`, `CALLFUNC p, args`, `CODEPTR(MySub)`.
<!-- legal -->
- `DECLARE … LIB "user32"` (cualquier DLL) llama a la función de la DLL **en
  Windows**, en las compilaciones nativas y en modo interpretado, con estas
  reglas: un número va por valor a menos que sea `BYREF`, un `STRING` y un `TYPE`
  van por dirección (lo que la DLL escribe se ve en la variable), las
  direcciones de `VARPTR` son punteros reales para la llamada. Hasta 16
  argumentos, de los cuales hasta 8 `DOUBLE` / `SINGLE`. Un puntero de 64 bits
  que una DLL devuelve o escribe en un `LONG` se guarda como un sustituto de
  32 bits que vuelve a convertirse en el puntero cuando se le da otra vez a una
  DLL. `SENDMESSAGE`, `POSTMESSAGE` y `KILLMESSAGE` son funciones de user32 de la
  misma manera. En macOS, Linux y la web el programa se compila y se detiene en
  la primera llamada de ese tipo con un error que nombra la función (y el
  equivalente portátil de RapidR cuando lo hay); una biblioteca propia del
  sistema (`LIB "libfoo.dylib"`, `"libfoo.so"`) se carga allí. Con
  `RAPIDR_SANDBOX` definida no se carga ninguna biblioteca. docs/windows-dll-calls.md
  tiene los detalles.
- `PEEK` / `POKE` / `PCOPY` funcionan sobre las páginas de la consola (la página 0
  es la pantalla: las direcciones pares son el carácter de una celda, las impares
  su color), y —una adición de RapidR— `PEEK` / `POKE` sobre la memoria propia
  del programa mediante `VARPTR`, en todos los sistemas. Cualquier otra
  dirección es un error de ejecución, nunca una lectura del proceso. `INP` /
  `OUT` (puertos de hardware) son un error de ejecución, como en todos los
  Windows desde 2000.
- `VARPTR` no ve los parámetros de una `SUB`: cuando un parámetro tiene el nombre
  de una variable global, `VARPTR(name)` dentro de la `SUB` es la dirección de la
  global.

  ```basic
  DIM Msg AS STRING = "global"
  SUB Greet(Msg AS STRING)
    PRINT Msg                       ' el parámetro: hello
    PRINT VARPTR$(VARPTR(Msg))      ' la global: global
  END SUB
  Greet "hello"
  ```

  `VARPTR` de un parámetro con cuyo nombre no coincide ninguna global da la
  dirección propia de ese parámetro.

## `TYPE` y objetos

Un `TYPE` es un registro, y puede tener métodos, un constructor, eventos y
herencia:

```basic
TYPE TCounter
    Count AS INTEGER
    Name AS STRING
    SUB Bump (n AS INTEGER)
        This.Count = This.Count + n
    END SUB
    FUNCTION Describe AS STRING
        Describe = This.Name + "=" + STR$(This.Count)
    END FUNCTION
    CONSTRUCTOR
        Name = "counter"
        Count = 100
    END CONSTRUCTOR
END TYPE

TYPE TLoud EXTENDS TCounter
    FUNCTION Shout AS STRING
        Shout = UCASE$(This.Describe)
    END FUNCTION
END TYPE

DIM c AS TCounter
c.Bump 5
PRINT c.Describe              ' counter=105
DIM l AS TLoud
l.Bump 1
PRINT l.Shout                 ' COUNTER=101
```

- `TYPE TMyButton EXTENDS RButton` crea un componente propio, con bloques
  `EVENT OnSomething … END EVENT`.
- `WITH obj … END WITH` y `.Member` dentro.
- Arreglos de `TYPE` y de componentes: `DIM lbl(1 TO 3) AS RLabel`.

## Componentes

```basic
CREATE Form AS RForm
    Caption = "Título"
    CREATE Btn AS RButton
        Caption = "Aceptar"
        OnClick = OkClick
    END CREATE
END CREATE
DIM Font AS RFont             ' un objeto no visual
```

Mira [Componentes y objetos](components.md).

## Cadenas de texto

Las funciones integradas son las clásicas: `LEFT$`, `MID$`, `RIGHT$`, `INSTR`,
`RINSTR`, `UCASE$`, `LCASE$`, `TRIM$`, `REPLACE$` (sobrescribe en una posición),
`REPLACESUBSTR$` (buscar y reemplazar), `INSERT$`, `DELETE$`, `REVERSE$`,
`FIELD$`, `TALLY`, `STRING$`, `SPACE$`, `CHR$`, `ASC`, `HEX$`, `BIN$`,
`OCT$`, `CONVBASE$`, `FORMAT$`, `STRF$`, … — la lista completa está en
[la referencia de funciones integradas](../reference/builtins.md). Las funciones
de cadenas de texto cuentan caracteres, no bytes.

- `""` dentro de una cadena de texto **no** es una comilla escapada (se lee como
  dos cadenas una al lado de la otra). Usa `CHR$(34)`, o activa
  `$ESCAPECHARS ON` para `\"`, `\n`, `\t`, `\\`, `\x41`.

### Cadenas largas y datos binarios

Una cadena de texto puede contener un archivo entero: `ReadBinStr` (o `ReadStr`)
en un `RFileStream` da un carácter por byte, `CHR$(0)` … `CHR$(255)`, y
`ASC(MID$(data, i, 1))` —o `ASC(data[i])`— es el byte en `i`. Leer cada byte de
un archivo de 64 KB de esta manera tardó 13 ms con `rapidr run`, 7 ms compilado
nativo y 23 ms en un navegador en un Apple M4 Max
([tools/bench](../../../tools/bench/README.md)), lo mismo por byte sin importar
lo larga que sea la cadena: `MID$`, `LEFT$`, `RIGHT$`, `LEN`, `INSTR` y `data[i]`
cuestan lo que devuelven, no la longitud de la cadena. Las cadenas se comparten,
no se copian —pasar una a una `SUB` o ponerla en otra variable no cuesta nada—,
y una cadena construida pieza por pieza, `s = s + x` o `s = s & x`, crece en su
lugar. (Escribe `s = s + (a + b)` en lugar de `s = s + a + b` cuando `s` es
larga: la segunda crea primero un `s + a` nuevo.) Para llevar los bytes a un
arreglo en una sola llamada, cópialos con `MemCopyTo`:

```basic
DIM mem(65535) AS BYTE
fs.Open("game.rom", fmOpenRead)
fs.MemCopyTo(VARPTR(mem(0)), fs.Size)     ' sus bytes, mem(0) el primero (no más de lo que cabe en el arreglo)
fs.Close
```

[bytes.rr](../../../examples/basics/bytes.rr) hace las dos cosas con un archivo
de 64 KB.

## Archivos

```basic
OPEN "notes.txt" FOR OUTPUT AS #1     ' también INPUT, APPEND, BINARY, RANDOM
PRINT #1, "primera línea"
CLOSE #1
f = FREEFILE
OPEN "notes.txt" FOR INPUT AS #f
WHILE NOT EOF(f)
    LINE INPUT #f, s$
    PRINT s$
WEND
CLOSE #f
PRINT FILEEXISTS("notes.txt"), FILELEN("notes.txt")
```

### Leer línea por línea lo que se escribe o se canaliza

La entrada propia de un programa de consola se lee a medida que llega.
`INPUT s$` toma una línea de la consola, y `OPEN "/dev/stdin" FOR INPUT AS #f` la
convierte en un archivo: `LINE INPUT #f, s$` devuelve cada línea **en cuanto ha
llegado**, no cuando ha llegado toda la entrada, y `EOF(f)` espera el siguiente
byte o el final de la entrada. Por lo tanto, un programa que responde una línea
con una línea (un servidor que habla líneas JSON en su entrada y su salida, como
un servidor MCP por stdio) se puede escribir con un ciclo simple:

```basic
$APPTYPE CONSOLE
OPEN "/dev/stdin" FOR INPUT AS #1
WHILE NOT EOF(1)
    LINE INPUT #1, request$
    PRINT "recibido: "; request$       ' se escribe antes de leer la siguiente línea
WEND
```

Funciona igual con `rapidr run`, en una compilación interpretada y en una
nativa. Una página web no tiene entrada estándar: allí `/dev/stdin` es un archivo
que no existe (los archivos de la página son propios de ella, mira
[La web](web.md)).

Los objetos de flujo hacen lo mismo con métodos: `RFileStream` (nombre anterior
`QFILESTREAM`: `Open`,
`ReadLine`, `WriteLine`, `ReadNum`, `WriteNum`, `Read`, `Write`, `Seek`,
`Size`, `Position`, `SaveArray`, `LoadArray`, …), `RMemoryStream` y
`RStringList` (`LoadFromFile`, `SaveToFile`). `DIR$`, `KILL`, `MKDIR`,
`RMDIR`, `RENAME`, `CHDIR`, `CURDIR$` y `DIREXISTS` trabajan con carpetas.
En un navegador, los archivos son propios de la página (mira [La web](web.md)).

### Texto y bytes: acentos, chino, emoji

Una cadena de texto se compone de caracteres y un archivo de bytes. RapidR los
mantiene separados, así que el texto nunca se pierde y los datos binarios nunca
se cambian:

| Usas | El archivo contiene |
|---|---|
| `PRINT #`, `LINE INPUT #`, `RStringList.SaveToFile` / `LoadFromFile`, archivos de traducción (`.po`) | Texto como **UTF-8**. La lectura también acepta el texto en Windows-1252 que escribieron programas y editores anteriores (una `é` como un solo byte): un carácter nunca se convierte en `?` ni en `�` al entrar. |
| `RFileStream` y `RMemoryStream`: `WriteStr`, `WriteLine`, `Write`, `ReadStr`, `Read`, `ReadLine` | **Bytes.** Una cadena cuyos caracteres son todos de hasta 255 se escribe a un byte por carácter (`CHR$(200)` es el byte 200), así que las imágenes, los sonidos y los paquetes construidos o copiados con cadenas vuelven exactos. Una cadena con algún carácter por encima de 255 (chino, emoji, `€`) es texto: se escribe entera como UTF-8, con los acentos incluidos. |
| `WriteText` y `ReadAllText` (de RapidR, en los dos flujos) | Texto como UTF-8 sea lo que sea lo que contenga la cadena, incluso solo acentos. `ReadAllText` lee el resto del flujo como UTF-8 (se descarta una marca de orden de bytes) o, cuando no es UTF-8, como Windows-1252. |
| `RMemo`, `REdit` y `RRichEdit` (texto simple), `RCodeEdit` | La codificación que tenía el archivo cuando se cargó: UTF-8 (con o sin marca de orden de bytes) o Windows-1252. Un texto que nunca se cargó en un cuadro de texto se escribe como Windows-1252 mientras quepa. |

`WriteStr(S, n)` cuenta **caracteres**, así que `WriteStr(S, LEN(S))` escribe
todo `S` en cualquier idioma. Un archivo **nunca se guarda con un `?` en lugar de
un carácter** que su codificación no tiene: un archivo en Windows-1252 en el que
se escribe chino (o el texto de un cuadro de texto que tiene un emoji) se guarda
como UTF-8, y `RCodeEdit.Encoding` dice “UTF-8” después.

```basic
DIM f AS RFileStream, s AS STRING
s = "Fecha: 日期 · café · 😀"
f.Open("note.txt", fmCreate)
f.WriteStr(s, LEN(s))              ' UTF-8: el chino y el emoji lo necesitan
f.Position = 0
PRINT IIF(f.ReadAllText = s, "igual", "diferente")    ' igual: todo volvió
f.Close
```

La imagen de abajo es el ejemplo de traducciones: el mismo saludo en una etiqueta
(arriba) y dibujado en un lienzo con `TextOut` (abajo), a partir de un catálogo en
chino y uno en español. Un lienzo dibuja los caracteres que faltan en las
fuentes propias de RapidR (chino, japonés, coreano) con la fuente CJK del
sistema en el escritorio y con las fuentes Noto de la página en la web, como
hacen sus etiquetas; en la web dibuja en `OnPaint`, que se ejecuta otra vez
cuando llega una fuente así.

![El ejemplo de traducciones en chino y en español](../images/translations/languages-chinese.png)
![Lo mismo en español](../images/translations/languages-spanish.png)
Un `RFileStream` escribe un archivo un byte a la vez tan rápido como todo de una
vez: guarda lo que se le da y lo escribe junto —en `Close`, cuando el programa
espera (sus eventos, `SLEEP`, `INPUT`) o termina (`END`, un error de ejecución),
cada 64 KB, y antes de que cualquier otra cosa use el archivo por su nombre
(`OPEN`, el `Open` de otro flujo, `FILELEN`, `KILL` …)—, así que lo que lea el
archivo ve cada byte, como si cada escritura hubiera ido directamente a él. Solo
un programa detenido desde fuera (terminado a la fuerza, se cortó la corriente)
puede perder lo que escribió al final, como máximo 64 KB. `SaveArray` y
`LoadArray` de un arreglo numérico mueven todos sus elementos de una sola vez.

Los nombres de archivo funcionan como en Windows, en todos los sistemas: tanto
`\` como `/` separan carpetas, y un nombre encuentra su archivo sin importar las
mayúsculas y minúsculas. Un programa escrito en Windows encuentra sus archivos en
macOS y Linux sin cambios:

```basic
Bitmap.BMP = "PICS\OPEN.BMP"            ' encuentra pics/open.bmp también en Linux
DIM Notes AS RStringList
Notes.LoadFromFile(Application.Path + "\notes.txt")
```

Un nombre que existe exactamente como está escrito siempre se toma tal cual; un
archivo nuevo conserva el nombre que le da el programa (en la carpeta que se
encontró sin importar las mayúsculas y minúsculas).

## La consola

`CLS`, `COLOR fg, bg`, `LOCATE row, col`, `CSRLIN`, `POS(0)`, `INPUT
[prompt,] var`, `INKEY$`, `INPUT$(n)`, `SLEEP seconds` (`SLEEP 0.5`), `BEEP`,
`SOUND`. Los colores y las posiciones se escriben como secuencias ANSI, así que
funcionan en cualquier terminal moderna y en el panel de salida del IDE. Un
`INPUT$` solo (sin cantidad) lee una línea completa — los programas anteriores
terminan con `a = INPUT$` para esperar antes de que se cierre la ventana.

**Caracteres para cuadros.** La consola clásica mostraba los caracteres del 128
al 255 en la antigua página de códigos 437 de DOS, así que los programas
anteriores dibujan cuadros con ellos:

```basic
PRINT CHR$(201); STRING$(10, 205); CHR$(187)    ' ╔══════════╗
```

RapidR muestra la consola de un programa anterior de la misma manera: un archivo
que no es UTF-8 (guardado por un editor antiguo de Windows o DOS), o un archivo
`.bas` / `.rqb` / `.rq` / `.rqw` de ASCII simple. Un programa de RapidR (un
archivo `.rr`, o cualquier archivo UTF-8 con letras acentuadas) imprime su texto
tal como es — `CHR$(233)` es é. Para dibujar cuadros en un programa de RapidR,
escribe los caracteres mismos (`PRINT "╔══╗"`).

### Formas anteriores que siguen funcionando

El compilador clásico aceptaba unas pocas formas que hoy parecen raras; RapidR
las acepta de la misma manera:

```basic
SUB Draw.Box (X AS INTEGER)       ' un punto en el nombre de una SUB / FUNCTION
    PRINT "caja"; X
END SUB
FUNCTION Calc.Twice (N AS INTEGER) AS INTEGER
    Calc.Twice = N * 2            ' el resultado, asignado al nombre completo
END FUNCTION
CONST Null = &0                   ' & antes de dígitos simples: el número (0)
CONST App.Name = "Demo"           ' un punto en el nombre de una CONST, que se lee como App.Name
Draw.Box 1
SLEEP(Calc.Twice(3)) / 10         ' (…) inicia el argumento: SLEEP 0.6
```

## El preprocesador

| Directiva | |
|---|---|
| `$INCLUDE "file"` | incluye un archivo (las carpetas de inclusión y las rutas con `\` funcionan; `RAPIDQ.INC` está integrado) |
| `$DEFINE NAME [text]`, `$UNDEF NAME` | sustitución de texto |
| `$IFDEF` / `$IFNDEF` / `$ELSE` / `$ENDIF` | compilación condicional |
| `$MACRO NAME(x) = text` | una macro con parámetros |
| `$APPTYPE CONSOLE \| GUI \| CGI \| WEB` | la clase de programa |
| `$RESOURCE NAME AS "file"` | un archivo integrado en el programa (`RESOURCE(n)`, `EXTRACTRESOURCE`, imágenes) |
| `$OPTION ICON "file.ico"` | el ícono del programa: el de sus ventanas y el de la aplicación compilada ([Compilar aplicaciones](building-apps.md)) |
| `$ESCAPECHARS ON` | secuencias de escape en las cadenas de texto de este archivo |
| `$TYPECHECK ON` / `OFF` (`$OPTION EXPLICIT`: ON) | toda variable tiene que estar declarada (`DIM`, `CONST`, un parámetro): a una no declarada a la que se le asigna un valor da `Undeclared identifier x`, y leída da `Undefined symbol X` |
| `$OPTIMIZE` | se acepta |
| `$THEME name` | de RapidR: el aspecto del escritorio ([Componentes](components.md#themes)) |
| `#If … Then` / `#ElseIf` / `#Else` / `#End If`, `#Const` | la compilación condicional de VB |

Un error en el código incluido señala el archivo incluido y la línea.

## Errores

- Los errores de compilación nombran el archivo, la línea y la columna, y se
  informa cada línea incorrecta, no solo la primera:
  `syn.bas:3:1: error: Syntax error in FOR statement`.
- Algunos dicen también la solución. Un nombre declarado otra vez —en un programa
  de varios archivos, a menudo en otro archivo— dice dónde está el primero:
  `Identifier siteCount already used, try another name — it is declared in
  globals.rr:12 already`. Una `SUB` o `FUNCTION` que ningún archivo tiene nombra
  las más cercanas: `Unknown SUB or FUNCTION 'ShowNumbr' — did you mean
  ShowNumber?`.
- El editor también avisa de código que se compila pero no puede estar bien:
  `IF NOT FILEEXISTS(f) THEN` siempre es verdadero, porque `FILEEXISTS` da 1 para
  sí y `NOT 1` es -2. La corrección rápida de la advertencia (Ctrl+.) escribe
  `IF FILEEXISTS(f) = 0 THEN`.
- Un error de ejecución en el código del programa principal detiene el programa
  con su mensaje y su línea:
  `run-time error: Division by zero (at err.bas line 4)`.
- Un error de ejecución en un **controlador de eventos** (el de un clic, el de un
  temporizador) termina ese controlador —y los controladores que lo
  dispararon— y la ventana sigue funcionando: el error se escribe en la salida de
  errores del programa (en Studio, la Salida). Un error que lanza un componente
  (`Cannot create file notes/today.txt.`) es un error de ejecución como cualquier
  otro, con su línea.
- Los controladores que siguen disparando eventos uno dentro de otro para siempre
  (un `Repaint` dentro del `OnPaint` del mismo lienzo) se detienen, a 200 niveles
  de profundidad, con un error de ejecución que nombra el evento —nunca un fallo
  ([Cuándo se ejecutan los eventos](components.md#when-events-run)).
- `ON ERROR GOTO` / `ON ERROR RESUME NEXT` (los de VB) se aceptan para que esos
  programas se compilen, y se **ignoran**.

### Cuando un controlador falla: `Application.OnError`

Un programa puede enterarse de cada uno de esos errores de controlador, para
avisar al usuario, escribir un registro o guardar su trabajo (propio de RapidR):

```basic
SUB Failed(Message AS STRING, File AS STRING, Line AS INTEGER)
  Status.Caption = "Algo salió mal: " + Message + " (línea " + STR$(Line) + ")"
END SUB

Application.OnError = Failed
```

`Message` es solo el error (`Division by zero`), `File` y `Line` dónde ocurrió
(una compilación nativa da `""` y `0`). Se dispara una vez por error, después de
que el controlador ha terminado — nunca por un error en la subrutina `Failed`
misma. Un error en el código propio del programa principal sigue terminando el
programa. RapidR Studio lo usa para mantener tu trabajo a salvo
([Studio conserva tu trabajo](studio-projects.md#studio-keeps-your-work)).
Prueba [examples/gui/on_error.rr](../../../examples/gui/on_error.rr): dos
botones, cada uno con un error, y una ventana que sigue funcionando.

## ¿Vienes de otro lenguaje?

Si escribes RapidR como escribes Python, JavaScript, C o Pascal, el compilador se
detiene en lo que BASIC no tiene. Su mensaje sigue siendo el propio del
compilador, y debajo de él Studio (el panel Problemas, la sugerencia del
subrayado ondulado), `rapidr check`, `rapidr run` y `rapidr build` añaden una
línea con la forma de RapidR:

```text
$ rapidr check example.rr
example.rr:2:1: error: Syntax error in SET statement
  hint: RapidR has no SET: write x = 1 (the name, =, then the value)
```

Las aplicaciones de IA y RAI reciben la misma línea debajo del error
(`check_code`). Cada sugerencia tiene un id estable (`hint.set` …) y un solo
lugar para sus palabras, así que se pueden traducir como cualquier mensaje.

| Escribiste | En RapidR | Id de la sugerencia |
|---|---|---|
| `SET x = 1` | `x = 1` | `hint.set` |
| `LET x = 1` | `x = 1` | `hint.let` |
| `var x = 1`, `let x: number = 1` | `DIM x AS INTEGER`, `CONST X = 1` | `hint.var` |
| `DIM a AS INTEGER AS INTEGER` | `DIM a AS INTEGER` | `hint.as-as` |
| `a % b`, `a %= b` | `a MOD b`, `a = a MOD b` | `hint.mod` |
| `a ^= 2`, `a \= 2` | `a = a ^ 2`, `a = a \ 2` (`+=`, `-=`, `*=`, `/=`, `&=` y `x++` sí funcionan) | `hint.compound` |
| `a ** b` | `a ^ b` | `hint.pow` |
| `a // b` (división entera) | `a \ b` | `hint.intdiv` |
| `c ? a : b` | `IF c THEN x = a ELSE x = b`, o `IIF(c, a, b)` | `hint.ternary` |
| `a != b`, `a !== b` | `a <> b` | `hint.ne` |
| `!a` | `NOT a`, o `a = 0` | `hint.bang` |
| `a \|\| b` | `a OR b` | `hint.or` |
| `a && b` (se compila, pero une texto: una advertencia) | `a AND b` | `hint.and-and` |
| `for i in range(10)`, `for (i = 0; i < 10; i++)`, `FOR EACH x IN list` | `FOR i = 1 TO 10 … NEXT i`; una lista: `FOR i = 0 TO List.ItemCount - 1` | `hint.for-foreign` |
| `END FOR`, `END WHILE`, `END DO` | `NEXT`, `WEND`, `LOOP` | `hint.end-loop` |
| un `FOR`, `WHILE`, `DO`, `IF` o `SELECT` que nunca se cierra (el *Missing NEXT statement* de RC.EXE …, una advertencia) | `NEXT`, `WEND`, `LOOP`, `END IF`, `END SELECT` | `hint.missing-next`, `hint.missing-wend`, `hint.missing-loop`, `hint.missing-end-if`, `hint.missing-end-select` |
| `ELSE IF c THEN` (un segundo `IF` que necesita su propio `END IF`) | `ELSEIF c THEN` | `hint.else-if` |
| `elif c THEN` | `ELSEIF c THEN` | `hint.elseif` |
| `ENDIF` | `END IF` | `hint.endif` |
| `ENDSUB`, `ENDFUNCTION`, `ENDSELECT`, `ENDWITH` | `END SUB`, `END FUNCTION`, `END SELECT`, `END WITH` | `hint.endword` |
| `IF a = 1` sin `THEN` | `IF a = 1 THEN` | `hint.if-then` |
| `IF a THEN x` en una línea, y luego `ELSE y` en la siguiente | `IF a THEN x ELSE y` en una línea, o un bloque `IF a THEN` … `ELSE` … `END IF` | `hint.else-line` |
| `if a == 1:`, `def f():` | sin `:`; `IF a = 1 THEN … END IF`, `SUB F … END SUB` | `hint.colon` |
| `IF a { … }`, `SUB Go() { … }` | `IF a THEN … END IF`, `SUB Go … END SUB` | `hint.braces` |
| `def f(a):`, `function f(a) { … }`, `void f()` | `SUB F(a AS INTEGER)`, `FUNCTION F(a AS INTEGER) AS INTEGER` | `hint.function` |
| `return a` | `Result = a` en una `FUNCTION` | `hint.return` |
| `a = 1;` | `a = 1` | `hint.semicolon` |
| `switch (a) { case 1: … }` | `SELECT CASE a` / `CASE 1` / `END SELECT` | `hint.switch` |
| `break`, `continue` | `EXIT FOR`, `EXIT DO`, `EXIT WHILE`; no hay continue: envuelve el resto en un `IF` | `hint.break`, `hint.continue` |
| `#include <x>` | `$INCLUDE "x.rr"` | `hint.include` |
| `new Button()` | `CREATE B AS RButton … END CREATE` | `hint.new` |
| `OnClick = ()`, `OnClick = ""` | `OnClick = GoClick` con `DECLARE SUB GoClick` | `hint.event` |
| `console.log("hi")` (se compila; una advertencia), `printf("hi")`, `echo "hi"` | `PRINT "hi"` | `hint.console-log`, `hint.print` |
| `names = ["Ana", "Luis"]`, `names[0]` | `DIM names(1) AS STRING`, `names(0) = "Ana"` | `hint.brackets` |
| `s = 'hello'` | `s = "hello"` (una comilla simple inicia un comentario) | `hint.squote` |
| `x = 1 // note` | `x = 1 ' note` | `hint.comment` |
| `null`, `None`, `nil` | `""` o `0` | `hint.null` |
| `true`, `false` | `1`, `0` en un `INTEGER` | `hint.bool` |
| `s.length` | `LEN(s)`; `.ItemCount` de una lista | `hint.length` |
| `str(n)`, `parseInt(s)`, `toUpperCase`, `substring`, `alert`, `random()` … | `STR$(n)`, `VAL(s)`, `UCASE$(s)`, `MID$(s, a, n)`, `ShowMessage`, `RND` | `hint.fn-<name>` (`hint.fn-str` …) |

`==`, `+=`, `++` y `print(x)` están bien en RapidR: se compilan y funcionan
(también son del compilador clásico), así que no se dice nada sobre ellos.

## Código nativo: `RUSTSTART`

Solo en las compilaciones nativas, `RUSTSTART … RUSTEND` pone código Rust en el
programa —en el programa principal o en una `SUB`, con más BASIC después—. El
intérprete y la web lo rechazan con un mensaje claro en su línea.

```basic
PRINT "antes"
RUSTSTART
println!("desde Rust");
RUSTEND
PRINT "después"
```
