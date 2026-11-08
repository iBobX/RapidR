<!-- translated-from: windows-api.md c085bed2f6105fe1 -->
# Um só padrão para a API do Windows

Se você já escreveu programas que falam com o próprio Windows — um `DECLARE …
LIB "user32"` aqui, um `CONST WM_CLOSE` ali — provavelmente conhece esta
história. Todo mundo tinha um `WIN32API.INC`. Ele veio de um amigo ou de um
exemplo antigo, e depois foi corrigido na mão: uma constante acrescentada
aqui, uma declaração corrigida ali. Com os anos surgiram muitas versões dele,
e elas não concordavam entre si. Duas delas no mesmo programa entravam em
conflito. Um trecho que funcionava para outra pessoa falhava para você,
porque o seu arquivo tinha outro nome, outro valor ou nada. Muitos de nós
perdemos horas com isso, e ninguém fez nada errado: só não existia um arquivo
comum para compartilhar.

O RapidR agora tem um: o **`RWin32.inc`**, integrado.

## Primeiro: nada do que você tem deixa de funcionar

- **Seus programas continuam funcionando como estão.** Um programa com o
  próprio `WIN32API.INC` (ou qualquer outro arquivo incluído) é compilado e
  executado exatamente como antes, por meio da camada de compatibilidade com
  o RapidQ do RapidR. Você não precisa mudar nada, nem hoje nem depois.
- **Seus arquivos incluídos continuam sendo seus.** O RapidR nunca os
  substitui nem os edita. Se você prefere o seu arquivo, fique com ele.
- **Nada é trocado por você.** O importador apenas *oferece* a mudança, e
  mostra cada diferença antes que qualquer coisa aconteça (abaixo).

## A mudança, se você quiser: uma linha

```basic
$INCLUDE "RWin32.inc"
```

É só isso. Não há arquivo para baixar, procurar ou copiar: o `RWin32.inc` faz
parte do RapidR, em todo computador e em toda instalação (inclusive nos
downloads para Windows 7 e XP). Qualquer caminho ou combinação de maiúsculas
funciona (`$INCLUDE "C:\rapidq\include\RWIN32.INC"` lê o mesmo arquivo
integrado), então uma linha antiga só precisa que o nome do arquivo seja
trocado.

**O RapidR ajuda você a trocar.** Quando você importa um programa antigo
([Importar programas antigos](../importing-rapidq.md)) que inclui um
`WIN32API.INC`:

- a cópia mantém o seu arquivo, como sempre, e o RapidR confere que ela
  compila exatamente como o original;
- o relatório então diz que o `RWin32.inc` pode ocupar o lugar dele, e
  **lista, para os nomes que o seu programa usa, tudo o que seria
  diferente**: uma constante com outro valor, uma função que o seu arquivo
  chamava por outro nome de DLL ou com outra quantidade de argumentos, um
  nome que o `RWin32.inc` não tem, e o que o seu arquivo fazia além de
  declarar (um `$DEFINE INTEGER SHORT`, um `$OPTION`);
- você decide. No RapidR Studio, Arquivo ▸ Importar projeto ou arquivo do
  RapidQ… pergunta depois da importação (**Manter meu arquivo** é a resposta
  padrão). Pela linha de comando, `rapidr import-rapidq myprogram.bas
  --rwin32` faz a troca na cópia e confere que a cópia ainda compila. O seu
  programa original nunca é alterado.

## Por que um padrão só ajuda todo mundo

- **Um arquivo para todos nós.** Um exemplo, uma resposta numa discussão ou um
  trecho do manual que usa o `RWin32.inc` funciona no seu computador do mesmo
  jeito que funciona no de quem o escreveu. A RAI, assistente do RapidR, e
  outros assistentes de IA também podem contar com os mesmos nomes, e por
  isso o código deles para Windows acerta com mais frequência.
- **Completo e atual.** O `RWin32.inc` não é digitado à mão: uma ferramenta do
  próprio RapidR o gera a partir dos metadados da API Win32 que a Microsoft
  publica. Ele tem 55 710 constantes, 3675 estruturas (`TYPE`) e 6506
  funções de 29 DLLs do Windows (kernel32, user32, gdi32, advapi32, shell32,
  comdlg32, comctl32, winmm, winspool, ole32, wininet, ws2_32 e outras).
