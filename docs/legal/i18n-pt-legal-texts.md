# Brazilian Portuguese (pt-BR): texts that carry legal weight, for the lawyer's review

Lane I18N-PT, Preview 6, 2026-11-06. Brazilian Portuguese (pt-BR) is a beta: an AI translation (Claude), reviewed by the lawyer agent (below) but not by a native speaker yet. Studio's Settings > Language marks it "beta".

What this is: every text that `ide/lang/es.po` and `crates/rapidr-value/lang/es.po` mark `# legal` (consent, permissions, cost and privacy notes, keys, age limits, School Lock, AI apps), 120 in all, in English and in Brazilian Portuguese, **as they are after the lawyer's changes**. They are translated by meaning, row by row, dropping nothing, as the glossary asks ([docs/i18n/pt-glossary.md](../i18n/pt-glossary.md), 1.11). In the `.po` files the same texts carry `# legal` above them (`grep -B1 "^# legal" ide/lang/pt-BR.po`).

What to check, text by text:

1. Every point of the English is still in the Portuguese (what is sent, to whom, what it costs, what stays on this computer, the age limit, "ask a parent or teacher").
2. Nothing was added that the English does not say (a promise, a guarantee).
3. Nothing reads as legal advice, and the wording is not stronger or weaker than the English.


## Review status

The lawyer agent reviewed these texts on 2026-11-06 ([docs/legal/review-2026-11-06-pt-br.md](review-2026-11-06-pt-br.md)): **OK with changes**. The six required groups (27 texts) and the seven recommended changes are applied in the strings below; the review's table says what each one was. The two source bugs it found are fixed for every language (V9: the key's place in RapidR's own text; S8: the APIKey row in Studio).

Kept as decided in the review:

- **"centavo de dólar"** for US cents (the providers bill in dollars; "centavo" alone reads as reais), and "US$" in the settings caption and the cost line.
- **"responsável"** for the adult in charge; **"professor"** in the six lines a child reads (S24-S27, S36, S61), **"docente"** in the School Lock lines a teacher reads.
- **"Bloqueio escolar"** for School Lock, **"Tutor"** for Coach.
- The ages (13: GitHub's own rule; 18) as the English has them: "para quem tem 18 anos ou mais" (not "adultos", which in Brazil labels adult content).
- "Licença MIT" in the copyright line (the licence text itself stays English).
- "Nada é enviado" statements, literal; they are product claims already checked in `six-traps-preview4.md` and `local-ai-review.md`.

Not for this translation, kept in mind (the review's Brazilian-law notes): no accounts, cloud service, chat, sharing, ads or telemetry without a new check against Brazil's law for children online (ECA Digital) and the LGPD; no limiting or disclaiming sentence in a dialog without a new review. Pt-BR stays marked "beta" in Settings > Language until a native reader has read these texts.

A test keeps the review's wording from coming back (`portuguese_keeps_the_legal_reviews_wording` in `crates/rapidr-i18n/tests/catalogs.rs`), and the pt-BR walk checks the key notes and the 18-or-older line on screen (`tests/studio_lang_pt.mjs`).

---

## Studio (`ide/lang/pt-BR.po`): 98 texts

**1.** `ide/ai.inc`

- EN: The last request, as sent (your key isn't in it)
- PT-BR: A última solicitação, como foi enviada (sua chave não está nela)

**2.** `ide/ai.inc`

- EN: Send shares your message, this chat, what the assistant reads of this project and, while your program runs, what its windows show (it asks you first) with {provider}: nothing else, nothing before.
- PT-BR: Enviar compartilha com {provider} sua mensagem, este chat, o que o assistente lê deste projeto e, enquanto seu programa é executado, o que as janelas dele mostram (antes, ele pergunta a você): nada além disso, nada antes.

**3.** `ide/ai.inc`

- EN: RAI asks to run your program or use it: Allow for this run, or Don't allow, in its chat.
- PT-BR: A RAI pede para executar o seu programa ou usá-lo: escolha Permitir nesta execução, ou Não permitir, no chat dela.

**4.** `ide/ai.inc`

- EN: Your key is never shown or logged. Every request and change is in the audit log on this computer: {location}
- PT-BR: Sua chave nunca é exibida nem registrada. Toda solicitação e alteração fica no registro de auditoria deste computador: {location}

**5.** `ide/ai.inc`

- EN: Type the API key in APIKey (kept in your keychain, never in the code): the provider's models are listed here then.
- PT-BR: Digite a chave de API em APIKey (guardada no seu chaveiro, nunca no código): os modelos do provedor aparecem aqui depois.

**6.** `ide/ai.inc`

- EN: The key is kept in {store} as {q}{name}{q}: the program names it, never the key.
- PT-BR: A chave fica guardada {store} como {q}{name}{q}: o programa usa o nome dela, nunca a chave.

**7.** `ide/ai.inc`

- EN: A key ({hint}) is kept in {store} as {q}{name}{q}: the code names it, never the key. Type another to replace it.
- PT-BR: Uma chave ({hint}) está guardada {store} como {q}{name}{q}: o código usa o nome dela, nunca a chave. Digite outra para substituí-la.

**8.** `ide/ai.inc`

- EN: A key ({hint}) is found in {source}: the code never holds it.
- PT-BR: Há uma chave ({hint}) {source}: o código nunca a contém.

**9.** `ide/aichats.inc`

- EN: Kept on this computer, in Studio's own folder — never in the project. Delete removes a chat for good.
- PT-BR: Guardados neste computador, na pasta do próprio Studio — nunca no projeto. Excluir remove um chat para sempre.

**10.** `ide/aisetup.inc`

- EN: It costs nothing, and Studio sends what you ask only to a local AI app on this computer — a free program you install that keeps AI models on your computer — so what you ask stays here, unless that app is set up to pass it on to a service elsewhere. Studio finds the models it already has and checks which ones fit this computer. Answers are slower and simpler than a cloud model's.
- PT-BR: Não custa nada, e o Studio envia o que você pergunta apenas a um app de IA local neste computador — um programa gratuito que você instala e que guarda modelos de IA no seu computador —, então o que você pergunta fica aqui, a menos que esse app esteja configurado para repassar a um serviço em outro lugar. O Studio encontra os modelos que você já tem e verifica quais cabem neste computador. As respostas são mais lentas e mais simples que as de um modelo na nuvem.

**11.** `ide/aisetup.inc`

- EN: Anthropic (Claude), OpenAI or DeepSeek: usually the best answers. You make a key on the provider's website and pay it for what you use: a question usually costs from a tenth of a cent (small, fast models) to a few cents (the largest). What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to that provider, under its privacy terms. Your key stays in this computer's keychain, never in a project.
- PT-BR: Anthropic (Claude), OpenAI ou DeepSeek: em geral, as melhores respostas. Você cria uma chave no site do provedor e paga a ele pelo que usar: uma pergunta costuma custar de um décimo de centavo de dólar (modelos pequenos e rápidos) a alguns centavos de dólar (os maiores). O que você envia — sua pergunta, o chat, as partes do seu projeto que a RAI lê e, enquanto seu programa é executado, o que as janelas dele mostram — vai para esse provedor, conforme a política de privacidade dele. Sua chave fica no chaveiro deste computador, nunca em um projeto.

**12.** `ide/aisetup.inc`

- EN: A local AI app's address on this computer, for example 127.0.0.1:8080. Studio only asks this computer.
- PT-BR: O endereço de um app de IA local neste computador, por exemplo 127.0.0.1:8080. O Studio só consulta este computador.

**13.** `ide/aisetup.inc`

- EN: Paste the key the provider gave you: it is kept in this computer's keychain, never in a project
- PT-BR: Cole a chave que o provedor deu a você: ela fica no chaveiro deste computador, nunca em um projeto

**14.** `ide/aisetup.inc`

- EN: What it costs, and what is sent
- PT-BR: Quanto custa e o que é enviado

**15.** `ide/aisetup.inc`

- EN: Runs elsewhere
- PT-BR: Executa em outro lugar

**16.** `ide/aisetup.inc`

- EN: Its local AI app runs this model on a service elsewhere: what you ask is sent there.
- PT-BR: O app de IA local dele executa este modelo em um serviço em outro lugar: o que você pergunta é enviado para lá.

**17.** `ide/aisetup.inc`

- EN: No AI model answered on this computer. A local AI app is a free program you install: it keeps AI models on your computer and answers RAI here, so what you ask stays on this computer, unless the app is set up to pass it on. Install one, add a model in it, start it, then go Back and Look Again — or type its address below if it uses another one.
- PT-BR: Nenhum modelo de IA respondeu neste computador. Um app de IA local é um programa gratuito que você instala: ele guarda modelos de IA no seu computador e responde à RAI aqui, então o que você pergunta fica neste computador, a menos que o app esteja configurado para repassar. Instale um, adicione um modelo nele, inicie-o, depois volte com Voltar e use Procurar de novo — ou digite o endereço dele abaixo, se usar outro.

**18.** `ide/aisetup.inc`

- EN: Licence, as its local AI app reports it (Studio doesn't check it; it can be wrong or out of date): {licence}.
- PT-BR: Licença, como o app de IA local a informa (o Studio não a verifica; pode estar errada ou desatualizada): {licence}.

**19.** `ide/aisetup.inc`

- EN: Its local AI app doesn't say its licence: check it where you got the model.
- PT-BR: O app de IA local dele não informa a licença: verifique-a onde você obteve o modelo.

**20.** `ide/aisetup.inc`

- EN: Building apps to sell? Check this model's licence: it may limit commercial use, or what you may do with what the model writes.
- PT-BR: Vai criar apps para vender? Verifique a licença deste modelo: ela pode limitar o uso comercial ou o que você pode fazer com o que o modelo escreve.

**21.** `ide/aisetup.inc`

- EN: Optional, about {n} minute: Studio loads each model that fits here, one at a time, and tries it — how fast it answers, whether it can use RAI's tools, and a tiny app — then lets it go (a model whose size isn't known is tried too). It uses this computer's memory and processor meanwhile, and may close a model another app has loaded. You can stop at any time. Studio only talks to this computer.
- EN (plural): Optional, about {n} minutes: Studio loads each model that fits here, one at a time, and tries it — how fast it answers, whether it can use RAI's tools, and a tiny app — then lets it go (a model whose size isn't known is tried too). It uses this computer's memory and processor meanwhile, and may close a model another app has loaded. You can stop at any time. Studio only talks to this computer.
- PT-BR [0]: Opcional, cerca de {n} minuto: o Studio carrega cada modelo que cabe aqui, um de cada vez, e o testa — a rapidez com que responde, se consegue usar as ferramentas da RAI e um app minúsculo — e depois o descarrega (um modelo de tamanho desconhecido também é testado). Enquanto isso, usa a memória e o processador deste computador, e pode fechar um modelo que outro app tenha carregado. Você pode parar a qualquer momento. O Studio só se comunica com este computador.
- PT-BR [1]: Opcional, cerca de {n} minutos: o Studio carrega cada modelo que cabe aqui, um de cada vez, e o testa — a rapidez com que responde, se consegue usar as ferramentas da RAI e um app minúsculo — e depois o descarrega (um modelo de tamanho desconhecido também é testado). Enquanto isso, usa a memória e o processador deste computador, e pode fechar um modelo que outro app tenha carregado. Você pode parar a qualquer momento. O Studio só se comunica com este computador.

**22.** `ide/aisetup.inc`

- EN: Studio only asks this computer: {why}
- PT-BR: O Studio só consulta este computador: {why}

**23.** `ide/aisetup.inc`

- EN: As its local AI app reports it (Studio doesn't check it). Building apps to sell? Check this model's licence: it may limit commercial use, or what you may do with what the model writes.
- PT-BR: Como o app de IA local a informa (o Studio não a verifica). Vai criar apps para vender? Verifique a licença deste modelo: ela pode limitar o uso comercial ou o que você pode fazer com o que o modelo escreve.

**24.** `ide/aisetup.inc`

- EN: Claude's small, fast models (Haiku) cost about a tenth of a cent a question; the largest (Opus) a few cents. Anthropic charges your account for what you use: set a monthly limit on its website. What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to Anthropic, under its privacy terms. Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key.
- PT-BR: Os modelos pequenos e rápidos do Claude (Haiku) custam cerca de um décimo de centavo de dólar por pergunta; o maior (Opus), alguns centavos de dólar. A Anthropic cobra da sua conta o que você usa: defina um limite mensal no site dela. O que você envia — sua pergunta, o chat, as partes do seu projeto que a RAI lê e, enquanto seu programa é executado, o que as janelas dele mostram — vai para a Anthropic, conforme a política de privacidade dela. Sua chave fica no chaveiro deste computador, nunca em um projeto. Os serviços na nuvem são para quem tem 18 anos ou mais: se você tem menos de 18 anos, peça a um responsável ou a um professor que crie a chave.

**25.** `ide/aisetup.inc`

- EN: OpenAI's small models (mini, nano) cost a fraction of a cent a question; the largest a few cents. OpenAI charges your account for what you use: set a monthly limit on its website. What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to OpenAI, under its privacy terms. Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key.
- PT-BR: Os modelos pequenos da OpenAI (mini, nano) custam uma fração de centavo de dólar por pergunta; o maior, alguns centavos de dólar. A OpenAI cobra da sua conta o que você usa: defina um limite mensal no site dela. O que você envia — sua pergunta, o chat, as partes do seu projeto que a RAI lê e, enquanto seu programa é executado, o que as janelas dele mostram — vai para a OpenAI, conforme a política de privacidade dela. Sua chave fica no chaveiro deste computador, nunca em um projeto. Os serviços na nuvem são para quem tem 18 anos ou mais: se você tem menos de 18 anos, peça a um responsável ou a um professor que crie a chave.

**26.** `ide/aisetup.inc`

- EN: DeepSeek's models cost a fraction of a cent a question. DeepSeek charges your account for what you use (you pay in advance on its website). What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to DeepSeek, in China, under its privacy terms. Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key.
- PT-BR: Os modelos da DeepSeek custam uma fração de centavo de dólar por pergunta. A DeepSeek cobra da sua conta o que você usa (você paga adiantado no site dela). O que você envia — sua pergunta, o chat, as partes do seu projeto que a RAI lê e, enquanto seu programa é executado, o que as janelas dele mostram — vai para a DeepSeek, na China, conforme a política de privacidade dela. Sua chave fica no chaveiro deste computador, nunca em um projeto. Os serviços na nuvem são para quem tem 18 anos ou mais: se você tem menos de 18 anos, peça a um responsável ou a um professor que crie a chave.

**27.** `ide/aisetup.inc`

- EN: You pay the provider for what you use. What you send — including, while your program runs, what its windows show — goes to it, under its privacy terms. Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key.
- PT-BR: Você paga ao provedor pelo que usar. O que você envia — inclusive, enquanto seu programa é executado, o que as janelas dele mostram — vai para ele, conforme a política de privacidade dele. Sua chave fica no chaveiro deste computador, nunca em um projeto. Os serviços na nuvem são para quem tem 18 anos ou mais: se você tem menos de 18 anos, peça a um responsável ou a um professor que crie a chave.

**28.** `ide/aisetup.inc`

- EN: Paste your API key first: the provider lists its models only to a key it knows.
- PT-BR: Cole antes a sua chave de API: o provedor só lista os modelos para uma chave que ele conhece.

**29.** `ide/aisetup.inc`

- EN: {model} runs on a service elsewhere, not on this computer: choose a model that runs here, or set that service up as a cloud service.
- PT-BR: {model} é executado em um serviço em outro lugar, não neste computador: escolha um modelo que rode aqui, ou configure esse serviço como um serviço na nuvem.

**30.** `ide/crash.inc` (retired by NOTIFY: see the note at 33)

- EN: Opens a new issue on GitHub in your browser, filled in with the report. Nothing is sent until you send it there yourself.
- PT-BR: Abre uma nova issue no GitHub no seu navegador, já preenchida com o relatório. Nada é enviado até você mesmo enviar lá.

**31.** `ide/crash.inc` (retired by NOTIFY: see the note at 33)

- EN: Closes this note. The report stays on this computer for 30 days.
- PT-BR: Fecha este aviso. O relatório fica neste computador por 30 dias.

**32.** `ide/crash.inc`

- EN: Its window hasn't answered for a few seconds. Wait gives it more time. Save Copies and Quit closes it: the copies of your unsaved work it keeps as you type (every 20 seconds) are offered when it starts again. A report was written on this computer.
- PT-BR: A janela dele não responde há alguns segundos. Aguardar dá mais tempo. Salvar cópias e sair o fecha: as cópias do seu trabalho não salvo, que ele guarda enquanto você digita (a cada 20 segundos), são oferecidas quando ele iniciar de novo. Um relatório foi escrito neste computador.

**33.** `ide/crash.inc` (replaced by NOTIFY)

NOTIFY (2026-11-06): the crash strip is now one line (RNotifyBar): its buttons have no descriptions of their own, and its one sentence replaces entries 30, 31 and 33 below; the promise "nothing is sent until you send it there yourself" is still in the consent window that Report on GitHub... opens (entries 34 to 38).

- EN: A report stays on this computer; nothing was sent.
- PT-BR: Um relatório fica neste computador; nada foi enviado.

**34.** `ide/crash.inc`

- EN: Your browser opens RapidR's page for a new issue on GitHub, filled in with this report. Nothing is sent until you send it there yourself.
- PT-BR: O seu navegador abre a página do RapidR para uma nova issue no GitHub, já preenchida com este relatório. Nada é enviado até você mesmo enviar lá.

**35.** `ide/crash.inc`

- EN: RapidR's version, your system, where it stopped, its last messages and the names of the last AI calls. Your home folder appears as ~; no file contents, keys or passwords.
- PT-BR: A versão do RapidR, seu sistema, onde ele parou, as últimas mensagens dele e os nomes das últimas chamadas de IA. Sua pasta pessoal aparece como ~; sem conteúdo de arquivos, chaves nem senhas.

**36.** `ide/crash.inc`

- EN: A GitHub account. GitHub's terms ask that you be 13 or older; a younger student can ask a teacher or a parent to report it.
- PT-BR: Uma conta do GitHub. Os termos do GitHub pedem que você tenha 13 anos ou mais; um estudante mais novo pode pedir a um professor ou a um responsável que relate.

**37.** `ide/crash.inc, ide/mcp.inc, ide/settings.inc`

- EN: Keep in mind
- PT-BR: Lembre-se

**38.** `ide/crash.inc`

- EN: Issues on GitHub are public: anyone can read them. Read the report first, and take out anything you don't want to share.
- PT-BR: As issues no GitHub são públicas: qualquer pessoa pode lê-las. Leia o relatório antes e retire o que você não quiser compartilhar.

**39.** `ide/mcp.inc`

- EN: Warning: AI apps may change and run anything in this project without asking
- PT-BR: Atenção: apps de IA podem alterar e executar qualquer coisa neste projeto sem perguntar

**40.** `ide/mcp.inc`

- EN: Off by default. On: an AI app on this computer that has the address and token below can work with your open project
- PT-BR: Desativado por padrão. Ativado: um app de IA neste computador que tenha o endereço e o token abaixo pode trabalhar com o seu projeto aberto

**41.** `ide/mcp.inc`

- EN: When on, an AI app on this computer that has the address and token below can work with your open project; you see everything it does in the AI Apps pane (Tools > AI Apps Activity). Paste both into the AI app's MCP settings.
- PT-BR: Quando ativado, um app de IA neste computador que tenha o endereço e o token abaixo pode trabalhar com o seu projeto aberto; você vê tudo o que ele faz no painel Apps de IA (Ferramentas > Atividade dos apps de IA). Cole os dois nas configurações de MCP do app de IA.

**42.** `ide/mcp.inc`

- EN: AI apps can look — read the project, check code, look things up, see the program's output and Studio — but not change, run, look at or use your program.
- PT-BR: Apps de IA podem olhar — ler o projeto, verificar código, consultar coisas, ver a saída do programa e o Studio —, mas não alterar, executar, observar nem usar o seu programa.

**43.** `ide/mcp.inc`

- EN: Changes to files are made at once (each one Undo step, listed in the AI Apps pane); running the program, looking at or using it, and asking RAI still wait for you.
- PT-BR: As alterações nos arquivos são feitas na hora (cada uma é um passo de Desfazer, listado no painel Apps de IA); executar o programa, observá-lo ou usá-lo, e perguntar à RAI continuam esperando a sua aprovação.

**44.** `ide/mcp.inc`

- EN: Nothing waits for you: AI apps change files, run the program, look at and use it, and ask RAI on their own — and a program can do anything you can on this computer. Everything is still listed, Stop ends it, and Studio's own tools never hand over keys or files outside the project.
- PT-BR: Nada espera pela sua aprovação: os apps de IA alteram arquivos, executam o programa, o observam e o usam, e perguntam à RAI por conta própria — e um programa pode fazer neste computador tudo o que você pode. Tudo continua listado, Parar encerra, e as ferramentas do próprio Studio nunca entregam chaves nem arquivos fora do projeto.

**45.** `ide/mcp.inc`

- EN: AI apps can look freely at the project; each change, run, look at your running program, or question to RAI waits for you in the AI Apps pane: Allow Once, Allow for This Session or Deny.
- PT-BR: Os apps de IA podem olhar o projeto livremente; cada alteração, execução, olhada no seu programa em execução ou pergunta à RAI espera a sua aprovação no painel Apps de IA: Permitir uma vez, Permitir nesta sessão ou Negar.

**46.** `ide/mcp.inc`

- EN: New project: Allow edits. Allow everything is not passed on to a new project: choose it again here if you want it.
- PT-BR: Novo projeto: Permitir edições. Permitir tudo não passa para um novo projeto: escolha de novo aqui se quiser.

**47.** `ide/mcp.inc`

- EN: Allow everything for AI apps in this project?
- PT-BR: Permitir tudo para os apps de IA neste projeto?

**48.** `ide/mcp.inc`

- EN: Nothing waits for you: AI apps change files, run the program, look at it, click and type in it and ask RAI (which may cost money) on their own.
- PT-BR: Nada espera pela sua aprovação: os apps de IA alteram arquivos, executam o programa, o observam, clicam e digitam nele e perguntam à RAI (o que pode custar dinheiro) por conta própria.

**49.** `ide/mcp.inc`

- EN: A program they run can do anything you can on this computer, and what they click in it is real. Choose this only while you watch an AI app you trust.
- PT-BR: Um programa que eles executam pode fazer neste computador tudo o que você pode, e o que clicarem nele é de verdade. Escolha isto apenas enquanto você observa um app de IA em que confia.

**50.** `ide/mcp.inc, ide/settings.inc`

- EN: You still see
- PT-BR: O que você continua vendo

**51.** `ide/mcp.inc`

- EN: Everything they do, listed in the AI Apps pane; Stop ends it.
- PT-BR: Tudo o que eles fazem, listado no painel Apps de IA; Parar encerra.

**52.** `ide/mcp.inc, ide/settings.inc`

- EN: Never
- PT-BR: Nunca

**53.** `ide/mcp.inc`

- EN: Studio's own tools never hand over your keys, or files outside the project.
- PT-BR: As ferramentas do próprio Studio nunca entregam suas chaves, nem arquivos fora do projeto.

**54.** `ide/mcp.inc`

- EN: AI apps change and run anything in this project without asking
- PT-BR: Apps de IA alteram e executam qualquer coisa neste projeto sem perguntar

**55.** `ide/mcp.inc`

- EN: It can see
- PT-BR: O que ele pode ver

**56.** `ide/mcp.inc`

- EN: It can
- PT-BR: O que ele pode fazer

**57.** `ide/mcp.inc`

- EN: It never sees
- PT-BR: O que ele nunca vê

**58.** `ide/mcp.inc`

- EN: The files of your open project — your code, forms and data files — and nothing else on your computer. It also gets pictures of Studio's window and of your running program's windows, and a written list of what they show, as you see them.
- PT-BR: Os arquivos do seu projeto aberto — seu código, formulários e arquivos de dados — e nada mais no seu computador. Ele também recebe imagens da janela do Studio e das janelas do seu programa em execução, e uma lista escrita do que elas mostram, como você as vê.

**59.** `ide/mcp.inc`

- EN: Check code, and read what your program prints and RAI's chat. It can also change forms and code, open other projects, run, debug, look at, click and type in your program, and ask RAI (which may cost money), each only as your permission mode allows; the default asks you first.
- PT-BR: Verificar o código e ler o que o seu programa imprime e o chat da RAI. Também pode alterar formulários e código, abrir outros projetos, executar, depurar, observar, clicar e digitar no seu programa e perguntar à RAI (o que pode custar dinheiro), cada coisa só como o seu modo de permissão permitir; o padrão pergunta antes.

**60.** `ide/mcp.inc`

- EN: Your AI provider keys or passwords: your keychain keeps them.
- PT-BR: As chaves ou senhas dos seus provedores de IA: o seu chaveiro as guarda.

**61.** `ide/mcp.inc`

- EN: RapidR sends none of this anywhere, but the AI app may pass what it reads to the company that makes it, under that company's terms. A program can show private data, yours or other people's, and clicking in it acts as you: a real Send, Pay or Delete button does it for real, so use test data. Many AI apps are for adults only: if you are under 18, set this up with a parent or teacher. Connect only AI apps you trust.
- PT-BR: O RapidR não envia nada disso a lugar nenhum, mas o app de IA pode repassar o que lê à empresa que o desenvolve, sob os termos dessa empresa. Um programa pode mostrar dados privados, seus ou de outras pessoas, e clicar nele age como você: um botão real de Enviar, Pagar ou Excluir faz isso de verdade, então use dados de teste. Muitos apps de IA são só para quem tem 18 anos ou mais: se você tem menos de 18 anos, configure isto com um responsável ou um professor. Conecte apenas apps de IA em que você confia.

**62.** `ide/mcp.inc`

- EN: Share only what you may share: schoolwork, an employer's code or someone else's licensed code may not be yours to share. Turn this off, or Reset Token, to disconnect every AI app. Studio keeps a list of what they did on this computer only; Clear Activity removes it.
- PT-BR: Compartilhe apenas o que você pode compartilhar: trabalhos escolares, o código de um empregador ou o código licenciado de outra pessoa podem não ser seus para compartilhar. Desative isto, ou use Redefinir token, para desconectar todos os apps de IA. O Studio guarda neste computador, e só nele, uma lista do que eles fizeram; Limpar atividade a remove.

**63.** `ide/mcp.inc`

- EN: {client} wants to look at or use your running program: {what}. Allow Once covers pictures, clicks and typing until this run ends. It may see whatever the program shows, including private data.
- PT-BR: {client} quer observar ou usar o seu programa em execução: {what}. Permitir uma vez vale para imagens, cliques e digitação até esta execução terminar. {client} pode ver tudo o que o programa mostra, inclusive dados privados.

**64.** `ide/mcp.inc`

- EN: RAI works on its own for you, but for an AI app's question it proposes changes for you to review and asks before it runs your program.
- PT-BR: A RAI trabalha sozinha para você, mas, para a pergunta de um app de IA, ela propõe alterações para você revisar e pergunta antes de executar o seu programa.

**65.** `ide/mcp.inc`

- EN: Let AI apps use RapidR Studio?
- PT-BR: Permitir que apps de IA usem o RapidR Studio?

**66.** `ide/mcp.inc`

- EN: With this on, an AI app on this computer that has your token can work with your open project through Studio, and you see everything it does in the AI Apps pane.
- PT-BR: Com isto ativado, um app de IA neste computador que tenha o seu token pode trabalhar com o seu projeto aberto por meio do Studio, e você vê tudo o que ele faz no painel Apps de IA.

**67.** `ide/mcp.inc`

- EN: AI apps that have the token can connect to RapidR Studio
- PT-BR: Os apps de IA que têm o token podem se conectar ao RapidR Studio

**68.** `ide/mcp.inc`

- EN: The list of what AI apps did is removed from this computer ({location}).
- PT-BR: A lista do que os apps de IA fizeram é removida deste computador ({location}).

**69.** `ide/schoollock.inc`

- EN: RAI stays on but only gives hints. It never writes the answer into the student's code. Uncheck AI in Studio to choose this.
- PT-BR: A RAI continua ativada, mas só dá dicas. Ela nunca escreve a resposta no código do estudante. Desmarque IA no Studio para escolher isto.

**70.** `ide/schoollock.inc`

- EN: RAI stays in Coach mode, and AI apps can't connect to RapidR Studio on this computer.
- PT-BR: A RAI continua no modo Tutor, e os apps de IA não podem se conectar ao RapidR Studio neste computador.

**71.** `ide/schoollock.inc`

- EN: RAI stays in Coach mode in RapidR Studio on this computer: it gives hints one step at a time and doesn't change the student's files.
- PT-BR: A RAI continua no modo Tutor no RapidR Studio neste computador: ela dá dicas um passo de cada vez e não altera os arquivos do estudante.

**72.** `ide/schoollock.inc`

- EN: Features of RapidR Studio are turned off for everyone who uses this account on this computer, until someone types the password. Everything else keeps working.
- PT-BR: Recursos do RapidR Studio ficam desativados para todos que usam esta conta neste computador, até alguém digitar a senha. Todo o resto continua funcionando.

**73.** `ide/schoollock.inc`

- EN: RapidR keeps only a one-way scrambled form of the password on this computer, never the password itself. Anyone who knows it can unlock until Studio closes, so keep it to yourself. Forgot it? The manual's page for teachers says what to do.
- PT-BR: O RapidR guarda neste computador apenas uma forma embaralhada da senha, sem volta, nunca a senha em si. Quem a souber pode desbloquear até o Studio fechar, então não a compartilhe. Esqueceu? A página do manual para docentes explica o que fazer.

**74.** `ide/settings.inc`

- EN: Off by default. On, Studio shows the AI Assistant panel (View > AI Assistant, Ctrl+I): ask about your project or for a new program. Nothing is sent until you press Send there. A new program it makes opens in Studio at once (Undo takes it back); changes to your files are shown for you to review, and nothing runs until you press Run.
- PT-BR: Desativado por padrão. Ativado, o Studio mostra o painel Assistente de IA (Exibir > Assistente de IA, Ctrl+I): pergunte sobre o seu projeto ou peça um novo programa. Nada é enviado até você pressionar Enviar ali. Um novo programa que ele cria abre no Studio na hora (Desfazer o desfaz); as alterações nos seus arquivos são mostradas para você revisar, e nada é executado até você pressionar Executar.

**75.** `ide/settings.inc`

- EN: Paste the key the provider gave you; it is kept in your system's keychain, never in a project
- PT-BR: Cole a chave que o provedor deu a você; ela fica no chaveiro do seu sistema, nunca em um projeto

**76.** `ide/settings.inc`

- EN: Kept in this browser's storage for the page: anyone using this browser profile could read it
- PT-BR: Guardada no armazenamento deste navegador para a página: qualquer pessoa que usar este perfil do navegador pode lê-la

**77.** `ide/settings.inc`

- EN: Off, the safer choice: a program RAI or an AI app runs can't call DLLs, SENDMESSAGE or SHELL and RUN; your own Run (F5) always can. On: the programs they run can do anything your own runs can on this computer.
- PT-BR: Desativado, a escolha mais segura: um programa que a RAI ou um app de IA executa não pode chamar DLLs, SENDMESSAGE, SHELL nem RUN; o seu próprio Executar (F5) sempre pode. Ativado: os programas que eles executam podem fazer neste computador tudo o que as suas próprias execuções podem.

**78.** `ide/settings.inc`

- EN: Send shares your message, the chat so far, RapidR's instructions and what the assistant reads of the open project — nothing outside it, nothing before you press Send. A log of every request and change stays on this computer.
- PT-BR: Enviar compartilha a sua mensagem, o chat até agora, as instruções do RapidR e o que o assistente lê do projeto aberto — nada fora dele, nada antes de você pressionar Enviar. Um registro de cada solicitação e alteração fica neste computador.

**79.** `ide/settings.inc`

- EN: the macOS Keychain
- PT-BR: no Acesso às Chaves do macOS

**80.** `ide/settings.inc`

- EN: Windows Credential Manager
- PT-BR: no Gerenciador de Credenciais do Windows

**81.** `ide/settings.inc`

- EN: the Secret Service (GNOME Keyring or KWallet)
- PT-BR: no Secret Service (GNOME Keyring ou KWallet)

**82.** `ide/settings.inc`

- EN: this browser tab, until it closes
- PT-BR: nesta aba do navegador, até ela fechar

**83.** `ide/settings.inc`

- EN: this browser, on this device
- PT-BR: neste navegador, neste dispositivo

**84.** `ide/settings.inc`

- EN: memory, until the program ends (this computer has no keychain RapidR can use)
- PT-BR: na memória, até o programa terminar (este computador não tem um chaveiro que o RapidR possa usar)

**85.** `ide/settings.inc`

- EN: memory, for this session only
- PT-BR: na memória, só nesta sessão

**86.** `ide/settings.inc`

- EN: this program's memory, until it ends
- PT-BR: na memória deste programa, até ele terminar

**87.** `ide/settings.inc`

- EN: an environment variable
- PT-BR: em uma variável de ambiente

**88.** `ide/settings.inc`

- EN: A local server needs no key (one is kept if you give it: some servers ask for one).
- PT-BR: Um servidor local não precisa de chave (uma é guardada se você informar: alguns servidores pedem uma).

**89.** `ide/settings.inc`

- EN: Your key {hint} is kept in {store}, never in a project.
- PT-BR: Sua chave {hint} fica guardada {store}, nunca em um projeto.

**90.** `ide/settings.inc`

- EN: Paste your key and press Save Key: it is kept in {store} (or on this device, if you tick the box), never in a project.
- PT-BR: Cole sua chave e pressione Salvar chave: ela fica guardada {store} (ou neste dispositivo, se você marcar a caixa), nunca em um projeto.

**91.** `ide/settings.inc`

- EN: Paste your key and press Save Key: it is kept in {store}, never in a project.
- PT-BR: Cole sua chave e pressione Salvar chave: ela fica guardada {store}, nunca em um projeto.

**92.** `ide/settings.inc`

- EN: Let the programs the AI runs use DLLs and SHELL?
- PT-BR: Permitir que os programas que a IA executa usem DLLs e SHELL?

**93.** `ide/settings.inc`

- EN: Off, a program that RAI or an AI app runs is sandboxed: a DLL call (DECLARE ... LIB), SENDMESSAGE and SHELL or RUN stop it with an error that says so. On, the programs they run have the same power as your own Run.
- PT-BR: Desativado, um programa que a RAI ou um app de IA executa roda com restrições (sandbox): uma chamada de DLL (DECLARE ... LIB), SENDMESSAGE e SHELL ou RUN o interrompem com um erro que diz isso. Ativado, os programas que eles executam têm o mesmo poder que o seu próprio Executar.

**94.** `ide/settings.inc`

- EN: Such a program can then start any other program and call the system's libraries: it can do anything you can on this computer. Choose this only for a project you trust, while you watch.
- PT-BR: Um programa assim pode então iniciar qualquer outro programa e chamar as bibliotecas do sistema: pode fazer neste computador tudo o que você pode. Escolha isto apenas para um projeto em que você confia, enquanto observa.

**95.** `ide/settings.inc`

- EN: Your own Run (F5) has this power whatever you choose here.
- PT-BR: O seu próprio Executar (F5) tem esse poder, seja qual for a sua escolha aqui.

**96.** `ide/shell.inc`

- EN: Copyright (c) 2025-2026 Ruta Internet SRL. MIT licence.
- PT-BR: Copyright (c) 2025-2026 Ruta Internet SRL. Licença MIT.

**97.** `ide/versions.inc`

- EN: This line looks like a key or a password. Versions are often shared, and a secret saved in one stays in the timeline: take it out first, or save anyway if it isn't a real one.
- EN (plural): These {n} lines look like keys or passwords. Versions are often shared, and a secret saved in one stays in the timeline: take them out first, or save anyway if they aren't real ones.
- PT-BR [0]: Esta linha parece uma chave ou uma senha. As versões costumam ser compartilhadas, e um segredo salvo em uma fica na linha do tempo: retire-o antes, ou salve mesmo assim se não for um segredo de verdade.
- PT-BR [1]: Estas {n} linhas parecem chaves ou senhas. As versões costumam ser compartilhadas, e um segredo salvo em uma fica na linha do tempo: retire-os antes, ou salve mesmo assim se não forem segredos de verdade.

**98.** `ide/versions.inc`

- EN: Not saved: lines look like keys or passwords. The dialog lists them.
- PT-BR: Não salvo: há linhas que parecem chaves ou senhas. A caixa de diálogo as lista.

## RapidR's own text (`crates/rapidr-value/lang/pt-BR.po`): 22 texts

**1.** `crates/rapidr-ai/src/rai.rs`

- EN: There is no {provider} API key yet: give one (APIKey), or say where it is kept (APIKeySource).
- PT-BR: Ainda não há chave de API de {provider}: informe uma (APIKey), ou diga onde ela fica guardada (APIKeySource).

**2.** `crates/rapidr-ai/src/rai.rs`

- EN: ({n} from the cache)
- PT-BR: ({n} do cache)

**3.** `crates/rapidr-ai/src/rai.rs`

- EN: {sent} tokens sent
- PT-BR: {sent} tokens enviados

**4.** `crates/rapidr-ai/src/rai.rs`

- EN: {n} received
- PT-BR: {n} recebidos

**5.** `crates/rapidr-ai/src/rai.rs`

- EN: about ${cost}
- PT-BR: cerca de US$ {cost}

**6.** `crates/rapidr-ai/src/rai.rs`

- EN: KeyOwner is RapidR Studio's own: a program's keys are always its own.
- PT-BR: KeyOwner é do próprio RapidR Studio: as chaves de um programa são sempre dele.

**7.** `crates/rapidr-ai/src/rai.rs`

- EN: APIKeySource names no keychain entry (keychain:NAME) to keep the key under.
- PT-BR: APIKeySource não nomeia nenhuma entrada do chaveiro (keychain:NAME) para guardar a chave.

**8.** `crates/rapidr-ai/src/rai.rs`

- EN: There is no key to keep: set APIKey first.
- PT-BR: Não há chave para guardar: defina antes APIKey.

**9.** `crates/rapidr-ai/src/rai.rs`

- EN: The key is kept in {place}.
- PT-BR: A chave fica guardada {place}.

**10.** `crates/rapidr-ai/src/rai.rs`

- EN: The key wasn't kept: {error}.
- PT-BR: A chave não foi guardada: {error}.

**11.** `crates/rapidr-ai/src/rai.rs`

- EN: The key is forgotten.
- PT-BR: A chave foi esquecida.

**12.** `crates/rapidr-ai/src/wire.rs`

- EN: Your {name} account has no credit left for this key{quoted}. Add credit on {name}'s website; RapidR can't do it for you.
- PT-BR: Sua conta de {name} não tem mais crédito para esta chave{quoted}. Adicione crédito no site de {name}; o RapidR não pode fazer isso por você.

**13.** `crates/rapidr-ai/src/wire.rs`

- EN: {name} sent the request on to another address ({status}): RapidR doesn't follow it with your key. If you trust that address, set the base URL to it.
- PT-BR: {name} redirecionou o pedido para outro endereço ({status}): o RapidR não o segue com a sua chave. Se você confia nesse endereço, defina a URL base para ele.

**14.** `crates/rapidr-studio/src/ai.rs`

- EN: RAI wants to run your program, then look at it, click and type in it. The program can do anything you can on this computer, and RAI sees whatever it shows, including private data.
- PT-BR: A RAI quer executar o seu programa e depois observá-lo, clicar e digitar nele. O programa pode fazer neste computador tudo o que você pode, e a RAI vê tudo o que ele mostra, inclusive dados privados.

**15.** `crates/rapidr-studio/src/ai.rs`

- EN: RAI wants to stop your running program.
- PT-BR: A RAI quer parar o seu programa em execução.

**16.** `crates/rapidr-studio/src/ai.rs`

- EN: RAI wants to click and type in your running program. It sees whatever the program shows, including private data.
- PT-BR: A RAI quer clicar e digitar no seu programa em execução. Ela vê tudo o que o programa mostra, inclusive dados privados.

**17.** `crates/rapidr-studio/src/ai.rs`

- EN: You allowed RAI to run and use your program for this run
- PT-BR: Você permitiu que a RAI execute e use o seu programa nesta execução

**18.** `crates/rapidr-studio/src/ai.rs`

- EN: You allowed RAI to use your running program for this run
- PT-BR: Você permitiu que a RAI use o seu programa, só durante esta execução

**19.** `crates/rapidr-studio/src/ai.rs`

- EN: You didn't allow RAI to run your program
- PT-BR: Você não permitiu que a RAI execute o seu programa

**20.** `crates/rapidr-studio/src/ai.rs`

- EN: You didn't allow RAI to stop your program
- PT-BR: Você não permitiu que a RAI pare o seu programa

**21.** `crates/rapidr-studio/src/ai.rs`

- EN: You didn't allow RAI to use your running program
- PT-BR: Você não permitiu que a RAI use o seu programa em execução

**22.** `crates/rapidr-studio/src/notes.rs`

- EN: {name}: you didn't allow it
- PT-BR: {name}: você não permitiu

## Added after the review

Written after the lawyer read the first 120: the three notes the Chinese review asked for (they are in English and every language now). Translated point by point and checked against the English, the reviewed Spanish and Chinese and this review's glossary (2026-11-06): "o que você pode compartilhar" and "podem não ser seus para enviar" as the reviewed S62; "do provedor" repeats the noun (glossary 1.9: "dele" could be read as the country's); "o texto em inglês é o que vale", not "prevalece" (there is no Portuguese licence for it to win over). A3 sits in the About text for a non-English UI and repeats glossary 1.11; it gets a Brazil (CDC) pass in the release's lawyer review.

**A1.** `ide/aisetup.inc`

- EN: Send only what you may share: other people's personal data, an employer's code or schoolwork may not be yours to send.
- PT-BR: Envie apenas o que você pode compartilhar: dados pessoais de outras pessoas, o código de um empregador ou trabalhos escolares podem não ser seus para enviar.

**A2.** `ide/aisetup.inc`

- EN: A provider may not offer its service where you live, and its servers may be in another country: check its terms.
- PT-BR: Um provedor pode não oferecer o serviço onde você mora, e os servidores dele podem estar em outro país: confira os termos do provedor.

**A3.** `ide/shell.inc`

- EN: Licences and notices are in English; the English text applies.
- PT-BR: As licenças e os avisos estão em inglês; o texto em inglês é o que vale.
