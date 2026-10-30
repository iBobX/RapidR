<!-- translated-from: getting-started.md d730607fc171e370 -->
<!-- A SAMPLE that proves the pipeline (lane I18N-STUDIO): the start of the
page only, to be reviewed against docs/i18n/glossary-es.md. The translation
lanes translate the whole page; `rapidr i18n manual-stamp es
getting-started.md` records the English it was made from. -->

# Primeros pasos

> *Traducción de muestra, por revisar: solo el comienzo de la página está en
> español. El resto está en [la página en inglés](../getting-started.md).*

**En tres líneas**

- **Descarga el SDK** (`RapidR-…`): Studio, el comando `rapidr` y todo lo necesario para compilar. *Es lo que la mayoría necesita.* El **Runtime** (`RapidR-Runtime-…`) solo ejecuta programas que hicieron otros, como un runtime de Java; el SDK lo incluye.
- **`rapidr run file.bas`** ejecuta un programa ahora. **`rapidr build-bc file.bas`** lo compila a `file.rrbc`, que se ejecuta con `rapidr file.rrbc` o con doble clic donde esté instalado el Runtime.
- **`rapidr build file.bas`** crea un ejecutable independiente: `--interp` no necesita Rust; el predeterminado, nativo, necesita Rust una vez (`rapidr setup`).

## Instalar

Descárgalo de [las versiones en GitHub](https://github.com/iBobX/RapidR/releases).
Hay dos tipos de paquete para cada sistema (elige el primero, salvo que solo
quieras ejecutar programas):

- **El SDK de RapidR**: el IDE, el comando `rapidr` (compilador y runtime),
  todo lo necesario para crear ejecutables independientes y paquetes web, y
  el código fuente del runtime de RapidR para las compilaciones nativas.
  Instálalo para escribir programas.
- **El Runtime de RapidR**: solo ejecuta programas (`.rrbc`, `.rr`, `.bas`,
  `.rqw`, `.rqb`, `.rq`), como un runtime de Java. Instálalo en equipos que
  solo ejecutan programas que alguien te dio.

El SDK incluye el Runtime. Se instala por usuario y no necesita permisos de
administrador (salvo los paquetes `.deb` de Linux, que se instalan para todo
el sistema).
