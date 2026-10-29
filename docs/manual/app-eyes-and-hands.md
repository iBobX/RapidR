# A program's eyes and hands: RWindowCapture and RAppHands

A RapidR program can see its own windows and use them as a person would:

- **RWindowCapture**, its eyes: a picture of its windows, and a report of
  where every control is and what looks wrong (text cut off, controls on
  top of each other, a control past the window's edge);
- **RAppHands**, its hands: clicks, typing, keys, drags and the mouse
  wheel, played into its own windows with a pointer you can see.

RapidR Studio uses the same eyes and hands when an AI looks at your
running program or clicks in it (the `app_*` tools, and RAI's: see below).

Both reach **only the program's own windows**. They never read the screen,
never move your real mouse or press your real keys, and can't see or touch
another program, the desktop or RapidR Studio. That is why they need no
Screen Recording or Accessibility permission from the system.

## RWindowCapture: pictures and the layout report

> **Use it responsibly.**
> - **Your windows can show other people's private data.** A picture or a layout report holds all of it: names, addresses, grades, messages. Keep them on the user's own computer. Before one goes anywhere else (a server, an email, an AI service, an analytics tool) the person must know and agree, and you must only collect what you need. Tell people, in plain words, what is captured and where it goes.
> - **No hidden recording.** Never capture or record a person's windows without telling them before it starts and showing that it is on. Sending pictures or recordings of users' screens to a server is processing personal data: in the EU and UK it needs a legal basis (usually consent) and your privacy notice; some US states treat secretly recording what people do on a page or app as wiretapping, and fines are counted per person. Keep it off by default, mask password and personal fields, and do not do it in programs for children without a parent's or school's permission.
> - **It acts only inside your own program, and must not be used to get around anything.** It plays input into your program's own windows and nothing else: not the system, not other programs. Do not use it to click "I agree", "Pay" or "Delete" for someone who has not chosen to, to answer a security question, a password or a permission prompt, to imitate people (fake votes, likes, game bots), or to automate another service against its terms.
> - **What it clicks happens for real.** Tours, demos and self-tests can send real emails, charge real cards and delete real data. Run them on test data.

![snapshot.rr after its Save button was narrowed and Check layout was pressed](images/app-eyes/snapshot-check.png)

The example is [examples/gui/snapshot.rr](../../examples/gui/snapshot.rr):
`rapidr run gui/snapshot.rr` from the examples folder. Press **Narrow**,
then **Check layout**: the program finds its own cut-off caption.

```basic
DIM Eyes AS RWindowCapture

Eyes.Scale = 1                         ' 1: one picture pixel per program pixel
IF Eyes.SaveToFile("snapshot.png", "Form") THEN
    PRINT "saved"; Eyes.Width; " x"; Eyes.Height
ELSE
    PRINT Eyes.Error                   ' no such window, a folder you can't write …
END IF

PRINT Eyes.LayoutReport               ' every window that shows, in words
IF Eyes.Problems <> "" THEN PRINT "Something looks wrong:"; Eyes.Problems
```

| Member | What it does |
|---|---|
| `SaveToFile(FileName [, Window])` | Saves a picture of one window — a form's name or its title — or, without one, of every window that shows, side by side. A PNG; a BMP when the name ends in `.bmp`. True when saved, else `Error` says why. |
| `Scale` | 0 (the default): the screen's scale, sharp on a high-DPI screen (twice the program's pixels on a 2x screen). 1: the program's own pixels. 2: twice. |
| `Frame` | True (the default): each window with its title bar, as RapidR draws a window on the web and in the designer. False: the inside only (what `Form.SaveImage` saves). |
| `Width`, `Height` | The last picture's size in pixels. |
| `LayoutReport([Window])` | A line a control: its name, type, caption or text, place and size in the window, `hidden`, `disabled` or `has the keyboard`; then the problems, or that there are none. |
| `LayoutJson([Window])` | The same as JSON, for a program that reads it with `RJson`. |
| `Problems([Window])` | Only what looks wrong, a line a problem with its numbers and what to do; `""` when nothing does. |
| `Windows` | The windows that show, back to front: the form's name and its title. |
| `Error` | Why the last call failed. A window the program hasn't got lists the windows it has. |

The problems it finds:

- a button's, check box's or label's caption that doesn't fit its width;
- a word-wrapped label whose lines don't fit its height;
- two controls of the same parent on top of each other (labels, panels,
  group boxes and other containers aren't counted: they hold others or
  sit under them on purpose);
- a control past its window's edge, or wholly outside it (inside a scroll
  box or a tab page that is normal, and not counted).

A password box's text shows as dots in the report, never what was typed.

Recording a video of the windows comes later (in formats that are free to
use: WebM, animated WebP or GIF).

## RAppHands: clicks, typing and keys, with a pointer

