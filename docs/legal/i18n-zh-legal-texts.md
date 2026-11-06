# Simplified Chinese: the texts that carry legal weight

For the lead's lawyer check (lane I18N-ZH, 2026-11-06). Every text the catalogs mark `# legal` (consent, permission, privacy, cost, key, age, School Lock, crash-report and licence texts), with the English it translates. The Chinese is an AI translation, **beta**, made **by meaning, point by point, dropping nothing** (docs/i18n/zh-glossary.md, 2.5); a native reader has not checked it. The lawyer reviewed the first 131 of them on 2026-11-06 (docs/legal/review-2026-11-06-zh.md: OK after changes) and the changes are applied here; the texts added since (the three new notes, the key note, and the two promises the review asked to mark) were written in the review's own words or in the same style. The numbers differ from the review's where texts were added.

**136 texts**: 131 the lawyer reviewed (numbers 1 to 131, as in the review) and 5 added after it (A1 to A5, at the end): 106 in Studio, 25 in RapidR's own text, 0 in the compiler's messages. Find them again with `grep -B1 "^# legal" <catalog>`.

## Points to check

- **Parties and amounts stay the same**: Anthropic, OpenAI and DeepSeek are named as in the English; DeepSeek is "位于中国的 DeepSeek" (in China); the prices are "十分之一美分" (a tenth of a cent), "不到一美分" (a fraction of a cent) and "几美分" (a few cents).
- **Age**: "Cloud services are for adults: if you're under 18" is "云服务面向成年人：如果你未满 18 岁" (18 is also the age of majority on the mainland); the GitHub text keeps "13 岁" (GitHub's own terms).
- **Where a key is kept** names the stores as their systems do: macOS 钥匙串, Windows 凭据管理器, Secret Service; "never in a project" is "绝不会放进项目里" every time.
- **Permission words match everywhere**: Allow Once = 允许一次, Allow for This Session = 本次会话中允许, Deny / Don't allow = 不允许, Allow for this run = 本次运行允许, Allow everything = 全部允许.
- **The About box's copyright line** keeps "Copyright (c)" and the company's name as written, with the licence's name in Chinese ("MIT 许可证"). The licences themselves and `LEGAL.md` are not translated: the English is the one that applies.
- **Wording I am least sure of** is marked `#, review` in the catalog where it exists; none of the legal texts is marked, but the cloud services' cost and privacy texts (Anthropic, OpenAI, DeepSeek and the general one) and the AI apps' permission texts are the longest and deserve the most careful read.

## RapidR Studio (`ide/lang/zh-Hans.po`)

### 1. ide/ai.inc

English:

> The last request, as sent (your key isn't in it)

简体中文:

> 上一次发送的请求原文（其中不含你的密钥）

### 2. ide/ai.inc

English:

> Send shares your message, this chat, what the assistant reads of this project and, while your program runs, what its windows show (it asks you first) with {provider}: nothing else, nothing before.

简体中文:

> “发送”会把你的消息、这段对话、助手读取的本项目内容，以及程序运行期间其窗口显示的内容（会先征求你的同意）分享给 {provider}：除此之外不会分享任何内容，也不会提前分享。

### 3. ide/ai.inc

English:

> RAI asks to run your program or use it: Allow for this run, or Don't allow, in its chat.

简体中文:

> RAI 请求运行你的程序或使用它：请在它的对话中选择“本次运行允许”或“不允许”。

### 4. ide/ai.inc

English:

> Your key is never shown or logged. Every request and change is in the audit log on this computer: {location}

简体中文:

> 你的密钥绝不会被显示或记录。每一次请求和更改都记在这台电脑的审计日志中：{location}

### 5. ide/ai.inc

English:

> Type the API key in APIKey (kept in your keychain, never in the code): the provider's models are listed here then.

简体中文:

> 请在 APIKey 中输入 API 密钥（保存在你的钥匙串中，绝不会放进代码里）：然后这里会列出服务商的模型。

### 6. ide/ai.inc

English:

> The key is kept in {store} as {q}{name}{q}: the program names it, never the key.

简体中文:

> 密钥保存在{store}中，名称为 {q}{name}{q}：程序里写的是这个名称，绝不会是密钥本身。

### 7. ide/ai.inc

English:

> A key ({hint}) is kept in {store} as {q}{name}{q}: the code names it, never the key. Type another to replace it.

简体中文:

> 密钥（{hint}）保存在{store}中，名称为 {q}{name}{q}：代码里写的是这个名称，绝不会是密钥本身。输入另一个密钥可替换它。

### 8. ide/ai.inc

English:

> A key ({hint}) is found in {source}: the code never holds it.

简体中文:

> 在{source}中找到了一个密钥（{hint}）：代码里绝不会存放它。

### 9. ide/aichats.inc

English:

> Kept on this computer, in Studio's own folder — never in the project. Delete removes a chat for good.

简体中文:

> 保存在这台电脑上 Studio 自己的文件夹中，绝不会放进项目里。“删除”会永久删除对话。

### 10. ide/aisetup.inc

English:

> It costs nothing, and Studio sends what you ask only to a local AI app on this computer — a free program you install that keeps AI models on your computer — so what you ask stays here, unless that app is set up to pass it on to a service elsewhere. Studio finds the models it already has and checks which ones fit this computer. Answers are slower and simpler than a cloud model's.

简体中文:

> 完全免费，Studio 只会把你的提问发送给这台电脑上的本地 AI 应用——一个你自己安装的免费程序，AI 模型就保存在你的电脑里——所以你的提问会留在这里，除非该应用被设置为把它转交给其他地方的服务。Studio 会找出已有的模型，并检查哪些适合这台电脑。回答比云端模型更慢、更简单。

### 11. ide/aisetup.inc

English:

> Anthropic (Claude), OpenAI or DeepSeek: usually the best answers. You make a key on the provider's website and pay it for what you use: a question usually costs from a tenth of a cent (small, fast models) to a few cents (the largest). What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to that provider, under its privacy terms. Your key stays in this computer's keychain, never in a project.

简体中文:

> Anthropic (Claude)、OpenAI 或 DeepSeek：通常能给出最好的回答。你在服务商的网站上创建密钥，并按用量向其付费：一个问题的费用通常从十分之一美分（小而快的模型）到几美分（最大的模型）不等。你发送的内容——你的提问、对话、RAI 读取的项目部分，以及程序运行期间其窗口显示的内容——会发送给该服务商，并受其隐私条款约束。你的密钥保存在这台电脑的钥匙串中，绝不会放进项目里。

### 12. ide/aisetup.inc

English:

> A local AI app's address on this computer, for example 127.0.0.1:8080. Studio only asks this computer.

简体中文:

> 这台电脑上本地 AI 应用的地址，例如 127.0.0.1:8080。Studio 只会向这台电脑发出询问。

### 13. ide/aisetup.inc

English:

> Paste the key the provider gave you: it is kept in this computer's keychain, never in a project

简体中文:

> 粘贴服务商给你的密钥：它会保存在这台电脑的钥匙串中，绝不会放进项目里

### 14. ide/aisetup.inc

English:

> What it costs, and what is sent

简体中文:

> 费用，以及发送的内容

### 15. ide/aisetup.inc

English:

> Runs elsewhere

简体中文:

> 在别处运行

### 16. ide/aisetup.inc

English:

> Its local AI app runs this model on a service elsewhere: what you ask is sent there.

简体中文:

> 它的本地 AI 应用在别处的服务上运行这个模型：你的提问会被发送到那里。

### 17. ide/aisetup.inc

English:

> No AI model answered on this computer. A local AI app is a free program you install: it keeps AI models on your computer and answers RAI here, so what you ask stays on this computer, unless the app is set up to pass it on. Install one, add a model in it, start it, then go Back and Look Again — or type its address below if it uses another one.

简体中文:

> 没有 AI 模型在这台电脑上响应。本地 AI 应用是一个你自己安装的免费程序：它把 AI 模型保存在你的电脑里，并在本机回答 RAI，因此你的提问会留在这台电脑上，除非该应用被设置为把它转交出去。请安装一个，在其中添加模型并启动它，然后点击“上一步”再点击“重新查找”——如果它使用其他地址，也可以在下方输入它的地址。

### 18. ide/aisetup.inc

English:

> Licence, as its local AI app reports it (Studio doesn't check it; it can be wrong or out of date): {licence}.

简体中文:

> 许可证（以其本地 AI 应用报告的为准；Studio 不会核实，可能有误或已过期）：{licence}。

### 19. ide/aisetup.inc

English:

> Its local AI app doesn't say its licence: check it where you got the model.

简体中文:

> 它的本地 AI 应用没有说明许可证：请到你获取该模型的地方去核实。

### 20. ide/aisetup.inc

English:

> Building apps to sell? Check this model's licence: it may limit commercial use, or what you may do with what the model writes.

简体中文:

> 要制作用于销售的应用吗？请查看此模型的许可证：它可能限制商业用途，或限制你如何使用模型所写的内容。

### 21. ide/aisetup.inc

English:

> Optional, about {n} minute: Studio loads each model that fits here, one at a time, and tries it — how fast it answers, whether it can use RAI's tools, and a tiny app — then lets it go (a model whose size isn't known is tried too). It uses this computer's memory and processor meanwhile, and may close a model another app has loaded. You can stop at any time. Studio only talks to this computer.  //  Optional, about {n} minutes: Studio loads each model that fits here, one at a time, and tries it — how fast it answers, whether it can use RAI's tools, and a tiny app — then lets it go (a model whose size isn't known is tried too). It uses this computer's memory and processor meanwhile, and may close a model another app has loaded. You can stop at any time. Studio only talks to this computer.

简体中文:

> 可选，大约 {n} 分钟：Studio 会逐个加载适合这里的模型并试用——测试回答速度、能否使用 RAI 的工具，以及能否做出一个小应用——然后释放它（大小未知的模型也会试用）。期间会占用这台电脑的内存和处理器，并可能关闭其他应用已加载的模型。你可以随时停止。Studio 只会与这台电脑通信。

### 22. ide/aisetup.inc

English:

> Studio only asks this computer: {why}

简体中文:

> Studio 只会向这台电脑发出询问：{why}

### 23. ide/aisetup.inc

English:

> As its local AI app reports it (Studio doesn't check it). Building apps to sell? Check this model's licence: it may limit commercial use, or what you may do with what the model writes.

简体中文:

> 以其本地 AI 应用报告的为准（Studio 不会核实）。要制作用于销售的应用吗？请查看此模型的许可证：它可能限制商业用途，或限制你如何使用模型所写的内容。

### 24. ide/aisetup.inc

English:

> Claude's small, fast models (Haiku) cost about a tenth of a cent a question; the largest (Opus) a few cents. Anthropic charges your account for what you use: set a monthly limit on its website. What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to Anthropic, under its privacy terms. Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key.

简体中文:

> Claude 的小型快速模型（Haiku）每个问题约需十分之一美分；最大的模型（Opus）约需几美分。Anthropic 会按你的用量从你的账户扣费：请在其网站上设置每月上限。你发送的内容——你的提问、对话、RAI 读取的项目部分，以及程序运行期间其窗口显示的内容——会发送给 Anthropic，并受其隐私条款约束。你的密钥保存在这台电脑的钥匙串中，绝不会放进项目里。云服务面向成年人：如果你未满 18 岁，请让家长或老师来创建密钥。

### 25. ide/aisetup.inc

English:

> OpenAI's small models (mini, nano) cost a fraction of a cent a question; the largest a few cents. OpenAI charges your account for what you use: set a monthly limit on its website. What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to OpenAI, under its privacy terms. Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key.

简体中文:

> OpenAI 的小型模型（mini、nano）每个问题只需不到一美分；最大的模型约需几美分。OpenAI 会按你的用量从你的账户扣费：请在其网站上设置每月上限。你发送的内容——你的提问、对话、RAI 读取的项目部分，以及程序运行期间其窗口显示的内容——会发送给 OpenAI，并受其隐私条款约束。你的密钥保存在这台电脑的钥匙串中，绝不会放进项目里。云服务面向成年人：如果你未满 18 岁，请让家长或老师来创建密钥。

### 26. ide/aisetup.inc

English:

> DeepSeek's models cost a fraction of a cent a question. DeepSeek charges your account for what you use (you pay in advance on its website). What you send — your question, the chat, the parts of your project RAI reads and, while your program runs, what its windows show — goes to DeepSeek, in China, under its privacy terms. Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key.

简体中文:

> DeepSeek 的模型每个问题只需不到一美分。DeepSeek 会按你的用量从你的账户扣费（需要在其网站上预先充值）。你发送的内容——你的提问、对话、RAI 读取的项目部分，以及程序运行期间其窗口显示的内容——会发送给位于中国的 DeepSeek，并受其隐私条款约束。你的密钥保存在这台电脑的钥匙串中，绝不会放进项目里。云服务面向成年人：如果你未满 18 岁，请让家长或老师来创建密钥。

### 27. ide/aisetup.inc

English:

> You pay the provider for what you use. What you send — including, while your program runs, what its windows show — goes to it, under its privacy terms. Your key stays in this computer's keychain, never in a project. Cloud services are for adults: if you're under 18, ask a parent or teacher to make the key.

简体中文:

> 你需要按用量向服务商付费。你发送的内容——包括程序运行期间其窗口显示的内容——会发送给它，并受其隐私条款约束。你的密钥保存在这台电脑的钥匙串中，绝不会放进项目里。云服务面向成年人：如果你未满 18 岁，请让家长或老师来创建密钥。

### 28. ide/aisetup.inc

English:

> Paste your API key first: the provider lists its models only to a key it knows.

简体中文:

> 请先粘贴你的 API 密钥：服务商只会向它认识的密钥列出模型。

### 29. ide/aisetup.inc

English:

> {model} runs on a service elsewhere, not on this computer: choose a model that runs here, or set that service up as a cloud service.

简体中文:

> {model} 在别处的服务上运行，而不是在这台电脑上：请选择在本机运行的模型，或把该服务设置为云服务。

### 30. ide/crash.inc (retired by NOTIFY: see the note at 33)

English:

> Opens a new issue on GitHub in your browser, filled in with the report. Nothing is sent until you send it there yourself.

简体中文:

> 在浏览器中打开 GitHub 上的新议题页面，并填入报告内容。在你亲自到那里提交之前，不会发送任何内容。

### 31. ide/crash.inc (retired by NOTIFY: see the note at 33)

English:

> Closes this note. The report stays on this computer for 30 days.

简体中文:

> 关闭此提示。报告会在这台电脑上保留 30 天。

### 32. ide/crash.inc

English:

> Its window hasn't answered for a few seconds. Wait gives it more time. Save Copies and Quit closes it: the copies of your unsaved work it keeps as you type (every 20 seconds) are offered when it starts again. A report was written on this computer.

简体中文:

> 它的窗口已有几秒钟没有回应。“等待”会再给它一些时间。“保存副本并退出”会关闭它：你输入时它每 20 秒保留的未保存工作副本，会在它再次启动时提供给你。这台电脑上已写入一份报告。

### 33. ide/crash.inc (replaced by NOTIFY)

NOTIFY (2026-11-06): the crash strip is now one line (RNotifyBar): its buttons have no descriptions of their own, and its one sentence replaces entries 30, 31 and 33 below; the promise "nothing is sent until you send it there yourself" is still in the consent window that Report on GitHub... opens (entries 34 to 38).

English:

> A report stays on this computer; nothing was sent.

简体中文:

> 报告保存在这台电脑上，没有发送任何内容。

### 34. ide/crash.inc

English:

> Your browser opens RapidR's page for a new issue on GitHub, filled in with this report. Nothing is sent until you send it there yourself.

简体中文:

> 你的浏览器会打开 RapidR 在 GitHub 上的新议题页面，并填入这份报告。在你亲自到那里提交之前，不会发送任何内容。

### 35. ide/crash.inc

English:

> RapidR's version, your system, where it stopped, its last messages and the names of the last AI calls. Your home folder appears as ~; no file contents, keys or passwords.

简体中文:

> RapidR 的版本、你的系统、停止的位置、最后几条消息，以及最后几次 AI 调用的名称。你的主文件夹显示为 ~；不含文件内容、密钥或密码。

### 36. ide/crash.inc

English:

> A GitHub account. GitHub's terms ask that you be 13 or older; a younger student can ask a teacher or a parent to report it.

简体中文:

> 一个 GitHub 账号。GitHub 的条款要求年满 13 岁；更小的学生可以请老师或家长代为报告。

### 37. ide/crash.inc ide/mcp.inc ide/settings.inc

English:

> Keep in mind

简体中文:

> 请注意

### 38. ide/crash.inc

English:

> Issues on GitHub are public: anyone can read them. Read the report first, and take out anything you don't want to share.

简体中文:

> GitHub 上的议题是公开的：任何人都可以阅读。请先阅读报告，并删去你不想分享的内容。

### 39. ide/mcp.inc

English:

> Warning: AI apps may change and run anything in this project without asking

简体中文:

> 警告：AI 应用可能不经询问就更改并运行本项目中的任何内容

### 40. ide/mcp.inc

English:

> Off by default. On: an AI app on this computer that has the address and token below can work with your open project

简体中文:

> 默认关闭。开启后：这台电脑上拿到下方地址和令牌的 AI 应用可以操作你打开的项目

### 41. ide/mcp.inc

English:

> When on, an AI app on this computer that has the address and token below can work with your open project; you see everything it does in the AI Apps pane (Tools > AI Apps Activity). Paste both into the AI app's MCP settings.

简体中文:

> 开启后，这台电脑上拿到下方地址和令牌的 AI 应用可以操作你打开的项目；它所做的一切都会显示在“AI 应用”面板中（工具 > AI 应用活动）。请把两者都粘贴到 AI 应用的 MCP 设置中。

### 42. ide/mcp.inc

English:

> AI apps can look — read the project, check code, look things up, see the program's output and Studio — but not change, run, look at or use your program.

简体中文:

> AI 应用可以查看——读取项目、检查代码、查阅资料、查看程序的输出和 Studio——但不能更改、运行、查看或使用你的程序。

### 43. ide/mcp.inc

English:

> Changes to files are made at once (each one Undo step, listed in the AI Apps pane); running the program, looking at or using it, and asking RAI still wait for you.

简体中文:

> 对文件的更改会立即生效（每次更改是一步“撤销”，并列在“AI 应用”面板中）；运行程序、查看或使用程序，以及向 RAI 提问，仍然需要等你同意。

### 44. ide/mcp.inc

English:

> Nothing waits for you: AI apps change files, run the program, look at and use it, and ask RAI on their own — and a program can do anything you can on this computer. Everything is still listed, Stop ends it, and Studio's own tools never hand over keys or files outside the project.

简体中文:

> 任何操作都不会等你确认：AI 应用会自行更改文件、运行程序、查看并使用程序，以及向 RAI 提问——而程序可以做你在这台电脑上能做的任何事。所有操作仍会列出，“停止”可以结束它，Studio 自带的工具也绝不会交出密钥或项目之外的文件。

### 45. ide/mcp.inc

English:

> AI apps can look freely at the project; each change, run, look at your running program, or question to RAI waits for you in the AI Apps pane: Allow Once, Allow for This Session or Deny.

简体中文:

> AI 应用可以随意查看项目；每一次更改、运行、查看你正在运行的程序，或向 RAI 提问，都会在“AI 应用”面板中等你决定：允许一次、本次会话中允许，或不允许。

### 46. ide/mcp.inc

English:

> New project: Allow edits. Allow everything is not passed on to a new project: choose it again here if you want it.

简体中文:

> 新项目：允许编辑。“全部允许”不会带到新项目：如果需要，请在这里重新选择。

### 47. ide/mcp.inc

English:

> Allow everything for AI apps in this project?

简体中文:

> 对本项目中的 AI 应用全部允许？

### 48. ide/mcp.inc

English:

> Nothing waits for you: AI apps change files, run the program, look at it, click and type in it and ask RAI (which may cost money) on their own.

简体中文:

> 任何操作都不会等你确认：AI 应用会自行更改文件、运行程序、查看程序、在程序中点击和输入，以及向 RAI 提问（这可能产生费用）。

### 49. ide/mcp.inc

English:

> A program they run can do anything you can on this computer, and what they click in it is real. Choose this only while you watch an AI app you trust.

简体中文:

> 它们运行的程序可以做你在这台电脑上能做的任何事，它们在程序中点击的内容也是真实有效的。只有在你亲自看着一个你信任的 AI 应用时，才选择这一项。

### 50. ide/mcp.inc ide/settings.inc

English:

> You still see

简体中文:

> 你仍然能看到

### 51. ide/mcp.inc

English:

> Everything they do, listed in the AI Apps pane; Stop ends it.

简体中文:

> 它们所做的一切，都列在“AI 应用”面板中；“停止”可以结束它。

### 52. ide/mcp.inc ide/settings.inc

English:

> Never

简体中文:

> 绝不会

### 53. ide/mcp.inc

English:

> Studio's own tools never hand over your keys, or files outside the project.

简体中文:

> Studio 自带的工具绝不会交出你的密钥，或项目之外的文件。

### 54. ide/mcp.inc

English:

> AI apps change and run anything in this project without asking

简体中文:

> AI 应用可不经询问就更改并运行本项目中的任何内容

### 55. ide/mcp.inc

English:

> It can see

简体中文:

> 它能看到

### 56. ide/mcp.inc

English:

> It can

简体中文:

> 它能做

### 57. ide/mcp.inc

English:

> It never sees

简体中文:

> 它绝不会看到

### 58. ide/mcp.inc

English:

> The files of your open project — your code, forms and data files — and nothing else on your computer. It also gets pictures of Studio's window and of your running program's windows, and a written list of what they show, as you see them.

简体中文:

> 你打开的项目的文件——你的代码、窗体和数据文件——除此之外，你电脑上的其他内容都看不到。它还会收到 Studio 窗口和你正在运行的程序窗口的图片，以及这些窗口所显示内容的文字清单，与你看到的一样。

### 59. ide/mcp.inc

English:

> Check code, and read what your program prints and RAI's chat. It can also change forms and code, open other projects, run, debug, look at, click and type in your program, and ask RAI (which may cost money), each only as your permission mode allows; the default asks you first.

简体中文:

> 检查代码，并读取你的程序输出的内容和 RAI 的对话。它还可以更改窗体和代码、打开其他项目、运行、调试、查看你的程序、在其中点击和输入，以及向 RAI 提问（这可能产生费用），每一项都只在你的权限模式允许时才能进行；默认会先询问你。

### 60. ide/mcp.inc

English:

> Your AI provider keys or passwords: your keychain keeps them.

简体中文:

> 你的 AI 服务商密钥或密码：它们由你的钥匙串保管。

### 61. ide/mcp.inc

English:

> RapidR sends none of this anywhere, but the AI app may pass what it reads to the company that makes it, under that company's terms. A program can show private data, yours or other people's, and clicking in it acts as you: a real Send, Pay or Delete button does it for real, so use test data. Many AI apps are for adults only: if you are under 18, set this up with a parent or teacher. Connect only AI apps you trust.

简体中文:

> RapidR 不会把这些内容发送到任何地方，但 AI 应用可能会把它读取到的内容转交给开发它的公司，并受该公司条款约束。程序可能显示私人数据（你的或他人的），而在其中点击就相当于以你的身份操作：真实的“发送”“支付”或“删除”按钮会真的执行，所以请使用测试数据。许多 AI 应用仅限成年人使用：如果你未满 18 岁，请在家长或老师的陪同下进行设置。只连接你信任的 AI 应用。

### 62. ide/mcp.inc

English:

> Share only what you may share: schoolwork, an employer's code or someone else's licensed code may not be yours to share. Turn this off, or Reset Token, to disconnect every AI app. Studio keeps a list of what they did on this computer only; Clear Activity removes it.

简体中文:

> 只分享你有权分享的内容：学校作业、雇主的代码，或受他人许可证约束的代码，你不一定有权分享。关闭此项或重置令牌，可断开所有 AI 应用。Studio 只在这台电脑上保存它们所做操作的列表；“清除活动记录”会删除它。

### 63. ide/mcp.inc

English:

> {client} wants to look at or use your running program: {what}. Allow Once covers pictures, clicks and typing until this run ends. It may see whatever the program shows, including private data.

简体中文:

> {client} 想查看或使用你正在运行的程序：{what}。“允许一次”涵盖截图、点击和输入，直到本次运行结束。{client} 可能看到程序显示的一切，包括私人数据。

### 64. ide/mcp.inc

English:

> RAI works on its own for you, but for an AI app's question it proposes changes for you to review and asks before it runs your program.

简体中文:

> RAI 会自己为你工作，但对于 AI 应用提出的问题，它会提出更改供你审阅，并且在运行你的程序之前先询问你。

### 65. ide/mcp.inc

English:

> Let AI apps use RapidR Studio?

简体中文:

> 允许 AI 应用使用 RapidR Studio？

### 66. ide/mcp.inc

English:

> With this on, an AI app on this computer that has your token can work with your open project through Studio, and you see everything it does in the AI Apps pane.

简体中文:

> 开启后，这台电脑上拿到你的令牌的 AI 应用可以通过 Studio 操作你打开的项目，它所做的一切都会显示在“AI 应用”面板中。

### 67. ide/mcp.inc

English:

> AI apps that have the token can connect to RapidR Studio

简体中文:

> 拿到令牌的 AI 应用可以连接到 RapidR Studio

### 68. ide/mcp.inc

English:

> The list of what AI apps did is removed from this computer ({location}).

简体中文:

> AI 应用做过的事的列表将从这台电脑上删除（{location}）。

### 69. ide/schoollock.inc

English:

> RAI stays on but only gives hints. It never writes the answer into the student's code. Uncheck AI in Studio to choose this.

简体中文:

> RAI 保持开启，但只给出提示。它绝不会把答案写进学生的代码。取消勾选“Studio 中的 AI”即可选择此项。

### 70. ide/schoollock.inc

English:

> RAI stays in Coach mode, and AI apps can't connect to RapidR Studio on this computer.

简体中文:

> RAI 保持在辅导模式，这台电脑上的 AI 应用无法连接到 RapidR Studio。

### 71. ide/schoollock.inc

English:

> RAI stays in Coach mode in RapidR Studio on this computer: it gives hints one step at a time and doesn't change the student's files.

简体中文:

> 在这台电脑上的 RapidR Studio 中，RAI 保持在辅导模式：它一步一步地给出提示，不会更改学生的文件。

### 72. ide/schoollock.inc

English:

> Features of RapidR Studio are turned off for everyone who uses this account on this computer, until someone types the password. Everything else keeps working.

简体中文:

> 在这台电脑上，使用此账户的所有人都无法使用 RapidR Studio 的某些功能，直到有人输入密码。其他一切照常运行。

### 73. ide/schoollock.inc

English:

> RapidR keeps only a one-way scrambled form of the password on this computer, never the password itself. Anyone who knows it can unlock until Studio closes, so keep it to yourself. Forgot it? The manual's page for teachers says what to do.

简体中文:

> RapidR 在这台电脑上只保存密码的一种无法还原的打乱形式（单向哈希），绝不保存密码本身。任何知道密码的人都可以解锁，直到 Studio 关闭，所以请保管好密码，不要告诉别人。忘记了？手册里面向老师的页面会告诉你怎么办。

### 74. ide/settings.inc

English:

> Off by default. On, Studio shows the AI Assistant panel (View > AI Assistant, Ctrl+I): ask about your project or for a new program. Nothing is sent until you press Send there. A new program it makes opens in Studio at once (Undo takes it back); changes to your files are shown for you to review, and nothing runs until you press Run.

简体中文:

> 默认关闭。开启后，Studio 会显示“AI 助手”面板（视图 > AI 助手，Ctrl+I）：你可以询问关于项目的问题，或请它写一个新程序。在你点击那里的“发送”之前，不会发送任何内容。它制作的新程序会立即在 Studio 中打开（“撤销”可收回）；对你的文件的更改会先展示给你审阅，在你点击“运行”之前，不会运行任何内容。

### 75. ide/settings.inc

English:

> Paste the key the provider gave you; it is kept in your system's keychain, never in a project

简体中文:

> 粘贴服务商给你的密钥；它会保存在你系统的钥匙串中，绝不会放进项目里

### 76. ide/settings.inc

English:

> Kept in this browser's storage for the page: anyone using this browser profile could read it

简体中文:

> 保存在此浏览器为该页面提供的存储中：使用这个浏览器用户配置文件的任何人都能读取它

### 77. ide/settings.inc

English:

> Off, the safer choice: a program RAI or an AI app runs can't call DLLs, SENDMESSAGE or SHELL and RUN; your own Run (F5) always can. On: the programs they run can do anything your own runs can on this computer.

简体中文:

> 关闭（更安全的选择）：RAI 或 AI 应用运行的程序不能调用 DLL、SENDMESSAGE，也不能使用 SHELL 和 RUN；你自己的“运行”(F5) 则始终可以。开启：它们运行的程序可以做你自己运行的程序在这台电脑上能做的任何事。

### 78. ide/settings.inc

English:

> Send shares your message, the chat so far, RapidR's instructions and what the assistant reads of the open project — nothing outside it, nothing before you press Send. A log of every request and change stays on this computer.

简体中文:

> “发送”会分享你的消息、到目前为止的对话、RapidR 的指令，以及助手读取的已打开项目的内容——不含项目之外的任何内容，在你点击“发送”之前也不会分享任何内容。每次请求和更改的日志都保存在这台电脑上。

### 79. ide/settings.inc

English:

> the macOS Keychain

简体中文:

> macOS 钥匙串

### 80. ide/settings.inc

English:

> Windows Credential Manager

简体中文:

> Windows 凭据管理器

### 81. ide/settings.inc

English:

> the Secret Service (GNOME Keyring or KWallet)

简体中文:

> Secret Service（GNOME Keyring 或 KWallet）

### 82. ide/settings.inc

English:

> this browser tab, until it closes

简体中文:

> 此浏览器标签页（直到关闭）

### 83. ide/settings.inc

English:

> this browser, on this device

简体中文:

> 此浏览器（在这台设备上）

### 84. ide/settings.inc

English:

> memory, until the program ends (this computer has no keychain RapidR can use)

简体中文:

> 内存（直到程序结束；这台电脑上没有 RapidR 可用的钥匙串）

### 85. ide/settings.inc

English:

> memory, for this session only

简体中文:

> 内存（仅限本次会话）

### 86. ide/settings.inc

English:

> this program's memory, until it ends

简体中文:

> 此程序的内存（直到程序结束）

### 87. ide/settings.inc

English:

> an environment variable

简体中文:

> 一个环境变量

### 88. ide/settings.inc

English:

> A local server needs no key (one is kept if you give it: some servers ask for one).

简体中文:

> 本地服务器不需要密钥（如果你提供了密钥，会保存下来：有些服务器会要求密钥）。

### 89. ide/settings.inc

English:

> Your key {hint} is kept in {store}, never in a project.

简体中文:

> 你的密钥 {hint} 保存在{store}中，绝不会放进项目里。

### 90. ide/settings.inc

English:

> Paste your key and press Save Key: it is kept in {store} (or on this device, if you tick the box), never in a project.

简体中文:

> 粘贴你的密钥并点击“保存密钥”：它会保存在{store}中（如果你勾选了该选项，则保存在这台设备上），绝不会放进项目里。

### 91. ide/settings.inc

English:

> Paste your key and press Save Key: it is kept in {store}, never in a project.

简体中文:

> 粘贴你的密钥并点击“保存密钥”：它会保存在{store}中，绝不会放进项目里。

### 92. ide/settings.inc

English:

> Let RAI work on its own?

简体中文:

> 让 RAI 自行工作？

### 93. ide/settings.inc

English:

> Nothing waits for you: RAI changes the project's files, runs the program, looks at it, clicks and types in it to test it on its own, then reports what it did.

简体中文:

> 任何操作都不会等你确认：RAI 会自行更改项目的文件、运行程序、查看程序、在其中点击和输入来测试它，然后报告它做了什么。

### 94. ide/settings.inc

English:

> A program RAI runs can do anything you can on this computer, what it clicks in it is real (Send, Pay, Delete), it sees whatever the program shows, and its changes are applied without your review. It may cost money with a paid model. Choose this only while you watch, on a project you have Versions of, with test data.

简体中文:

> RAI 运行的程序可以做你在这台电脑上能做的任何事，它在程序中点击的内容也是真实有效的（发送、支付、删除），它能看到程序显示的一切，它所做的更改也不会经过你的审阅就直接应用。使用付费模型时还可能产生费用。只有在你亲自看着、项目已有版本记录、并且使用测试数据时，才选择这一项。

### 95. ide/settings.inc

English:

> Every step in RAI's chat; Undo and Versions take any change back; Stop ends a run and RAI's answer.

简体中文:

> 每一步都显示在 RAI 的对话中；“撤销”和“版本”可以收回任何更改；“停止”可结束运行和 RAI 的回答。

### 96. ide/settings.inc

English:

> RAI never sees your keys, or files outside the project.

简体中文:

> RAI 绝不会看到你的密钥，也不会看到项目之外的文件。

### 97. ide/settings.inc

English:

> RAI changes and runs the project without asking

简体中文:

> RAI 可不经询问就更改并运行项目

### 98. ide/settings.inc

English:

> Let the programs the AI runs use DLLs and SHELL?

简体中文:

> 允许 AI 运行的程序使用 DLL 和 SHELL？

### 99. ide/settings.inc

English:

> Off, a program that RAI or an AI app runs is sandboxed: a DLL call (DECLARE ... LIB), SENDMESSAGE and SHELL or RUN stop it with an error that says so. On, the programs they run have the same power as your own Run.

简体中文:

> 关闭时，RAI 或 AI 应用运行的程序处于沙盒中：DLL 调用 (DECLARE ... LIB)、SENDMESSAGE，以及 SHELL 或 RUN 都会使它出错停止，并给出说明。开启时，它们运行的程序拥有与你自己的“运行”相同的权限。

### 100. ide/settings.inc

English:

> Such a program can then start any other program and call the system's libraries: it can do anything you can on this computer. Choose this only for a project you trust, while you watch.

简体中文:

> 这样的程序就可以启动任何其他程序、调用系统的库：它可以做你在这台电脑上能做的任何事。只有对你信任的项目、并且在你亲自看着的时候，才选择这一项。

### 101. ide/settings.inc

English:

> Your own Run (F5) has this power whatever you choose here.

简体中文:

> 无论你在这里怎么选择，你自己的“运行”(F5) 都拥有这种权限。

### 102. ide/settings.inc

English:

> The programs RAI and AI apps run may call DLLs and start other programs

简体中文:

> RAI 和 AI 应用运行的程序可以调用 DLL 并启动其他程序

### 103. ide/settings.inc

English:

> The programs RAI and AI apps run stay sandboxed

简体中文:

> RAI 和 AI 应用运行的程序保持在沙盒中

### 104. ide/shell.inc

English:

> Copyright (c) 2025-2026 Ruta Internet SRL. MIT licence.

简体中文:

> Copyright (c) 2025-2026 Ruta Internet SRL。MIT 许可证。

### 105. ide/versions.inc

English:

> This line looks like a key or a password. Versions are often shared, and a secret saved in one stays in the timeline: take it out first, or save anyway if it isn't a real one.  //  These {n} lines look like keys or passwords. Versions are often shared, and a secret saved in one stays in the timeline: take them out first, or save anyway if they aren't real ones.

简体中文:

> 这 {n} 行看起来像密钥或密码。版本常常会被分享，保存进版本的密钥或密码会一直留在时间线里：请先把它们去掉，如果它们不是真的，也可以仍然保存。

### 106. ide/versions.inc

English:

> Not saved: lines look like keys or passwords. The dialog lists them.

简体中文:

> 未保存：有些行看起来像密钥或密码。对话框中列出了它们。

## RapidR's own text, the runtime (`crates/rapidr-value/lang/zh-Hans.po`)

### 107. crates/rapidr-ai/src/rai.rs

English:

> There is no {provider} API key yet: give one (APIKey), or say where it is kept (APIKeySource).

简体中文:

> 还没有 {provider} 的 API 密钥：请提供一个 (APIKey)，或指明它保存在哪里 (APIKeySource)。

### 108. crates/rapidr-ai/src/rai.rs

English:

> ({n} from the cache)

简体中文:

> （其中 {n} 来自缓存）

### 109. crates/rapidr-ai/src/rai.rs

English:

> {sent} tokens sent

简体中文:

> 已发送 {sent} 个 token

### 110. crates/rapidr-ai/src/rai.rs

English:

> {n} received

简体中文:

> 已接收 {n} 个

### 111. crates/rapidr-ai/src/rai.rs

English:

> about ${cost}

简体中文:

> 约 {cost} 美元

### 112. crates/rapidr-ai/src/rai.rs

English:

> KeyOwner is RapidR Studio's own: a program's keys are always its own.

简体中文:

> KeyOwner 是 RapidR Studio 自己专用的：程序的密钥始终是程序自己的。

### 113. crates/rapidr-ai/src/rai.rs

English:

> APIKeySource names no keychain entry (keychain:NAME) to keep the key under.

简体中文:

> APIKeySource 没有指定用来保存密钥的钥匙串条目 (keychain:NAME)。

### 114. crates/rapidr-ai/src/rai.rs

English:

> There is no key to keep: set APIKey first.

简体中文:

> 没有可保存的密钥：请先设置 APIKey。

### 115. crates/rapidr-ai/src/rai.rs

English:

> The key is kept in {place}.

简体中文:

> 密钥保存在{place}中。

### 116. crates/rapidr-ai/src/rai.rs

English:

> The key wasn't kept: {error}.

简体中文:

> 密钥没有保存：{error}。

### 117. crates/rapidr-ai/src/rai.rs

English:

> The key is forgotten.

简体中文:

> 密钥已被清除。

### 118. crates/rapidr-ai/src/wire.rs

English:

> Your {name} account has no credit left for this key{quoted}. Add credit on {name}'s website; RapidR can't do it for you.

简体中文:

> 你的 {name} 账户中，这个密钥已没有余额{quoted}。请在 {name} 的网站上充值；RapidR 无法替你充值。

### 119. crates/rapidr-ai/src/wire.rs

English:

> {name} sent the request on to another address ({status}): RapidR doesn't follow it with your key. If you trust that address, set the base URL to it.

简体中文:

> {name} 把请求转到了另一个地址 ({status})：RapidR 不会带着你的密钥跟过去。如果你信任该地址，请把基础 URL 设为它。

### 120. crates/rapidr-studio/src/ai.rs

English:

> RAI wants to run your program, then look at it, click and type in it. The program can do anything you can on this computer, and RAI sees whatever it shows, including private data.

简体中文:

> RAI 想运行你的程序，然后查看它、在其中点击和输入。这个程序可以做你在这台电脑上能做的任何事，RAI 也能看到它显示的一切，包括私人数据。

### 121. crates/rapidr-studio/src/ai.rs

English:

> RAI wants to bring your project back to an earlier version: your files become as they were then, saved as a new version. Nothing is lost, and Undo in Versions takes it back.

简体中文:

> RAI 想让你的项目回到较早的版本：你的文件会恢复成当时的样子，并保存为一个新版本。不会丢失任何内容，在“版本”中使用“撤销”可以收回。

### 122. crates/rapidr-studio/src/ai.rs

English:

> RAI wants to start Versions in this project, so it can save versions of your work as it goes.

简体中文:

> RAI 想在这个项目中开始记录版本，这样它就能在工作过程中保存你的工作版本。

### 123. crates/rapidr-studio/src/ai.rs

English:

> RAI wants to click and type in your running program. It sees whatever the program shows, including private data.

简体中文:

> RAI 想在你正在运行的程序中点击和输入。它能看到程序显示的一切，包括私人数据。

### 124. crates/rapidr-studio/src/ai.rs

English:

> You allowed RAI to run and use your program for this run

简体中文:

> 你已允许 RAI 在本次运行中运行并使用你的程序

### 125. crates/rapidr-studio/src/ai.rs

English:

> You allowed RAI to bring the project back to an earlier version

简体中文:

> 你已允许 RAI 让项目回到较早的版本

### 126. crates/rapidr-studio/src/ai.rs

English:

> You allowed RAI to start Versions

简体中文:

> 你已允许 RAI 开始记录版本

### 127. crates/rapidr-studio/src/ai.rs

English:

> You allowed RAI to use your running program for this run

简体中文:

> 你已允许 RAI 在本次运行中使用你正在运行的程序

### 128. crates/rapidr-studio/src/ai.rs

English:

> You didn't allow RAI to use your running program

简体中文:

> 你没有允许 RAI 使用你正在运行的程序

### 129. crates/rapidr-studio/src/notes.rs

English:

> {name}: you didn't allow it

简体中文:

> {name}：你没有允许

### 130. crates/rapidr-value/src/objects/midiport.rs

English:

> This program also wants to send and receive "system exclusive" (SysEx) messages. These can change settings stored inside a MIDI device and, on some devices, even replace its firmware. Allow it only for programs you trust.

简体中文:

> 这个程序还想发送和接收“系统专有” (SysEx) 消息。这些消息可以更改 MIDI 设备内部保存的设置，在某些设备上甚至可以替换它的固件。只对你信任的程序允许这样做。

### 131. crates/rapidr-value/src/objects/midiport.rs

English:

> This program wants to use your MIDI devices (keyboards, controllers, synthesizers). Your browser will ask you to allow it. RapidR does not send the list of your devices anywhere.

简体中文:

> 这个程序想使用你的 MIDI 设备（键盘、控制器、合成器）。你的浏览器会请你确认是否允许。RapidR 不会把你的设备列表发送到任何地方。

## The compiler's messages (`crates/rapidr-diagnostics/lang/zh-Hans.po`)

No text is marked `# legal` in this catalog.

## Added after the review

Written or marked `# legal` after the lawyer read the first 131 (they are in the catalogs in the places their texts belong; the numbers above stay the review's). Three are the new notes the review asked for ("Send only what you may share", "A provider may not offer its service", "Licences and notices are in English"), in the review's own Chinese; two are promises the review found unmarked ("never writes the answer", "nothing is sent to an AI"), faithful as they were.

### A1. ide/aicoach.inc

English:

> RAI gives hints one step at a time and never writes the answer into your code

简体中文:

> RAI 一步一步地给出提示，绝不会把答案写进你的代码

### A2. ide/aisetup.inc

English:

> Send only what you may share: other people's personal data, an employer's code or schoolwork may not be yours to send.

简体中文:

> 只发送你有权发送的内容：他人的个人信息、雇主的代码或学校作业，你不一定有权发送。

### A3. ide/aisetup.inc

English:

> A provider may not offer its service where you live, and its servers may be in another country: check its terms.

简体中文:

> 服务商可能不在你所在的地区提供服务，其服务器也可能位于其他国家或地区：请查看其条款。

### A4. ide/schoollock.inc

English:

> The AI Assistant pane, Set Up RAI and Settings > AI: nothing is sent to an AI

简体中文:

> “AI 助手”面板、配置 RAI，以及设置 > AI：不会向任何 AI 发送任何内容

### A5. ide/shell.inc

English:

> Licences and notices are in English; the English text applies.

简体中文:

> 许可证和声明只提供英文，以英文文本为准。
