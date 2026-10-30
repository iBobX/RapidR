<!-- translated-from: cli-and-runtime.md 87146dfd490330b9 -->
# La CLI y el RapidR Runtime

Un solo comando, `rapidr`, es el compilador, la herramienta de compilación y el
entorno de ejecución.

## Comandos

| Comando | |
|---|---|
| `rapidr run <file> [args]` | Ejecuta un programa: un `.rrbc`, o un código fuente (`.rr` / `.bas`, o las extensiones de código fuente anteriores `.rqw`, `.rqb` y `.rq`; se compila primero en memoria). Esto es el RapidR Runtime. |
| `rapidr <file.rrbc> [args]` | Lo mismo, para un programa compilado (y para los scripts `#!/usr/bin/env rapidr`) |
| `rapidr open <file> [args]` | Lo ejecuta como lo hace un doble clic: un archivo descargado pregunta primero, un programa de consola recibe una terminal |
| `rapidr info <file>` | El tipo del programa, el formato del bytecode y el Runtime más antiguo que necesita |
| `rapidr ide [file]` | El IDE (SDK) |
| `rapidr examples [copy <name\|all> [folder]]` | Los programas de ejemplo: se listan, o se copian (con sus archivos incluidos y de datos) a una carpeta para jugar con ellos — por nombre (`linked_data`), por tema (`data/linked_data`, `rapidq/shapes`) o `all`. Los archivos que ya están en la carpeta nunca se sobrescriben. RapidR Studio hace la misma copia, en `Documents/RapidR/Examples/<nombre>`, cuando abres un ejemplo donde RapidR está instalado |
| `rapidr serve <file> [--open]` | La compilación web de un programa, servida solo en esta computadora (127.0.0.1); `--open` la abre en el navegador |
| `rapidr build <file> [outdir] [flags]` | Compila un ejecutable o una compilación web (más abajo) |
| `rapidr <file.rr\|.bas> [flags]` | Atajo de `build`, optimizado de forma predeterminada |
| `rapidr build <app.rrproj>` | Compila el archivo principal de un proyecto con la configuración de su aplicación |
| `rapidr install-app <Name.AppDir>` | Linux: pone una aplicación compilada en tu menú de aplicaciones |
| `rapidr check <file…>` | Lo que el panel Problemas de Studio enumera para el programa: sus errores y advertencias con su ubicación, y debajo de un error que es una costumbre de otro lenguaje, una línea `  hint:` con la forma de RapidR ([¿Vienes de otro lenguaje?](language.md#vienes-de-otro-lenguaje)). No se compila ni se ejecuta nada; el estado de salida es 1 cuando hay un error |
| `rapidr build-bc <file> [-o out.rrbc]` | Compila a bytecode, para el Runtime |
| `rapidr bundle-bc <file> [-o out.zip]` | Un paquete web estático (un `.zip`) |
| `rapidr import-rapidq <file\|folder\|.rrproj> [out_dir]` | Escribe una copia convertida de un programa anterior —y de cada archivo que este incluye con `$INCLUDE`— con los nombres de RapidR, y un informe (`rapidr-import-report.md`), en `out_dir` (también `-o out_dir`; si no, `<nombre>-rapidr` junto a él). El original no se toca, y se comprueba que cada programa se compila al mismo bytecode. Un archivo incluido que no se encuentra en ninguna parte se imprime con lo que hay que hacer (código de salida 1). El comando de importación del menú **Archivo** de RapidR Studio hace lo mismo ([Archivos incluidos](importing-rapidq.md#archivos-incluidos)) |
| `rapidr upgrade-names <file> [--dry-run]` | La misma conversión en el mismo archivo, para los archivos propios de RapidR; `--dry-run` imprime las diferencias y no cambia nada |
| `rapidr setup [--check] [--yes] [--no-path]` | Instala el Rust que usan las compilaciones nativas; pone `rapidr` en el PATH. `rapidr setup --rust` solo responde si las compilaciones nativas pueden ejecutarse (código de salida 0 sí, 1 no); un `rapidr build` nativo sin Rust se detiene y dice que uses `--interp` o ejecutes `rapidr setup` |
| `rapidr notices [<os>-<arch>\|web\|tools-<os>] [-o file]` | Imprime los avisos de terceros que lleva un tipo de compilación |
| `rapidr ai providers`, `rapidr ai models --provider P`, `rapidr ai ask --provider P [--model M] "question"` | Los proveedores de IA de RapidR desde una terminal: cuáles hay, los modelos propios de un proveedor, una pregunta y su respuesta a medida que llega; la clave viene de la variable de entorno del proveedor o del llavero ([El asistente de IA y RAI](ai-assistant.md)) |
| `rapidr school-lock hash [--stdin]`, `rapidr school-lock check [file]` | El bloqueo escolar para el equipo de informática de una escuela: la línea `password_hash` del archivo de política para una contraseña escrita dos veces (nunca se muestra ni se guarda), y qué bloquea la política de la computadora (o un archivo) ([Para docentes](for-teachers.md#para-el-equipo-de-informática-de-la-escuela-la-política-de-la-computadora)) |
| `rapidr lsp`, `rapidr dap` | El servidor de lenguaje y el adaptador de depuración (protocolos estándar sobre stdin y stdout), para otros editores como VS Code |
| `rapidr lang export --json\|--prompt\|--manual\|--all` | Lo que genera el registro del lenguaje: los datos para el autocompletado y la información flotante, un texto de instrucciones para la IA, y [las tablas de referencia](../reference/components.md) de este manual |
| `rapidr about`, `rapidr version` | |
| `rapidr --log <file> <command…>` | La salida de un comando (y la de cargo) en un archivo |
| `rapidr preprocess`, `lex`, `parse`, `codegen <file> [outdir]` | Las etapas del compilador, por curiosidad y para informes de errores |

Todo comando que compila acepta **`--include-dir <folder>`** (cualquier cantidad
de veces): una carpeta donde `$INCLUDE` busca después de la carpeta propia del
programa. Escríbelo antes de los argumentos propios del programa:
`rapidr run --include-dir ~/include prog.bas`. Cómo encuentra `$INCLUDE` su
archivo: [Archivos incluidos](importing-rapidq.md#archivos-incluidos).

## Compilaciones

```sh
rapidr build prog.bas --interp             # independiente, con el intérprete adentro
rapidr build prog.bas --interp --target windows-x86_64
rapidr build prog.bas                      # nativa: una compilación final, optimizada
rapidr build prog.bas --debug              # nativa, para depurar (se compila más rápido)
rapidr build prog.bas dist                 # la aplicación en dist/
rapidr build prog.bas --keep-rust          # y el Rust generado en prog-rust-source/
rapidr build prog.bas --web                # código nativo para el navegador
rapidr build prog.bas --web --interp       # paquete web: prog-web.zip
```

| Opción | |
|---|---|
| `--interp`, `-i` | el intérprete de bytecode: no necesita Rust |
| `--release`, `-r` / `--debug`, `-d` | optimizada, la que distribuyes (el valor predeterminado) / rápida de compilar, para depurar |
| `[output folder]`, `--output <folder>`, `-o <folder>` | dónde va la aplicación: si no, la carpeta de salida del proyecto (`build`); si no, la carpeta del código fuente |
| `--keep-rust` / `--no-keep-rust` | una compilación nativa deja también el Rust que generó en `<salida>/prog-rust-source/` (con un README) / lo elimina (el valor predeterminado; el `keep_rust` de un proyecto dice lo contrario) |
| `--web`, `-w` | para el navegador (también cuando el programa dice `$APPTYPE WEB`) |
| `--csp "connect-src https://api.example.com; …"` | (compilaciones web, y `bundle-bc`) orígenes que la Content-Security-Policy de la página añade a lo que necesitan los componentes del programa ([Compilar aplicaciones](building-apps.md#compilar-una-aplicación-web)) |
| `--icon`, `--name`, `--bundle-id`, `--app-version`, `--company`, `--project`, `--bundle` / `--no-bundle`, `-g<icon>` | la aplicación en que se convierte y su ícono: [Compilar aplicaciones y sus íconos](building-apps.md) |
| `--target <os>-<arch>` | una compilación interpretada para otra arquitectura, a partir de los ejecutores (runners) que incluye el SDK: en Windows `windows-x86_64` y `windows-aarch64`; en macOS `macos` (universal, el valor predeterminado), `macos-arm64`, `macos-x86_64`; en Linux la propia de la máquina (`linux-x86_64` o `linux-aarch64`). Una copia del código fuente compila con cargo el ejecutor que necesita |

Adónde va cada cosa — la carpeta de salida: la que indicas; si no, la del
proyecto (`[build] output`, `build` de forma predeterminada) cuando el programa
tiene un `.rrproj`; si no, la propia carpeta del código fuente. Recibe solo lo que
distribuyes:

| Compilación | Salida |
|---|---|
| `--interp` | `prog` (`prog.exe` en Windows) — en macOS, universal (Apple silicon e Intel) de forma predeterminada; un programa con ventanas se convierte en `prog.app` (macOS) o `prog.AppDir` (Linux), y cada `.exe` recibe su ícono ([Compilar aplicaciones](building-apps.md)) |
| nativa | lo mismo (el Rust generado y los archivos de cargo se quedan en la caché de compilación: más abajo) |
| `--web` | `prog_web/`: `index.html`, el `.wasm` y el `.js` del programa |
| `--web --interp`, `bundle-bc` | `prog-web.zip` |

<!-- legal -->
Cada compilación escribe también **`THIRD-PARTY-NOTICES.txt`** junto al
ejecutable, dentro de la aplicación (`Contents/Resources`, el AppDir) o en la
compilación web: los avisos y los textos de licencia de todo lo que lleva dentro,
hechos a partir del grafo real de dependencias de la compilación. Distribúyelo
con el programa; eso es todo lo que piden las licencias
([LEGAL.md](../../../LEGAL.md)).

En Windows, un programa con interfaz gráfica se compila como un ejecutable con
ventanas: con él no se abre ninguna ventana de consola.

### Compilaciones nativas

Una compilación nativa traduce el programa a Rust y lo compila con cargo:

- Ejecuta `rapidr setup` una vez. Un SDK instalado siempre compila con la versión
  de Rust con la que se probó (indicada a cargo, nunca convertida en tu valor
  predeterminado) y funciona sin conexión: el código fuente del entorno de
  ejecución y sus crates están en el SDK.
- Windows: el SDK incluye LLVM-MinGW como enlazador, así que no hace falta Visual
  Studio, y los programas no necesitan ninguna DLL a su lado.
  `RAPIDR_TOOLCHAIN=msvc` (con `rapidr setup --toolchain msvc`) usa en su lugar el
  conjunto de herramientas de Microsoft.
- macOS: una compilación final tiene Apple silicon e Intel unidos en un
  solo ejecutable cuando Rust tiene ambos destinos (`rapidr setup` los añade); una
  compilación de depuración es solo de la arquitectura de esta Mac.
- Dónde se hace una compilación: la caché de compilación, fuera de la carpeta de
  salida — `~/Library/Caches/RapidR/build` (macOS),
  `%LOCALAPPDATA%\RapidR\Cache\build` (Windows), `$XDG_CACHE_HOME/rapidr/build` o
  `~/.cache/rapidr/build` (Linux), o `RAPIDR_BUILD_CACHE`. El Rust generado para el
  programa se escribe allí y se elimina después de la compilación (`--keep-rust`
  lo copia antes junto a la aplicación); la carpeta de destino de cargo allí la
  comparten todos los programas, así que el entorno de ejecución de RapidR se
  compila una sola vez (unos minutos) y las compilaciones siguientes tardan
  segundos; los archivos propios de cada programa en ella se quitan cuando su
  aplicación está hecha. Otra versión de RapidR recibe la suya; la que lleva 30
  días sin usarse se elimina. Toda la caché se puede eliminar sin riesgo.
  `CARGO_TARGET_DIR` indica otra carpeta de destino.

## El RapidR Runtime

El Runtime ejecuta programas sin compilarlos, como un entorno de ejecución de
Java: un archivo `.rrbc` es un programa compilado (`rapidr build-bc`), y los
archivos de código fuente se ejecutan directamente. Es el mismo intérprete que
lleva dentro cada ejecutable `--interp`.

- El **formato de bytecode 3** registra el Runtime más antiguo que necesita un
  programa y su tipo (consola o interfaz gráfica). Un runtime más antiguo dice
  “this program needs RapidR Runtime x.y.z or newer” con un enlace a las versiones
  publicadas; los archivos de formato 2 todavía se ejecutan.
  `rapidr info prog.rrbc` lo muestra todo:

  ```
  apptype: console
  format: 3
  needs-runtime: 2.119.0
  runtime: 2.119.0
  runs: yes
  downloaded: no
  ```

- **Tipos de archivo** (los instaladores los registran por usuario; los
  desinstaladores los quitan):

  | Archivo | Doble clic | También |
  |---|---|---|
  | `.rrbc` | se ejecuta | |
  | `.rr`, `.bas` (y los anteriores `.rqw`, `.rqb`, `.rq`) | se abre en el IDE (con solo el Runtime: se ejecuta) | Windows: **Ejecutar** (*Run*); macOS / Linux: **Abrir con > RapidR Runtime** (*Open With > RapidR Runtime*) |

  En Windows, `.bas` pasa a ser de RapidR de forma predeterminada solo si marcas la
  casilla del instalador (otros BASIC también usan `.bas`); RapidR siempre aparece
  en **Abrir con** (*Open with*). Windows usa `rapidrw.exe`, un iniciador con
  ventanas que le da una consola a un programa de consola y ninguna a un programa
  de interfaz gráfica.
<!-- legal -->
- **Los archivos descargados preguntan una vez** antes de ejecutarse: la
  cuarentena de macOS, la Marca de la Web de Windows (*Mark of the Web*) y el
  origen que Chrome y Firefox registran en Linux. La respuesta se conserva por
  contenido del archivo en `trusted-files.txt` en tu carpeta de RapidR
  (`~/Library/Application Support/RapidR`, `%APPDATA%\RapidR`,
  `~/.config/rapidr`).
- **Los programas de consola abiertos desde el escritorio** reciben una terminal
  en macOS y Linux, que sigue abierta cuando terminan.
- **Scripts**: en macOS y Linux, un archivo `.rr` / `.bas` cuya primera línea es
  `#!/usr/bin/env rapidr`, hecho ejecutable, se ejecuta como `./script.bas args`.

Los ejecutables independientes `--interp` siguen existiendo para los programas que
se entregan a personas que no tienen el Runtime.

## `rapidr setup`

```sh
rapidr setup --check      # solo informa
rapidr setup              # pregunta antes de cambiar nada
rapidr setup --yes        # responde que sí
```

<!-- legal -->
Informa la carpeta principal de RapidR y el Rust que encontró, instala el conjunto
exacto de herramientas de Rust que usan las compilaciones nativas (junto al tuyo,
con rustup; rustup en sí solo si no hay ningún Rust, después de preguntar), y
ofrece enlazar `rapidr` en `/usr/local/bin` o `~/.local/bin` cuando no está en tu
PATH (`--no-path`: no ofrecerlo). Desinstalar RapidR no toca Rust (ejecutar
`rustup toolchain uninstall <version>` te corresponde a ti).

## Dónde encuentra RapidR sus archivos

`RAPIDR_HOME` si está definida; si no, `<la carpeta del ejecutable>/../lib/rapidr`
de una instalación (contiene `release.toml`); si no, la copia del código fuente
desde la que se ejecuta el comando. `rapidr setup --check` imprime cuál.

## Variables de entorno

| Variable | |
|---|---|
| `RAPIDR_THEME=name` | el tema para los programas que no indican ninguno ([Componentes](components.md#temas)) |
| `RAPIDR_RENDERER=cpu\|gpu` | fuerza el dibujo en la CPU o en la GPU (de forma predeterminada la GPU, o la CPU cuando la única GPU es una por software) |
| `RAPIDR_PRINT_TO=<file or folder>` | los documentos impresos van allí como PDF en lugar de a una impresora |
| `RAPIDR_REGISTRY=<file>` | RRegistry guarda sus claves en este archivo en lugar de en el del sistema (el registro de Windows; un almacén por usuario en otros sistemas) |
| `RAPIDR_SCHOOL_POLICY=<file>` | la política del bloqueo escolar se lee de este archivo cuando la computadora no tiene una (la de las pruebas): puede añadir un bloqueo, nunca quitar el de la computadora |
| `RAPIDR_CONFIG_DIR=<folder>` | tu carpeta de RapidR (la lista de archivos de confianza) |
| `RAPIDR_DOCUMENTS=<folder>` | la carpeta que se usa como tus Documentos: las copias de los ejemplos de RapidR Studio (`RapidR/Examples/`) y las compilaciones que no pueden ir junto a su programa (`RapidR/Builds/`) |
| `RAPIDR_HOME=<folder>` | la carpeta principal de RapidR (arriba) |
| `RAPIDR_TOOLCHAIN=msvc` | compilaciones nativas de Windows con el conjunto de herramientas de Microsoft |
| `RAPIDR_INCLUDE_PATH=<folders>` | carpetas de inclusión, separadas como en `PATH` (`:`; `;` en Windows), que se buscan después de las de `--include-dir` y las de RapidR Studio |
| `RAPIDR_RUNTIME` | lo define el Runtime para el programa que ejecuta: el `rapidr` que lo ejecuta |

Las variables `RAPIDR_TEST_*` y `RAPIDR_CAPTURE*` controlan programas en las
pruebas propias de RapidR ([COMPILER_MANUAL.md](../../../COMPILER_MANUAL.md)).
