<!-- translated-from: windows-api.md c085bed2f6105fe1 -->
# 一个统一的 Windows API 标准

如果你写过直接和 Windows 打交道的程序——这里一个 `DECLARE … LIB "user32"`，那里一个 `CONST WM_CLOSE`——你多半听过这样的故事。大家都有一份 `WIN32API.INC`。它来自朋友或一个旧示例，后来又被人手工改过：这里补一个常量，那里修一处声明。多年下来出现了很多版本，彼此并不一致。同一个程序里放两份就会冲突。别人那里能用的代码片段，到你这里却失败了，因为你的文件名字不同、值不同，或者干脆没有那一项。我们中的许多人为此浪费了好几个小时，而这并不是谁的错：只是一直没有一个可以共享的通用文件。

RapidR 现在有了：内置的 **`RWin32.inc`**。

## 首先：你现有的一切都不会失效

- **你的程序照常运行。** 带有自己 `WIN32API.INC`（或任何其他包含文件）的程序，通过 RapidR 的 RapidQ 兼容层，编译和运行的方式与以前完全一样。无论今天还是以后，你都不需要改任何东西。
- **你自己的包含文件仍然归你。** RapidR 绝不会替换或编辑它们。如果你更喜欢自己的文件，就保留它。
- **不会替你切换任何东西。** 导入器只是*提供*这个更改，并且在任何事情发生之前，把每一处差异都展示给你（见下文）。

## 如果你愿意，更改只需一行

```basic
$INCLUDE "RWin32.inc"
```

就这些。不需要下载、查找或复制任何文件：`RWin32.inc` 是 RapidR 的一部分，每台电脑、每一次安装都有（Windows 7 和 XP 的下载包也一样）。任何路径或大小写都可以（`$INCLUDE "C:\rapidq\include\RWIN32.INC"` 读取的是同一个内置文件），所以旧的那一行只需要改一下文件名。

**RapidR 帮你切换。** 当你导入一个包含 `WIN32API.INC` 的旧程序（[导入旧程序](../importing-rapidq.md)）时：

- 副本会照旧保留你的文件，并且会检查它的编译结果与原程序完全一样；
- 随后报告会说明 `RWin32.inc` 可以取代它，并且**针对你的程序用到的名称，列出所有会不同的地方**：值不一样的常量、你的文件用另一个 DLL 名称或不同参数个数调用的函数、`RWin32.inc` 里没有的名称，以及你的文件除了声明之外还做了什么（比如 `$DEFINE INTEGER SHORT`、`$OPTION`）；
- 由你决定。在 RapidR Studio 中，导入完成后，文件 ▸ 导入 RapidQ 项目或文件… 会问你一声（默认答案是**保留我的文件**）。在命令行中，`rapidr import-rapidq myprogram.bas --rwin32` 会在副本中完成切换，并检查副本仍能编译。你的原程序绝不会被改动。

## 为什么一个统一的标准对大家都有好处

- **所有人共用一个文件。** 使用 `RWin32.inc` 的示例、讨论里的回答或手册里的代码片段，在你的电脑上和在作者的电脑上运行结果一样。RapidR 的助手 RAI 和其他 AI 助手也可以依赖同样的名称，所以它们写出的 Windows 代码会更准确。
- **完整而且是最新的。** `RWin32.inc` 不是手工输入的：它由 RapidR 自己的工具，根据 Microsoft 发布的 Win32 API 元数据生成。其中有 55,710 个常量、3,675 个结构体（`TYPE`）和来自 29 个 Windows DLL 的 6,506 个函数（kernel32、user32、gdi32、advapi32、shell32、comdlg32、comctl32、winmm、winspool、ole32、wininet、ws2_32 等）。
- **在 64 位和 32 位 Windows 上都正确。** 旧的包含文件是为 32 位 Windows 写的，在那里指针和句柄是 4 字节。在如今的 64 位 Windows 上它们是 8 字节，所以像 `NOTIFYICONDATA`、`MSG` 或 `SYSTEM_INFO` 这样的结构体需要另一种布局。`RWin32.inc` 两种都有，你的程序会得到与目标电脑相符的那一种。
- **经典名称都在。** `MessageBox`、`GetWindowText`、`FindWindow`、`SendMessage`、`ShellExecute`、`RECT`、`POINTAPI`、`LOGFONT` 等，含义和以前一样：不带后缀的名称就是 ANSI（“A”）版的函数或结构体，和经典文件里一样。一份广泛使用的经典 `WIN32API.INC` 声明了 1,514 个函数，其中 1,496 个（98.8%）在 `RWin32.inc` 中以同样的名称存在；现代的 A 和 W 名称（`MessageBoxA`、`MessageBoxW`）也都有。
- **速度快。** 程序只获得它用到的东西：一个有二十次 Windows 调用的程序，编译用时远不到一秒，而且只带上这二十个声明。

