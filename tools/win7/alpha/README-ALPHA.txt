RapidR 2.121.0 for Windows 7, 8 and 8.1 (32-bit) - ALPHA 1
===========================================================

English first. Español más abajo (after the line of equal signs).


THIS IS AN ALPHA TEST BUILD.
It is made for trying RapidR on old PCs and telling us what breaks. It is
not the finished download. Please do not trust it with work you cannot lose.
This test build is provided as is, without warranty of any kind (see LICENSE).
Use it at your own risk and keep a copy of anything you care about.


What it is
----------
RapidR is a free (MIT licence) programming language and editor, compatible
with RapidQ. This folder is RapidR 2.121.0 built for Windows 7, 8 and 8.1,
32-bit, so that older PCs can use it. (On Windows 10 and 11 please use the
normal download from https://github.com/iBobX/RapidR instead.)
Windows 7, 8 and 8.1 no longer receive security updates from Microsoft. Use
this build on such a PC at your own risk, and preferably offline for anything
sensitive.


How to start
------------
1. Unzip the whole folder somewhere you can write to (your Desktop, or
   C:\RapidR). Not into "Program Files". Nothing is installed; to remove
   RapidR, delete the folder.
2. Double-click  RapidR-Studio.bat  to open RapidR Studio (the editor). A
   black window stays open behind Studio: that is normal in this alpha, and
   it shows any error text. Closing it closes Studio.
   The first start is slow on an old PC (up to a minute). Please wait.
3. In Studio, open a program from the  examples  folder and press Run.
4. Optional check (about 1 minute):  tests\run-tests.bat  runs five small
   tests and writes the answers to  tests\results.txt . If something goes
   wrong, send us that file.

The program itself is  rapidr.exe . For example, in a Command Prompt:
    rapidr.exe version
    rapidr.exe run examples\basics\hello.rr


What works
----------
 - RapidR Studio: the editor, the examples, the debugger and its panels.
 - Running programs (console programs and windows), including programs
   written for RapidQ.
 - It does not need a modern graphics card: the processor (the CPU) does
   the drawing.
It has run on Windows 7 (32-bit, no updates) in a virtual machine. We have
NOT yet tried it on real old PCs: that is what this test is for.


Known limits of this alpha
--------------------------
 - Making a stand-alone .exe from your program (the Build command,
   "rapidr build") is not in this alpha. Run your programs from Studio or
   with "rapidr.exe run".
 - Console programs: text colours, CLS and LOCATE may not work on Windows 7,
   8 and 8.1 (their console does not understand the codes RapidR sends).
   The text still appears.
 - Online things (the AI helper, downloads) need Windows 7 Service Pack 1
   with TLS 1.2 switched on (Microsoft's update KB3140245 does it) and
   up-to-date root certificates. On a PC that never got its updates they
   will probably fail. Everything else works offline.
 - Keys you type for the AI helper are not kept after you close Studio.
 - Slow first start, and slower drawing than on a new PC.
 - It is NOT signed. Windows may warn you: on Windows 8 and 8.1 (SmartScreen)
   click "More info", then "Run anyway"; on Windows 7 ("Open File - Security
   Warning") click "Run". Some antivirus programs distrust any new, unsigned
   program: if yours blocks it, that is a false alarm we want to hear about.


How to tell us what happened
----------------------------
Please write in the RapidQ group on groups.io (where you found this), or open
an issue: https://github.com/iBobX/RapidR/issues/new/choose
Tell us:
 - your Windows version and whether it is 32- or 64-bit;
 - the processor, the memory (RAM) and the graphics card, if you know them;
 - what you did, and what you saw instead of what you expected;
 - the file  tests\results.txt  if you ran the tests;
 - a photo or the exact words of any error window. If Windows says a DLL or
   a "procedure entry point" is missing, those words are the whole report.
Good news is welcome too: "it works on my PC" with the Windows version helps
as much as a bug.


Licences
--------
RapidR is MIT-licensed: LICENSE, NOTICE, LEGAL.md. The libraries inside it
are credited, with their full licence texts and copyright lines, in
THIRD-PARTY-NOTICES.txt (all the libraries, and Rust's standard library), and
also in THIRD_PARTY_NOTICES.md and LICENSES.md (open these files with Notepad
or WordPad). The few pieces of the C/C++ toolchain that the
program contains (the mingw-w64 runtime and LLVM's libunwind) have their
licence texts in the  licenses  folder. No part of this download is GPL or
LGPL.

Windows is a trademark of the Microsoft group of companies. RapidR is not
affiliated with or endorsed by Microsoft.


Support RapidR
--------------
RapidR is free and open source (MIT). If it is useful to you, you can support
its development: https://github.com/sponsors/iBobX or
https://www.buymeacoffee.com/roanbema
Support is voluntary, received by Ruta Internet S.R.L., and comes with
nothing in return: RapidR stays free for everyone. You can give once or
monthly; monthly support renews every month until you cancel, any time, in
your GitHub Sponsors or Buy Me a Coffee settings. Payment is handled by
GitHub or Buy Me a Coffee, never by RapidR.


=============================================================================

RapidR 2.121.0 para Windows 7, 8 y 8.1 (32 bits) - ALFA 1
=========================================================

ESTA ES UNA VERSIÓN ALFA DE PRUEBA.
Sirve para probar RapidR en PCs antiguas y decirnos qué falla. No es la
descarga definitiva. Por favor, no le confíe trabajo que no pueda perder.
Esta versión de prueba se ofrece tal cual, sin garantía de ningún tipo (vea
LICENSE). Úsela bajo su propia responsabilidad y guarde una copia de todo lo
que le importe.


Qué es
------
RapidR es un lenguaje de programación y editor gratuito (licencia MIT),
compatible con RapidQ. Esta carpeta es RapidR 2.121.0 compilado para
Windows 7, 8 y 8.1 de 32 bits, para que lo usen los PCs más viejos. (En
Windows 10 y 11 use mejor la descarga normal de
https://github.com/iBobX/RapidR.)
Windows 7, 8 y 8.1 ya no reciben actualizaciones de seguridad de Microsoft.
Use esta versión en un PC así bajo su propia responsabilidad y, para todo lo
delicado, mejor sin conexión.


Cómo empezar
------------
1. Descomprima toda la carpeta en un sitio donde se pueda escribir (el
   Escritorio, o C:\RapidR). No en "Archivos de programa". No se instala
   nada; para quitar RapidR, borre la carpeta.
2. Haga doble clic en  RapidR-Studio.bat  para abrir RapidR Studio (el
   editor). Detrás de Studio queda abierta una ventana negra: es normal en
   esta alfa, y muestra el texto de cualquier error. Si la cierra, se cierra
   Studio. El primer arranque es lento en un PC viejo (hasta un minuto).
   Espere, por favor.
3. En Studio abra un programa de la carpeta  examples  y pulse Run.
4. Comprobación opcional (1 minuto aprox.):  tests\run-tests.bat  hace cinco
   pruebas pequeñas y guarda el resultado en  tests\results.txt . Si algo
   falla, envíenos ese archivo.

El programa en sí es  rapidr.exe . Por ejemplo, en una ventana de comandos:
    rapidr.exe version
    rapidr.exe run examples\basics\hello.rr


Qué funciona
------------
 - RapidR Studio: el editor, los ejemplos, el depurador y sus paneles.
 - Ejecutar programas (de consola y con ventanas), también los escritos para
   RapidQ.
 - No necesita una tarjeta gráfica moderna: el procesador (la CPU) hace el
   dibujo.
Ha funcionado en Windows 7 (32 bits, sin actualizaciones) dentro de una
máquina virtual. TODAVÍA no lo hemos probado en PCs antiguas reales: para
eso es esta prueba.


Límites conocidos de esta alfa
------------------------------
 - Crear un .exe independiente de su programa (el comando Build,
   "rapidr build") no está en esta alfa. Ejecute sus programas desde Studio
   o con "rapidr.exe run".
 - Programas de consola: los colores del texto, CLS y LOCATE pueden no
   funcionar en Windows 7, 8 y 8.1 (su consola no entiende los códigos que
   envía RapidR). El texto sí aparece.
 - Lo que usa internet (el asistente de IA, las descargas) necesita Windows 7
   con el Service Pack 1, TLS 1.2 activado (la actualización KB3140245 de
   Microsoft lo hace) y los certificados raíz al día. En un PC que nunca se
   actualizó probablemente fallará. Todo lo demás funciona sin conexión.
 - Las claves que escriba para el asistente de IA no se guardan al cerrar
   Studio.
 - Primer arranque lento y dibujo más lento que en un PC nuevo.
 - NO está firmado. Windows puede avisarle: en Windows 8 y 8.1 (SmartScreen)
   pulse "Más información" y luego "Ejecutar de todas formas"; en Windows 7
   ("Abrir archivo - Advertencia de seguridad") pulse "Ejecutar". Algunos
   antivirus desconfían de cualquier programa nuevo sin firmar: si el suyo lo
   bloquea, es una falsa alarma y queremos saberlo.


Cómo contarnos lo que pasó
--------------------------
Escriba en el grupo de RapidQ en groups.io (donde encontró esto), o abra una
incidencia: https://github.com/iBobX/RapidR/issues/new/choose
Díganos:
 - su versión de Windows y si es de 32 o de 64 bits;
 - el procesador, la memoria (RAM) y la tarjeta gráfica, si los sabe;
 - qué hizo y qué vio en lugar de lo esperado;
 - el archivo  tests\results.txt  si hizo las pruebas;
 - una foto o las palabras exactas de cualquier ventana de error. Si Windows
   dice que falta una DLL o un "punto de entrada del procedimiento", esas
   palabras son todo el informe.
Las buenas noticias también valen: "funciona en mi PC", con la versión de
Windows, ayuda tanto como un fallo.


Licencias
---------
RapidR tiene licencia MIT: LICENSE, NOTICE, LEGAL.md. Las bibliotecas que
lleva dentro se reconocen, con sus textos de licencia completos y sus líneas de
copyright, en THIRD-PARTY-NOTICES.txt (todas las bibliotecas y la biblioteca
estándar de Rust), y también en THIRD_PARTY_NOTICES.md y LICENSES.md (abra
esos archivos con el Bloc de notas o WordPad). Las pocas piezas de la cadena de
herramientas C/C++ que contiene el programa (el runtime de mingw-w64 y la
libunwind de LLVM) tienen sus textos de licencia en la carpeta  licenses .
Nada de esta descarga es GPL ni LGPL.

Windows es una marca comercial del grupo de empresas Microsoft. RapidR no
está afiliado a Microsoft ni cuenta con su respaldo.


Apoye RapidR
------------
RapidR es gratis y de código abierto (MIT). Si le sirve, puede apoyar su
desarrollo: https://github.com/sponsors/iBobX o
https://www.buymeacoffee.com/roanbema
El apoyo es voluntario, lo recibe Ruta Internet S.R.L. y no da nada a
cambio: RapidR sigue siendo gratis para todos. Puede dar una vez o cada mes;
el apoyo mensual se renueva cada mes hasta que lo cancele, en cualquier
momento, en los ajustes de GitHub Sponsors o de Buy Me a Coffee. El pago lo
gestiona GitHub o Buy Me a Coffee, nunca RapidR.
