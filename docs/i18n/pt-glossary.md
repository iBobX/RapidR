# Portuguese glossary: português do Brasil (pt-BR) for RapidR

**Status: DRAFT 1 (2026-11-06), lane I18N-PT, Preview 6 beta.** An AI translation: the words below are my choices, made to match the Spanish glossary ([glossary-es.md](glossary-es.md)) where Portuguese allows, and open to correction by anyone who speaks Brazilian Portuguese (Settings > Language marks the language "beta"). Nothing here was reviewed by a native speaker yet.

It is the one list of words every pt-BR text of RapidR follows: RapidR Studio's screens (`ide/lang/pt-BR.po`), RapidR's own text (`crates/rapidr-value/lang/pt-BR.po`) and the compiler's messages (`crates/rapidr-diagnostics/lang/pt-BR.po`). The components' Help and the manual come in a later release. It is written in English so it can be checked against the English product; the Portuguese is in the middle column.

The test for every word: would a 12-year-old in São Paulo, Recife or Porto Alegre stop and wonder? If a word is regional, take the plainer one. If Brazilians say the English word every day (*app*, *link*, *e-mail*, *mouse*), use it.

## 1. Style guide

### 1.1 Voice

- **Você, always.** One person, spoken to directly. Never *tu* (clica, escolhe), never *o senhor/a senhora*. Instructions use the **você imperative** ("Escolha um modelo", "Clique em Salvar", "Pressione F5", "Digite o que procura"); that is the form Brazilian software uses.
- **Buttons, menu items, check boxes, dialog titles and chips use the infinitive**: Salvar, Abrir, Permitir que apps de IA usem o RapidR Studio. Never the imperative on a control ("Salve").
- **Questions to the person**: "Quer salvar as alterações?" (not "Deseja…", which is stiff).
- **Who speaks.** Studio speaks about itself in the third person, with its article: "O Studio salvou uma cópia". RAI speaks in the first person: "Não posso executar o programa só com o código"; RAI takes the feminine article (*a RAI*, like *a IA*). The English has no "please", no "thanks", no exclamation marks and no emoji; Portuguese has none either (the only "!" is a greeting: "Olá, mundo!").
- **Friendly and short, like the English.** One idea per sentence; say what happens, not how it is built. Prefer a plain verb to a noun phrase ("Não foi possível salvar" over "Ocorreu um erro de gravação"). Avoid *deve/deverá*; say what the person can do. Gerunds are fine for progress ("Testando a chave…"), not for instructions.
- **No gender in the person.** *Você* avoids it: "Boas-vindas" (the noun, not "Bem-vindo/a"), "Quer continuar?" (not "Tem certeza?" with a gendered adjective). No "@", no "x", no "o/a" pairs.
- **Plain words** (`docs/design-guidelines.md`, "Words"): no jargon where a word will do; no other product's name; a third party is named only where the person must recognise it (an AI provider, GitHub).

### 1.2 Brazilian words: say this, not that

| Say | Not | Why |
|---|---|---|
| arquivo, pasta | ficheiro, diretório | *ficheiro* is Portugal's |
| computador | ordenador | |
| clicar, clique, clique duplo, clique com o botão direito | cliquear | |
| pressionar (a key) | premir, carregar | *premir* is Portugal's |
| mouse | rato | |
| tela | ecrã | *ecrã* is Portugal's |
| ícone | icone | with the accent |
| baixar | descarregar | *descarregar* is Portugal's |
| salvar | gravar | *gravar* is for sound and video |
| configurações | definições, ajustes | *definições* is Portugal's |
| link | ligação, vínculo | |
| senha (password), chave de API (the secret) | palavra-passe | never use *chave* for a password |
| e-mail | correio eletrónico | with the hyphen, as Brazilian Portuguese writes it |
| navegador | browser | |
| captura de tela | print, screenshot | |
| executar | correr | *correr* is Portugal's |
| aplicativo (a finished program) | aplicação | *aplicação* is Portugal's; *app* in "Apps de IA" and in running text where it is shorter |
| ele(s)/ela(s), você(s) | tu, vós | |
| excluir / remover / limpar | apagar for everything | see 1.3 |

### 1.3 Words with two jobs: one Portuguese word each

