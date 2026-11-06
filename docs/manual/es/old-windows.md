# RapidR en Windows antiguos (7, 8 y XP)

RapidR también funciona en Windows de 32 bits: Windows 7, 8 y 8.1 con la **descarga para Windows 7**
(una alfa), y Windows XP SP3 sin nada más con la **descarga de prueba para Windows XP** (no oficial:
está probada, pero no tiene soporte). Esta página dice qué funciona allí y qué no. Todo lo demás de
este manual vale igual.

## Qué necesitas

- **Windows 7, 8 u 8.1** (32 o 64 bits): el zip `win7-x86`. Descomprímelo donde quieras y abre
  `RapidR-Studio.bat`.
- **Windows XP SP3** (32 bits): el zip `winxp-x86-test`, del mismo modo. XP necesita un procesador con
  SSE2 (un Pentium 4, un Athlon 64 o más nuevo). No hay que instalar nada en XP: ni One-Core-API ni un
  runtime más nuevo.

## Qué funciona

Igual en Windows 7 que en Windows XP sin nada más:

- **RapidR Studio**: el diseñador, el editor de código, el panel Proyecto, la Ayuda (F1), la
  Configuración, los menús del botón derecho (clic derecho, Mayús+F10 y la tecla Menú) y los atajos de
  teclado (Ctrl+S, Ctrl+O, Ctrl+Mayús+B ...).
- **Ejecutar un programa** con F5, con su salida en el panel Salida de Studio.
- **Compilar**: en Windows de 32 bits, Ejecutar > **Compilar (aplicación interpretada)** hace la
  aplicación enseguida. Una compilación nativa necesita Rust, que Windows de 32 bits no tiene, así que
  Studio no la ofrece allí y nunca pregunta; la aplicación interpretada ejecuta el mismo programa y no
  necesita nada en el equipo al que la copies. (Las descargas de 32 bits son programas de 32 bits
  también en un Windows 7 de 64 bits; la descarga para Windows de 64 bits tiene Compilar y Compilar
  aplicación nativa como en todas partes.)
- **Los programas que compilas**, y `rapidr run` en un Símbolo del sistema: formularios, la consola,
  archivos y carpetas, números grandes (`INT64`, `UINT64`, `UINT32`), llamadas a DLL (`DECLARE ...
  LIB`) y callbacks (`WndProc`), `RFormEx` (siempre encima, transparencia, el agujero, el icono de la
  bandeja), menús del botón derecho, MIDI con los dispositivos MIDI del propio Windows (`RMIDIPort`; un
  equipo sin dispositivo MIDI no lista ninguno y lo dice).
- **Los diálogos Abrir y Guardar**: en Windows XP son los propios de XP (Abrir, Guardar y Buscar
  carpeta), con los mismos filtros, carpeta inicial y nombre de archivo.
- **Idiomas**: English, Español y Português (Brasil) en ambos; 中文 (chino simplificado) en Windows 7 y
  posteriores. En Windows XP, Studio en chino muestra cuadros vacíos y lo dice en su panel Salida: XP
  sin nada más no tiene fuente china, y RapidR todavía no usa las fuentes instaladas en XP (abajo), así
  que instalar la compatibilidad con Asia oriental de XP todavía no ayuda allí. RapidR nunca descarga ni
  incluye una fuente de Windows.
- **Símbolos** como ✓ ✗ ⚠ salen de las fuentes del propio Windows; en un Windows sin fuentes que RapidR
  pueda usar (XP sin nada más), RapidR los dibuja con su propia pequeña fuente de símbolos.

## Qué no funciona (todavía)

| En | Qué | Por qué, y qué hacer |
|---|---|---|
| Windows de 32 bits | Compilaciones nativas | No hay Rust para Windows de 32 bits: Compilar hace la aplicación interpretada (arriba). |
| Windows de 32 bits | El JIT (código máquina mientras el programa corre) | Todavía no se compila para x86 de 32 bits: los programas corren en el intérprete, con los mismos resultados. `RAPIDR_JIT=always` y `rapidr run --verbose` dicen "JIT: not on this kind of computer". |
| Windows XP | Un lector de pantalla | XP no tiene UI Automation, que es lo que habla la accesibilidad de RapidR. Windows 7 y posteriores sí. |
| Windows XP | Proveedores de IA (el asistente, `RAI`) | La seguridad de red de XP es demasiado antigua para ellos (TLS 1.0, sin SNI): rechazan la conexión. Una IA en el propio equipo por `http://127.0.0.1` funciona. |
| Windows XP | Las fuentes instaladas en Windows, texto en chino | RapidR dibuja allí con sus propias fuentes (cubren latín, griego y cirílico, y los símbolos de arriba); un programa que pide una fuente de Windows recibe la más parecida de RapidR. El texto en chino (y japonés, coreano) necesita una fuente que RapidR no incluye: en XP se ve como cuadros por ahora. |

## Saber en qué Windows corre un programa

`RSystemInfo.OS` es "Windows", y `RSystemInfo.OSVersion` dice cuál: "5.1 (2600)" en XP, "6.1 (7601)"
en Windows 7, "11 24H2 (26100)" en Windows 11. El ejemplo `examples/system_info.rr` lista todo lo que
sabe `RSystemInfo`.