- **Certo no Windows de 64 e de 32 bits.** Os arquivos incluídos antigos foram
  escritos para o Windows de 32 bits, em que um ponteiro e um handle têm 4
  bytes. No Windows de 64 bits de hoje eles têm 8, então uma estrutura como
  `NOTIFYICONDATA`, `MSG` ou `SYSTEM_INFO` precisa de outro layout. O
  `RWin32.inc` tem os dois, e o seu programa recebe o certo para o computador
  para o qual ele é compilado.
- **Os nomes clássicos estão lá.** `MessageBox`, `GetWindowText`,
  `FindWindow`, `SendMessage`, `ShellExecute`, `RECT`, `POINTAPI`, `LOGFONT`
  … significam o que sempre significaram: o nome sem sufixo é a função ou
  estrutura ANSI (“A”), como nos arquivos clássicos. 1496 das 1514 funções
  que um `WIN32API.INC` clássico muito usado declara (98,8%) estão no
  `RWin32.inc` com os mesmos nomes; os nomes modernos com A e W
  (`MessageBoxA`, `MessageBoxW`) também estão lá.
- **Rápido.** Um programa recebe só o que usa: um programa com vinte chamadas
  ao Windows compila em bem menos de um segundo e leva só essas vinte
  declarações.

## A escolha é sua