> **Use it responsibly.** It acts only inside your own program and must not
> be used to get around anything: never click "I agree", "Pay" or "Delete"
> for someone who has not chosen to, answer a security question, a password
> or a permission prompt, imitate people (fake votes, likes, game bots) or
> automate another service against its terms. What it clicks happens for
> real — tours, demos and self-tests can send real emails, charge real cards
> and delete real data: run them on test data. (The whole box is under
> [RWindowCapture](#rwindowcapture-pictures-and-the-layout-report).)

![guided_tour.rr while its tour types into the Task box: the pointer and the ring of its click](images/app-eyes/tour-pointer.png)

The example is [examples/gui/guided_tour.rr](../../examples/gui/guided_tour.rr):
a to-do list whose **Show me** button adds two tasks for you.

```basic
CREATE Tour AS RAppHands
    OnDone = StepDone
END CREATE

SUB TourClick
    Tour.TypeText "label:Task", "Buy milk"   ' the box beside the label "Task:"
    Tour.Click "Add"                         ' the button that shows "Add"
    Tour.PressKeys "Ctrl+A Backspace", "TaskEdit"
END SUB

SUB StepDone(Id AS INTEGER, Ok AS INTEGER, Text AS STRING)
    IF Ok THEN Steps.AddItems Text ELSE Steps.AddItems "Not done: " + Text
END SUB
```

Each call queues its step and returns at once (the step's number); the
steps are played one after the other while your program runs on, and
**OnDone(Id, Ok, Text)** says how each went — what was done, or what is
there instead:

> No button (or other control) shows "Go" in form. Buttons now: "Add",
> "Clear", "Show me".

![The tour done: two tasks added, each step said in the list](images/app-eyes/tour-done.png)

| Member | What it does |
|---|---|
| `Click(Target)`, `DoubleClick`, `RightClick` | Clicks the control as a person would: the pointer glides there, presses, lets go. |
| `ClickAt(Window, X, Y)` | Clicks at a point of a window's inside, in the program's pixels (a canvas, a picture). |
| `TypeText(Target, Text)` | Clicks the control, then types the text a key at a time (a line break is Enter). `""` as the target: what has the keyboard. Characters no key types (é, 中) arrive as the system's input method gives them. |
| `PressKeys(Keys [, Target])` | `"Enter"`, `"Ctrl+S"`, `"Shift+Tab"`, `"Alt+F4"`, `"F5"`, several separated by spaces (`"Tab Tab Enter"`). Ctrl is the shortcut key on every system (Command's shortcuts on a Mac). |
| `Drag(Target, ToX, ToY)` | Presses on the control, moves to the point of the same window, lets go. |
| `Scroll(Target [, Notches])` | Turns the mouse wheel over the control: 3 notches down unless you say; negative: up. |
| `Hover(Target)` | Moves the pointer there and stays a moment (its hint shows). |
| `Wait(Milliseconds)` | Lets the program run on its own before the next step. |
| `Cancel` | Drops the steps still waiting. |
| `ShowCursor` | True (the default): the pointer and the ring of each click are drawn. False: the steps happen at once, unseen. |
| `Speed` | 1 a person's pace, 2 twice as fast, 0 at once. |
| `Busy`, `Pending`, `LastResult`, `Error` | Steps waiting or running; how many; the last step's sentence; why the last one failed. |

**Targets** are found as a person finds them: a control's name, what it
shows (a button's caption: `"Add"`, `"7"`), or the label beside it (the
edit or list nearest to the right of or under a label saying it). Force
one with `name:`, `caption:` or `label:` — `"label:Task"`. A window under
a modal one takes no input, as for you: the step says which window waits
for an answer. A disabled control isn't clicked: the step says it is
disabled.

**The pointer** is an arrow in the theme's text colour with a rim in its
window colour, and a dot in the theme's accent at its tail: it is the
program's, not yours. Each click shows a ring that grows and fades. With
the system's **reduced motion** turned on (macOS: Accessibility > Display;
Windows: Animation effects off) the pointer jumps instead of gliding and the
ring doesn't grow. Your real mouse keeps working all along.

## In RapidR Studio: AI apps and RAI look at your running program

When an AI works with RapidR Studio — an AI app connected to Studio
([Let AI apps work in Studio](ai-apps.md)) or RAI, Studio's own assistant
([The AI assistant](ai-assistant.md)) — it can look at the program you run
and use it, through these same eyes and hands:

| Tool | What it does | Permission |
|---|---|---|
| `app_layout` | The running program's windows and controls in words, and what looks wrong | counts as running it (asks like using it) |
| `app_screenshot` | A picture of its windows (AI apps; RAI gets the layout instead) | counts as running it (asks like using it) |
| `app_click`, `app_type`, `app_key`, `app_drag`, `app_scroll` | Clicks and types in it — you see the pointer do it | counts as running it |
| `app_wait` | Lets it work on its own a moment | always |

For AI apps the permission menu decides: **Read only** lets them neither
look at nor use the program; **Ask before changes** and **Allow edits** ask
you once for each run of the program (Allow Once covers pictures, clicks and
typing until that run ends; Allow for This Session, every run until you
close the project or turn AI apps off); **Allow everything** doesn't ask.
RAI asks the same way, once for each run, with a card in its chat: **Allow
for this run** lets it look, click and type until the run ends; **Don't
allow** answers it no for that answer (it asks again in a later one). With
its permission set to reading only it can't look at or use the program at
all. It gets these tools only while your program runs.

![RAI's card in its chat: RAI wants to click and type in your running program, Allow for this run or Don't allow; the status line waits for your answer](images/app-eyes/rai-asks.png)

**What the AI sees.** A picture or the layout report shows whatever your program shows: your data, or other people's. With an AI app it goes to that app and, under its maker's terms, maybe to the company behind it; with RAI it goes to the AI provider you chose. RapidR sends nothing itself, keeps no picture (it is deleted as soon as it is handed over) and writes only that a picture was asked for into the activity list, never the picture, never what was typed. Use test data, and close what you don't want seen. A program under test may send real email, make real purchases or delete real data when it is clicked: test with test data.

While the program is **paused in the debugger**, an AI can still look at
it (the picture and the layout are of the windows as they are), but not
use it: it is told where the program is paused and to continue it first.

They work the same in the web Studio, where your program runs in the page.

## Where it works

| | Desktop (interpreted and native) | Web |
|---|---|---|
| `RWindowCapture` | yes | yes (`SaveToFile` writes into the page's files: RapidR Studio keeps them in your project) |
| `RAppHands` | yes | yes |
| Studio's `app_*` tools | yes (AI apps and RAI) | RAI's (AI apps reach only the desktop Studio: a web page can't listen for them) |
