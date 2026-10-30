"""The programs the corpus is made of, all compiled by RapidR itself before
they are used.

1. REAL: programs of this repository (examples/, the recipes that are not
   an eval solution), with a request and a description in English, Spanish
   and Portuguese written for each.
2. TEMPLATES: small programs written for this corpus (templates: provenance),
   each with parameters (numbers, words, captions) so one template gives many
   different programs, and every caption in the three languages.

Nothing here comes from RapidQ's corpus, a cloud model, or any other
project.
"""

from __future__ import annotations

import random
from dataclasses import dataclass, field
from pathlib import Path

from rai_corpus import EVAL_RECIPES, HOLDOUT_ADJACENT_RECIPES, REPO

LANGS = ("en", "es", "pt")


@dataclass
class Program:
    id: str
    files: list[tuple[str, str]]
    main: str
    sources: list[str]
    generator: str
    kind: str  # "gui" | "console"
    req: dict[str, str]  # noun phrase for a request, per language ("a program that ...")
    does: dict[str, str]  # one sentence of what it does, per language
    recipe: str = ""  # the recipe file it is, when it is one
    tags: list[str] = field(default_factory=list)
    req_alts: dict[str, list[str]] = field(default_factory=dict)  # other ways to ask for it

    @property
    def text(self) -> str:
        return dict(self.files)[self.main]


