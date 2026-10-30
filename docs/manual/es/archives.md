<!-- translated-from: archives.md 5fe176e706655247 -->
# Archivos comprimidos: zip, 7z, tar y gzip

`RArchive` abre archivos comprimidos, enumera lo que contienen y lo
descomprime, y crea archivos zip, tar, tar.gz y .gz. Deduce el formato a partir
de los primeros bytes de un archivo, nunca de su nombre, así que un programa
puede tomar una instantánea de un juego tal cual, o la misma instantánea dentro
de un `.zip`, un `.7z` o un `.gz`, como hacen los emuladores.

![El ejemplo de archivos comprimidos: las entradas de un zip enumeradas, el .z80 de su interior encontrado y sus primeros bytes a la vista](../images/archives/archives-zip.png)

El ejemplo es [examples/data/archives.rr](../../../examples/data/archives.rr):
desde `examples/data`, `rapidr run archives.rr`. Elige una muestra, o abre tu
propio archivo, y haz clic en una entrada para ver sus primeros bytes.

## Qué lee y qué crea

| Formato | Abre | Crea | Notas |
|---|---|---|---|
| zip (`.zip`) | sí | sí | Entradas Stored y Deflate, archivos zip de más de 4 GB, nombres UTF-8 y nombres antiguos de la página de códigos 437 |
| tar (`.tar`) | sí | sí | Nombres largos y UTF-8 |
| tar.gz (`.tar.gz`, `.tgz`) | sí | sí | |
| gzip (`.gz`) | sí | sí | Un solo archivo: un archivo comprimido de una entrada |
| 7z (`.7z`) | sí | no | LZMA, LZMA2, Deflate |
| xz y tar.xz | sí | no | |
| zstd y tar.zst | sí | no | |
| bzip2 | no | no | Se reconoce; `Error` dice que todavía no se puede abrir |
| RAR | no | no | Se rechaza: "RAR files can't be opened: it's a closed, proprietary format. Ask for a .zip or .7z." |

`Format` dice qué se abrió: `"zip"`, `"7z"`, `"tar"`, `"tar.gz"`, `"gzip"`,
`"tar.xz"`, `"xz"`, `"tar.zst"` o `"zstd"`.

## Abrir y descomprimir

```basic
DIM Archive AS RArchive

IF Archive.Open("games.zip") THEN
    FOR i = 0 TO Archive.Count - 1
        PRINT Archive.Name(i); "  "; Archive.Size(i); " bytes"
    NEXT
    Snapshot$ = Archive.Extract("roms/game.z80")   ' sus bytes
    PRINT LEN(Snapshot$); " bytes, el primero "; ASC(Snapshot$)
ELSE
    PRINT "No se pudo abrir: "; Archive.Error
END IF
```

Un archivo comprimido puede venir de un archivo, de un flujo, de una cadena de
bytes (una descarga) o del propio programa:

```basic
$RESOURCE GAMES AS "games.zip"       ' incluido en el programa
Archive.Open GAMES                   ' el identificador de un $RESOURCE
Archive.OpenStream Mem               ' todo un flujo de memoria o de archivo
Archive.OpenData Downloaded$         ' una cadena de bytes
```

Una entrada se indica por su índice (0 es la primera) o por su nombre, con
cualquier combinación de mayúsculas y minúsculas salvo que `CaseSensitive` sea
True. `Extract` da los bytes de una entrada como una cadena, un carácter por
byte (`LEN` es su tamaño, `ASC(MID$(...))` lee un byte). `ExtractText` da texto
(UTF-8 cuando lo es). `ExtractToStream`, `ExtractToFile` y `ExtractAll` lo
escriben.

| Miembro | Qué hace |
|---|---|
| `Open(FileName)` | Abre un archivo comprimido, o un `$RESOURCE`. Da True cuando se abrió. |
| `OpenStream(Stream)`, `OpenData(Data$)` | Lo mismo, a partir de los bytes de un flujo o de una cadena. |
| `FormatOf(FileName)` | Qué es un archivo según sus primeros bytes, sin abrirlo: `"zip"`, `"7z"`, `"gzip"`, `"rar"` …, o `""` cuando no es un archivo comprimido. |
| `Count`, `Format`, `FileName` | Cuántas entradas, qué formato, qué archivo. |
| `Name(i)`, `Size(i)`, `CompressedSize(i)`, `DateTime(i)` | El nombre de una entrada (las carpetas con `/`), los tamaños y cuándo cambió (`"2026-10-29 14:05:00"`, hora local). |
| `IsFolder(i)`, `IsEncrypted(i)` | Qué es. |
| `Find(Name)` | El índice de una entrada, o -1. |
| `Extract(i)`, `ExtractText(i)` | Sus bytes, o su texto. |
| `ExtractToStream(i, Stream)`, `ExtractToFile(i, FileName)` | Lo escribe en un flujo o en un archivo. |
| `ExtractAll(Folder)` | Descomprime todo en una carpeta (se crea si hace falta). |
| `Close` | Escribe un archivo comprimido que se está creando, y libera uno que está abierto. |
| `Error` | Por qué falló la última llamada, en palabras sencillas; `""` cuando funcionó. |

## Una instantánea, o un archivo comprimido que la contiene

Este es el patrón que necesita un emulador: el archivo que le dan las personas
puede ser la instantánea misma o cualquier archivo comprimido que tenga una
dentro.