## 选择权在你

如果喜欢，就在新程序里使用 `RWin32.inc`；方便的时候再切换旧程序；或者保留你自己的文件：这三种做法都可以，RapidR 对每一种都支持。如果 `RWin32.inc` 里缺少什么，或者有看起来不对的地方，请在 [Discussions](https://github.com/iBobX/RapidR/discussions) 告诉我们，或者提交一个 [issue](https://github.com/iBobX/RapidR/issues)：它是生成出来的，所以一次修复就能惠及所有人。

RapidR 是免费且开放的，为的是让任何人都能学会编程；我们一起让它不断成长，人和 AI 一同参与。谢谢你成为其中的一员。

---

## 工作原理

### 只包含你的程序用到的部分，并且你自己的名称优先

`RWin32.inc` 很大，所以 RapidR 不会把它整个编译进你的程序。它会读取你的程序，看出它用到了哪些名称，然后只提供这些名称的声明（以及这些声明所需的 `TYPE`），别的一概不加。

**程序自己定义的名称永远优先**：`CONST`、`DECLARE`、`TYPE`、`SUB` 或 `FUNCTION`、`DIM`、`CREATE`、`$DEFINE`，或者你给它赋值的变量。因此，一个自己声明了 `CONST WM_MOUSEWHEEL = &H20A`，或者自己写了 `DECLARE FUNCTION GetTickCount …` 的旧程序，配合 `RWin32.inc` 编译时不会出现“已经定义过了”的错误：用的是你的。（变量和 `TYPE` 可以同名：`DIM msg AS MSG` 可以正常工作。）

### RAPIDQ.INC 的数值，以及 True 和 False

RAPIDQ.INC 同样提供的 Windows 数值（`MB_OK`、`MB_ICONERROR`、`VK_F1`、`SND_ASYNC`、字符集等）不会在 `RWin32.inc` 中重复，所以无论哪个先出现，两者都不会冲突。

- 在 `.bas` 程序中，如果包含了 `RWin32.inc` 而**没有** `$INCLUDE "RAPIDQ.INC"`，RapidR 的 RapidQ 兼容层会提供程序用到的那些数值，数值取 RAPIDQ.INC 的（与 Windows 的相同），并且 `True` = 1、`False` = 0，和经典的 `WIN32API.INC` 一样。
- 在 RapidR 程序（`.rr`）中它们已经内置，`True` 仍然是 RapidR 的 -1。

### RapidR 也有的名称：Sleep、Beep、MessageBox

有几个 Windows 函数与 RapidR 自己的某个功能同名（`Sleep`、`Beep`、`MessageBox`、`SendMessage`、`PlaySound`）。

- 在经典程序（`.bas`）中，`RWin32.inc` 给你的是 Windows 的函数，和旧的包含文件一样：`Sleep 500` 通过 kernel32 等待 500 毫秒。
- 在 RapidR 程序（`.rr`）中，保留的是 RapidR 自己的：`SLEEP 1` 在任何系统上都等待一秒。要在那里调用 Windows 的那个，请用你自己的名称来声明它：`DECLARE SUB WinSleep LIB "kernel32" ALIAS "Sleep" (BYVAL ms AS LONG)`。

### 64 位和 32 位：`WIN64`

当程序为 64 位编译时（x64 和 ARM64 上的 Windows、macOS、Linux 以及 Web），RapidR 会定义 `WIN64`，`$IFDEF WIN64` 会选出每个结构体的 64 位布局；为 32 位 Windows（Windows 7 和 XP 的下载包）编译时则不会定义它。你自己的代码也可以用它：

```basic
$IFDEF WIN64
PRINT "64-bit"
$ELSE
PRINT "32-bit"
$ENDIF
```

（Web 也会定义 `WIN64`，所以程序里的结构体在那里的大小与 64 位桌面系统上相同；不管怎样，Windows 的函数在 Web 上都无法运行。）

### 声明是怎么写的

- **句柄、指针和其他与指针等宽的数值**是 `LONG` 参数：RapidR 用 64 位的槽位传递它们，并替你保存 DLL 返回的 64 位指针（[DLL 调用](../language.md#subs-and-functions)）。
- **文本**（`LPSTR`、`LPCSTR`）是 `STRING`：给它文本，或者一个由函数填充的 `SPACE$(n)` 缓冲区。宽文本（W 函数）是 `LONG`：UTF-16 缓冲区的地址。经典名称是 A 函数，接收的是普通字符串。
- **结构体**按引用传递（`lprc AS RECT`）。要表示“无”（`NULL`），就传 `0`：RapidR 会传入一个空指针。
- **在 `TYPE` 中**，指针或句柄字段在 64 位 Windows 上是 `INT64`，在 32 位上是 `LONG`；文本指针字段是 `STRING`（在 64 位上，它后面还有一个 `…Hi` `LONG`，用来放指针的高半部分，始终为 0）；Windows 的编译器在字段之间留下的空隙本身也是字段（`rrPad1`、`rrPad2` 等），所以 `SIZEOF` 就是结构体的真实大小。无名联合体取其最大的成员。
- **按值传递的小结构体**（`PtInRect`、`WindowFromPoint` 中的 `POINT`）是一个 `INT64`：`x + y * 4294967296`（也就是说，`y` 在高 32 位）。64 位 Windows 就是这样接收它的；在 32 位 Windows 上，同一个 `INT64` 给出的字节相同。

### 其中没有的内容

GUID 常量、COM 接口、参数个数可变的函数（`wsprintf`），以及少数按值接收大型结构体的函数。有些领域暂时没有收录（加密、GDI+、ODBC、WinHTTP、IP Helper 等）：RapidR 为其中大多数提供了自己的组件（`RSQLITE`、`RDB`、`RSOCKET` 等），你也随时可以在包含文件旁边用 `DECLARE` 自己声明需要的东西。

### 在 macOS、Linux 和 Web 上

包含 `RWin32.inc` 的程序在所有平台上都能编译；对 Windows DLL 的调用在 Windows 上运行，在其他系统上会停下来，并给出一条指明函数名称的清晰信息。参见[这个示例](../../../examples/windows/rwin32_system_info.rr)。

### 文件本身

你可以阅读它：RapidR 源代码中的 [crates/rapidr-preprocessor/src/libraries/RWin32.inc](../../../crates/rapidr-preprocessor/src/libraries/RWin32.inc)。文件头说明了它的来源：由 RapidR 自己的工具（`tools/gen-win32`），根据 Microsoft 在 windows-rs 项目中发布的 Win32 API 元数据生成，采用 MIT 许可证，并附有 Microsoft 的版权声明（[来源说明](../../legal/win32-declarations-provenance.md)）。`$INCLUDE "RWin32.inc"` 读取的始终是 RapidR 内置的文件，绝不会是你磁盘上同名的文件。