# ---- 1. real programs ---------------------------------------------------------------------------
# path -> (request en/es/pt, does en/es/pt). Only what is small, runs on every system and is not
# (next to) an eval task: the stopwatch and the to-do list are the eval's own apps.
REAL: dict[str, tuple[tuple[str, str, str], tuple[str, str, str]]] = {
    "examples/basics/hello.rr": (
        ("a first console program that says hello and uses a few variables", "un primer programa de consola que saluda y usa algunas variables", "um primeiro programa de console que cumprimenta e usa algumas variáveis"),
        ("It prints a greeting and shows a few variables.", "Muestra un saludo y algunas variables.", "Mostra uma saudação e algumas variáveis."),
    ),
    "examples/basics/input.rr": (
        ("a console program that asks for my name and answers", "un programa de consola que me pregunta el nombre y responde", "um programa de console que pergunta meu nome e responde"),
        ("It asks a question with INPUT and prints the answer.", "Hace una pregunta con INPUT e imprime la respuesta.", "Faz uma pergunta com INPUT e imprime a resposta."),
    ),
    "examples/basics/files.rr": (
        ("a program that writes a text file and reads it back", "un programa que escribe un archivo de texto y lo vuelve a leer", "um programa que escreve um arquivo de texto e o lê de volta"),
        ("It writes lines to a file, appends one, and reads them back.", "Escribe líneas en un archivo, añade una y las lee de nuevo.", "Escreve linhas em um arquivo, acrescenta uma e as lê de novo."),
    ),
    "examples/basics/language.rr": (
        ("a tour of the language: constants, arrays, SELECT CASE, loops and string functions", "un recorrido por el lenguaje: constantes, arreglos, SELECT CASE, bucles y funciones de texto", "um tour pela linguagem: constantes, arrays, SELECT CASE, laços e funções de texto"),
        ("It shows the main parts of the language one after another.", "Muestra las partes principales del lenguaje una tras otra.", "Mostra as principais partes da linguagem uma após a outra."),
    ),
    "examples/basics/unicode.rr": (
        ("a program that writes and reads text in several languages in a file", "un programa que escribe y lee texto en varios idiomas en un archivo", "um programa que escreve e lê texto em vários idiomas em um arquivo"),
        ("It saves text in several scripts to a file and reads it back.", "Guarda texto en varias escrituras en un archivo y lo lee de nuevo.", "Salva texto em várias escritas em um arquivo e o lê de volta."),
    ),
    "examples/basics/bytes.rr": (
        ("a program that writes a binary file and looks at its bytes", "un programa que escribe un archivo binario y mira sus bytes", "um programa que escreve um arquivo binário e olha seus bytes"),
        ("It writes bytes to a file with RFileStream and reads them back.", "Escribe bytes en un archivo con RFileStream y los lee de nuevo.", "Escreve bytes em um arquivo com RFileStream e os lê de volta."),
    ),
    "examples/data/json.rr": (
        ("a program that reads and writes JSON", "un programa que lee y escribe JSON", "um programa que lê e escreve JSON"),
        ("It parses JSON text, reads values by path, changes them and writes it out.", "Analiza texto JSON, lee valores por su ruta, los cambia y lo escribe.", "Analisa texto JSON, lê valores pelo caminho, os altera e escreve de volta."),
    ),
    "examples/data/numbers.rr": (
        ("a program that does arithmetic on whole arrays of numbers at once", "un programa que hace aritmética con arreglos de números completos", "um programa que faz aritmética com arrays inteiros de números"),
        ("It makes number arrays with RNum, summarises them and computes on every element.", "Crea arreglos con RNum, los resume y calcula sobre cada elemento.", "Cria arrays com RNum, os resume e calcula sobre cada elemento."),
    ),
    "examples/data/sqlite.rr": (
        ("a small book catalogue in SQLite", "un pequeño catálogo de libros en SQLite", "um pequeno catálogo de livros em SQLite"),
        ("It creates a table, inserts with parameters, queries and updates it.", "Crea una tabla, inserta con parámetros, consulta y actualiza.", "Cria uma tabela, insere com parâmetros, consulta e atualiza."),
    ),
    "examples/data/transactions.rr": (
        ("a window that shows how a database transaction works", "una ventana que muestra cómo funciona una transacción de base de datos", "uma janela que mostra como funciona uma transação de banco de dados"),
        ("Begin starts a transaction; Commit keeps the changes and Rollback undoes them.", "Begin inicia una transacción; Commit guarda los cambios y Rollback los deshace.", "Begin inicia uma transação; Commit guarda as mudanças e Rollback as desfaz."),
    ),
    "examples/data/dataframe.rr": (
        ("a window that loads a CSV file into a table and a chart", "una ventana que carga un archivo CSV en una tabla y un gráfico", "uma janela que carrega um arquivo CSV em uma tabela e um gráfico"),
        ("It reads a CSV into data frames and shows one in a grid and one as a chart.", "Lee un CSV en marcos de datos y muestra uno en una tabla y otro como gráfico.", "Lê um CSV em quadros de dados e mostra um em uma tabela e outro como gráfico."),
    ),
    "examples/gui/dialogs.rr": (
        ("a program that opens the standard dialogs: open a file, pick a colour and a font", "un programa que abre los diálogos estándar: abrir archivo, elegir color y fuente", "um programa que abre os diálogos padrão: abrir arquivo, escolher cor e fonte"),
        ("Each button opens a standard dialog and shows what was chosen.", "Cada botón abre un diálogo estándar y muestra lo elegido.", "Cada botão abre um diálogo padrão e mostra o que foi escolhido."),
    ),
    "examples/gui/focus_events.rr": (
        ("a form where each box shows its help when I click into it", "un formulario donde cada cuadro muestra su ayuda cuando entro en él", "um formulário onde cada caixa mostra sua ajuda quando eu entro nela"),
        ("OnEnter shows a box's help and OnExit checks what was typed.", "OnEnter muestra la ayuda del cuadro y OnExit revisa lo escrito.", "OnEnter mostra a ajuda da caixa e OnExit confere o que foi digitado."),
    ),
    "examples/gui/hello_form.rr": (
        ("a window with a text box, a button and a label that greets me", "una ventana con un cuadro de texto, un botón y una etiqueta que me saluda", "uma janela com uma caixa de texto, um botão e um rótulo que me cumprimenta"),
        ("Type a name and press Greet: the label answers.", "Escribe un nombre y pulsa Saludar: la etiqueta responde.", "Digite um nome e pressione Cumprimentar: o rótulo responde."),
    ),
    "examples/gui/links.rr": (
        ("a form with labels that work as links I can reach with the keyboard", "un formulario con etiquetas que funcionan como enlaces alcanzables con el teclado", "um formulário com rótulos que funcionam como links acessíveis pelo teclado"),
        ("Tab walks the three links; Enter or Space follows the focused one.", "Tab recorre los tres enlaces; Enter o Espacio sigue el enfocado.", "Tab percorre os três links; Enter ou Espaço segue o focado."),
    ),
    "examples/gui/markdown_code.rr": (
        ("a viewer that shows Markdown with coloured code blocks", "un visor que muestra Markdown con bloques de código coloreados", "um visualizador que mostra Markdown com blocos de código coloridos"),
        ("An RMarkdownView colours each code block by its language.", "Un RMarkdownView colorea cada bloque de código según su lenguaje.", "Um RMarkdownView colore cada bloco de código conforme sua linguagem."),
    ),
    "examples/gui/menus.rr": (
        ("a window with a main menu with shortcuts, a pop-up menu and a status bar", "una ventana con menú principal con atajos, un menú emergente y una barra de estado", "uma janela com menu principal com atalhos, um menu de contexto e uma barra de status"),
        ("The menus have shortcuts, a check item and radio items; the status bar says what happened.", "Los menús tienen atajos, un elemento marcable y opciones de radio; la barra de estado dice qué pasó.", "Os menus têm atalhos, um item marcável e opções de rádio; a barra de status diz o que aconteceu."),
    ),
    "examples/gui/on_error.rr": (
        ("a program that keeps going after a mistake in its own code and says so", "un programa que sigue funcionando tras un error en su propio código y lo avisa", "um programa que continua após um erro no próprio código e avisa"),
        ("Each button has a bug; the program catches it and reports it.", "Cada botón tiene un error; el programa lo captura y lo informa.", "Cada botão tem um erro; o programa o captura e informa."),
    ),
    "examples/gui/pantry.rr": (
        ("a pantry window with a list of shelves and a grid with the items of the chosen shelf", "una ventana de despensa con una lista de estantes y una tabla con los artículos del estante elegido", "uma janela de despensa com uma lista de prateleiras e uma tabela com os itens da prateleira escolhida"),
        ("Pick a shelf and its items show in the grid; add an item and the totals follow.", "Elige un estante y sus artículos aparecen en la tabla; añade uno y los totales se actualizan.", "Escolha uma prateleira e seus itens aparecem na tabela; adicione um e os totais acompanham."),
    ),
    "examples/gui/svg_icons.rr": (
        ("a tree with SVG icons that stay sharp at every screen scale", "un árbol con iconos SVG que se ven nítidos en cualquier escala de pantalla", "uma árvore com ícones SVG nítidos em qualquer escala de tela"),
        ("The tree's pictures are SVGs in an RImageList.", "Las imágenes del árbol son SVG en un RImageList.", "As imagens da árvore são SVGs em um RImageList."),
    ),
    "examples/gui/themes.rr": (
        ("a form that follows the light, dark and high-contrast looks", "un formulario que sigue los aspectos claro, oscuro y de alto contraste", "um formulário que segue os visuais claro, escuro e de alto contraste"),
        ("It opens in the system's look and lets you switch between the themes.", "Se abre con el aspecto del sistema y permite cambiar entre los temas.", "Abre com o visual do sistema e permite alternar entre os temas."),
    ),
    "examples/gui/updown.rr": (
        ("a medicine reminder with up-down arrows for how many tablets a day", "un recordatorio de medicinas con flechas arriba-abajo para cuántas pastillas al día", "um lembrete de remédios com setas para cima e para baixo para quantos comprimidos por dia"),
        ("RUpDown steps the number in the edit beside it.", "RUpDown cambia el número del cuadro de al lado.", "RUpDown altera o número da caixa ao lado."),
    ),
    "examples/gui/value_tree.rr": (
        ("a tree that shows values the way a debugger does", "un árbol que muestra valores como lo hace un depurador", "uma árvore que mostra valores como um depurador faz"),
        ("Each node shows a value with its kind's colour.", "Cada nodo muestra un valor con el color de su tipo.", "Cada nó mostra um valor com a cor do seu tipo."),
    ),
    "examples/graphics/canvas.rr": (
        ("a program that draws a bar chart on a canvas and dots where I click", "un programa que dibuja un gráfico de barras en un lienzo y puntos donde hago clic", "um programa que desenha um gráfico de barras em uma tela e pontos onde eu clico"),
        ("It draws a bar chart in OnPaint and a dot at each mouse click.", "Dibuja un gráfico de barras en OnPaint y un punto en cada clic.", "Desenha um gráfico de barras em OnPaint e um ponto a cada clique."),
    ),
    "examples/network/download.rr": (
        ("a program that downloads a file with a progress gauge", "un programa que descarga un archivo con un indicador de progreso", "um programa que baixa um arquivo com um indicador de progresso"),
        ("RDownload fetches the file while a gauge follows it.", "RDownload descarga el archivo mientras un indicador lo sigue.", "RDownload baixa o arquivo enquanto um indicador o acompanha."),
    ),
    "examples/network/echo_server.rr": (
        ("a small web server that answers requests itself", "un pequeño servidor web que responde las peticiones por sí mismo", "um pequeno servidor web que responde às requisições sozinho"),
        ("An RServerSocket in http mode answers each request.", "Un RServerSocket en modo http responde cada petición.", "Um RServerSocket em modo http responde a cada requisição."),
    ),
    "examples/network/http_json.rr": (
        ("a program that fetches a JSON weather forecast over HTTP and shows it", "un programa que obtiene un pronóstico del tiempo en JSON por HTTP y lo muestra", "um programa que busca uma previsão do tempo em JSON por HTTP e a mostra"),
        ("RHttp downloads the document and RJson reads the three days.", "RHttp descarga el documento y RJson lee los tres días.", "RHttp baixa o documento e RJson lê os três dias."),
    ),
    "examples/media/video.rr": (
        ("a video player with Play and Pause and a frame counter", "un reproductor de video con Reproducir, Pausa y un contador de cuadros", "um reprodutor de vídeo com Play, Pausa e um contador de quadros"),
        ("An RVideo plays an AVI file on the form.", "Un RVideo reproduce un archivo AVI en el formulario.", "Um RVideo reproduz um arquivo AVI no formulário."),
    ),
    "examples/media/wave.rr": (
        ("a program that plays a WAV sound and records one from the microphone", "un programa que reproduce un sonido WAV y graba uno del micrófono", "um programa que toca um som WAV e grava um do microfone"),
        ("RWave plays a file, and records a second from the microphone.", "RWave reproduce un archivo y graba un segundo del micrófono.", "RWave toca um arquivo e grava um segundo do microfone."),
    ),
    "examples/media/midi.rr": (
        ("a MIDI song player with Play, Pause, Stop and a position bar", "un reproductor de canciones MIDI con Reproducir, Pausa, Detener y barra de posición", "um reprodutor de músicas MIDI com Play, Pausa, Parar e barra de posição"),
        ("An RMIDI plays a song and a track bar shows the position.", "Un RMIDI reproduce una canción y una barra muestra la posición.", "Um RMIDI toca uma música e uma barra mostra a posição."),
    ),
    "examples/web/browser.rr": (
        ("a program that knows it runs in a web page and moves between pages", "un programa que sabe que corre en una página web y se mueve entre páginas", "um programa que sabe que roda em uma página web e navega entre páginas"),
        ("It asks the browser with RJavaScript and routes pages with RRouter.", "Consulta al navegador con RJavaScript y enruta páginas con RRouter.", "Consulta o navegador com RJavaScript e roteia páginas com RRouter."),
    ),
    "examples/translations/languages.rr": (
        ("one program that speaks four languages", "un programa que habla cuatro idiomas", "um programa que fala quatro idiomas"),
        ("Its text is written in English, wrapped in TR$, and shown in the language you pick.", "Su texto está en inglés, dentro de TR$, y se muestra en el idioma elegido.", "Seu texto está em inglês, dentro de TR$, e é mostrado no idioma escolhido."),
    ),
    "examples/debugging/shop.rr": (
        ("a small shop order book with nested TYPEs, to practise the debugger", "un pequeño libro de pedidos de una tienda con TYPE anidados, para practicar el depurador", "um pequeno livro de pedidos de uma loja com TYPEs aninhados, para praticar o depurador"),
        ("It builds orders from nested TYPEs and arrays of them.", "Construye pedidos con TYPE anidados y arreglos de ellos.", "Constrói pedidos com TYPEs aninhados e arrays deles."),
    ),
    "examples/ai/ask.rr": (
        ("a console program that asks an AI a question", "un programa de consola que le hace una pregunta a una IA", "um programa de console que faz uma pergunta a uma IA"),
        ("It sends a question to RAI and prints the answer.", "Envía una pregunta a RAI e imprime la respuesta.", "Envia uma pergunta ao RAI e imprime a resposta."),
    ),
}

