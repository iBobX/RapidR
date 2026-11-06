# Simplified Chinese glossary: 简体中文 (zh-Hans) for RapidR

**Status: beta, an AI translation (lane I18N-ZH, 2026-11-06).** No native reader has checked it yet; corrections are welcome. It is the one list of words every Chinese text of RapidR follows: RapidR Studio's screens, RAI, the runtime's messages and the compiler's messages (the components' Help and the manual come next). It is written in English so it can be reviewed against the English product; the Chinese is in the second column of each table.

The three catalogs it governs: `ide/lang/zh-Hans.po` (Studio, 1631 texts), `crates/rapidr-value/lang/zh-Hans.po` (RapidR's own text, 952) and `crates/rapidr-diagnostics/lang/zh-Hans.po` (the compiler, 120). `rapidr i18n studio|runtime|compiler --check` keeps them whole and keeps every `{name}`, `%1`, `` `code` `` and `&` mnemonic of the English.

Chinese is written in **Simplified characters** (the mainland's 简体). Traditional Chinese (繁體) is a separate language, `zh-Hant`, with its own catalogs one day; it does not fall back to this one on purpose.

## 1. Style

### 1.1 Voice

- **你, never 您.** One person spoken to directly, friendly, like the English. 您 sounds like a bank. Where the English has no "you" ("Nothing to undo"), the Chinese has no pronoun either: 没有可撤销的内容.
- **Plain words** (docs/design-guidelines.md, "Words"): a short sentence says what happened, then what to do. 出了点问题 ("something went wrong"), not 发生了异常错误. No "please" (请) except where the English asks the AI or the person to do something ("请先选择模型"), and no exclamation marks.
- **Studio speaks of itself by its name**, RAI as 助手 ("the assistant") or by its name: "RAI 想在你正在运行的程序中点击和输入".
- **Kids and engineers both**: the everyday word where one exists (文件夹, 窗口, 保存), the programmer's word where it is the standard one (断点, 调用堆栈, 事件处理程序).
- **Buttons, menu items and titles are short verb phrases**: 保存, 另存为..., 全部接受. Sentences end with 。; labels and menu items have no final mark; a field's label ends with `:` (see 2.1).

### 1.2 Words for the screen

| English | 简体中文 | Note |
|---|---|---|
| window | 窗口 | A window, the Window menu, the program's running window |
| form | 窗体 | An `RForm` as the designer and the code treat it. 窗口 stays for any window |
| component | 组件 | What you drop on a form. A running program's controls are also 组件 |
| control | 控件 | Only where the English says control |
| pane, panel | 面板 | One word for both |
| dialog | 对话框 | |
| tab | 标签页 | A document's or a pane's tab. 标签 is a *label* (an `RLabel`) |
| tool bar / status bar | 工具栏 / 状态栏 | |
| tooltip | 提示 | A hint on hover. The AI's "hint" in Coach mode is 提示 too; they never share a surface |
| icon | 图标 | |
| check box / radio button | 复选框 / 单选按钮 | |
| list box / combo box | 列表框 / 下拉列表 | |
| click / double-click / right-click | 点击 / 双击 / 右键点击 | On a touch screen, "tap" is also 点击 |
| select / choose | 选择 | |
| folder | 文件夹 | Never 目录 |
| file | 文件 | |
| theme | 主题 | Light / Dark / High Contrast / Classic = 浅色 / 深色 / 高对比度 / 经典 |
| Welcome page | 欢迎页 | The tab is 欢迎 |
| the Settings window | 设置 | Menu: 工具 > 设置 |
| Help | 帮助 | |
| Examples | 示例 | |

### 1.3 Menus and commands

| English | 简体中文 |
|---|---|
| File / Edit / View / Project / Run / Debug / Format / Tools / Window / Help | 文件 / 编辑 / 视图 / 项目 / 运行 / 调试 / 格式 / 工具 / 窗口 / 帮助 |
| New Project... / Open Project or File... | 新建项目... / 打开项目或文件... |
| Save / Save As... / Save All | 保存 / 另存为... / 全部保存 |
| Close Document / Close Project / Exit | 关闭文档 / 关闭项目 / 退出 |
| Undo / Redo | 撤销 / 重做 |
| Cut / Copy / Paste / Select All | 剪切 / 复制 / 粘贴 / 全选 |
| Find... / Replace... / Find in Files... / Go to Line... | 查找... / 替换... / 在文件中查找... / 转到行... |
| Settings... | 设置... |
| Command Palette... | 命令面板... |
| Keyboard Shortcuts | 键盘快捷键 |
| Toggle Comment / Format Document | 切换注释 / 格式化文档 |
| Quick Fix... | 快速修复... |
| Add File / Add Form / Add Module | 添加文件 / 添加窗体 / 添加模块 |
| Reset Layout | 重置布局 |
| OK / Cancel / Apply / Close | 确定 / 取消 / 应用 / 关闭 |
| Next > / < Back / Finish | 下一步 > / < 上一步 / 完成 |
| Browse... | 浏览... |
| Add / Remove / Delete / Clear | 添加 / 移除 / 删除 / 清除 |
| Rename | 重命名 |
| Learn more / Not Now | 了解更多 / 以后再说 |
| Discard / Don't Save | 放弃 / 不保存 |
| Send / Stop | 发送 / 停止 |
| Test | 测试 |
| Preview | 预览 |

**Delete / Remove / Clear.** 删除 takes something away for good (a chat, a file); 移除 takes it out of a list and keeps it (an include folder, a file from a project); 清除 empties a list or a log.

### 1.4 The programming words

| English | 简体中文 | Note |
|---|---|---|
| project | 项目 | |
| run | 运行 | |
| build | 构建 | Build Native App = 构建原生应用. Where code really is compiled (bytecode, machine code): 编译 / 编译器 |
| debug / debugger | 调试 / 调试器 | |
| breakpoint | 断点 | The breakpoint dialog's labels: 条件 / 命中次数 / 日志 |
| watch | 监视 | The pane; Add Watch = 添加监视 |
| call stack | 调用堆栈 | |
| variable / constant | 变量 / 常量 | locals / globals = 局部变量 / 全局变量 |
| array | 数组 | |
| string | 字符串 | |
| integer / decimal number | 整数 / 小数 | |
| function / SUB | 函数 / 例程 (as a noun) | The keywords `SUB`, `FUNCTION`, `TYPE` stay as written, with no plural |
| parameter | 参数 | |
| property / event / method | 属性 / 事件 / 方法 | |
| event handler | 事件处理程序 | |
| module | 模块 | |
| include file / include folder | 包含文件 / 包含文件夹 | The `$INCLUDE` directive stays |
| library | 库 | |
| keyword | 关键字 | |
| loop | 循环 | |
| database / table / row / column / query | 数据库 / 表 / 行 / 列 / 查询 | |
| server / client / request | 服务器 / 客户端 / 请求 | |
| Toolbox / Properties / Outline | 工具箱 / 属性 / 大纲 | |
| Output / Problems / Immediate | 输出 / 问题 / 即时 | |
| Variables / Watch / Call Stack / Breakpoints | 变量 / 监视 / 调用堆栈 / 断点 | |
| Step Into / Step Over / Step Out | 单步进入 / 单步跳过 / 单步跳出 | |
| error / warning / note | 错误 / 警告 / 备注 | The Problems pane's three kinds |
| run-time error / compile error | 运行时错误 / 编译错误 | |
| Design / Code | 设计 / 代码 | The two views of a form |
| designer | 设计器 | |
| caption | 标题 / 文字 | 标题 of a window; 文字 of a button or label; the property `Caption` is code |
| Bring to Front / Send to Back | 置于顶层 / 置于底层 | |
| Menu Editor / Tab Order | 菜单编辑器 / Tab 键顺序 | |

### 1.5 Versions (the Simple words)

| English | 简体中文 | Git's name |
|---|---|---|
| Versions | 版本 | The product never calls itself Git; tooltips say "Git 称之为 commit" |
| Start versioning this project | 开始为此项目记录版本 | init |
| Save a Version... | 保存版本... | commit |
| Timeline | 时间线 | log |
| What Changed? | 有哪些更改？ | diff |
| Go Back to This Version... | 回到此版本... | revert (never a hard reset) |
| Undo this version | 撤销此版本 | |
| Local History | 本地历史记录 | |
| experiment | 实验 | branch |
| the Pro level | English Git words stay: commit, merge, rebase, stash... | people search the web for them |

### 1.6 AI

| English | 简体中文 | Note |
|---|---|---|
| AI | AI | Latin letters, as every Chinese product writes it |
| RAI | RAI | The brand, never translated |
| assistant | 助手 | The AI Assistant pane = AI 助手 |
| chat / New Chat | 对话 / 新对话 | |
| model | 模型 | |
| provider | 服务商 | |
| API key | API 密钥 | The secret. 密码 is only for a password |
| token (AI: a piece of a word) | token | Latin letters, as the providers write it |
| token (access, for AI apps) | 令牌 | |
| keychain | 钥匙串 | |
| context / context window | 上下文 / 上下文窗口 | |
| reasoning effort | 推理强度 | Auto / Low / Medium / High = 自动 / 低 / 中 / 高 |
| AI apps | AI 应用 | An outside program that connects |
| local AI app | 本地 AI 应用 | |
| cloud service | 云服务 | |
| licence | 许可证 | |
| Accept / Reject | 接受 / 拒绝 | A change the AI made |
| Allow Once / Allow for This Session / Deny | 允许一次 / 本次会话中允许 / 不允许 | A permission |
| Read only / Ask before changes / Allow edits / Allow everything | 只读 / 更改前询问 / 允许编辑 / 全部允许 | The four permission modes |
| Build / Coach (the dial) | 构建 / 辅导 | "Coach mode" = 辅导模式 |
| hint (Coach mode's ladder) | 提示 | 提示 1/4: 点拨 → 思路 → 代码的大致结构 → 代码 |
| School Lock | 学校锁定 | A teacher's password locks the AI |
| recipe (small-model recipes) | 配方 | |

### 1.7 Errors and messages

| English | 简体中文 |
|---|---|
| can't X / couldn't X | 无法 X |
| not found | 找不到 X / 未找到 |
| doesn't exist / already exists | 不存在 / 已存在 |
| expected X | 应为 X |
| unexpected X | 不应出现 X |
| Syntax error | 语法错误 |
| read-only | 只读 |
| unsaved changes | 未保存的更改 |
| Nothing to undo | 没有可撤销的内容 |
| Something went wrong in Studio | Studio 出了点问题 |
| Your work is safe | 你的工作是安全的 |

## 2. Writing rules

### 2.1 Punctuation

- **Full-width marks in sentences**: ，。：；？！（）“”‘’、 and the dash ——. The ellipsis of progress texts is the source's "…" or "...", copied as the English has it.
- **A menu or button that asks for more keeps the three ASCII dots** (`另存为...`), exactly as the English does: Studio strips exactly those to make tool-bar tips and palette lines (`PlainTitle`), and `rapidr i18n --check` demands them.
- **A field's label ends with an ASCII colon**, `名称:`, as the English: Studio's layout reads it. Inside a sentence the colon is the full-width ：.
- **Quotes** around names of Studio's own buttons and panes: “发送”, “问题”. Code and file names stay in code font or plain, never in quotes.
- **A space between Chinese and Latin letters or digits**: `按 F5 运行`, `{n} 个文件`, `RAI 想…`. No space next to a full-width mark.

### 2.2 Mnemonics (the letter you press with Alt)

Chinese words have no letter to underline, so the mnemonic is written **after the word**, in brackets, as Windows does: `文件(&F)`, `另存为(&A)...`, `全部接受(&A)`. The letter is **the English source's letter**, so it is unique on its menu exactly where the English was, and the Alt keys people already know do not change. A mnemonic on an accented letter or a Chinese character is never used (`rapidr i18n --check` refuses it: it can't be typed with Alt on every keyboard).

Studio's tool-bar tips, the command palette and any other place that shows a command's plain title drop the whole `(&X)` (`PlainTitle` in `ide/commands.inc`).

Shortcuts are physical keys and never change with the language: Ctrl+F finds, F5 runs. Key names stay as the English shows them: `Ctrl`, `Shift`, `Alt`, `Enter`, `Esc`, `Tab`, `Del`, F1–F12.

### 2.3 Numbers, dates, units

- Counts come in as placeholders (`{n}`). Chinese has **one plural form** (`nplurals=1; plural=0;`), so one sentence serves 0, 1 and many: `{n} 个文件`. A text whose English singular has no `{n}` ("1 file") and whose plural has it uses the plural's shape.
- Money: 美元 ("dollars") or 美分 ("cents"), never a bare $, which is also the yuan in some fonts.
- Units: GB, MB, px, s, ms, min stay in Latin letters, with a space after the number. Seconds in prose: 秒; minutes: 分钟.
- Numeric dates are ISO (2026-11-06) in logs. Time is 24-hour.

### 2.4 Never translate

The language (`PRINT`, `DIM`, `SUB`, `FUNCTION`, `TYPE`, `$INCLUDE`, `TR$`); names in code (`RButton`, `RForm`, `Caption`, `OnClick`, `Application`); file names, extensions, commands and addresses (`.rr`, `rapidr setup`, `127.0.0.1`); command ids and tool names (`propose_edit`, `write_file`); product and protocol names (RapidR, RapidR Studio, Studio, RAI, Git, GitHub, MCP, Anthropic, Claude, OpenAI, DeepSeek, Windows, macOS, Linux); acronyms (API, URL, CSV, JSON, SQL, CPU, GPU); `{name}` placeholders and `&`; key names. **Licences, notices and `LEGAL.md` are not translated:** the English text is the one that applies (the About box's copyright line is the one exception, with its licence's name next to it).

### 2.5 Texts that carry legal weight

The consent dialogs, cost and privacy notes, key and secret-check warnings, "what is sent" texts, School Lock's and the crash report's are translated **by meaning, point by point, dropping nothing**. They carry `# legal` in the catalogs and are listed with their English in `docs/legal/i18n-zh-legal-texts.md` for the lawyer check.

### 2.6 Length

Chinese runs shorter than English (a third to a half), so no text had to be shortened. The `(&X)` after a button's word is what grows. Chinese has **no spaces**, so a long label would run past its box: the shared word-wrapper (`rapidr_value::autosize`, labels, message boxes, code popups) lets a line end between two Chinese characters, with the usual rules (no line starts with ，。！？）” or ends with （“《).

## 3. Fonts

RapidR's bundled fonts have no Chinese glyphs. With a Chinese UI the kernel draws Chinese with **the system's own Simplified Chinese font** (Microsoft YaHei UI on Windows, Heiti SC on macOS, Noto Sans CJK SC on Linux), named before the system's own choice, which on an English Windows is a Japanese font that lacks many Simplified characters. RapidR never bundles or copies Apple's or Microsoft's fonts (their licences forbid it). On a Linux with no CJK font Studio says so in its Output, in Chinese and English, and names the free package `fonts-noto-cjk`. On the web a subset of Noto Sans SC (OFL-1.1) is fetched only when the page needs a Chinese character. Evidence, measurements and the licence check: [docs/legal/i18n-zh-font.md](../legal/i18n-zh-font.md).

## 4. Words to check first (for a native reader)

This is an AI translation. If you read Chinese and have ten minutes, these choices matter most; every other row of the tables above follows from them.

| English | Used | Alternatives |
|---|---|---|
| Build (the command) | 构建 | 生成, 编译 (kept for code that is really compiled) |
| Coach mode | 辅导模式 | 教练模式, 导师模式 |
| School Lock | 学校锁定 | 课堂锁定, 校园锁 |
| experiment (a version-control branch) | 实验 | 分支 (the word its tooltip gives) |
| Immediate (pane) | 即时 | 即时窗口, 立即窗口 |
| Watch (pane) | 监视 | 监控, 查看 |
| token (an access token) | 令牌 | 密钥 (already used for the API key) |
| provider (AI) | 服务商 | 提供商, 供应商 |
| recipe (the small-model recipes) | 配方 | 食谱, 范例 |
| a nudge (the first hint) | 点拨 | 提示, 推一把 |
| assets | 素材 | 资源 (used for resources) |
| event handler | 事件处理程序 | 事件处理器, 事件过程 |
| include file | 包含文件 | 引用文件 |
| chat | 对话 | 聊天 |
| keychain | 钥匙串 | 密钥链 |
| "you" for the person | 你 | 您 (rejected: too formal for kids) |

Menu mnemonics are written `文件(&F)`; if native readers prefer to drop them on macOS, `rapidr i18n --check` would need a rule that allows a Chinese text with none.