```basic
DIM Archive AS RArchive
DIM File AS RFileStream

FUNCTION LoadSnapshot (FileName AS STRING) AS STRING
    DIM i AS INTEGER
    IF Archive.FormatOf(FileName) = "" THEN
        ' no es un archivo comprimido: la instantánea misma
        File.Open FileName, 0
        LoadSnapshot = File.ReadBinStr(File.Size)
        File.Close
    ELSEIF Archive.Open(FileName) THEN
        FOR i = 0 TO Archive.Count - 1
            IF LCASE$(RIGHT$(Archive.Name(i), 4)) = ".z80" THEN
                LoadSnapshot = Archive.Extract(i)
                EXIT FUNCTION
            END IF
        NEXT
    END IF
END FUNCTION
```

![La misma instantánea encontrada dentro de un archivo 7z](../images/archives/archives-7z.png)

## Crear archivos comprimidos

```basic
Archive.Level = 9                          ' 0 almacena, 9 es el más pequeño
IF Archive.Create("notes.zip") THEN        ' .zip, .tar, .tar.gz o .gz
    Archive.AddText "notes/hello.txt", "Hello from RapidR!"
    Archive.AddFile "photo.jpg"             ' con su propio nombre
    Archive.AddFile "save.dat", "saves/slot1.dat"
    Archive.AddFolder "levels"              ' una carpeta y todo lo que contiene
    IF Archive.Close = 0 THEN PRINT Archive.Error
END IF
```

`CreateStream(Stream, "zip")` crea el archivo comprimido en un flujo en lugar de
en un archivo. `AddData(Name, Data$)` añade una cadena de bytes, `AddStream(Name,
Stream)` los de un flujo. Un `.gz` contiene un solo archivo. Los nombres nunca
empiezan con `/` ni contienen `..`: RapidR nunca crea un archivo comprimido que
otro programa descomprimiría fuera de su carpeta.

## Empaquetar datos: COMPRESS$ y DECOMPRESS$

Para datos en lugar de archivos:

```basic
Packed$ = COMPRESS$(Text$)                  ' gzip, nivel 6
Packed$ = COMPRESS$(Text$, "zlib", 9)       ' "gzip", "zlib" o "deflate"
Text$ = DECOMPRESS$(Packed$)                ' se reconocen gzip, zlib, xz y zstd
Text$ = DECOMPRESS$(Raw$, "deflate")        ' el Deflate sin formato hay que nombrarlo
Text$ = DECOMPRESS$(Packed$, "", 1000000)   ' como máximo 1 000 000 bytes
```

`DECOMPRESS$` descomprime como máximo 256 MB a menos que le des otro límite. Los
datos que no están empaquetados, o que están dañados, detienen el programa con un
error que dice por qué. Para datos de fuera que pueden estar dañados, usa
`OpenData`, que informa en `Error` en su lugar.

## Seguro con archivos de cualquier procedencia

Un archivo comprimido de una descarga o de un desconocido no es de confianza.
RArchive está hecho para eso:

- **Bombas zip.** Unos pocos kilobytes pueden afirmar que son gigabytes.
  RArchive cuenta los bytes que realmente descomprime, no los tamaños que dice
  el archivo comprimido. Se detiene en `MaxSize` (1 GB, 256 MB en un navegador:
  una entrada, todo `ExtractAll`, un `.tar.gz` completo al hacer `Open`), y en
  `MaxRatio` (1000: una entrada de más de 1 MB que crece más de 1000 veces). Los
  dos dan un error, nunca una memoria llena. Auméntalos cuando esperes archivos
  enormes; `MaxRatio = 0` desactiva la comprobación de la proporción.
- **Rutas que salen de la carpeta.** Una entrada llamada `../../evil.txt`,
  `/etc/passwd`, `C:\Windows\x.dll` o `CON` se enumera pero nunca se escribe:
  `ExtractAll` comprueba primero todos los nombres, así que un archivo
  comprimido así no escribe nada.
- **Enlaces.** Los enlaces y los dispositivos de un archivo comprimido nunca se
  extraen, y `ExtractAll` nunca escribe a través de un enlace que ya esté en la
  carpeta.
- **Carpetas a medio escribir.** `ExtractAll` descomprime y comprueba todo antes
  de escribir nada, así que una bomba o una entrada dañada no dejan a medias el
  archivo comprimido.
<!-- legal -->
- **Contraseñas.** Las entradas cifradas se enumeran (`IsEncrypted`) y se
  rechazan. RArchive no abre archivos comprimidos protegidos con contraseña.
- **Los archivos dañados** dan un error, nunca un bloqueo. Los lectores se
  prueban con muchos miles de archivos comprimidos dañados.

```basic
IF Archive.Open(Download$) = 0 THEN
    ShowMessage "Ese archivo no se puede usar: " + Archive.Error
ELSEIF Archive.ExtractAll("downloads") = 0 THEN
    ShowMessage Archive.Error     ' p. ej. "../evil.txt" sube una carpeta (..)
END IF
```

## En un navegador

RArchive funciona igual en un navegador: los archivos comprimidos vienen de los
archivos de la página (`Open`), de un flujo o de una descarga (`OpenData`), o
están incluidos con `$RESOURCE`. `ExtractAll` y `Create` escriben en los
archivos de la página. `AddFolder` necesita una carpeta en el disco, así que en
un navegador añades cada archivo.

## Programas más pequeños

Una compilación nativa lleva el código de RArchive solo cuando el programa usa
`RArchive`, `COMPRESS$` o `DECOMPRESS$`. Los demás programas no crecen.