Use o `RWin32.inc` em programas novos se gostar dele, troque os antigos quando
for conveniente, ou fique com o seu arquivo: as três opções são boas, e o
RapidR aceita cada uma delas. Se algo no `RWin32.inc` estiver faltando ou
parecer errado para você, conte para a gente em
[Discussions](https://github.com/iBobX/RapidR/discussions) ou abra um
[issue](https://github.com/iBobX/RapidR/issues): ele é gerado, então uma
correção chega a todos de uma vez.

O RapidR é gratuito e aberto, feito para que qualquer pessoa possa aprender a
programar, e nós o fazemos crescer juntos, pessoas e IA. Agradecemos por você
fazer parte disso.

---

## Como funciona

### Só o que o seu programa usa, e os seus nomes primeiro

O `RWin32.inc` é grande, então o RapidR não o compila inteiro dentro do seu
programa. Ele lê o seu programa, vê quais nomes ele usa e entrega as
declarações desses nomes (e os `TYPE` de que elas precisam) — mais nada.

**Um nome que o seu programa tem por conta própria sempre vence**: uma
constante (`CONST`), uma declaração (`DECLARE`), um tipo (`TYPE`), uma `SUB`
ou `FUNCTION`, uma variável (`DIM`), um objeto (`CREATE`), uma definição
(`$DEFINE`) ou uma variável em que você guarda um valor. Assim, um programa
antigo que declara ele mesmo `CONST WM_MOUSEWHEEL = &H20A`, ou o seu próprio
`DECLARE FUNCTION GetTickCount …`, compila com o `RWin32.inc` sem o erro “já
foi definido”: o seu é usado. (Uma variável e um `TYPE` podem ter o mesmo
nome: `DIM msg AS MSG` funciona.)

### Os números do RAPIDQ.INC, e True e False

Os números do Windows que o RAPIDQ.INC também tem (`MB_OK`, `MB_ICONERROR`,
`VK_F1`, `SND_ASYNC`, os conjuntos de caracteres …) não se repetem no
`RWin32.inc`, então os dois nunca entram em conflito, qualquer que venha
primeiro.

- Em um programa `.bas` que inclui o `RWin32.inc` **sem** `$INCLUDE
  "RAPIDQ.INC"`, a camada de compatibilidade com o RapidQ do RapidR fornece os
  que ele usa, com os números do RAPIDQ.INC (os mesmos do Windows), e `True` =
  1 e `False` = 0, como fazia um `WIN32API.INC` clássico.
- Em um programa RapidR (`.rr`) eles já são integrados, e `True` continua
  sendo o -1 do RapidR.

### Nomes que o RapidR também tem: Sleep, Beep, MessageBox

Algumas funções do Windows têm o mesmo nome de algo que o RapidR já tem
(`Sleep`, `Beep`, `MessageBox`, `SendMessage`, `PlaySound`).

- Em um programa clássico (`.bas`), o `RWin32.inc` entrega a função do
  Windows, como fazia o arquivo incluído antigo: `Sleep 500` espera 500
  milissegundos, via kernel32.
- Em um programa RapidR (`.rr`), a do próprio RapidR é mantida: `SLEEP 1`
  espera um segundo em qualquer sistema. Para chamar a do Windows ali,
  declare-a com um nome seu: `DECLARE SUB WinSleep LIB "kernel32" ALIAS
  "Sleep" (BYVAL ms AS LONG)`.

### 64 bits e 32 bits: `WIN64`

Quando um programa é compilado para 64 bits (Windows em x64 e ARM64, macOS,
Linux e a web), o RapidR define `WIN64`, e `$IFDEF WIN64` escolhe o layout de
64 bits de cada estrutura; para o Windows de 32 bits (os downloads para
Windows 7 e XP) ele não é definido. O seu próprio código também pode usá-lo:

```basic
$IFDEF WIN64
PRINT "64-bit"
$ELSE
PRINT "32-bit"
$ENDIF
```

(A web também define `WIN64`, então as estruturas de um programa têm lá os
mesmos tamanhos que nos desktops de 64 bits; as funções do Windows não são
executadas na web de qualquer maneira.)

### Como as declarações são escritas

- **Handles, ponteiros e outros números do tamanho de um ponteiro** são
  parâmetros `LONG`: o RapidR os passa em um espaço de 64 bits e guarda para
  você um ponteiro de 64 bits que uma DLL devolve
  ([Chamadas de DLL](../language.md#subs-and-functions)).
- **Texto** (`LPSTR`, `LPCSTR`) é `STRING`: dê um texto, ou um buffer
  `SPACE$(n)` que a função preenche. O texto largo (as funções W) é um
  `LONG`: o endereço de um buffer UTF-16. Os nomes clássicos são as funções A,
  que recebem strings comuns.
- **Uma estrutura** é passada por referência (`lprc AS RECT`). Passe `0` para
  “nenhuma” (`NULL`): o RapidR passa um ponteiro nulo.
- **Em um `TYPE`**, um campo de ponteiro ou de handle é um `INT64` no Windows
  de 64 bits e um `LONG` no de 32; um campo de ponteiro para texto é do tipo
  `STRING` (e, em 64 bits, um `LONG` `…Hi` logo depois, para a metade alta do
  ponteiro, sempre 0); os espaços que o compilador do Windows deixa entre os
  campos são campos próprios (`rrPad1`, `rrPad2` …), então `SIZEOF` é o
  tamanho real da estrutura. Uma união sem nome é o seu maior membro.
- **Uma estrutura pequena passada por valor** (`POINT` em `PtInRect`,
  `WindowFromPoint`) é um único `INT64`: `x + y * 4294967296` (ou seja, `y`
  nos 32 bits altos). É assim que o Windows de 64 bits a recebe; no Windows de
  32 bits o mesmo `INT64` dá os mesmos bytes.

### O que não está nele

Constantes GUID, interfaces COM, funções com número variável de argumentos
(`wsprintf`) e algumas funções que recebem uma estrutura grande por valor.
Algumas áreas ficam de fora por enquanto (criptografia, GDI+, ODBC, WinHTTP,
IP Helper …): o RapidR tem componentes próprios para a maioria delas
(`RSQLITE`, `RDB`, `RSOCKET` …), e você sempre pode usar `DECLARE` para o que
precisar, junto ao arquivo incluído.

### No macOS, no Linux e na web

Um programa que inclui o `RWin32.inc` compila em qualquer lugar; uma chamada
a uma DLL do Windows é executada no Windows e, nos outros sistemas, para com
uma mensagem clara que nomeia a função. Veja
[o exemplo](../../../examples/windows/rwin32_system_info.rr).

### O arquivo em si

Você pode lê-lo: [crates/rapidr-preprocessor/src/libraries/RWin32.inc](../../../crates/rapidr-preprocessor/src/libraries/RWin32.inc)
no código-fonte do RapidR. O cabeçalho dele diz de onde ele vem: gerado pela
ferramenta do próprio RapidR (`tools/gen-win32`) a partir dos metadados da API
Win32 que a Microsoft publica no projeto windows-rs, sob a Licença MIT, com o
aviso de copyright da Microsoft
([procedência](../../legal/win32-declarations-provenance.md)).
`$INCLUDE "RWin32.inc"` sempre lê o arquivo integrado do RapidR, nunca um
arquivo com esse nome que esteja no seu disco.