| English word | Portuguese | When |
|---|---|---|
| Delete / Remove / Clear | Excluir / Remover / Limpar | **Excluir** takes something away for good (a chat, a file). **Remover** takes it out of a list and keeps it (Remove an include folder). **Limpar** empties a list or a log (Clear Recent, Clear Activity). |
| Turn on / Enable | Ativar / Habilitar | **Ativar** (and *Desativar*) is a feature or switch (AI apps). **Habilitado / desabilitado** is a control that is or isn't greyed out. |
| Accept / OK | Aceitar / OK | The OK button stays **OK** (Brazilians say it); "Accept" a change is *Aceitar*. |
| Update / Refresh | Atualizar | The ↻ button and a software update both read *Atualizar*. |
| Window / Form | Janela / Formulário | **Janela**: any window, the Window menu, the program's running window. **Formulário**: an `RForm` as the designer and the code treat it. |
| Layout | Layout | Where panes sit (Reset Layout) and a property group. **Design** is the Design view and the designer. |
| Key | Chave / Tecla | **Chave de API** is the secret. **Tecla** is a keyboard key. |
| Back | Voltar | `< Back` in a wizard and returning to an older version are both *Voltar*. |
| Edit | Editar | The menu and the verb. |
| Format | Formatar | The designer's menu is *Formatar*; Format Document is *Formatar documento*. |
| Tag | Marca / Tag | In Simple, "Mark this version" makes a **marca**. In Pro it is a **tag** (the Git word). |
| Cursor / Pointer | Cursor / Ponteiro | **Cursor** is the text caret. **Ponteiro** is the mouse pointer. |
| Search / Find | Buscar / Localizar | The Search pane and its button: *Buscar*. Find in the editor (Ctrl+F): *Localizar*. |
| Case, Word (search toggles) | Maiúsculas/minúsculas, Palavra inteira | Regex stays **Regex**. |

### 1.4 Capitalisation

**Sentence case everywhere**: menus, commands, buttons, dialog titles, pane and tab names, column headers, chips, list items, headings. Capitalise the first word and proper nouns only ("Salvar como...", "Permitir nesta sessão", "Apps de IA", "Pontos de interrupção", "Opções do projeto"). This replaces the English Title Case rule of the design guide on purpose, as the Spanish does. Stay capitalised: RapidR, RapidR Studio, Studio, RAI, GitHub, Git, Anthropic, Claude, OpenAI, DeepSeek, and acronyms (IA, API, MCP, CSV, SQL). Lowercase: months, weekdays, languages (português, inglês), "internet", "web".

### 1.5 Punctuation

- **Ellipsis.** The English source writes three ASCII dots (`Save As...`) and Studio's code strips exactly those (`PlainTitle` in `ide/commands.inc`): in a menu or button string **copy the dots exactly as the English line has them** ("Salvar como..."). In prose use the single character "…".
- **Quotation marks**: typographic double “ ” for names and quoted words. Straight quotes stay inside code.
- **Menu paths** keep the English shape with the translated names: `Ferramentas > Configurações`.
- **Colon** ends a field label, no space before it: "Nome:". A label that is a sentence has no final period; a sentence does.
- **Dashes.** A single dash works like a colon and is spaced on both sides, as in the English.
- **Quoted UI names in prose** are written exactly as the translated UI shows them.
- No inverted marks, no special rules for `?` and `!` (unlike Spanish).

### 1.6 Numbers, dates, units

- Counts and amounts come in as placeholders (`{n}`). Portuguese counts **0 and 1 as singular** (CLDR, `Plural-Forms: nplurals=2; plural=(n > 1);`): "0 arquivo", "1 arquivo", "2 arquivos". Give each form a whole sentence so the verb agrees ("Foi salvo 1 arquivo" / "Foram salvos {n} arquivos").
- In prose: the decimal separator is the comma ("1,5 s") and thousands take a non-breaking space (32 000), none for four digits (4096); prefer rephrasing to a decimal ("um décimo de centavo"). Programs print the point; the text of a program is not translated.
- **Percent**: "45%", no space.
- **Money**: write **US$** or **dólares**. A bare **$** could be reais. "$ per million tokens" becomes "US$ por milhão de tokens".
- **Million and billion.** *Milhão* is 10⁶ and *bilhão* is 10⁹ in Brazil, as in English.
- **Dates** in prose: "6 de novembro de 2026". Numeric dates are ISO (2026-11-06) in logs; never 06/11/2026, which is two days in two countries. **Time** is 24-hour (14:30). Relative times: "há 2 min".
- **Units**: GB, MB, px, s, ms, min, with a space before the unit. `Ln 12, Col 5` becomes `Lin 12, Col 5`.

