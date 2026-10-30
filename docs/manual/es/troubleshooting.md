<!-- translated-from: troubleshooting.md 2863513ef74f168c -->
# Solución de problemas

## Instalar y empezar

**macOS: “RapidR Studio” no se abrió: Apple no pudo verificar que no contiene
software malicioso** (*“RapidR Studio” Not Opened — Apple could not verify it is
free of malware*) — las descargas de RapidR todavía no están firmadas. Haz clic
en **Listo** (*Done*), abre **Configuración del Sistema > Privacidad y
seguridad** (*System Settings > Privacy & Security*), baja hasta *Seguridad*
(*Security*) y haz clic en **Abrir de todos modos** (*Open Anyway*), junto a
“RapidR Studio” fue bloqueada (después, tu contraseña o Touch ID). Una vez es
suficiente. Clic derecho > Abrir no funciona en macOS 15 ni en las versiones
posteriores. En la Terminal:
`xattr -dr com.apple.quarantine "/Applications/RapidR Studio.app"`. Con capturas
de pantalla: [Instalar en macOS](getting-started.md#instalar-en-macos).

**Windows: “Windows protegió su PC”** (*Windows protected your PC*) — SmartScreen,
por el mismo motivo. Haz clic en **Más información** (*More info*) y luego en
**Ejecutar de todos modos** (*Run anyway*)
([Instalar en Windows](getting-started.md#instalar-en-windows)).

**Linux: `N: Download is performed unsandboxed as root as file … couldn't be
accessed by user '_apt'`** — es el aviso de apt sobre un `.deb` que está en tu
carpeta personal. Es inofensivo: la instalación continuó.

**Linux: `error while loading shared libraries: libssl.so.3`** — RapidR necesita
OpenSSL 3: Ubuntu 22.04 o Debian 12 y posteriores (`sudo apt install libssl3`).
Ubuntu 20.04 y Debian 11 solo tienen OpenSSL 1.1 y no son compatibles.

**Windows: “Esta aplicación no se puede ejecutar en el equipo”** (*This app can't
run on your PC*) — tienes el instalador para el otro tipo de procesador. Mira
*Configuración > Sistema > Acerca de > Tipo de sistema* (*Settings > System >
About > System type*) y elige el instalador `x64` o el `arm64` que corresponda
(mira [Instalar en Windows](getting-started.md#instalar-en-windows)).

**Linux: `apt` dice `Unable to locate package rapidr_2.119.0_amd64.deb`** — pon
`./` delante del nombre del archivo (`sudo apt install
./rapidr_2.119.0_amd64.deb`) para que apt lo lea como un archivo de esta
carpeta.

**Linux: `dpkg` dice "package architecture (amd64) does not match system
(arm64)"** — es el archivo equivocado para esta máquina.
`dpkg --print-architecture` dice cuál elegir (`amd64` o `arm64`).

**`rapidr: command not found`** — ejecuta `rapidr setup` desde la instalación
(macOS: `/Applications/RapidR Studio.app/Contents/MacOS/rapidr setup`) para
enlazarlo a tu PATH; en Windows, marca “Add rapidr to PATH” en el instalador.

**“This program needs RapidR Runtime x.y.z or newer”** — el programa lo compiló
una versión más nueva de RapidR. Instala el Runtime más nuevo (el mensaje enlaza
con las versiones publicadas).

**Un programa descargado pregunta antes de ejecutarse** — es a propósito: un
programa de internet pregunta una sola vez; la respuesta se recuerda para ese
archivo exacto.

## Compilar

**`Failed to run cargo … Native builds compile with Rust`** — ejecuta
`rapidr setup`. Las compilaciones nativas son las únicas que necesitan Rust:
`rapidr run`, `build --interp` y `bundle-bc` no lo necesitan.

**macOS: `codesign … 3dcube.app: No such process`** — `codesign` lee un nombre
que empieza con un dígito como un número de proceso. `rapidr build` ahora pasa la
ruta completa de la aplicación y la firma sin problema; cuando ejecutes
`codesign` tú mismo, escribe `./3dcube.app` (mira [El nombre de la
aplicación](../building-apps.md#the-apps-name)).

**La primera compilación nativa tarda minutos** — compila el entorno de
ejecución de RapidR una sola vez; las siguientes compilaciones lo reutilizan
(conserva `CARGO_TARGET_DIR` si definiste uno).

**Windows: `Access is denied. (os error 5)` en el programa nuevo, o `Your
antivirus blocked the new program`** — mira [Antivirus](#antivirus).

**`run-time error: 'GetTickCount' is a Windows function (kernel32): this
program calls Windows itself, so it runs on Windows only. For every system,
use TIMER`** — el programa llama a las DLL de Windows, que solo existen en
Windows. Ejecútalo allí, o usa el equivalente portátil que nombra el mensaje.

**`'X.DLL' is a 32-bit DLL; RapidR programs are 64-bit, so Windows can't
load it`** — la DLL vino con un programa antiguo; hace falta una versión de 64
bits de ella (o el propio componente de RapidR para ese trabajo).

**`can't find the DLL 'X.DLL'`** — la DLL no es una de las de Windows y no está
en la carpeta actual ni junto al programa.

**`the call to F in user32 crashed (access violation …)`** — a la DLL se le dio
algo que no es el puntero o el identificador que espera: a menudo un `TYPE`
cuyos campos guardan punteros o identificadores como `LONG` (el Windows de 64
bits los quiere de 8 bytes de ancho), o el `Handle` de un control (solo el de un
formulario es una ventana real).

**`'SetWindowLongA' is given CODEPTR(WndProc), a callback …`** — las `SUB` que se
entregan a una DLL para que las llame de vuelta todavía no son compatibles.

**`address … isn't memory of this program`** — PEEK / POKE / MEMCPY solo llegan a
la memoria propia del programa (`VARPTR` de una variable, de un elemento, de un
`TYPE`, el `Pointer` de un flujo) y a las páginas de la consola.

**`RUSTSTART ... RUSTEND blocks only work in native builds`** — como dice.

**`--web` falla con "Failed to run wasm-bindgen"** — las compilaciones web nativas
necesitan `rustup target add wasm32-unknown-unknown` y `cargo install
wasm-bindgen-cli --version 0.2.129`. `rapidr bundle-bc` no necesita ninguno de
los dos.

## Antivirus

Un antivirus de Windows examina cada `.exe` nuevo en el momento en que se
escribe. Un programa que nunca ha visto (y cada programa que compilas es nuevo)
no puede estar en su lista de programas conocidos, así que lo juzga por
conjeturas, y la conjetura a veces es errónea: una **falsa alarma** (un *falso
positivo*). No le pasa nada a tu programa ni a RapidR.

**Qué ves**

- Una compilación nativa se detiene con `…\release\<programa>.exe: Access is
  denied. (os error 5)` y luego `Your antivirus blocked the new program`. El
  antivirus retiene el archivo: RapidR no puede leerlo.
- O `The new program disappeared right after it was built`: el antivirus lo
  movió a su cuarentena.
- O el aviso propio del antivirus (“amenaza bloqueada”, un nombre como
  `Gen:Variant…`).

**Qué hace RapidR**

- Espera hasta 10 segundos, por si el antivirus solo lo está examinando (por lo
  general lo suelta), y luego sigue con la compilación.
- Un programa nativo se enlaza con información de versión y un manifiesto (no
  pide más derechos que los que tienes), así que parece un programa corriente. En
  las pruebas propias de RapidR, un antivirus que rechazaba programas nativos
  pequeños sin ellos dejó pasar los mismos programas con ellos. El ícono y los
  detalles de tu aplicación se escriben encima después.
- No puede imponerse al antivirus. Si aun así bloquea el programa, el mensaje
  nombra las carpetas de las que hay que ocuparse: la carpeta de compilación de
  RapidR (de forma predeterminada `%LOCALAPPDATA%\RapidR\Cache`, o tu
  `RAPIDR_BUILD_CACHE`) y la carpeta a la que va la aplicación.

**Qué haces tú**

1. Abre los avisos o la cuarentena del antivirus, restaura el programa y elige
   *permitir* (o *confiar*).
2. O añade las dos carpetas a las excepciones del antivirus y vuelve a
   compilar. Hazlo solo con carpetas que contengan tus propios programas.
3. **Avisa a quien fabrica el antivirus.** Las falsas alarmas se corrigen cuando
   se informan: cada fabricante tiene una página para eso (busca el nombre del
   producto y “report a false positive”). Envía el archivo de la cuarentena y el
   nombre de la amenaza que aparece en el aviso. La corrección suele llegar en
   unos días, en una actualización, para todos.

**Firmar.** Un programa firmado con un certificado de firma de código es mucho
más confiable para los antivirus y para SmartScreen. Cuando le das una aplicación
a otras personas, firmarla con tu propio certificado (`signtool sign …`) es la
verdadera cura para las advertencias y las falsas alarmas; las descargas propias
de RapidR todavía no están firmadas ([Compilar aplicaciones](building-apps.md#windows)).
Una aplicación interpretada independiente (`rapidr build prog.bas --interp`) se
hace con el ejecutor ya preparado de RapidR; en las pruebas de RapidR no fue
rechazada.

## La salida del programa

**`PRINT a, b` imprime los valores pegados** — la coma funciona como un punto y
coma. Pon tú los espacios, o usa `TAB(n)` / `FORMAT$`.

**Los números se imprimen con nueve decimales (`3.500000000`)** — es el formato
clásico. Usa `FORMAT$("%.2f", x)` o `STRF$` para otros.

**Un número grande se imprime como `-2147483648`** — `PRINT` muestra un número
entero como un entero de 32 bits. `STR$(x)` da `1E10`, `FORMAT$("%.0f", x)` da
`10000000000`.

**`i = 2.7` guarda 2** — las asignaciones a variables enteras truncan. Usa
`ROUND` (que es `INT(x + 0.5)`) cuando quieras redondear.

**Una cadena de texto con `""` dentro sale mal** — en el BASIC de RapidR, `""` no
es una comilla con escape. Usa `CHR$(34)` o `$ESCAPECHARS ON` y `\"`.

**`ON ERROR GOTO` no captura un error** — se acepta y se ignora; los errores de
ejecución terminan el programa con su mensaje y su línea.

**`[WARN] x.Method() not implemented for type …`** — el componente no tiene ese
miembro en este entorno de ejecución (por ejemplo, un componente solo para la web
en el escritorio). El programa continúa.

## Ventanas y dibujo

**Una ventana en blanco o que falla en una máquina virtual o por escritorio
remoto** — RapidR dibuja con la CPU cuando la única GPU es una por software; si
la detección falla, define `RAPIDR_RENDERER=cpu`.

**El programa no se ve como el aspecto clásico de Windows** — los programas se
dibujan con el aspecto propio de RapidR, a menos que digan `$THEME Classic` (el
aspecto clásico; los tamaños y las posiciones son los mismos en ambos). Revisa
también si hay una variable `RAPIDR_THEME` en tu entorno, que da su tema a los
programas que no nombran ninguno ([Temas](components.md#temas)).

## La web

**Una página en blanco desde un paquete** — sirve la carpeta por HTTP
(`python3 -m http.server -d carpeta 8080`); los navegadores no cargan
WebAssembly desde `file://`.

**RMySQL / RServerSocket no hacen nada en el navegador** — los navegadores no
pueden abrir conexiones TCP directas. Llama a un servidor tuyo mediante RHttp.

**Un archivo que lee el programa no se encuentra en la web** — los archivos junto
al programa se incluyen en la página solo con las extensiones de la lista
(`.csv`, `.db`, `.txt`, `.png`, …); los demás nombres se piden al servidor de la
página. Mira [La web](web.md#archivos-y-recursos).

## Datos y tablas

**Un `RCSV` o un `RDBGrid` no muestra filas** — no se leyó el archivo: el `Error`
del `RCSV` dice por qué (`there is no file "people.csv"`) y se dispara su
`OnError(Message)`; el programa continúa. Un `FileName` sin carpeta se busca en la
carpeta actual y después junto al programa; en un navegador, el archivo tiene que
ser uno de los de la página ([Ciencia de datos con RapidR](data-science-with-rapidr.md#conviene-saber)).

**El diseñador no muestra datos de un componente vinculado** — el diseñador solo
lee archivos dentro de tu proyecto (junto al programa), y solo para leerlos.

## El asistente de IA y RAI

**No hay panel ni botón Asistente** — aparece cuando el asistente está activado y
configurado: un proveedor, una clave (un servidor de modelos en tu propia
computadora no necesita ninguna) y un modelo. **Herramientas > Configuración de
IA…** muestra lo que falta
([Configurarlo](ai-assistant.md#configurarlo)).

**“an API key is written in the program…” al compilar** — una clave dentro de un
programa puede leerla cualquiera que tenga la aplicación. Quítala y haz que el
programa la pida: `APIKeySource = "env:NAME|ask"`
([Claves en los programas que das a otras personas](ai-assistant.md#claves-en-los-programas-que-das-a-otras-personas)).

**La respuesta de un proveedor es un error** — `OnError` y el asistente lo dicen
en palabras sencillas: una clave rechazada, sin crédito, un modelo que tu clave no
puede usar, demasiadas solicitudes. En una página web, un servidor de modelos de
tu computadora (Ollama, LM Studio) tiene que permitir la dirección de la página
([Proveedores](ai-assistant.md#proveedores)).

## Cuando RapidR falla

<!-- legal -->
RapidR Studio está escrito para que un error en su propio código no lo termine:
el error se dice, tu trabajo sin guardar se conserva y Studio sigue funcionando.
Cuando pasa algo peor (un error en el propio código de RapidR que termina Studio
o tu programa, un desbordamiento de pila o la ventana de Studio sin responder),
RapidR escribe un **informe en esta computadora**. No se envía nada a ninguna
parte: RapidR no tiene telemetría. Tú decides si alguien ve un informe.

**La próxima vez que se inicie Studio**, una franja bajo la barra de herramientas
lo dice:

![La última vez, RapidR Studio se cerró de forma inesperada: Ver informe, Copiar informe, Guardar informe..., Informar en GitHub..., Ocultar](../images/crash/crash-banner-light.png)

- **Ver informe** lo abre en una pestaña; **Copiar informe** copia su texto (para
  pegarlo donde pidas ayuda); **Guardar informe...** guarda una copia donde
  elijas.
<!-- legal -->
- **Informar en GitHub...** dice primero qué pasa: tu navegador abre la página de
  RapidR para un problema nuevo en GitHub, con el informe ya escrito. No se envía
  nada hasta que lo envías tú allí. Necesitas una cuenta de GitHub (las
  condiciones de GitHub piden tener 13 años o más; un estudiante más joven puede
  pedirle a un docente, a su madre, a su padre o a su tutor que lo informe); los
  problemas informados son públicos, así que lee primero el informe. Un informe
  demasiado largo para la dirección de la página se copia, para pegarlo en el
  problema.
- **Ocultar** cierra la nota. Con cualquier respuesta, Studio no vuelve a
  preguntar.
- También se ofrece tu trabajo sin guardar (“¿Restaurar tu trabajo sin
  guardar?”), como después de cualquier final inesperado.
- Con el Bloqueo escolar (*School Lock*) activado, la franja no se muestra.

**Cuando tu programa se detiene** por un problema en RapidR (no por un error en
tu código: esos son errores de ejecución, que se muestran como siempre), la Salida
dice dónde estaba, con enlaces a la línea y al informe:

```
[main.rr terminó, código de salida 101]
Tu programa se detuvo: index out of bounds, en main.rr línea 24, en SUB Tick (OnTimer de Ticker).
RapidR lo detuvo: es un problema de RapidR, no de tu código. Un informe queda en esta computadora: Ver informe
```

**Cuando Studio deja de responder** durante 5 segundos, se escribe de inmediato
un informe de bloqueo y una ventana pequeña pregunta: *RapidR Studio no
responde*: **Esperar** (vuelve a preguntar a los 30 segundos si sigue sin
responder) o **Guardar copias y salir** (Studio se cierra; las copias de tu
trabajo sin guardar que conserva mientras escribes, cada 20 segundos, se ofrecen
cuando vuelve a iniciarse). Si Studio vuelve a responder, la ventana desaparece
sola y el informe dice cuánto tardó.

![RapidR Studio no responde: Esperar, Guardar copias y salir](../images/crash/crash-hang-light.png)

<!-- legal -->
**Qué contiene un informe**: qué pasó en palabras sencillas y el mensaje del
error; la versión de RapidR y su commit de git; el nombre de tu sistema; dónde
estaba tu programa o Studio (archivo, línea, `SUB` o `FUNCTION`, el evento que se
atendía); qué estaba haciendo Studio (su último comando); el seguimiento de pila
de Rust (los nombres de las funciones de RapidR); los nombres de las últimas
llamadas de IA y de MCP (nunca lo que se envió); los últimos 50 mensajes propios
de RapidR. **Lo que nunca contiene**: la ruta de tu carpeta personal (aparece
como `~`), el contenido de tus archivos, claves, tokens ni contraseñas (se quitan
por su forma, y también cualquier valor que tu programa haya marcado como
secreto), ni valores de tu programa.

<!-- legal -->
**Dónde están**: `~/Library/Application Support/RapidR/crashes` (macOS),
`%APPDATA%\RapidR\crashes` (Windows), `~/.local/share/rapidr/crashes` (Linux), un
archivo de texto cada uno, legible solo por ti. Los informes de más de 30 días, y
los que quedan después de los 20 más recientes, se eliminan. El ejemplo
[`examples/studio/crash_reports.rr`](../../../examples/studio/crash_reports.rr)
los enumera (`CrashList`, `CrashText` y `CrashField` de RProject).

**Lo que todavía no cubre**: las aplicaciones que compilas y das a otras personas
(los informes voluntarios para ellas llegarán más adelante); RapidR Studio en la
web; en Windows, un desbordamiento de pila termina el programa con el mensaje del
sistema y sin informe (un pánico sí se informa). Después de un desbordamiento de
pila o de una señal fatal, el informe es más corto, sin seguimiento de pila: en
ese momento solo se puede leer con seguridad el lugar de tu programa. Un informe
de bloqueo nombra los hilos, pero no tiene el seguimiento de pila del que se
quedó atascado.

## Informar un problema

Abre un problema (*issue*) en GitHub (hay formularios para errores, para
compatibilidad y para informes de pruebas): <https://github.com/iBobX/RapidR/issues>.
Un programa anterior que se comporta de otra manera en RapidR es un error: por
favor, inclúyelo. Informa los problemas de seguridad en privado, como explica
[SECURITY.md](../../../SECURITY.md).
