<!-- translated-from: README.md ad4c61b433cb8b9f -->
# El manual de usuario de RapidR

RapidR es un BASIC moderno con su propio IDE, RapidR Studio: una
implementación original, escrita desde cero en Rust puro, que funciona en tres
entornos de ejecución: ejecutables nativos, el intérprete (el RapidR Runtime) y
el navegador web. Los programas escritos para RapidQ también se ejecutan.

Este manual describe RapidR **2.119.0** (Preview 2). Dice lo que el código hace
hoy; donde algo aún no está hecho, lo indica.

**¿Eres nuevo por aquí?** Lee [Primeros pasos](getting-started.md) (sus tres
primeras líneas dicen qué descarga elegir y cómo ejecutar y compilar), y
después [Diseñar formularios en RapidR Studio](../designing-forms.md). ¿Quieres
saber en qué destaca RapidR? [Por qué RapidR](../why-rapidr.md).

## Para empezar

| Capítulo | Qué contiene |
|---|---|
| [Por qué RapidR](../why-rapidr.md) | Lo que RapidR hace mejor, solo con números medidos (con sus condiciones y los scripts que los produjeron), y lo que no afirma |
| [Galería](../showcase.md) | Programas más grandes escritos a partir de una sola idea, cada uno con una imagen y sus mejores líneas explicadas: fuegos artificiales, un visualizador MIDI que lee el propio archivo de la canción, y más |
| [Primeros pasos](getting-started.md) | Qué descarga elegir (el SDK o el Runtime), instalar en Windows, macOS y Linux, tus primeros programas de consola y de interfaz gráfica, ejecutar (`rapidr run`), compilar (`build-bc`), crear ejecutables (`build`) y la web |

## RapidR Studio

| Capítulo | Qué contiene |
|---|---|
| [El editor de código](studio-editor.md) | Autocompletado, Ctrl+Space, información de parámetros, información flotante, F12 Ir a la definición, problemas y correcciones rápidas, los atajos de teclado |
| [Proyectos](studio-projects.md) | El panel Proyecto: los formularios salen de tu código (varios en un archivo), Archivo > Abrir reciente, volver a abrir el último proyecto, el PATH que reciben tus programas (macOS, Linux) |
| [Studio en tu idioma](studio-languages.md) | Herramientas > Configuración > General > Idioma (Predeterminado del sistema, English, Español), que cambia al instante; los mensajes del compilador y la Ayuda en tu idioma; `RAPIDR_LANG`, `rapidr check`; cómo ayudar a traducir RapidR (`rapidr i18n`), el seudoidioma |
| [Diseñar formularios](../designing-forms.md) | Un tutorial (un programa de dos formularios, paso a paso), la caja de herramientas, mover y alinear, el inspector y los eventos, los editores de menús y de orden de tabulación, programas de varios formularios, el código que escribe el diseñador |
| [Depuración](debugging.md) | Ejecutar y detener, puntos de interrupción (condiciones, número de veces, mensajes de registro), ejecución paso a paso, Entrar en llamada específica, Establecer la siguiente instrucción, Pausar, valores como árboles (sugerencias de datos, Variables, Observar), la Pila de llamadas, Inmediato, errores de ejecución, qué tan rápido se ejecuta, el escritorio y el navegador |
| [El asistente de IA y RAI](ai-assistant.md) | Configurar un proveedor y una clave, preguntar, cambios en forma de diferencias que aceptas, “Hazme…”, qué se envió, privacidad y tokens; el componente `RAI` para tus propios programas, y cómo mantener las claves fuera de ellos |
| [Para docentes: bloqueo escolar](../for-teachers.md) (en inglés) | Desactivar la IA de Studio (RAI y las aplicaciones de IA) con una contraseña, lo que ve cada estudiante, desbloquear para una clase, la espera tras contraseñas incorrectas; para el equipo de informática de la escuela: el archivo de política de la computadora y `rapidr school-lock hash` |
| [Deja que las aplicaciones de IA trabajen en Studio](../ai-apps.md) | Activarlo (desactivado de forma predeterminada, solo en esta computadora, con un token), darle a una aplicación de IA la dirección y el encabezado, el menú de permisos (Solo lectura, Preguntar antes de cambiar, Permitir ediciones, Permitir todo), ver cada llamada en el panel Aplicaciones de IA, las herramientas, lo que pueden ver las aplicaciones de IA, el registro de actividad |
| [Archivos Markdown y RMarkdownView](../markdown.md) | Los archivos `.md` con formato en RapidR Studio (Vista previa \| Código fuente), y Markdown mostrado en tus propios programas, con sus bloques de código coloreados según el lenguaje (SQL, BASIC, JSON …) |
| [Versiones: nunca pierdas tu trabajo](versions.md) | Guardar versiones, la línea de tiempo, volver atrás, experimentos, carpetas compartidas, puntos de seguridad y Deshacer, la revisión de secretos, todo en palabras sencillas con los nombres de Git al lado (el panel de Studio llega en Preview 4); las vistas de texto, de formulario y de tabla de `RDiffView` en tus propios programas |

