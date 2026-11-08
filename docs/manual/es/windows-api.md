<!-- translated-from: windows-api.md c085bed2f6105fe1 -->
# Un solo estándar para la API de Windows

Si has escrito programas que hablan con el propio Windows —un `DECLARE …
LIB "user32"` por aquí, un `CONST WM_CLOSE` por allá—, seguro conoces esta
historia. Todos teníamos un `WIN32API.INC`. Llegó de un amigo o de un ejemplo
antiguo, y después se corrigió a mano: una constante añadida aquí, una
declaración corregida allá. Con los años hubo muchas versiones, y no
coincidían entre sí. Dos de ellas en un mismo programa chocaban. Un fragmento
que le funcionaba a otra persona a ti te fallaba, porque tu archivo tenía otro
nombre, otro valor, o nada. Muchos perdimos horas por eso, y nadie hizo nada
mal: simplemente no existía un archivo común que compartir.

RapidR ahora tiene uno: **`RWin32.inc`**, integrado.

## Primero: nada de lo que tienes deja de funcionar

- **Tus programas siguen funcionando tal como están.** Un programa con su
  propio `WIN32API.INC` (o cualquier otro archivo incluido) se compila y se
  ejecuta exactamente como antes, mediante la capa de compatibilidad con
  RapidQ de RapidR. No tienes que cambiar nada, ni hoy ni más adelante.
- **Tus archivos incluidos siguen siendo tuyos.** RapidR nunca los reemplaza
  ni los edita. Si prefieres tu archivo, quédate con él.
- **Nada se cambia por ti.** El importador solo *ofrece* el cambio, y te
  muestra cada diferencia antes de que ocurra nada (más abajo).

## El cambio, si lo quieres: una línea

```basic
$INCLUDE "RWin32.inc"
```

Eso es todo. No hay ningún archivo que descargar, buscar ni copiar:
`RWin32.inc` es parte de RapidR, en cada computadora y en cada instalación
(también en las descargas para Windows 7 y XP). Sirve cualquier ruta y
cualquier combinación de mayúsculas (`$INCLUDE
"C:\rapidq\include\RWIN32.INC"` lee el mismo archivo integrado), así que a una
línea antigua solo hay que cambiarle el nombre del archivo.

**RapidR te ayuda a cambiar.** Cuando importas un programa antiguo
([Importar programas antiguos](importing-rapidq.md)) que incluye un
`WIN32API.INC`:

- la copia conserva tu archivo, como siempre, y se comprueba que se compile
  exactamente igual que el original;
- el informe dice entonces que `RWin32.inc` puede ocupar su lugar, y
  **enumera, para los nombres que usa tu programa, todo lo que sería
  distinto**: una constante con otro valor, una función que tu archivo
  llamaba con otro nombre de DLL o con otra cantidad de argumentos, un nombre
  que `RWin32.inc` no tiene, y lo que tu archivo hacía además de declarar (un
  `$DEFINE INTEGER SHORT`, un `$OPTION`);
- tú decides. En RapidR Studio, Archivo ▸ Importar proyecto o archivo de
  RapidQ… te pregunta después de la importación (**Mantener mi archivo** es
  la respuesta predeterminada). Con la línea de comandos, `rapidr
  import-rapidq myprogram.bas --rwin32` hace el cambio en la copia y comprueba
  que la copia todavía se compile. Tu programa original nunca se modifica.

## Por qué un solo estándar nos ayuda a todos

- **Un solo archivo para todos.** Un ejemplo, una respuesta en una discusión o
  un fragmento del manual que usa `RWin32.inc` funciona en tu computadora
  igual que en la de quien lo escribió. RAI, el asistente de RapidR, y otros
  asistentes de IA también pueden contar con los mismos nombres, así que su
  código para Windows acierta más a menudo.
- **Completo y actual.** `RWin32.inc` no se escribe a mano: una herramienta
  propia de RapidR lo genera a partir de los metadatos de la API de Win32 que
  publica Microsoft. Tiene 55 710 constantes, 3675 estructuras (`TYPE`) y
  6506 funciones de 29 DLL de Windows (kernel32, user32, gdi32, advapi32,
  shell32, comdlg32, comctl32, winmm, winspool, ole32, wininet, ws2_32 y
  más).
