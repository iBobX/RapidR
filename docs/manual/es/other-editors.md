<!-- translated-from: other-editors.md 3ebdd571e7247db1 -->
# RapidR en otros editores

RapidR Studio es el editor hecho para RapidR, pero un programa es un archivo de texto, y dos programas que vienen con RapidR permiten que cualquier editor que hable los protocolos estándar lo entienda como lo entiende Studio:

- **`rapidr lsp`** es el servidor de lenguaje (el Language Server Protocol, por entrada y salida estándar). Es el motor del editor de código de Studio, así que otro editor obtiene el mismo autocompletado, la misma información flotante, la información de parámetros, Ir a la definición, referencias, renombrar, estructura, formato, correcciones rápidas y los mismos problemas, redactados como los redacta el compilador.
- **`rapidr dap`** es el adaptador de depuración (el Debug Adapter Protocol), el motor detrás del depurador de Studio: puntos de interrupción, ejecución paso a paso, la pila de llamadas, variables, expresiones observadas.

Hay una extensión de RapidR ya preparada para Visual Studio Code. Este capítulo dice qué hace y cómo encuentra RapidR, y luego qué le puede pedir otro editor al servidor.

## La extensión de Visual Studio Code

Instala primero RapidR (la [página de descargas](https://github.com/iBobX/RapidR/releases)), y luego la extensión, `rapidr-<versión>.vsix` de la misma versión publicada (**Extensiones ▸ ⋯ ▸ Instalar desde VSIX…**, *Extensions ▸ ⋯ ▸ Install from VSIX…*). Abre un archivo `.bas` o `.rr`: la barra de estado muestra **RapidR 2.121.3** (la versión en uso). La extensión no tiene conocimiento propio del lenguaje; todo lo que sigue es de RapidR.

Los textos de Visual Studio Code van en español, con el original en inglés en cursiva; la extensión todavía está solo en inglés, así que sus textos van en inglés, con el significado en español en cursiva.

### Qué `rapidr` usa

La extensión busca en `rapidr.path` (si lo estableciste), luego en el `PATH`, luego donde los instaladores de RapidR lo dejan (`/Applications/RapidR Studio.app` en macOS, la carpeta Program Files o `%LOCALAPPDATA%\Programs\RapidR` en Windows, `/usr/bin` en Linux), y le pregunta su versión a cada uno. Usa **el primero que sea tan nuevo como la extensión**. Un `rapidr` anterior a la 2.117 no tiene servidor de lenguaje ni depurador (el `rapidr` 0.1.0 que dejó `cargo install` es el que la gente se encuentra): la extensión lo omite, te lo dice una vez y usa en su lugar el RapidR instalado. La salida **RapidR Language Server** (*Servidor de lenguaje de RapidR*) enumera cada uno que revisó y por qué lo usó o lo omitió. Si ninguno es lo bastante nuevo, una notificación ofrece **Use the installed RapidR** (*Usar el RapidR instalado*), **Choose…** (*Elegir…*, un selector de archivos) y **Download** (*Descargar*).

### Qué obtienes

- **Autocompletado** de palabras clave, funciones integradas, componentes (con los nombres de RapidR) con sus propiedades, métodos y eventos, y tus propios nombres, con los archivos `$INCLUDE` incluidos. Escribe `DIM test AS ` y la lista de tipos y componentes se abre sola.

  ![La lista de tipos después de AS](../../../utilities/vscodeext/rapidr/images/completion-as.png)

- **Información flotante** e **información de parámetros**, con el nombre del parámetro escrito antes de cada argumento literal. La información flotante termina con **Open in RapidR Help** (*Abrir en la Ayuda de RapidR*).
- **La Ayuda de RapidR junto a tu código**: **Shift+F1** sobre una palabra abre la página que el panel Ayuda de Studio muestra para ella — qué es, la sintaxis, qué hace, las propiedades, los métodos y los eventos de un componente como enlaces, tus propios nombres con el lugar donde se declaran. *Insertar en el código*, *Atrás* y un cuadro de búsqueda están en la página.

  ![La Ayuda de RapidR junto al código](../../../utilities/vscodeext/rapidr/images/help.png)

- **Muestras de color** antes de `RGB(…)`, `&HBBGGRR` y los nombres `cl…`, y el selector de color del editor escribe el color de nuevo en la misma forma en que lo escribió el programa.

  ![Nombres de parámetros y colores](../../../utilities/vscodeext/rapidr/images/hints.png)

- **Ir a definición** (*Go to Definition*), **Buscar todas las referencias** (*Find All References*), **Cambiar el nombre del símbolo** (*Rename*), el **Esquema** (*Outline*), los resaltados del nombre que está bajo el cursor, las correcciones rápidas y Dar formato al documento (RapidR es el formateador predeterminado de los archivos de RapidR; activa `"editor.formatOnSave"` para el lenguaje RapidR para dar formato al guardar).
- Los comandos **Run**, **Debug** y **Build** (*Ejecutar*, *Depurar*, *Compilar*) y, para un proyecto (`.rrproj`), **Run Project**, **Debug Project**, **Build Project** y **Open the Project's Main File** (*Ejecutar el proyecto*, *Depurar el proyecto*, *Compilar el proyecto*, *Abrir el archivo principal del proyecto*), con el archivo principal, los argumentos y la carpeta del proyecto.
- **Depuración** con F5: puntos de interrupción (con condiciones, número de veces y mensajes de registro), ejecución paso a paso, la pila de llamadas, Locales, Globales y Componentes del formulario, expresiones observadas, y la Consola de depuración, donde se imprime una expresión y se ejecuta una instrucción.

  ![Depuración en el editor](../../../utilities/vscodeext/rapidr/images/debug.png)

La página propia de la extensión, con su configuración (`rapidr.path`, `rapidr.rapidqCompatible`, `rapidr.keywordCase`, `rapidr.identifierCase`) y su solución de problemas, es su [README](../../../utilities/vscodeext/rapidr/README.md); cómo se compila y se prueba está en [VSCODE_EXTENSION.md](../../../VSCODE_EXTENSION.md).

## Cualquier editor que hable LSP

`rapidr lsp` acepta estas **opciones de inicialización** (todas opcionales): `rapidqCompatible` (true: advierte de lo que un programa clásico no admite), `keywordCase` (`upper`, `lower`, `proper`, `preserve`), `identifierCase` (`declaration`, `preserve`) e `includeDirs` (más carpetas para `$INCLUDE`). Lee archivos solo dentro de las carpetas del espacio de trabajo que el editor abrió y de las carpetas de los documentos que abrió.

| Qué | Solicitud |
|---|---|
| Autocompletado (caracteres de activación `.` y `$`), información flotante, información de parámetros | `textDocument/completion`, `hover`, `signatureHelp` |
| Ir a la definición, referencias, resaltados, renombrar (con preparación) | `definition`, `references`, `documentHighlight`, `rename` |
| Estructura | `documentSymbol` |
| Formato, de un rango y al escribir (las mayúsculas y minúsculas de las palabras clave) | `formatting`, `rangeFormatting`, `onTypeFormatting` |
| Correcciones rápidas | `codeAction` |
| Nombres según lo que son | `semanticTokens/full` |
| Nombres de parámetros | `inlayHint` |
| Colores | `documentColor`, `colorPresentation` |
| Problemas | `publishDiagnostics` (los de un archivo principal y los de sus archivos `$INCLUDE`) |
| La página de Ayuda de RapidR de una palabra o de la palabra en una posición | `rapidr/helpPage` (`{ word?, of?, textDocument?, position?, canInsert?, key? }` → `{ kind, markdown, insert, uri, line, column }`) |
| Qué puede significar una palabra escrita | `rapidr/helpSuggest` |

El `markdown` de `rapidr/helpPage` es la página que muestra Studio; sus enlaces son `help:Word[@Type]` (otra entrada), `help-insert:` (escribe `insert` en el cursor), `help-goto:` (abre `uri` en `line`, `column`) y `help-refs:` (busca referencias). El servidor indica que los tiene en `capabilities.experimental.rapidr`.

Lo que el servidor no tiene: símbolos del espacio de trabajo, rangos de plegado (la extensión usa expresiones regulares), rangos de selección, lentes de código y jerarquía de llamadas.

El editor inicia `rapidr dap` como adaptador de depuración: `{ "type": "rapidr", "request": "launch", "program": "main.bas", "args": [], "cwd": ".", "stopOnEntry": false, "noDebug": false, "env": {} }`. Admite puntos de interrupción condicionales, número de veces, puntos de registro (logpoints), el filtro de errores de ejecución, establecer variable y establecer expresión (cualquier expresión de BASIC), entrar en una llamada específica (step in targets) e ir a una línea (`goto`).

## Si no funciona

- **La barra de estado dice que RapidR es demasiado antiguo, o no aparece ningún autocompletado.** Ejecuta `rapidr version` en una terminal (`which -a rapidr` muestra todos los que hay en tu `PATH`) y luego **RapidR: Show Language Server Output** (*RapidR: Mostrar la salida del servidor de lenguaje*) en el editor.
- **La Ayuda dice que este `rapidr` no tiene páginas de Ayuda.** La extensión es más nueva que el `rapidr` que encontró: actualiza RapidR.

Vuelve a crear las imágenes: `node test/screenshots/capture.js` en `utilities/vscodeext/rapidr` (macOS).