## El lenguaje y sus componentes

| Capítulo | Qué contiene |
|---|---|
| [El lenguaje](../language.md) | La estructura de un programa, tipos, variables, arreglos (arrays), operadores, control de flujo, `SUB` y `FUNCTION`, `TYPE` y objetos, el preprocesador, archivos, la consola, y en qué se diferencia el BASIC de RapidR de otros BASIC |
| [Tu programa en todos los idiomas](../translations.md) | `TR$` y `TRN$`, `Application.Language`, los archivos de traducción (`lang/es.po`) junto al programa, plurales, marcadores de posición, pasar de `es-AR` a `es`, textos que siguen un cambio de idioma, `Application.Languages` y `LanguageName`, las herramientas (`rapidr i18n`) y el seudoidioma |
| [Componentes y objetos](../components.md) | `CREATE`, propiedades, métodos y eventos; los nombres de RapidR y los nombres Q anteriores; los objetos globales; temas, alta densidad de píxeles (high-DPI) y accesibilidad |
| [Dibujo: lienzos, mapas de bits y píxeles](../graphics.md) | `RCanvas`, `RBitmap` y `RImage` dibujan de la misma manera; **dibujar muchos píxeles rápido**: `SetPixels`, `SetPixels8` y `GetPixels` pasan una imagen completa a un arreglo, o desde él, en una sola llamada (una pantalla de 256 x 192 en una décima de milisegundo, no en 70), `StretchDraw x, y, w, h` y `Smooth`, una imagen cada 20 ms con un `RTimer` |
| [Bases de datos](../databases.md) | RSQLite y RMySQL, enlace de parámetros, eventos, conexiones cifradas a MySQL (`SSLMode`) |
| [Los ojos y las manos de un programa](../app-eyes-and-hands.md) | `RWindowCapture`: imágenes de las propias ventanas del programa y un informe de lo que se ve mal (texto cortado, controles unos sobre otros); `RAppHands`: clics, texto y teclas reproducidos en ellas con un puntero que ves: recorridos, demostraciones, pruebas automáticas; cómo miran y usan tu programa en ejecución las aplicaciones de IA y RAI dentro de Studio |
| [Guardar secretos](../secrets.md) | `RSecrets`: contraseñas y claves en el llavero del sistema operativo, cada programa con el suyo, nunca mostradas por el depurador ni a una IA; ninguna en un navegador |
| [Archivos comprimidos: zip, 7z, tar y gzip](../archives.md) | `RArchive`: abrir archivos zip, 7z, tar.gz, .gz, .xz y .zst por sus primeros bytes, listarlos y descomprimirlos, crear archivos zip y tar.gz; se rechazan las bombas zip, las rutas fuera de la carpeta, los enlaces y las entradas cifradas; `COMPRESS$` / `DECOMPRESS$` |
| [Tus propios íconos SVG](../svg-icons.md) | `RImageList.AddSVGFile`, SVG que toman los colores del tema (`currentColor`, los tokens de color de RapidR), SVG incluidos en el programa con `$RESOURCE` |
| [Bases de datos](../databases.md) | RSQLite y RMySQL, enlace de parámetros, eventos; consultas en segundo plano (`QueryAsync`, `Cancel`) que llevan un millón de filas a un `RDBGrid`; nombres y tipos de columnas; el editor de SQL |
| [El editor de código en tus programas](../code-editor.md) | `RCodeEditor`: lenguajes, archivos, autocompletado; su modo SQL (dialectos, autocompletado a partir de tus tablas, Ctrl+Enter ejecuta la instrucción) |
| [DirectX y multimedia](../directx-and-media.md) | RDXScreen y los objetos Direct3D, sonido, joysticks, RMIDI / RWave / RVideo / RCDAudio, sonido creado muestra por muestra (RAudioOut), la bandeja del sistema, CGI, descargas |
| [Redes: sockets y un servidor web local](../network.md) | Servidores en `127.0.0.1` (solo esta computadora) o en la red; `RServerSocket` por líneas, bytes o solicitudes HTTP completas; `RHttpServer`: una API local, Server-Sent Events, el token, las solicitudes que rechaza por sí mismo; `RMcpServer`: las herramientas propias de tu programa para las aplicaciones de IA |
| [Recetas para programas de sistemas](../recipes-systems.md) | Diez programas pequeños sobre las entrañas de una máquina, una técnica cada uno, con una computadora diminuta entre todos: la memoria como un arreglo de bytes, bits e indicadores, una CPU que obtiene y decodifica, la pila, un volcado hexadecimal, un desensamblador, un archivo binario, la memoria de video en un lienzo, una matriz de teclado, ciclos por cuadro; qué tan rápido los ejecuta RapidR |
| [Puertos seriales y placas IoT](../serial-ports.md) | RComPort (nombre anterior QCOMPORT): abrir puertos, listarlos con sus ID de USB, líneas de entrada, eventos de conexión, reiniciar un ESP32 con DTR / RTS, Web Serial |