### 1.7 Mnemonics and shortcuts

- **`&` stays in the string; the translator chooses the letter.** Unique on its menu or surface; every button and field label has one except OK and Cancelar. Prefer the first letter of the main word if it is free, otherwise a clear consonant; avoid letters with descenders (g, j, p, q, y) and accented letters. `&&` shows a literal ampersand. The menu-clash test (`cargo test -p rapidr-i18n`) finds two items with one letter.
- **Shortcuts are physical keys and never move.** Ctrl+F finds, Ctrl+G goes to a line, Ctrl+H replaces, F5 runs. Key names stay as the English shows them (`Ctrl`, `Shift`, `Alt`, `Enter`, `Esc`, `Tab`, `Del`, F1–F12).
- **Notifications.** The bell, the history and the settings say *notificações*. The View menu's item says **A&visos**: every letter of *Notificações* is already some other View item's mnemonic in Portuguese (the menu-clash check refuses each of them), and an item with no mnemonic is refused too.
- The ten menu-bar mnemonics, all different: `&Arquivo`, `&Editar`, `E&xibir`, `&Projeto`, `Exe&cutar`, `&Depurar`, `For&matar`, `Fe&rramentas`, `&Janela`, `Aj&uda`.

### 1.8 Code inside sentences

- **Never translate or inflect code**: keywords (`PRINT`, `DIM`), component names (`RButton`, `RForm`), properties, events, methods, file names, commands, error codes. In prose they are in code font, with no plural s: "duas `SUB`", "as `FUNCTION`".
- **Put a Portuguese noun in front of the code word** so gender never shows: "o componente `RButton`", "a propriedade `Caption`", "o evento `OnClick`", "o método `Show`", "a instrução `PRINT`", "a função `LEFT$`", "o tipo `STRING`". `SUB` (sub-rotina) and `FUNCTION` (função) are feminine; `TYPE` (tipo) masculine; `CONST` (constante) feminine.

### 1.9 Placeholders and sentences