# The recipes that are not an eval solution (tests/ai-eval/apps) nor next to a school task.
REAL_RECIPES: dict[str, tuple[tuple[str, str, str], tuple[str, str, str]]] = {
    "memory_bytes.rr": (
        ("memory as a byte array with peek, poke and 16-bit words", "la memoria como un arreglo de bytes con peek, poke y palabras de 16 bits", "a memória como um array de bytes com peek, poke e palavras de 16 bits"),
        ("It shows a byte array as memory: read and write bytes and 16-bit words.", "Muestra un arreglo de bytes como memoria: lee y escribe bytes y palabras de 16 bits.", "Mostra um array de bytes como memória: lê e escreve bytes e palavras de 16 bits."),
    ),
    "flags_bits.rr": (
        ("a window that adds, subtracts and rotates 8-bit numbers and shows the carry, zero and sign flags", "una ventana que suma, resta y rota números de 8 bits y muestra los indicadores de acarreo, cero y signo", "uma janela que soma, subtrai e rotaciona números de 8 bits e mostra os sinalizadores de carry, zero e sinal"),
        ("It does 8-bit arithmetic and shows which flags it sets.", "Hace aritmética de 8 bits y muestra qué indicadores activa.", "Faz aritmética de 8 bits e mostra quais sinalizadores ativa."),
    ),
    "hex_dump.rr": (
        ("a hex dump viewer for a block of bytes with an ASCII column", "un visor de volcado hexadecimal de un bloque de bytes con columna ASCII", "um visualizador de dump hexadecimal de um bloco de bytes com coluna ASCII"),
        ("It lists bytes sixteen to a line with their addresses and ASCII.", "Lista los bytes de dieciséis en dieciséis con sus direcciones y ASCII.", "Lista os bytes de dezesseis em dezesseis com seus endereços e ASCII."),
    ),
    "load_binary.rr": (
        ("a loader that reads a binary file into a byte array and shows its header", "un cargador que lee un archivo binario en un arreglo de bytes y muestra su cabecera", "um carregador que lê um arquivo binário em um array de bytes e mostra seu cabeçalho"),
        ("It reads a file with RFileStream into memory and decodes its header.", "Lee un archivo con RFileStream en memoria y decodifica su cabecera.", "Lê um arquivo com RFileStream na memória e decodifica seu cabeçalho."),
    ),
    "video_memory.rr": (
        ("a window that draws a block of video memory as pixels with colours on a canvas", "una ventana que dibuja un bloque de memoria de video como píxeles con colores en un lienzo", "uma janela que desenha um bloco de memória de vídeo como pixels com cores em uma tela"),
        ("It turns bitmap bytes and colour attributes into pixels on an RCanvas.", "Convierte bytes de mapa de bits y atributos de color en píxeles en un RCanvas.", "Converte bytes de bitmap e atributos de cor em pixels em um RCanvas."),
    ),
    "keyboard_matrix.rr": (
        ("a keyboard matrix: key down and key up update a port the program reads", "una matriz de teclado: al pulsar y soltar una tecla se actualiza un puerto que el programa lee", "uma matriz de teclado: ao pressionar e soltar uma tecla atualiza-se uma porta que o programa lê"),
        ("OnKeyDown and OnKeyUp set and clear bits of a virtual port.", "OnKeyDown y OnKeyUp activan y borran bits de un puerto virtual.", "OnKeyDown e OnKeyUp ligam e desligam bits de uma porta virtual."),
    ),
}

# Other ways to ask for the six recipe programs that are in the corpus (the recipe search must still
# find the recipe for them: build_corpus.py asks it, and drops the phrasing when it does not).
REQ_ALTS: dict[str, dict[str, list[str]]] = {
    "memory_bytes.rr": {
        "en": ["a program that treats an array of bytes as a computer's memory", "a memory simulator: read and write bytes at addresses, and 16-bit words"],
        "es": ["un programa que trata un arreglo de bytes como la memoria de una computadora", "un simulador de memoria: leer y escribir bytes en direcciones y palabras de 16 bits"],
        "pt": ["um programa que trata um array de bytes como a memória de um computador", "um simulador de memória: ler e escrever bytes em endereços e palavras de 16 bits"],
    },
    "flags_bits.rr": {
        "en": ["a bit calculator that adds, subtracts and rotates bytes and shows the flags", "an ALU demo with carry, zero, sign and overflow flags"],
        "es": ["una calculadora de bits que suma, resta y rota bytes y muestra las banderas", "una demostración de una ALU con las banderas de acarreo, cero, signo y desbordamiento"],
        "pt": ["uma calculadora de bits que soma, subtrai e rotaciona bytes e mostra os sinalizadores", "uma demonstração de uma ULA com os sinalizadores de carry, zero, sinal e estouro"],
    },
    "hex_dump.rr": {
        "en": ["a hex editor style viewer that lists memory sixteen bytes per row", "a memory viewer with addresses, hexadecimal bytes and an ASCII column"],
        "es": ["un visor estilo editor hexadecimal que lista la memoria de dieciséis en dieciséis bytes", "un visor de memoria con direcciones, bytes en hexadecimal y una columna ASCII"],
        "pt": ["um visualizador estilo editor hexadecimal que lista a memória de dezesseis em dezesseis bytes", "um visualizador de memória com endereços, bytes em hexadecimal e uma coluna ASCII"],
    },
    "load_binary.rr": {
        "en": ["a program that loads a ROM image file into memory and reads its header", "a binary file loader that shows the header fields of a snapshot"],
        "es": ["un programa que carga un archivo de imagen ROM en memoria y lee su cabecera", "un cargador de archivos binarios que muestra los campos de la cabecera de una instantánea"],
        "pt": ["um programa que carrega um arquivo de imagem ROM na memória e lê seu cabeçalho", "um carregador de arquivos binários que mostra os campos do cabeçalho de um snapshot"],
    },
    "video_memory.rr": {
        "en": ["a screen drawn from a framebuffer with ink and paper colour attributes", "a window that shows video memory as a bitmap on a canvas"],
        "es": ["una pantalla dibujada desde un framebuffer con atributos de color de tinta y papel", "una ventana que muestra la memoria de video como un mapa de bits en un lienzo"],
        "pt": ["uma tela desenhada a partir de um framebuffer com atributos de cor de tinta e papel", "uma janela que mostra a memória de vídeo como um bitmap em uma tela"],
    },
    "keyboard_matrix.rr": {
        "en": ["a virtual keyboard matrix with rows and columns read through an input port", "a program that turns key presses into bits of a port, active low"],
        "es": ["una matriz de teclado virtual con filas y columnas leída por un puerto de entrada", "un programa que convierte las teclas pulsadas en bits de un puerto, activo en bajo"],
        "pt": ["uma matriz de teclado virtual com linhas e colunas lida por uma porta de entrada", "um programa que converte teclas pressionadas em bits de uma porta, ativo em baixo"],
    },
}

