# Offer your app's tools to AI agents

`RAgentTools` lets your web app offer tools to AI agents that run in the
browser, such as a browser's own assistant or an extension. It works with
browsers that implement the WebMCP draft, a Draft Community Group Report of
the W3C Web Machine Learning Community Group. It is a draft: it is not a W3C
Standard, it can change, and today only some browsers offer it (see the
table). Where a browser does not offer it, `RAgentTools.Available` is 0 and
your program keeps running.

<!-- legal -->
WebMCP is the name of a draft specification of the W3C Web Machine Learning
Community Group; RapidR uses the name only to say which draft its browser
feature follows. WebMCP is a different specification from the Model Context
Protocol; Model Context Protocol® and MCP™ are marks of LF Projects, LLC
(https://www.lfprojects.org/policies/), and RapidR uses them only to say
which protocol it speaks. W3C and browser names belong to their owners
(https://www.w3.org/policies/). RapidR is independent of all of them and is
not made, sponsored or endorsed by any of them. No text, interface
description or code of the draft is included in RapidR.

## A tool in four lines

```rapidr
DIM Agent AS RAgentTools
SUB ToolCalled (Tool AS STRING, Arguments AS STRING, CallId AS INTEGER)
    IF Tool = "add" THEN Agent.Reply CallId, STR$(VAL(Agent.Argument("A")) + VAL(Agent.Argument("B")))
END SUB
Agent.OnToolCall = ToolCalled
Agent.AddTool "add", "Adds two numbers.", "A AS DOUBLE, B AS DOUBLE", "read"
IF Agent.Start = 0 THEN PRINT Agent.Error
```

- **AddTool** offers a tool: its name, one sentence the AI reads to decide
  when to call it, its arguments written like a SUB's parameters, and how
  it acts: `"read"` (it only reads), `"change"`, `"delete"` or `"network"`.
  Add `untrusted` when its answer holds what other people wrote (a note, a
  message, a web page): `"read untrusted"`. The agent is told to treat that
  text as data.
- **Start** offers every tool to the browser's agents; a tool added or
  removed later follows at once. **Stop** takes them all away.
- **OnToolCall** gets each call, one at a time: the tool, its arguments as
  JSON (`Argument` reads one) and the call's number. **Reply** answers, now
  or later (keep the number). `Reply CallId, "what went wrong", 1` marks a
  failure: say what to do instead, the agent acts on it.
- A wrong call (an argument missing, a word where a number goes) never
  reaches your program: the agent is told what the tool needs.

The example [`examples/web/agent_tools.rr`](../../examples/web/agent_tools.rr)
is a list of notes an agent can read and add to.

## Safe by default

- **Changes ask first.** With `ConfirmChanges` True (the default), a call of
  a tool that isn't `"read"` shows the browser's own question to the person
  using your app before `OnToolCall` sees it. If they say no, the agent is
  told so. Set it False only if your program asks itself (`MESSAGEDLG`).
- **Nothing of your windows is shown** unless you say so. `Expose = "read"`
  adds two tools the agents can use on what your windows show:
  `app_components` (each component's name, type, what it shows, whether it
  shows and is enabled) and `app_get` (one property). `Expose = "write"` adds
  `app_set` (one property set as your program would, asked first). Never a
  component that isn't on a window, never a property that holds a
  credential.
- **Answers are checked**: a key-shaped word (`sk-…`) is never sent.
- **Short answers**: an answer longer than `MaxResultSize` characters is cut,
  saying how much was left out. In a browser it is 1500 unless you set it:
  the browsers' makers advise short answers (an agent may refuse a long
  one).
- **This page only**: the tools are offered to agents of your page's own
  origin, never to other sites, and only on a secure page (HTTPS, or your
  own computer while you test).
- **The browser says nothing of who calls**: `ClientName` is
  `"browser agent"`. Offer only what any agent in the user's browser may
  use. What your program offers, and the privacy of its users' data, are
  your responsibility.

## Which browsers offer it (2026-11-07)

| Browser | Today |
|---|---|
| Chrome | A trial (versions 149 to 156) for sites that register for it; on your own computer, turn on `chrome://flags/#enable-webmcp-testing`. |
| Edge | A trial from version 150. |
| Firefox | Not offered. |
| Safari | Not offered. |

Everywhere else `Available` is 0, `Start` returns False and `Error` says why
in plain words; your app works as before. Write the line your users see the
same way: say what they can still do.

## On the desktop

The same program runs on the desktop too: there `RAgentTools` serves AI apps
on this computer over MCP, as `RMcpServer` does. `Start` makes a token when
`Token` is empty; `Url` and `Token` are what the person types into their AI
app. `ConfirmChanges` can't ask from there: a change tool's call is refused
while it is True, so set it False and ask in `OnToolCall` (`MESSAGEDLG`).

## See also

- [Let AI apps work in Studio](ai-apps.md): RapidR Studio offers its own
  tools the same way, on the desktop and in a browser.
- [Network](network.md): `RMcpServer`, for desktop programs that serve AI
  apps.