- `{name}`, `{n}`, `{file}` and the like are never translated or split; they may move within the sentence.
- **Whole sentences only.** Don't translate fragments.
- **No article before a placeholder** of unknown gender, and no *de/em + o* contraction in front of one (*do*, *no* can't be formed): write "Arquivo: {file}" or "Erro em {file}", not "O {thing}".
- **A place carries its own preposition** (the legal review's R6). Where a key is kept is a placeholder (`{store}`, `{place}`, `{source}`) filled with a phrase that already has its *em*: *no Acesso às Chaves do macOS*, *no Gerenciador de Credenciais do Windows*, *nesta aba do navegador*, *neste navegador*, *na memória*, *na memória deste programa*, *em uma variável de ambiente*. The sentence says "guardada {store}", never "guardada em {store}" (it would read "em o Acesso às Chaves", "em a memória"). `crates/rapidr-i18n/tests/catalogs.rs` keeps it so.
- **A placeholder may be said again** where the English says "it" and the Portuguese would be unclear ("{client} pode ver tudo o que o programa mostra", not "Ele pode ver…", which could be read as the button).

### 1.10 Length

Portuguese runs 15 to 25% longer than English. The controls do not grow, so **shorten the Portuguese**: keep the verb, drop the adjective ("Perguntar antes de alterar"). If a string still doesn't fit, say so in a translator comment (`#. too long`); don't ask for a wider control.

### 1.11 Texts that carry legal weight

The consent dialogs, the AI cost and privacy notes, the secret-check warnings and the "what is sent" texts were written point by point after a legal review. They are translated **by meaning, row by row, dropping nothing**, marked `# legal` in the .po files, and listed with their translations in [docs/legal/i18n-pt-legal-texts.md](../legal/i18n-pt-legal-texts.md) for the lawyer's review. Licences, notices and the legal files (`LICENSE`, `NOTICE`, `LEGAL.md`) are **not translated**: the English text is the one that applies.

The lawyer's review of 2026-11-06 ([review-2026-11-06-pt-br.md](../legal/review-2026-11-06-pt-br.md): OK with changes, applied) fixed these words for every legal text:

- **The age is said**: "para quem tem 18 anos ou mais", never "para adultos" (in Brazil *adultos* labels adult content). 13 (GitHub's rule) and 18 stay as the English has them.
- **Who may help a child**: *responsável* for the adult in charge; *professor* in the lines a child reads; *docente* only in School Lock's teacher-facing lines.
- **"Nothing waits for you"** says whose approval: "Nada espera pela sua aprovação", and the other "waits for you" lines say "esperando a sua aprovação".
- **Where something is kept is said where it is kept**: "guarda neste computador, e só nele, uma lista…" (the list is local, not the AI apps' actions).
- **A sandbox promises no isolation**: "roda com restrições (sandbox)", with the four blocked things named with *nem* each (DLLs, SENDMESSAGE, SHELL, RUN); never "ambiente isolado".
- **Money**: "centavo de dólar" and "US$ {cost}", never a bare *centavo* or *$*. **Privacy**: "conforme a política de privacidade dele/dela", the name Brazilians know.
- **Pt-BR stays "beta"** in Settings > Language until a native reader has read the 120 texts.

## 2. The terms

### 2.1 The menu bar

| English | Português (Brasil) | Mnemonic |
|---|---|---|
| File | Arquivo | `&Arquivo` |
| Edit | Editar | `&Editar` |
| View | Exibir | `E&xibir` |
| Project | Projeto | `&Projeto` |
| Run | Executar | `Exe&cutar` |
| Debug | Depurar | `&Depurar` |
| Format | Formatar | `For&matar` |
| Tools | Ferramentas | `Fe&rramentas` |
| Window | Janela | `&Janela` |
| Help | Ajuda | `Aj&uda` |

### 2.2 Common commands

| English | Português (Brasil) |
|---|---|
| New Project... | Novo projeto... |
| Open Project or File... | Abrir projeto ou arquivo... |
| Open Recent | Abrir recente |
| Save / Save As... / Save All | Salvar / Salvar como... / Salvar tudo |
| Close Document / Close Project | Fechar documento / Fechar projeto |
| Exit | Sair |
| Undo / Redo | Desfazer / Refazer |
| Cut / Copy / Paste | Recortar / Copiar / Colar |
| Find... / Replace... / Find in Files... / Go to Line... | Localizar... / Substituir... / Localizar nos arquivos... / Ir para a linha... |
| Settings... | Configurações... |
| Command Palette... | Paleta de comandos... |
| Keyboard Shortcuts | Atalhos de teclado |
| Toggle Comment | Alternar comentário |
| Quick Fix... | Correção rápida... |
| Indent Lines / Outdent Lines | Aumentar recuo / Diminuir recuo |
| Add File / Add Form / Add Module | Adicionar arquivo / Adicionar formulário / Adicionar módulo |
| Project Options... | Opções do projeto... |
| Theme: Light / Dark / High Contrast / Classic | Tema: Claro / Escuro / Alto contraste / Clássico |
| Reset Layout | Redefinir layout |

### 2.3 Buttons and small words

| English | Português (Brasil) | Note |
|---|---|---|
| OK / Cancel | OK / Cancelar | no mnemonic, as in English |
| Apply | Aplicar | also an AI action |
| `Next >` / `< Back` | `Avançar >` / `< Voltar` | the wizard |
| Browse... | Procurar... | not *Buscar* (that is Search) |
| Add / Remove | Adicionar / Remover | |
| Reset / Restore | Redefinir / Restaurar | |
| Turn On / Turn Off | Ativar / Desativar | the consent button: **Ativar** |
| Learn more | Saiba mais | a link |
| Not Now | Agora não | |
| Discard / Don't Save | Descartar / Não salvar | |
| Send / Stop | Enviar / Parar | |
| Test | Testar | "Test the Key" = *Testar a chave* |
| Preview | Visualizar | as a noun: *Visualização* |
| Default | Padrão | |
| Recent / Examples / Start | Recentes / Exemplos / Começar | the Welcome page |

### 2.4 Parts of the screen

| English | Português (Brasil) | Note |
|---|---|---|
| window | janela | |
| dialog | caixa de diálogo | *diálogo* alone in tight spots |
| pane, panel | painel | one word for both |
| tab | aba | |
| tool bar / status bar | barra de ferramentas / barra de status | |
| splitter | divisor | |
| tooltip, hint | dica | |
| icon | ícone | |
| check box / radio button | caixa de seleção / botão de opção | |
| combo box | lista suspensa | |
| text box, field | campo (de texto) | |
| label | rótulo | |
| select / choose | selecionar / escolher | *selecionar* for text and items |
| theme / look | tema / aparência | |
| Welcome page | Página de boas-vindas | the pane tab is *Boas-vindas* |

### 2.5 The language and programming

| English | Português (Brasil) | Note |
|---|---|---|
| app (a finished program) | aplicativo, app | |
| form | formulário | |
| component | componente | |
| control | controle | *componente* when building, *controle* for what the user touches |
| property | propriedade | |
| event | evento | |
| method | método | |
| event handler | manipulador de evento | |
| variable / constant | variável / constante | |
| array | array | element = *elemento*, position = *índice* |
| string | texto (in a screen), string (in code talk) | |
| integer / decimal number | inteiro / número decimal | |
| function / SUB | função / sub-rotina (`SUB`) | |
| parameter / argument | parâmetro / argumento | |
| module | módulo | |
| include file | arquivo incluído | `$INCLUDE` stays |
| library | biblioteca | |
| keyword | palavra-chave | |
| loop | laço | |
| compile / build | compilar | "Build Native App" = *Compilar app nativo* |
| run / debug | executar / depurar | |
| interpreter / interpreted | interpretador / interpretado | |
| runtime | ambiente de execução | the product **RapidR Runtime** keeps its name |
| bytecode | bytecode | |
| database / table / row / column / query | banco de dados / tabela / linha / coluna / consulta | |
| server / client / request / response | servidor / cliente / requisição / resposta | |

### 2.6 Studio's panes and areas

| English | Português (Brasil) |
|---|---|
| Project | Projeto |
| Toolbox | Caixa de ferramentas |
| Properties | Propriedades |
| Outline | Estrutura |
| Output | Saída |
| Problems | Problemas |
| Immediate | Imediato |
| Variables | Variáveis |
| Watch | Inspeção |
| Call Stack | Pilha de chamadas |
| Breakpoints | Pontos de interrupção |
| Search | Busca |
| Help | Ajuda |
| Assistant | Assistente |
| AI Apps | Apps de IA |
| Design / Code | Design / Código |
| Ln / Col | Lin / Col |

### 2.7 Running and debugging

| English | Português (Brasil) |
|---|---|
| Run / Run Without Debugging | Executar / Executar sem depurar |
| Stop / Restart | Parar / Reiniciar |
| Pause / Continue | Pausar / Continuar |
| Step Into / Step Over / Step Out | Entrar na chamada / Passar por cima / Sair da chamada |
| Run to Cursor | Executar até o cursor |
| Set Next Statement | Definir próxima instrução |
| Toggle Breakpoint | Alternar ponto de interrupção |
| running / paused / stopped | em execução / pausado / parado |
| run-time error / compile error | erro de execução / erro de compilação |
| error / warning / note | erro / aviso / nota |
| Run in Browser | Executar no navegador |

### 2.8 Versions (Simple and Pro)

| English | Simple | Pro |
|---|---|---|
| version | versão | commit |
| Save a version | Salvar uma versão | Commit |
| branch | ramificação (Pro: branch) | branch |
| merge | mesclar | merge |
| What changed? | O que mudou? | diff |

(The Pro words are the Git commands, which Brazilians say in English.)

### 2.9 RAI, School Lock and Versions

| English | Português (Brasil) | Note |
|---|---|---|
| Coach / Build (RAI's two modes) | Tutor / Construir | "modo Tutor"; *Construir* is the AI's Build mode, not *Compilar* |
| hint (a Coach's) | dica | a tooltip is a *dica* too |
| provider | provedor | |
| API key | chave de API | the secret; *senha* is a password |
| keychain | chaveiro | the system's store (macOS calls it *Acesso às Chaves*) |
| token (AI) | token | |
| context, effort, compact (a chat) | contexto, esforço, compactar | |
| AI apps | apps de IA | |
| School Lock | Bloqueio escolar | |
| teacher / parent | professor (a child reads it) or docente (School Lock, a teacher reads it) / responsável | gender-neutral *responsável*; *professor* as Brazilians use it, generic |
| request (to the AI) / request (of a client to a server) / a user's request | solicitação / requisição / pedido | |
| IT (a school's) | TI | |
| local history | histórico local | |
| Timeline | Linha do tempo | |
| experiment (Git's branch) | experimento | the Pro word is *branch* |
| checkpoint (AI) | checkpoint | |
| cloud service | serviço na nuvem | |

### 2.10 The RAI row (ai.rs `WORDS`)

`Studio is in Portuguese (Brazil). Say its words and RapidR's terms as Studio does, menu paths too (Ferramentas > Configurações)`: see `crates/rapidr-studio/src/ai.rs`, kept in sync with this glossary and under 900 characters.