- **Correcto en Windows de 64 y de 32 bits.** Los archivos incluidos antiguos
  se escribieron para Windows de 32 bits, donde un puntero y un handle miden
  4 bytes. En el Windows de 64 bits de hoy miden 8, así que una estructura
  como `NOTIFYICONDATA`, `MSG` o `SYSTEM_INFO` necesita otra disposición.
  `RWin32.inc` tiene las dos, y tu programa recibe la correcta para la
  computadora para la que se compila.
- **Los nombres clásicos están.** `MessageBox`, `GetWindowText`, `FindWindow`,
  `SendMessage`, `ShellExecute`, `RECT`, `POINTAPI`, `LOGFONT` … significan lo
  que siempre significaron: el nombre sin sufijo es la función o estructura
  ANSI (la “A”), como en los archivos clásicos. 1496 de las 1514 funciones
  que declara un `WIN32API.INC` clásico muy usado (98.8%) están en
  `RWin32.inc` con los mismos nombres; los nombres modernos con A y con W
  (`MessageBoxA`, `MessageBoxW`) también están.
- **Rápido.** Un programa recibe solo lo que usa: uno con veinte llamadas a
  Windows se compila en bastante menos de un segundo y lleva solo esas veinte
  declaraciones.

## Tú eliges

Usa `RWin32.inc` en los programas nuevos si te gusta, cambia los antiguos
cuando te convenga, o conserva tu propio archivo: las tres opciones están
bien, y RapidR admite cada una. Si algo de `RWin32.inc` falta o te parece
incorrecto, cuéntanoslo en
[Discussions](https://github.com/iBobX/RapidR/discussions) o abre un
[issue](https://github.com/iBobX/RapidR/issues): se genera, así que una
corrección llega a todos a la vez.

RapidR es gratuito y abierto, hecho para que cualquiera pueda aprender a
programar, y lo hacemos crecer juntos, personas e IA por igual. Gracias por
ser parte de esto.

---

## Cómo funciona

### Solo lo que tu programa usa, y tus propios nombres primero

`RWin32.inc` es grande, así que RapidR no lo compila entero dentro de tu
programa. Lee tu programa, ve qué nombres usa y le da esas declaraciones (y
los `TYPE` que necesitan), nada más.

**Un nombre propio de tu programa siempre gana**: una constante (`CONST`), una
declaración (`DECLARE`), un tipo (`TYPE`), una `SUB` o `FUNCTION`, una
variable (`DIM`), un objeto (`CREATE`), una definición (`$DEFINE`) o una
variable a la que le asignas un valor. Así, un programa antiguo que declara él
mismo `CONST WM_MOUSEWHEEL = &H20A`, o su propio `DECLARE FUNCTION
GetTickCount …`, se compila con `RWin32.inc` sin el error “ya está definido”:
se usa lo tuyo. (Una variable y un `TYPE` pueden compartir nombre: `DIM msg
AS MSG` funciona.)

### Los números de RAPIDQ.INC, y True y False

Los números de Windows que RAPIDQ.INC también tiene (`MB_OK`,
`MB_ICONERROR`, `VK_F1`, `SND_ASYNC`, los juegos de caracteres …) no se
repiten en `RWin32.inc`, así que los dos nunca chocan, venga primero el que
venga.

- En un programa `.bas` que incluye `RWin32.inc` **sin**
  `$INCLUDE "RAPIDQ.INC"`, la capa de compatibilidad con RapidQ de RapidR da
  los que usa, con los números de RAPIDQ.INC (los mismos de Windows), y
  `True` = 1 y `False` = 0, como hacía un `WIN32API.INC` clásico.
- En un programa de RapidR (`.rr`) ya están integrados, y `True` se queda en
  el -1 de RapidR.

### Nombres que RapidR también tiene: Sleep, Beep, MessageBox

Unas pocas funciones de Windows comparten nombre con algo que RapidR ya tiene
(`Sleep`, `Beep`, `MessageBox`, `SendMessage`, `PlaySound`).

- En un programa clásico (`.bas`), `RWin32.inc` te da la función de Windows,
  como hacía el archivo incluido antiguo: `Sleep 500` espera 500
  milisegundos, a través de kernel32.
- En un programa de RapidR (`.rr`), se queda la propia de RapidR: `SLEEP 1`
  espera un segundo en cualquier sistema. Para llamar allí a la de Windows,
  declárala con un nombre propio: `DECLARE SUB WinSleep LIB "kernel32" ALIAS
  "Sleep" (BYVAL ms AS LONG)`.

### 64 bits y 32 bits: `WIN64`

Cuando un programa se compila para 64 bits (Windows en x64 y ARM64, macOS,
Linux y la web), RapidR define `WIN64`, y `$IFDEF WIN64` elige la disposición
de 64 bits de cada estructura; para Windows de 32 bits (las descargas para
Windows 7 y XP) no está definido. Tu propio código también puede usarlo:

```basic
$IFDEF WIN64
PRINT "64-bit"
$ELSE
PRINT "32-bit"
$ENDIF
```

(La web también define `WIN64`, así que las estructuras de un programa tienen
allí los mismos tamaños que en las computadoras de escritorio de 64 bits; las
funciones de Windows no se ejecutan en la web de todos modos.)

### Cómo están escritas las declaraciones

- **Los handles, los punteros y otros números del tamaño de un puntero** son
  parámetros `LONG`: RapidR los pasa en un espacio de 64 bits y conserva por
  ti un puntero de 64 bits que devuelve una DLL
  ([Llamadas a DLL](language.md#sub-y-function)).
- **El texto** (`LPSTR`, `LPCSTR`) es `STRING`: dale texto, o un búfer
  `SPACE$(n)` que la función llena. El texto ancho (las funciones W) es un
  `LONG`: la dirección de un búfer UTF-16. Los nombres clásicos son las
  funciones A, que reciben cadenas de texto normales.
- **Una estructura** se pasa por referencia (`lprc AS RECT`). Da `0` para
  “ninguna” (`NULL`): RapidR pasa un puntero nulo.
- **En un `TYPE`**, un campo de puntero o de handle es un `INT64` en Windows de
  64 bits y un `LONG` en el de 32; un campo de puntero a texto es un `STRING`
  (y, en 64 bits, un `LONG` `…Hi` justo después, para la mitad alta del
  puntero, siempre 0); los huecos que el compilador de Windows deja entre
  campos son campos propios (`rrPad1`, `rrPad2` …), así que `SIZEOF` es el
  tamaño real de la estructura. Una unión sin nombre es su miembro más grande.
- **Una estructura pequeña pasada por valor** (`POINT` en `PtInRect`,
  `WindowFromPoint`) es un solo `INT64`: `x + y * 4294967296` (es decir, `y`
  en los 32 bits altos). Así la recibe Windows de 64 bits; en Windows de
  32 bits el mismo `INT64` da los mismos bytes.

### Lo que no incluye

Las constantes GUID, las interfaces COM, las funciones con un número variable
de argumentos (`wsprintf`) y unas pocas funciones que reciben una estructura
grande por valor. Algunas áreas quedan fuera por ahora (criptografía, GDI+,
ODBC, WinHTTP, IP Helper …): RapidR tiene componentes propios para la mayoría
de ellas (`RSQLITE`, `RDB`, `RSOCKET` …), y siempre puedes usar `DECLARE` para
lo que necesites, junto al archivo incluido.

### En macOS, Linux y la web

Un programa que incluye `RWin32.inc` se compila en todas partes; una llamada a
una DLL de Windows se ejecuta en Windows y en los demás sistemas se detiene
con un mensaje claro que nombra la función. Mira
[el ejemplo](../../../examples/windows/rwin32_system_info.rr).

### El archivo en sí

Puedes leerlo: [crates/rapidr-preprocessor/src/libraries/RWin32.inc](../../../crates/rapidr-preprocessor/src/libraries/RWin32.inc)
en el código fuente de RapidR. Su encabezado dice de dónde viene: lo genera
una herramienta propia de RapidR (`tools/gen-win32`) a partir de los
metadatos de la API de Win32 que Microsoft publica en el proyecto windows-rs,
bajo la licencia MIT, con el aviso de derechos de autor de Microsoft
([procedencia](../../legal/win32-declarations-provenance.md)).
`$INCLUDE "RWin32.inc"` siempre lee el archivo integrado de RapidR, nunca un
archivo con ese nombre que esté en tu disco.