MAX_REAL_LINES = 160


def load_real(checker) -> tuple[list[Program], list[str]]:
    """The real programs that compile (and the notes of those that do not)."""
    out: list[Program] = []
    notes: list[str] = []
    items: list[tuple[str, tuple, str]] = [(p, v, "") for p, v in REAL.items()] + [(f"examples/recipes/{f}", v, f) for f, v in REAL_RECIPES.items()]
    for rel, (req, does), recipe in items:
        if recipe in EVAL_RECIPES or recipe in HOLDOUT_ADJACENT_RECIPES:
            raise RuntimeError(f"{rel}: an eval or hold-out-adjacent recipe is not allowed")
        path = REPO / rel
        text = path.read_text(encoding="utf-8")
        if len(text.splitlines()) > MAX_REAL_LINES:
            notes.append(f"{rel}: {len(text.splitlines())} lines, over {MAX_REAL_LINES}")
            continue
        main = "main.rr"
        ok, said = checker.compiles([(main, text)], main)
        if not ok:
            notes.append(f"{rel}: does not compile alone: {said.splitlines()[0]}")
            continue
        out.append(
            Program(
                id=rel.replace("/", ":"),
                files=[(main, text)],
                main=main,
                sources=[rel],
                generator="real",
                kind="gui" if "CREATE" in text and "ShowModal" in text else "console",
                req=dict(zip(LANGS, req)),
                does=dict(zip(LANGS, does)),
                recipe=recipe,
                req_alts=REQ_ALTS.get(recipe, {}),
            )
        )
    return out, notes


# ---- 2. templates ---------------------------------------------------------------------------------------
def _hdr(title: str, what: str) -> str:
    return f"' {title}: {what}\n"


# Words in three languages that the templates use for captions.
W = {
    "name": ("Name", "Nombre", "Nome"),
    "your_name": ("Your name:", "Tu nombre:", "Seu nome:"),
    "greet": ("&Greet", "&Saludar", "&Cumprimentar"),
    "hello": ("Hello", "Hola", "Olá"),
    "plus": ("+", "+", "+"),
    "reset": ("&Reset", "&Reiniciar", "&Reiniciar"),
    "count": ("Count", "Contador", "Contador"),
    "check": ("&Check", "&Comprobar", "&Verificar"),
    "start": ("&Start", "&Iniciar", "&Iniciar"),
    "number": ("Number:", "Número:", "Número:"),
    "text": ("Text:", "Texto:", "Texto:"),
    "words": ("Words", "Palabras", "Palavras"),
    "letters": ("Letters", "Letras", "Letras"),
    "yes_pal": ("is a palindrome", "es un palíndromo", "é um palíndromo"),
    "no_pal": ("is not a palindrome", "no es un palíndromo", "não é um palíndromo"),
    "guess": ("Your guess:", "Tu intento:", "Seu palpite:"),
    "too_low": ("Too low", "Muy bajo", "Muito baixo"),
    "too_high": ("Too high", "Muy alto", "Muito alto"),
    "right": ("Right!", "¡Correcto!", "Certo!"),
    "red": ("Red", "Rojo", "Vermelho"),
    "green": ("Green", "Verde", "Verde"),
    "blue": ("Blue", "Azul", "Azul"),
    "show": ("&Show", "&Mostrar", "&Mostrar"),
    "stop": ("S&top", "&Detener", "&Parar"),
    "is_prime": ("is prime", "es primo", "é primo"),
    "not_prime": ("is not prime", "no es primo", "não é primo"),
    "agree": ("I agree", "Estoy de acuerdo", "Eu concordo"),
    "news": ("Send me news", "Envíame novedades", "Envie-me novidades"),
    "chosen": ("You chose:", "Elegiste:", "Você escolheu:"),
    "progress": ("Progress", "Progreso", "Progresso"),
}


def w(key: str, lang: int) -> str:
    return W[key][lang]


def _form(title: str, cw: int, ch: int, body: str, decls: str = "", globals_: str = "", subs: str = "", tail: str = "Form.ShowModal") -> str:
    parts = [decls.rstrip() + "\n\n" if decls else "", globals_.rstrip() + "\n\n" if globals_ else ""]
    parts.append(f'CREATE Form AS RForm\n    Caption = "{title}"\n    ClientWidth = {cw}: ClientHeight = {ch}\n    Center\n{body}END CREATE\n\n')
    if subs:
        parts.append(subs.rstrip() + "\n\n")
    parts.append(tail + "\n")
    return "".join(parts)


