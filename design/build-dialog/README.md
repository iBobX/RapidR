# The Build dialog: design for review (plan B4, phase 1)

Studio's **Run > Build** opens one dialog, the same in the desktop and the web Studio: where the app should run, the app's details, what goes in, then Build, with a progress list per app and the zips. Spec: [docs/build-for-every-platform-plan.md](../../docs/build-for-every-platform-plan.md) section 2. Made to [docs/design-guidelines.md](../../docs/design-guidelines.md). No code yet: code starts after Robert's OK.

- **Design file:** `build-dialog.pen` (started as a copy of `design/rapidr-brand.pen`: its palette and the brand frame are kept). All colours are variables with three looks (`mode`: light, dark, hc), set from [docs/theme-tokens.md](../../docs/theme-tokens.md) and the icon palette.
- **Pictures:** every frame as `NAME-LANG-LOOK@2x.png` here (81 files; LANG `en`/`es`, LOOK `light`/`dark`/`hc`). Start with `00-index@2x.png`.
- **Icons:** RapidR's own (`design/icons`), drawn from their SVG sources. Four new ones are proposed in `icons-proposed/` (24-px masters, icon-guide strokes and tokens): Windows, macOS, Linux and a phone.

| Frame | What it shows |
|---|---|
| 01 Where it runs | Target cards (Windows x64/ARM64/32-bit, macOS Universal, Linux x64/ARM64, Web), All Desktop / Everything, a plain-words line, Android/iOS/boards greyed "Coming later" |
| 02 Your app | Name, version, author or company, copyright, description, website, app id, category, kind; the icon from one picture with 16 to 48 shown at real size |
| 02b Your app, needs care | A small picture (a soft warning) and an app id with a space (the field's error, said in words) |
| 03 What goes in | The program (compiled once), the data files ticked, the files left out with the reason, each app's size |
| 04 Ready to build | One row per app with its zip's name and size, Save in, Pack as, the runners that download first |
| 05 Building | One row per app: its stages (runner checked, packed, zipped) and its own progress bar |
| 06 Done | The zips, then how to open the app the first time on each system |
| 07 Error: a runner refused | The other apps built; macOS refused with what happened and Download Again |
| 08 Error: offline, no runners | Two apps built; two need their runner; what to do (and the classroom tip) |
| 09a / 09b Narrow | Under 640 px: one scrolling page, footer pinned (as seen, and the whole length) |
| 10a / 10b Web Studio | The same dialog in the browser: "Nothing leaves your computer", the browser saves the zips |
| 11 Keyboard and screen reader | Tab order on step 1, every control's name and role, the keys, what is announced |
| 00 Components | Buttons, check boxes, fields, chips, notes, step rows, a card, progress, the icons; light, dark, high contrast |

Frames 01 to 08 exist in English and Spanish, each in light, dark and high contrast; 09 and 10 too; 11 in English light.

## Decisions

1. **Steps on the left, any step any time.** Four steps (research: three to five is the range that stays clear). Every step has a working default, so the list is also navigation: click a step, or Ctrl+Tab. Each step shows a one-line summary of its choice, so the list doubles as the review. The footer has **Back** and **Next**; on step 4 the primary is **Build 4 Apps** (it counts, so you know what will happen).
2. **One primary, quiet everything else.** Dialog buttons 32 px with mnemonics; quick choices are ghost buttons; row actions are compact (24 px). Never more than two visible actions; the rest is in the **More** (dots) menu: Copy Command Line, Save All in One Zip, Help.
3. **No Cancel button.** The choices are saved in the project as you go ("Saved in the project as you go" says so at the foot of the steps). The window's close X and Esc close it; nothing is lost. While building, Esc stops, and nothing half-made is kept.
4. **The default is this computer's system** the first time (proposed), so a first click on Build always works offline; the frames show a richer choice.
5. **Plain words on the card, the jargon in brackets.** "64-bit (x64): most PCs". A help line explains x64 and ARM64 once. 32-bit says what it costs ("the fast interpreter, no JIT").
6. **Sizes and downloads are said before you build.** Step 3 gives each app's size and that most of it is the runner; step 4 says which runners download first, how big, from where, and that each is checked against RapidR's signature.
7. **Progress per app, one message per milestone.** Each app has its own row, stages and bar; a screen reader hears one polite message per milestone, never every percent (research). Errors stay in the row they belong to.
8. **Errors say what happened, what is fine, and the next step.** A refused runner: it was deleted and not used, the other apps are fine, Download Again. Offline: which runners are missing, how big, Try Again, and the classroom tip (build once online, the runners stay).
9. **Teach the first launch.** Done shows how to open the app the first time on each system (the same text is in every zip's README.txt): first-time users think a warned app is broken.
10. **Web Studio, same dialog.** No folder field; the browser saves the zips. A soft success note says nothing leaves the computer; Done offers Save All in One Zip for browsers that ask about several files.
11. **Narrow is one page.** Under 640 px the steps become numbered sections down one scrolling page, labels go above fields, one pinned Build button.
12. **Pictures, not logos.** The system pictures are our own drawings of a window as each system draws it (controls at the right, three dots at the left, a prompt); no system logos, which are trademarks. The system's name is always beside the picture.
13. **Colour never alone.** Every state has a word and an icon; chips and notes use the soft status colours; high contrast frames every control (cyan selection, yellow focus 3 px).

## Research (one hour)

Public design-system guidance and developers' reports on multi-target export dialogs, summarised. Wizards help for staged input, and hurt when people want to jump around: so the step list is also navigation. Three to five steps; fewer is a plain dialog, more is too long. Confusion comes from target-versus-host mix-ups (which machine is it built for?), from architecture words with no explanation, from labels that offer choices that don't really work, and from silent failures per target. Progress for several parallel jobs: one live summary, milestones only, each bar with a stable name. First launches on another system are the moment most people give up: say how, in the place they look.

## Open questions for Robert

1. **System pictures:** neutral window pictures instead of logos (no trademark risk). OK? They also need the lawyer agent's look as new assets, and a place in `design/icons`.
2. **First-time default:** only this computer's system ticked, or all desktop?
3. **Saved as you go, no Cancel:** fine, or should choices only be saved when you build?
4. **Done's primary:** "Show in Finder" (File Explorer on Windows, Files on Linux), or close?
5. **The 32-bit line:** keep "the fast interpreter, no JIT", or plainer for kids ("runs a little slower")?
6. **Offline classrooms:** add a "runner pack" teachers can copy to every computer (B3)? Today's answer is "build once online on each computer".
7. **Web Studio address:** the browser bar in 10a/10b shows a placeholder; where will the web Studio live?
8. **Spanish:** "ejecutor" for runner (as the Spanish manual says), "computadora" (as Studio's Spanish says). The macOS button "Abrir igualmente" must be checked on a Mac in Spanish.
9. **Native code:** folded under Advanced in step 3, desktop only. Right place?
10. **Project Options:** step 2 and Project Options' app fields should be one form (one place to keep them). Agree?

## For the B4 code lane

- Sizes: dialog 860 x 600 (fits 1280 x 720 at 85 %), rail 204, padding 20, groups 16 apart, fields 24, dialog buttons 32 (at least 96 wide), compact 24, rows 22 to 32. Fonts: UIFont 13, DimFont 12, SubFont 15 bold for the step heading. Tokens: `face`, `window`, `card`, `border`, `accent`, `page.dim`, `page.link`, `status.*`; the selected card is `status.info` with an `accent` edge.
- Check the classic look too (not drawn here), and run the guide's checklist; captures next to these frames.
- Build Again (Ctrl+Shift+B) shows no dialog: progress in the status bar and a toast "4 apps built" with **Show**; a failure is a toast that stays until closed, with **Open Build**.
- The AI facet (`build_app`) takes the same choices; Copy Command Line gives the CLI form.