## Ciencia de datos y texto enriquecido

| Capítulo | Qué contiene |
|---|---|
| [Ciencia de datos con RapidR](../data-science-with-rapidr.md) | Un comienzo paso a paso: una tabla de un archivo en un formulario, en vivo en el diseñador sin código, la fila actual, filtrar y ordenar, un millón de filas, registros JSON |
| [Ciencia de datos](../data-science.md) | RNum, RDataFrame, RPlot, RJson y los componentes de datos vinculados (RCSV, RDataSource, RDBGrid): cada método, qué tan rápido, el tutorial del Explorador de CSV |
| [Texto enriquecido: RRichEdit](../rich-text.md) | Dar formato al texto desde un programa (`SelAttributes`), lo que escribe el usuario, párrafos, archivos RTF, lectores de pantalla |

## Compilar, importar y distribuir

| Capítulo | Qué contiene |
|---|---|
| [Compilar aplicaciones y sus íconos](../building-apps.md) | `Name.app`, el ícono y la versión del `.exe`, `Name.AppDir`; tu ícono o el de RapidR; Compilar de Studio; Compilar aplicación web |
| [Importar programas anteriores](../importing-rapidq.md) | Una copia de un programa anterior (`.bas`, `.rqw`, `.rqb`, `.rq`) con los nombres de RapidR, comprobada para que se compile al mismo bytecode: en RapidR Studio y con `rapidr import-rapidq`; archivos incluidos |
| [La web](../web.md) | Ejecutar programas en un navegador: paquetes, compilaciones `--web`, archivos y recursos, componentes solo para la web, ventanas más grandes que la página |
| [RapidR en otros editores](../other-editors.md) | La extensión de Visual Studio Code (qué `rapidr` usa, autocompletado, Ayuda junto al código, colores, proyectos, depuración), y lo que `rapidr lsp` y `rapidr dap` le ofrecen a cualquier editor que hable esos protocolos |
| [La CLI y el RapidR Runtime](../cli-and-runtime.md) | Todos los comandos de `rapidr`, los tipos de compilación, el Runtime, los tipos de archivo, `rapidr setup`, las variables de entorno, los avisos que llevan las compilaciones |

## Cuando algo es distinto o sale mal

| Capítulo | Qué contiene |
|---|---|
| [Diferencias y extensiones](../differences.md) | Lo que RapidR añade, lo que hace de otra manera a propósito y lo que no hace |
| [Solución de problemas](troubleshooting.md) | Mensajes frecuentes y qué hacer con ellos |

## Tablas de referencia

Generadas a partir del registro del lenguaje de RapidR (`rapidr lang export --manual`):

- [Componentes](../reference/components.md), y [sus nombres Q anteriores](../reference/classic-names.md)
- [Propiedades, métodos y eventos de cada componente](../reference/members.md)
- [Funciones integradas](../reference/builtins.md)
- [Instrucciones, directivas, palabras clave y tipos](../reference/statements.md)
- [Constantes](../reference/constants.md)
- [Miembros de ciencia de datos](../reference/data-science.md)

En otras partes del repositorio (estos enlaces funcionan en GitHub y en una copia del repositorio; una copia instalada del manual solo tiene su propia carpeta):

- [Los ejemplos](../../../examples/README.md), en el orden en que conviene leerlos
- [CHANGELOG.md](../../../CHANGELOG.md): qué cambió en cada versión
- [ROADMAP.md](../../../ROADMAP.md): qué viene después
- [LEGAL.md](../../../LEGAL.md): qué puedes hacer con RapidR y con los programas que compilas
- Cómo va la compatibilidad: la [suite de compatibilidad en Windows](../../rapidq-corpus-status.md) y [en macOS, Linux y la web](../../rapidq-corpus-status-unix.md)
- La referencia de cada componente y sus miembros está en
  [reference/](../reference/components.md), con las palabras propias de RapidR.

El SDK de RapidR instala este manual junto con sus otros documentos: en Windows
en `share\doc\rapidr\manual`, dentro de la carpeta de instalación
(`%LOCALAPPDATA%\Programs\RapidR`); en macOS dentro de la aplicación
(`RapidR Studio.app/Contents/Resources/doc/manual`); en Linux en
`/usr/share/doc/rapidr/manual` (el `.deb`) o en
`~/.local/share/doc/rapidr/manual` (el `install.sh` del `.tar.gz`).
