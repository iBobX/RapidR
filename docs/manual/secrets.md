# Keeping secrets: RSecrets

`RSecrets` keeps passwords, tokens and keys in the operating system's
keychain, never in the program, a file or its settings:

- the macOS Keychain;
- Windows Credential Manager;
- the Secret Service on Linux (GNOME Keyring or KWallet).

RapidR Studio keeps its AI keys the same way, through the same code.

![The secrets example after a password was saved and read back](images/secrets/secrets-read-back.png)

The example is [examples/data/secrets.rr](../../examples/data/secrets.rr):
`rapidr run data/secrets.rr` from the examples folder.

## Saving, reading, deleting

A secret has a service and a name, both your own words. The service says
what it is for (`"MySQL"`), and the name which one (`"office"`).

```basic
DIM Secrets AS RSecrets

IF Secrets.Save("MySQL", "office", PasswordEdit.Text) THEN
    PasswordEdit.Text = ""
END IF

pw$ = Secrets.Read("MySQL", "office")       ' "" when nothing is kept
IF Secrets.Exists("MySQL", "office") THEN Secrets.Delete "MySQL", "office"
```

| Member | What it does |
|---|---|
| `Save(Service, Name, Value)` | Keeps `Value`, replacing what was kept there. True when it was kept. |
| `Read(Service, Name)` | What is kept, or `""` when nothing is (`Exists` tells them apart). |
| `Delete(Service, Name)` | Removes it. True when there was one. |
| `Exists(Service, Name)` | True when something is kept. |
| `Error` | Why the last call failed, `""` when it worked. It never contains the value. |
| `Available` | True where secrets can be kept. |
| `Store` | Where they are kept, in words: "the macOS Keychain" … |
| `Program` | The name this program's secrets are kept under. |

A value is at most 1,024 characters, and an empty one is refused (use
`Delete`). A service or a name can't be empty or hold a line break.

## Each program has its own

A program's secrets are kept under its own name: service `MySQL` of
`dbstudio.rr` is the keychain item `rapidr-app:dbstudio:MySQL`. The name
is the program's file name in lower case, so `dbstudio.rr`, `DBStudio.exe`
and `DBStudio.app` share theirs. No program reaches another program's
secrets, or RapidR Studio's keys, through RSecrets.

The operating system may ask the user once whether a program can use the
keychain, for example the first time a newly built app reads an item that
an older build saved.

## Never shown

What `Read` returns and what `Save` is given are credentials:

- **The debugger** shows `••••` for a variable, a watch or a data tip that
  holds one, and a log message (a logpoint) masks it. A component's secret
  properties, such as `RMySQL.Password` and `RAI.APIKey`, show `••••` too.
- **An AI is never given it.** The language registry marks `Read` and
  `Save` as `secret`, and what RapidR gives an AI about a program (the
  debugger's values, a component's secret properties) shows `••••`
  instead. Text your program prints is its own, so don't print secrets.
- **RapidR's own messages** never print it. `Error` never contains it.

Your program can still use the value, of course: a `PRINT pw$` prints it,
as in any BASIC.

## In a browser

A web page has no keychain, and RapidR never keeps secrets in the page's
storage, where any script on the page could read them. On the web
`Available` is False, and every call fails with "RSecrets isn't available in
the browser" in `Error`. Ask the user for the password each time instead.

## Tests

`RAPIDR_AI_KEYSTORE=memory` (or `RAPIDR_KEYSTORE=memory`) keeps everything
in the program's memory for that run, and the keychain is never touched.
RapidR's own tests always run this way. `Store` then says "memory, for this
run only (the test keystore)".