def t_greeter(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    title = r.choice(["Greeter", "Hello", "Welcome", "Say hi"]) if lang == 0 else r.choice(["Saludador", "Bienvenida", "Hola"]) if lang == 1 else r.choice(["Saudador", "Boas-vindas", "Olá"])
    default = r.choice(["World", "Ana", "Sam", "Luz", "Mundo"])
    body = (
        f'    CREATE NameLabel AS RLabel\n        Caption = "{w("your_name", lang)}"\n        Left = 16: Top = 22: Width = 90\n    END CREATE\n'
        f'    CREATE NameEdit AS REdit\n        Text = "{default}"\n        Left = 112: Top = 18: Width = 190\n    END CREATE\n'
        f'    CREATE GreetButton AS RButton\n        Caption = "{w("greet", lang)}"\n        Left = 112: Top = 54: Width = 100\n        Default = 1\n        OnClick = Greet\n    END CREATE\n'
        f'    CREATE Answer AS RLabel\n        Caption = ""\n        Left = 16: Top = 96: Width = 286\n    END CREATE\n'
    )
    subs = f'SUB Greet\n    Answer.Caption = "{w("hello", lang)}, " + NameEdit.Text + "!"\nEND SUB'
    code = _hdr(title, "type a name and press the button") + _form(title, 320, 130, body, decls="DECLARE SUB Greet", subs=subs, tail="NameEdit.SetFocus\nForm.ShowModal")
    req = {
        "en": "a window where I type my name and a button greets me with a label",
        "es": "una ventana donde escribo mi nombre y un botón me saluda en una etiqueta",
        "pt": "uma janela onde digito meu nome e um botão me cumprimenta em um rótulo",
    }
    does = {"en": "Type a name and press the button: the label greets you.", "es": "Escribe un nombre y pulsa el botón: la etiqueta te saluda.", "pt": "Digite um nome e pressione o botão: o rótulo cumprimenta você."}
    return code, req, does


def t_counter(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    step = r.choice([1, 1, 2, 5, 10])
    limit = r.choice([0, 0, 10, 20, 100])
    title = w("count", lang)
    cap = f"+{step}"
    cap_en = f"{step}"
    body = (
        '    CREATE Display AS RLabel\n        Align = alTop: Height = 64\n        Caption = "0": Alignment = 2\n        Font.Size = 28\n    END CREATE\n'
        f'    CREATE PlusBtn AS RButton\n        Caption = "{cap}"\n        Left = 20: Top = 76: Width = 100: Height = 32\n        OnClick = Plus\n    END CREATE\n'
        f'    CREATE ResetBtn AS RButton\n        Caption = "{w("reset", lang)}"\n        Left = 140: Top = 76: Width = 100: Height = 32\n        OnClick = ResetCount\n    END CREATE\n'
    )
    if limit:
        plus = f"SUB Plus\n    IF count + {step} <= {limit} THEN count = count + {step}\n    Display.Caption = STR$(count)\nEND SUB"
    else:
        plus = f"SUB Plus\n    count = count + {step}\n    Display.Caption = STR$(count)\nEND SUB"
    subs = plus + "\n\nSUB ResetCount\n    count = 0\n    Display.Caption = \"0\"\nEND SUB"
    code = _hdr(title, "a counter") + _form(title, 260, 124, body, decls="DECLARE SUB Plus\nDECLARE SUB ResetCount", globals_="DIM count AS INTEGER", subs=subs)
    lim = {"en": f", up to {limit}", "es": f", hasta {limit}", "pt": f", até {limit}"} if limit else {"en": "", "es": "", "pt": ""}
    req = {
        "en": f"a counter with a button that adds {cap_en} each time{lim['en']} and a reset button",
        "es": f"un contador con un botón que suma {cap_en} cada vez{lim['es']} y un botón para reiniciar",
        "pt": f"um contador com um botão que soma {cap_en} a cada vez{lim['pt']} e um botão para reiniciar",
    }
    does = {"en": f"The button adds {step} to the number shown; Reset sets it to 0.", "es": f"El botón suma {step} al número mostrado; Reiniciar lo pone en 0.", "pt": f"O botão soma {step} ao número mostrado; Reiniciar o põe em 0."}
    return code, req, does


def t_palindrome(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    title = ["Palindrome checker", "Comprobar palíndromos", "Verificar palíndromos"][lang]
    body = (
        f'    CREATE TextLabel AS RLabel\n        Caption = "{w("text", lang)}"\n        Left = 16: Top = 22: Width = 60\n    END CREATE\n'
        '    CREATE WordEdit AS REdit\n        Text = "level"\n        Left = 80: Top = 18: Width = 220\n    END CREATE\n'
        f'    CREATE CheckBtn AS RButton\n        Caption = "{w("check", lang)}"\n        Left = 80: Top = 54: Width = 100\n        OnClick = CheckWord\n    END CREATE\n'
        '    CREATE Answer AS RLabel\n        Caption = ""\n        Left = 16: Top = 96: Width = 284\n    END CREATE\n'
    )
    subs = (
        "SUB CheckWord\n    DIM s AS STRING\n    s = LCASE$(WordEdit.Text)\n    IF s = REVERSE$(s) THEN\n"
        f'        Answer.Caption = WordEdit.Text + " {w("yes_pal", lang)}"\n    ELSE\n        Answer.Caption = WordEdit.Text + " {w("no_pal", lang)}"\n    END IF\nEND SUB'
    )
    code = _hdr(title, "is a word the same backwards?") + _form(title, 316, 130, body, decls="DECLARE SUB CheckWord", subs=subs)
    req = {"en": "a program that checks whether a word is a palindrome", "es": "un programa que comprueba si una palabra es un palíndromo", "pt": "um programa que verifica se uma palavra é um palíndromo"}
    does = {"en": "It compares the word, in lower case, with itself reversed.", "es": "Compara la palabra, en minúsculas, con ella misma al revés.", "pt": "Compara a palavra, em minúsculas, com ela mesma ao contrário."}
    return code, req, does


def t_table(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    upto = r.choice([10, 12, 20])
    title = ["Times table", "Tabla de multiplicar", "Tabuada"][lang]
    body = (
        f'    CREATE NumLabel AS RLabel\n        Caption = "{w("number", lang)}"\n        Left = 16: Top = 18: Width = 70\n    END CREATE\n'
        '    CREATE NumEdit AS REdit\n        Text = "7"\n        Left = 90: Top = 14: Width = 60\n    END CREATE\n'
        f'    CREATE ShowBtn AS RButton\n        Caption = "{w("show", lang)}"\n        Left = 160: Top = 12: Width = 90: Height = 26\n        OnClick = ShowTable\n    END CREATE\n'
        '    CREATE Lines AS RListBox\n        Left = 16: Top = 48: Width = 234: Height = 190\n    END CREATE\n'
    )
    subs = (
        "SUB ShowTable\n    DIM n AS INTEGER, i AS INTEGER\n    n = VAL(NumEdit.Text)\n    Lines.Clear\n"
        f'    FOR i = 1 TO {upto}\n        Lines.AddItems STR$(n) + " x " + STR$(i) + " = " + STR$(n * i)\n    NEXT i\nEND SUB'
    )
    code = _hdr(title, f"the table of a number up to {upto}") + _form(title, 266, 252, body, decls="DECLARE SUB ShowTable", subs=subs)
    req = {
        "en": f"a program that shows the multiplication table of a number up to {upto} in a list",
        "es": f"un programa que muestra en una lista la tabla de multiplicar de un número hasta el {upto}",
        "pt": f"um programa que mostra em uma lista a tabuada de um número até {upto}",
    }
    does = {"en": f"Type a number and press the button: the list shows its table from 1 to {upto}.", "es": f"Escribe un número y pulsa el botón: la lista muestra su tabla del 1 al {upto}.", "pt": f"Digite um número e pressione o botão: a lista mostra sua tabuada de 1 a {upto}."}
    return code, req, does


def t_guess(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    top = r.choice([10, 50, 100, 1000])
    title = ["Guess the number", "Adivina el número", "Adivinhe o número"][lang]
    body = (
        f'    CREATE GuessLabel AS RLabel\n        Caption = "{w("guess", lang)}"\n        Left = 16: Top = 22: Width = 90\n    END CREATE\n'
        '    CREATE GuessEdit AS REdit\n        Text = ""\n        Left = 112: Top = 18: Width = 80\n    END CREATE\n'
        f'    CREATE TryBtn AS RButton\n        Caption = "{w("check", lang)}"\n        Left = 204: Top = 16: Width = 90\n        Default = 1\n        OnClick = TryGuess\n    END CREATE\n'
        '    CREATE Hint AS RLabel\n        Caption = ""\n        Left = 16: Top = 64: Width = 280\n    END CREATE\n'
    )
    subs = (
        f"SUB NewSecret\n    secret = INT(RND * {top}) + 1\nEND SUB\n\n"
        "SUB TryGuess\n    DIM g AS INTEGER\n    g = VAL(GuessEdit.Text)\n    IF g < secret THEN\n"
        f'        Hint.Caption = "{w("too_low", lang)}"\n    ELSEIF g > secret THEN\n        Hint.Caption = "{w("too_high", lang)}"\n    ELSE\n        Hint.Caption = "{w("right", lang)}"\n        NewSecret\n    END IF\nEND SUB'
    )
    code = _hdr(title, f"a number from 1 to {top}") + _form(title, 310, 110, body, decls="DECLARE SUB NewSecret\nDECLARE SUB TryGuess", globals_="DIM secret AS INTEGER", subs=subs, tail="RANDOMIZE\nNewSecret\nGuessEdit.SetFocus\nForm.ShowModal")
    req = {
        "en": f"a number guessing game: the program picks a number from 1 to {top} and says too low or too high",
        "es": f"un juego de adivinar un número: el programa elige del 1 al {top} y dice si es muy bajo o muy alto",
        "pt": f"um jogo de adivinhar o número: o programa escolhe de 1 a {top} e diz se é muito baixo ou muito alto",
    }
    does = {"en": "It picks a secret number and answers each guess with too low, too high or right.", "es": "Elige un número secreto y responde a cada intento con muy bajo, muy alto o correcto.", "pt": "Escolhe um número secreto e responde a cada palpite com muito baixo, muito alto ou certo."}
    return code, req, does


def t_clock(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    size = r.choice([24, 32, 40])
    title = ["Clock", "Reloj", "Relógio"][lang]
    body = (
        f'    CREATE Face AS RLabel\n        Align = alClient\n        Caption = "": Alignment = 2\n        Font.Size = {size}\n    END CREATE\n'
        "    CREATE Ticker AS RTimer\n        Interval = 1000\n        Enabled = 1\n        OnTimer = Tick\n    END CREATE\n"
    )
    subs = "SUB Tick\n    Face.Caption = TIME$\nEND SUB"
    code = _hdr(title, "the time, updated every second") + _form(title, 260, 90, body, decls="DECLARE SUB Tick", subs=subs, tail="Tick\nForm.ShowModal")
    req = {"en": "a digital clock window that shows the time and updates every second", "es": "una ventana con un reloj digital que muestra la hora y se actualiza cada segundo", "pt": "uma janela com um relógio digital que mostra a hora e atualiza a cada segundo"}
    does = {"en": "An RTimer ticks every second and the label shows TIME$.", "es": "Un RTimer late cada segundo y la etiqueta muestra TIME$.", "pt": "Um RTimer bate a cada segundo e o rótulo mostra TIME$."}
    return code, req, does


def t_mixer(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    title = ["Colour mixer", "Mezclador de colores", "Misturador de cores"][lang]
    rows = []
    for i, (nm, key) in enumerate([("Red", "red"), ("Green", "green"), ("Blue", "blue")]):
        rows.append(
            f'    CREATE {nm}Label AS RLabel\n        Caption = "{w(key, lang)}"\n        Left = 16: Top = {16 + i * 36}: Width = 56\n    END CREATE\n'
            f"    CREATE {nm}Bar AS RTrackBar\n        Left = 80: Top = {12 + i * 36}: Width = 200: Height = 30\n        Min = 0: Max = 255: Position = {r.choice([0, 64, 128, 255])}\n        OnChange = Mix\n    END CREATE\n"
        )
    rows.append("    CREATE Swatch AS RPanel\n        Left = 16: Top = 128: Width = 264: Height = 60\n    END CREATE\n")
    subs = "SUB Mix\n    Swatch.Color = RGB(RedBar.Position, GreenBar.Position, BlueBar.Position)\nEND SUB"
    code = _hdr(title, "three sliders make a colour") + _form(title, 296, 204, "".join(rows), decls="DECLARE SUB Mix", subs=subs, tail="Mix\nForm.ShowModal")
    req = {"en": "a colour mixer with red, green and blue sliders and a panel showing the colour", "es": "un mezclador de colores con deslizadores de rojo, verde y azul y un panel que muestra el color", "pt": "um misturador de cores com controles deslizantes de vermelho, verde e azul e um painel que mostra a cor"}
    does = {"en": "Three track bars set the red, green and blue of the panel.", "es": "Tres barras fijan el rojo, verde y azul del panel.", "pt": "Três barras definem o vermelho, verde e azul do painel."}
    return code, req, does


def t_prime(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    title = ["Prime checker", "Comprobar primos", "Verificar primos"][lang]
    body = (
        f'    CREATE NumLabel AS RLabel\n        Caption = "{w("number", lang)}"\n        Left = 16: Top = 22: Width = 70\n    END CREATE\n'
        f'    CREATE NumEdit AS REdit\n        Text = "{r.choice([7, 91, 97, 221, 541])}"\n        Left = 90: Top = 18: Width = 90\n    END CREATE\n'
        f'    CREATE CheckBtn AS RButton\n        Caption = "{w("check", lang)}"\n        Left = 190: Top = 16: Width = 90\n        Default = 1\n        OnClick = CheckNumber\n    END CREATE\n'
        '    CREATE Answer AS RLabel\n        Caption = ""\n        Left = 16: Top = 62: Width = 270\n    END CREATE\n'
    )
    subs = (
        "FUNCTION IsPrime(n AS INTEGER) AS INTEGER\n    DIM d AS INTEGER\n    IF n < 2 THEN\n        IsPrime = 0\n        EXIT FUNCTION\n    END IF\n    d = 2\n    DO WHILE d * d <= n\n        IF n MOD d = 0 THEN\n            IsPrime = 0\n            EXIT FUNCTION\n        END IF\n        d = d + 1\n    LOOP\n    IsPrime = 1\nEND FUNCTION\n\n"
        f'SUB CheckNumber\n    DIM n AS INTEGER\n    n = VAL(NumEdit.Text)\n    IF IsPrime(n) THEN\n        Answer.Caption = STR$(n) + " {w("is_prime", lang)}"\n    ELSE\n        Answer.Caption = STR$(n) + " {w("not_prime", lang)}"\n    END IF\nEND SUB'
    )
    code = _hdr(title, "is a number prime?") + _form(title, 300, 100, body, decls="DECLARE FUNCTION IsPrime(n AS INTEGER) AS INTEGER\nDECLARE SUB CheckNumber", subs=subs)
    req = {"en": "a program that tells me whether a number is prime", "es": "un programa que me dice si un número es primo", "pt": "um programa que me diz se um número é primo"}
    does = {"en": "A FUNCTION tries the divisors up to the square root.", "es": "Una FUNCTION prueba los divisores hasta la raíz cuadrada.", "pt": "Uma FUNCTION testa os divisores até a raiz quadrada."}
    return code, req, does


def t_choices(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    title = ["Options", "Opciones", "Opções"][lang]
    body = (
        f'    CREATE AgreeBox AS RCheckBox\n        Caption = "{w("agree", lang)}"\n        Left = 16: Top = 16: Width = 200\n    END CREATE\n'
        f'    CREATE NewsBox AS RCheckBox\n        Caption = "{w("news", lang)}"\n        Left = 16: Top = 44: Width = 200\n    END CREATE\n'
        f'    CREATE ShowBtn AS RButton\n        Caption = "{w("show", lang)}"\n        Left = 16: Top = 78: Width = 100\n        OnClick = ShowChoices\n    END CREATE\n'
        '    CREATE Answer AS RLabel\n        Caption = ""\n        Left = 16: Top = 118: Width = 270\n    END CREATE\n'
    )
    subs = (
        f'SUB ShowChoices\n    DIM s AS STRING\n    s = "{w("chosen", lang)} "\n    IF AgreeBox.Checked THEN s = s + "{w("agree", lang)}; "\n    IF NewsBox.Checked THEN s = s + "{w("news", lang)}"\n    Answer.Caption = s\nEND SUB'
    )
    code = _hdr(title, "two check boxes and a button that reports them") + _form(title, 300, 150, body, decls="DECLARE SUB ShowChoices", subs=subs)
    req = {"en": "a form with two check boxes and a button that tells me which are ticked", "es": "un formulario con dos casillas y un botón que me dice cuáles están marcadas", "pt": "um formulário com duas caixas de seleção e um botão que me diz quais estão marcadas"}
    does = {"en": "The button reads each box's Checked and lists the ticked ones.", "es": "El botón lee el Checked de cada casilla y lista las marcadas.", "pt": "O botão lê o Checked de cada caixa e lista as marcadas."}
    return code, req, does


def t_colors(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    title = ["Colour list", "Lista de colores", "Lista de cores"][lang]
    body = (
        f'    CREATE Picker AS RComboBox\n        Left = 16: Top = 16: Width = 150\n        AddItems "{w("red", lang)}", "{w("green", lang)}", "{w("blue", lang)}"\n        ItemIndex = 0\n        OnChange = Pick\n    END CREATE\n'
        "    CREATE Swatch AS RPanel\n        Left = 16: Top = 56: Width = 150: Height = 60\n    END CREATE\n"
    )
    subs = "SUB Pick\n    SELECT CASE Picker.ItemIndex\n        CASE 0: Swatch.Color = clRed\n        CASE 1: Swatch.Color = clGreen\n        CASE 2: Swatch.Color = clBlue\n    END SELECT\nEND SUB"
    code = _hdr(title, "a drop-down that colours a panel") + _form(title, 184, 132, body, decls="DECLARE SUB Pick", subs=subs, tail="Pick\nForm.ShowModal")
    req = {"en": "a drop-down list of colours that paints a panel the colour I pick", "es": "una lista desplegable de colores que pinta un panel del color que elijo", "pt": "uma lista suspensa de cores que pinta um painel da cor que escolho"}
    does = {"en": "A SELECT CASE on the combo box's ItemIndex sets the panel's colour.", "es": "Un SELECT CASE sobre el ItemIndex de la lista fija el color del panel.", "pt": "Um SELECT CASE no ItemIndex da lista define a cor do painel."}
    return code, req, does


def t_progress(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    stepn = r.choice([2, 5, 10])
    title = w("progress", lang)
    body = (
        '    CREATE Bar AS RProgressBar\n        Left = 16: Top = 20: Width = 260: Height = 22\n        Max = 100: Position = 0\n    END CREATE\n'
        f'    CREATE StartBtn AS RButton\n        Caption = "{w("start", lang)}"\n        Left = 16: Top = 56: Width = 100\n        OnClick = Begin\n    END CREATE\n'
        "    CREATE Ticker AS RTimer\n        Interval = 200\n        Enabled = 0\n        OnTimer = Tick\n    END CREATE\n"
    )
    subs = f"SUB Begin\n    Bar.Position = 0\n    Ticker.Enabled = 1\nEND SUB\n\nSUB Tick\n    Bar.Position = Bar.Position + {stepn}\n    IF Bar.Position >= 100 THEN Ticker.Enabled = 0\nEND SUB"
    code = _hdr(title, "a bar that fills while a timer runs") + _form(title, 292, 100, body, decls="DECLARE SUB Begin\nDECLARE SUB Tick", subs=subs)
    req = {"en": "a window with a progress bar that fills up when I press Start", "es": "una ventana con una barra de progreso que se llena al pulsar Iniciar", "pt": "uma janela com uma barra de progresso que enche ao pressionar Iniciar"}
    does = {"en": f"Start enables a timer that adds {stepn} to the bar each tick until it is full.", "es": f"Iniciar activa un temporizador que suma {stepn} a la barra en cada pulso hasta llenarla.", "pt": f"Iniciar ativa um temporizador que soma {stepn} à barra a cada pulso até enchê-la."}
    return code, req, does


# -- console programs ------------------------------------------------------------------------------------
def t_fib(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    n = r.choice([8, 10, 12, 15, 20, 25])
    code = (
        _hdr("Fibonacci", f"the first {n} Fibonacci numbers")
        + f"DIM a AS LONG, b AS LONG, t AS LONG, i AS INTEGER\na = 0\nb = 1\nFOR i = 1 TO {n}\n    PRINT a\n    t = a + b\n    a = b\n    b = t\nNEXT i\n"
    )
    req = {"en": f"a console program that prints the first {n} Fibonacci numbers", "es": f"un programa de consola que imprime los primeros {n} números de Fibonacci", "pt": f"um programa de console que imprime os primeiros {n} números de Fibonacci"}
    does = {"en": "A FOR loop adds the last two numbers to make the next.", "es": "Un bucle FOR suma los dos últimos números para hacer el siguiente.", "pt": "Um laço FOR soma os dois últimos números para fazer o próximo."}
    return code, req, does


def t_fizz(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    a, b = r.choice([(3, 5), (2, 7), (3, 4), (4, 6)])
    n = r.choice([20, 30, 50, 100])
    code = (
        _hdr("Fizz and buzz", f"count to {n}; multiples of {a} say Fizz, of {b} say Buzz")
        + f'DIM i AS INTEGER\nFOR i = 1 TO {n}\n    IF i MOD {a} = 0 AND i MOD {b} = 0 THEN\n        PRINT "FizzBuzz"\n    ELSEIF i MOD {a} = 0 THEN\n        PRINT "Fizz"\n    ELSEIF i MOD {b} = 0 THEN\n        PRINT "Buzz"\n    ELSE\n        PRINT i\n    END IF\nNEXT i\n'
    )
    req = {
        "en": f"a console program that counts from 1 to {n}, saying Fizz for multiples of {a}, Buzz for multiples of {b} and FizzBuzz for both",
        "es": f"un programa de consola que cuenta del 1 al {n} y dice Fizz en los múltiplos de {a}, Buzz en los de {b} y FizzBuzz en los de ambos",
        "pt": f"um programa de console que conta de 1 a {n} e diz Fizz nos múltiplos de {a}, Buzz nos de {b} e FizzBuzz nos de ambos",
    }
    does = {"en": "MOD finds the multiples; ELSEIF picks the word.", "es": "MOD encuentra los múltiplos; ELSEIF elige la palabra.", "pt": "MOD encontra os múltiplos; ELSEIF escolhe a palavra."}
    return code, req, does


def t_factorials(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    n = r.choice([5, 8, 10, 12, 15])
    code = (
        _hdr("Factorials", f"the factorials of 1 to {n}")
        + f"FUNCTION Factorial(n AS INTEGER) AS DOUBLE\n    DIM i AS INTEGER, f AS DOUBLE\n    f = 1\n    FOR i = 2 TO n\n        f = f * i\n    NEXT i\n    Factorial = f\nEND FUNCTION\n\nDIM k AS INTEGER\nFOR k = 1 TO {n}\n    PRINT k; \"! = \"; Factorial(k)\nNEXT k\n"
    )
    req = {"en": f"a console program that prints the factorials of 1 to {n} using a FUNCTION", "es": f"un programa de consola que imprime los factoriales del 1 al {n} usando una FUNCTION", "pt": f"um programa de console que imprime os fatoriais de 1 a {n} usando uma FUNCTION"}
    does = {"en": "A FUNCTION multiplies 2 up to n; the loop prints each result.", "es": "Una FUNCTION multiplica de 2 a n; el bucle imprime cada resultado.", "pt": "Uma FUNCTION multiplica de 2 a n; o laço imprime cada resultado."}
    return code, req, does


def t_sort(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    nums = r.sample(range(1, 100), r.choice([5, 6, 8]))
    n = len(nums)
    sets = "\n".join(f"v({i}) = {x}" for i, x in enumerate(nums))
    code = (
        _hdr("Sort", "bubble sort of a small array")
        + f"DIM v({n - 1}) AS INTEGER, i AS INTEGER, j AS INTEGER, t AS INTEGER\n{sets}\nFOR i = 0 TO {n - 2}\n    FOR j = 0 TO {n - 2} - i\n        IF v(j) > v(j + 1) THEN\n            t = v(j)\n            v(j) = v(j + 1)\n            v(j + 1) = t\n        END IF\n    NEXT j\nNEXT i\nFOR i = 0 TO {n - 1}\n    PRINT v(i);\n    PRINT \" \";\nNEXT i\nPRINT\n"
    )
    req = {"en": f"a console program that sorts these {n} numbers with a bubble sort and prints them: {', '.join(map(str, nums))}", "es": f"un programa de consola que ordena estos {n} números con el método de burbuja y los imprime: {', '.join(map(str, nums))}", "pt": f"um programa de console que ordena estes {n} números com o método da bolha e os imprime: {', '.join(map(str, nums))}"}
    does = {"en": "Two nested loops swap neighbours until the array is in order.", "es": "Dos bucles anidados intercambian vecinos hasta ordenar el arreglo.", "pt": "Dois laços aninhados trocam vizinhos até ordenar o array."}
    return code, req, does


def t_vowels(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    sample = r.choice(["Hello, RapidR", "programming is fun", "la vida es bella", "BASIC never dies"])
    code = (
        _hdr("Vowels", "counts the vowels of a text")
        + f'DIM s AS STRING, i AS INTEGER, c AS STRING, n AS INTEGER\ns = "{sample}"\nFOR i = 1 TO LEN(s)\n    c = LCASE$(MID$(s, i, 1))\n    IF INSTR("aeiou", c) > 0 THEN n = n + 1\nNEXT i\nPRINT s; " has "; n; " vowels"\n'
    )
    req = {"en": f'a console program that counts the vowels in "{sample}"', "es": f'un programa de consola que cuenta las vocales de "{sample}"', "pt": f'um programa de console que conta as vogais de "{sample}"'}
    does = {"en": "MID$ takes each letter and INSTR looks for it among the vowels.", "es": "MID$ toma cada letra e INSTR la busca entre las vocales.", "pt": "MID$ pega cada letra e INSTR a procura entre as vogais."}
    return code, req, does


def t_leap(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    y0 = r.choice([1900, 1996, 2000, 2020])
    y1 = y0 + r.choice([10, 20, 30])
    code = (
        _hdr("Leap years", f"the leap years from {y0} to {y1}")
        + f'DIM y AS INTEGER\nFOR y = {y0} TO {y1}\n    IF (y MOD 4 = 0 AND y MOD 100 <> 0) OR y MOD 400 = 0 THEN PRINT y; " is a leap year"\nNEXT y\n'
    )
    req = {"en": f"a console program that lists the leap years from {y0} to {y1}", "es": f"un programa de consola que lista los años bisiestos del {y0} al {y1}", "pt": f"um programa de console que lista os anos bissextos de {y0} a {y1}"}
    does = {"en": "The leap-year rule is one IF with MOD, AND and OR.", "es": "La regla del año bisiesto es un IF con MOD, AND y OR.", "pt": "A regra do ano bissexto é um IF com MOD, AND e OR."}
    return code, req, does


def t_select(r: random.Random, lang: int) -> tuple[str, dict, dict]:
    day = r.randint(1, 7)
    names = ["Monday", "Tuesday", "Wednesday", "Thursday", "Friday", "Saturday", "Sunday"]
    cases = "\n".join(f'    CASE {i + 1}: PRINT "{nm}"' for i, nm in enumerate(names))
    code = _hdr("Day names", "SELECT CASE on a number") + f'DIM d AS INTEGER\nd = {day}\nSELECT CASE d\n{cases}\n    CASE ELSE: PRINT "not a day"\nEND SELECT\n'
    req = {"en": f"a console program that prints the name of day number {day} of the week with SELECT CASE", "es": f"un programa de consola que imprime el nombre del día {day} de la semana con SELECT CASE", "pt": f"um programa de console que imprime o nome do dia {day} da semana com SELECT CASE"}
    does = {"en": "SELECT CASE picks the name; CASE ELSE catches anything else.", "es": "SELECT CASE elige el nombre; CASE ELSE atrapa cualquier otro valor.", "pt": "SELECT CASE escolhe o nome; CASE ELSE pega qualquer outro valor."}
    return code, req, does


TEMPLATES = {
    "greeter": (t_greeter, "gui"),
    "counter": (t_counter, "gui"),
    "palindrome": (t_palindrome, "gui"),
    "table": (t_table, "gui"),
    "guess": (t_guess, "gui"),
    "clock": (t_clock, "gui"),
    "mixer": (t_mixer, "gui"),
    "prime": (t_prime, "gui"),
    "choices": (t_choices, "gui"),
    "colors": (t_colors, "gui"),
    "progress": (t_progress, "gui"),
    "fib": (t_fib, "console"),
    "fizz": (t_fizz, "console"),
    "factorials": (t_factorials, "console"),
    "sort": (t_sort, "console"),
    "vowels": (t_vowels, "console"),
    "leap": (t_leap, "console"),
    "select": (t_select, "console"),
}


def make_templates(checker, seed: int = 20261029, per_template: int = 12) -> tuple[list[Program], list[str]]:
    """Template programs that compile; each template is tried with different parameters and
    languages (the captions of a GUI program are in the request's language)."""
    r = random.Random(seed)
    out: list[Program] = []
    notes: list[str] = []
    seen: set[str] = set()
    for name, (fn, kind) in TEMPLATES.items():
        made = 0
        for attempt in range(per_template * 6):
            if made >= per_template:
                break
            lang = attempt % 3
            code, req, does = fn(r, lang)
            if code in seen:
                continue
            seen.add(code)
            ok, said = checker.compiles([("main.rr", code)], "main.rr")
            if not ok:
                notes.append(f"template {name}: {said.splitlines()[0]}")
                if len([n for n in notes if n.startswith(f"template {name}:")]) >= 3:
                    break
                continue
            out.append(Program(id=f"templates:{name}:{made}", files=[("main.rr", code)], main="main.rr", sources=[f"templates:{name}"], generator="template:" + name, kind=kind, req=req, does=does, tags=[f"lang={LANGS[lang]}"]))
            made += 1
    return out, notes
