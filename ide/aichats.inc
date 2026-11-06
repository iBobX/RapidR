' aichats.inc: the assistant's chats, kept (docs/preview4-plan.md R1, R2).
'
' Each project's chats are kept in Studio's data folder — never the
' project's, so a chat is never shared or committed with it: AITools'
' ChatsFolder says where (rapidr_ai::chats; keys and tokens removed before
' anything is written). A chat is kept after every answer, a decision on
' its changes, Compact, an edit; opening a project shows its newest chat
' again, so closing Studio loses nothing. Chats lists them: open one,
' rename it, delete it (Undo puts it back), search their titles and text.
' New Chat keeps the one it replaces.
'
' Editing a sent question (its Edit link) or Retry under the last answer:
' what followed the question moves to a saved copy of the chat ("… (before
' the edit)"), nothing lost; then it is asked again.

DECLARE SUB AIChatsBack
DECLARE SUB AIChatsOpenClick
DECLARE SUB AIChatsRenameClick
DECLARE SUB AIChatsDeleteClick
DECLARE SUB AIChatListClick
DECLARE SUB AIChatListKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB AIChatRowAction(Index AS INTEGER, Action AS STRING, Sender AS RListBox)
DECLARE SUB AIChatsStartClick
DECLARE SUB AIChatsUndoClick
DECLARE SUB AIChatsSearchChanged
DECLARE SUB AIChatsSearchKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB AIChatEdited(Index AS INTEGER, S AS STRING)
DECLARE FUNCTION AIRenameChatTo(Id AS STRING, Title AS STRING) AS INTEGER
DECLARE SUB AIChatsCopyMarkdown
DECLARE SUB AIFillChats
DECLARE SUB AIOpenChat(Id AS STRING, Title AS STRING)
DECLARE FUNCTION AIChatPicked AS STRING
DECLARE SUB AISelectChat(Id AS STRING)
DECLARE SUB AIShowEditText
DECLARE SUB AiChatsLanguageChanged

SUB SetUpAIChats
  AIChatProject = CHR$(1)
  AIChatsPanel.Parent = AIPanel
  AIChatsPanel.Align = 5
  AIChatsPanel.BevelOuter = 0
  AIChatsPanel.Visible = 0
  AIChatsPanel.AccessibleName = TR$("Chats")
  ' (the search box on top)
  AIChatsHead.Parent = AIChatsPanel
  AIChatsHead.Align = 1
  AIChatsHead.Height = 40
  AIChatsHead.BevelOuter = 0
  AIChatSearchLabel.Parent = AIChatsHead
  AIChatSearchLabel.Caption = TR$("Search:")
  AIChatSearchLabel.AutoSize = 0
  AIChatSearch.Parent = AIChatsHead
  AIChatSearch.AccessibleName = TR$("Search chats")
  AIChatSearch.AccessibleDescription = TR$("Their titles and what was said; Down goes to the list, Escape back to the chat")
  AIChatSearch.Hint = TR$("Search chats")
  AIChatSearch.OnChange = AIChatsSearchChanged
  AIChatSearch.OnKeyDown = AIChatsSearchKey
  ' (where chats are kept, at the bottom — Back is the Chats button's, at
  ' the top —; Undo after a delete; the new name while renaming. Open,
  ' Rename and Delete are on the rows themselves and in their right-click
  ' menu: docs/context-menus-design.md §3)
  AIChatsFoot.Parent = AIChatsPanel
  AIChatsFoot.Align = 2
  AIChatsFoot.Height = 52
  AIChatsFoot.BevelOuter = 0
  AIChatUndoBar.Parent = AIChatsPanel
  AIChatUndoBar.Align = 2
  AIChatUndoBar.Height = 36
  AIChatUndoBar.BevelOuter = 0
  AIChatUndoBar.Visible = 0
  AIChatUndoBar.AccessibleName = TR$("Chat deleted")
  AIChatUndoText.Parent = AIChatUndoBar
  AIChatUndoText.AutoSize = 0: AIChatUndoText.WordWrap = 0
  AIChatUndoBtn.Parent = AIChatUndoBar
  AIChatUndoBtn.Caption = TR$("Undo")
  AIChatUndoBtn.AccessibleName = TR$("Undo: put the chat back")
  AIChatUndoBtn.OnClick = AIChatsUndoClick
  ' (where they are kept, said under the list: personal data, plainly)
  AIChatsNote.Parent = AIChatsFoot
  AIChatsNote.AutoSize = 0: AIChatsNote.WordWrap = 1
  AIChatsNote.AccessibleName = TR$("Where chats are kept")
  AIChatList.Parent = AIChatsPanel
  AIChatList.Align = 5
  AIChatList.AccessibleName = TR$("This project's chats, the newest first")
  ' (a click opens a chat; the arrows only move; Enter opens. Rename,
  ' Delete and More on the row under the mouse and the selected one, F2
  ' and Del too; its menu by right click, Shift+F10 or More)
  AIChatList.OnClick = AIChatListClick
  AIChatList.OnKeyDown = AIChatListKey
  AIChatList.OnRowAction = AIChatRowAction
  ' (Rename: the title edited in the row itself; Enter keeps, Esc leaves it)
  AIChatList.OnEdited = AIChatEdited
  AIChatsEmpty.Parent = AIChatsPanel
  AIChatsEmpty.AutoSize = 0: AIChatsEmpty.WordWrap = 1
  AIChatsEmpty.Alignment = 2
  AIChatsEmpty.Visible = 0
  AIChatsStartBtn.Parent = AIChatsPanel
  AIChatsStartBtn.Caption = TR$("&Start a Chat")
  AIChatsStartBtn.Primary = 1
  AIChatsStartBtn.Visible = 0
  AIChatsStartBtn.OnClick = AIChatsStartClick
  AIShowChatTitle
END SUB

SUB LayOutAIChats
  DIM w AS INTEGER, bh AS INTEGER, bw AS INTEGER
  w = AIPanel.ClientWidth
  IF w < 200 THEN w = 200
  IF ThemeName = "classic" THEN bh = 23 ELSE bh = 28
  AIChatSearchLabel.Left = 8: AIChatSearchLabel.Top = 11: AIChatSearchLabel.Width = 56: AIChatSearchLabel.Height = 18
  AIChatSearch.Left = 66: AIChatSearch.Top = 6: AIChatSearch.Width = w - 72
  AIChatsNote.Left = 8: AIChatsNote.Top = 6: AIChatsNote.Width = w - 16: AIChatsNote.Height = 40
  ' (rows tall enough for the row actions' 24-pixel squares)
  IF ThemeName = "classic" THEN AIChatList.ItemHeight = 24 ELSE AIChatList.ItemHeight = 28
  ' (no chats yet: a line and the one big button, in the list's place)
  AIChatsEmpty.Left = 16: AIChatsEmpty.Top = 64: AIChatsEmpty.Width = w - 32: AIChatsEmpty.Height = 40
  bw = AIChatsStartBtn.Width
  IF bw < 120 THEN bw = 120
  AIChatsStartBtn.Width = bw: AIChatsStartBtn.Height = UiButtonH
  AIChatsStartBtn.Left = (w - bw) \ 2: AIChatsStartBtn.Top = 112
  AIChatUndoBtn.Left = w - 6 - 72: AIChatUndoBtn.Top = (36 - bh) \ 2: AIChatUndoBtn.Width = 72: AIChatUndoBtn.Height = bh
  AIChatUndoText.Left = 8: AIChatUndoText.Top = 9: AIChatUndoText.Width = w - 16 - 76: AIChatUndoText.Height = 18
END SUB

SUB AIChatsFonts
  AIChatSearch.Font = UIFont
  AIChatSearchLabel.Font = UIFont
  AIChatSearchLabel.Font.Color = TC("page.text")
  AIChatList.Font = UIFont
  AIChatsEmpty.Font = UIFont
  AIChatsEmpty.Font.Color = TC("page.dim")
  AIChatsStartBtn.Font = UIFont
  AIChatUndoText.Font = UIFont
  AIChatUndoText.Font.Color = TC("page.text")
  AIChatUndoBtn.Font = UIFont
  AIChatsNote.Font = DimFont
  AIChatsNote.Font.Color = TC("page.dim")
END SUB

' (the chat's title beside Chats: its name, or "New chat")
SUB AIShowChatTitle
  DIM t AS STRING, w AS INTEGER
  IF AIChatName = "" THEN t = TR$("New chat") ELSE t = AIChatName
  ' (as much as fits, then "…": the whole title in its hint and its name)
  AIChatTitle.Hint = t
  AIChatTitle.ShowHint = 1
  AIChatTitle.AccessibleName = TR$("This chat: {title}", "title", t)
  AIMeterMeasure.Font.Name = UIFont.Name: AIMeterMeasure.Font.Size = UIFont.Size: AIMeterMeasure.Font.Bold = 1
  w = AIChatTitle.Width - 4
  IF w > 20 AND AIMeterMeasure.TextWidth(t) > w THEN
    DO WHILE LEN(t) > 1 AND AIMeterMeasure.TextWidth(t + "…") > w
      t = LEFT$(t, LEN(t) - 1)
    LOOP
    t = RTRIM$(t) + "…"
  END IF
  AIMeterMeasure.Font.Bold = 0
  AIChatTitle.Caption = t
END SUB

' (the project the chats are of now: its project file, "" none)
FUNCTION AIChatsProjectNow AS STRING
  IF ProjectOpen THEN Result = ProjectFile ELSE Result = ""
END FUNCTION

' Keeps the chat as it is now (nothing until it has a question): under its
' project, its title made from its first question until the user names it.
SUB AISaveChat
  DIM r AS STRING, title AS STRING
  IF AIChatProject = CHR$(1) THEN EXIT SUB
  IF AI.QuestionCount = 0 THEN EXIT SUB
  title = AIChatName
  IF title = "" THEN title = AITools.ChatTitle(AI.QuestionText(1))
  r = AITools.SaveChat(AIChatProject, AIChatId, title, AI.Model, AI.ChatState)
  IF LEFT$(r, 6) = "error:" OR r = "" THEN
    OutputLine TR$("AI: this chat couldn't be kept: {error}", "error", MID$(r, 8))
    EXIT SUB
  END IF
  AIChatId = r
  AIChatName = title
  AIShowChatTitle
  IF AIChatsPanel.Visible THEN AIFillChats
END SUB

' The project changed (opened, closed): its newest chat shown, the last
' one kept first — unless the assistant made (or undid) the program, then
' the chat goes with it.
SUB AILoadProjectChat
  DIM cur AS STRING, prev AS STRING, list AS STRING
  cur = AIChatsProjectNow
  IF cur = AIChatProject THEN EXIT SUB
  IF AIRehoming AND AIChatProject <> CHR$(1) THEN
    prev = AIChatProject
    AIChatProject = cur
    IF AIChatId <> "" THEN
      AISaveChat
      list = AITools.DeleteChat(prev, AIChatId)
    END IF
    EXIT SUB
  END IF
  ' (the project closed, none opened: the chat stays on show — still its
  ' project's —, its cards' Open and Undo at hand)
  IF cur = "" AND AIChatProject <> CHR$(1) THEN EXIT SUB
  IF AI.Busy THEN AI.Cancel
  AISaveChat
  AIChatProject = cur
  AIEditing = 0
  AIEditBar.Visible = 0
  list = AITools.ChatList(cur)
  IF list <> "" THEN
    AIOpenChat FIELD$(FIELD$(list, CHR$(10), 1), CHR$(9), 1), FIELD$(FIELD$(list, CHR$(10), 1), CHR$(9), 2)
  ELSE
    AI.NewChat
    ' (the last project's waiting changes are its chat's, kept with it:
    ' never offered — or applied — in this project)
    AITools.ClearProposals
    AIReviewBar.Visible = 0
    AIChatId = ""
    AIChatName = ""
    AIChat.Text = AI.Transcript
    AIShowChatTitle
  END IF
  IF AIChatsPanel.Visible THEN AIFillChats
END SUB

' A kept chat in place of the one shown (that one kept first).
SUB AIOpenChat(Id AS STRING, Title AS STRING)
  DIM state AS STRING
  state = AITools.LoadChat(AIChatProject, Id)
  IF state = "" THEN
    AIStatus.SimpleText = TR$("That chat can't be read any more (removed outside Studio?).")
    EXIT SUB
  END IF
  AI.ChatState = state
  AIChatId = Id
  AIChatName = Title
  AIEditing = 0
  AIEditBar.Visible = 0
  AIChat.Text = AI.Transcript
  ' (the changes it left waiting, kept with it: still to review and apply —
  ' LoadChat gave them back to AITools)
  AIReviewBar.Visible = 0
  AIShowReviewBar
  AIShowChatTitle
  AIEnableSend
END SUB

' New Chat (and /new): this one kept, a new one started.
SUB AIStartNewChat
  DIM had AS INTEGER
  IF AI.Busy THEN AI.Cancel
  had = AI.QuestionCount
  AISaveChat
  AI.NewChat
  AIChatId = ""
  AIChatName = ""
  AIEditing = 0
  AIEditBar.Visible = 0
  AIChat.Text = AI.Transcript
  AIShowChatTitle
  AIShowChats 0
  IF had > 0 THEN
    AIStatus.SimpleText = TR$("A new chat. The last one is kept: Chats lists it.")
  ELSE
    AIStatus.SimpleText = TR$("A new chat.")
  END IF
END SUB

' ---- the list ------------------------------------------------------------------

SUB AIChatsClick
  AIShowChats Flag(AIChatsPanel.Visible = 0)
END SUB

SUB AIChatsBack
  AIShowChats 0
END SUB

' Chats shown in the conversation's place (Back, Escape, or Chats again
' return to it).
SUB AIShowChats(Show AS INTEGER)
  IF Show THEN
    ' (what is said now is in the list too)
    AISaveChat
    AIRenameBar.Visible = 0
    AIChatSearch.Text = ""
    AIChatsNote.Caption = TR$("Kept on this computer, in Studio's own folder — never in the project. Delete removes a chat for good.")
    AIChatsNote.Hint = AITools.ChatsFolder
    AIChatsNote.ShowHint = 1
    AIChatsNote.AccessibleDescription = TR$("Kept in {folder}", "folder", AITools.ChatsFolder)
    ' (the list gets the pane: the box and its buttons come back with Back)
    AIChat.Visible = 0
    AIBottom.Visible = 0
    AIChatsPanel.Visible = 1
    AIChatsBtn.Caption = TR$("Back")
    AIChatsBtn.AccessibleName = TR$("Back to the chat")
    LayOutAIChats
    AIFillChats
    AIChatSearch.SetFocus
  ELSE
    IF AIChatsPanel.Visible = 0 THEN EXIT SUB
    AIChatsPanel.Visible = 0
    AIBottom.Visible = 1
    AIChat.Visible = 1
    AIChatsBtn.Caption = TR$("Chats")
    AIChatsBtn.AccessibleName = TR$("Chats")
    AIInput.SetFocus
  END IF
END SUB

' (the list, as the search box says: the newest first, this chat marked)
' (CONTEXT-MENUS 2b) A day's number from a date's "YYYY-MM-DD" (days
' from 1970-01-01: Howard Hinnant's days-from-civil)
FUNCTION AIDayNumber(Y AS INTEGER, M AS INTEGER, D AS INTEGER) AS LONG
  DIM yy AS LONG, era AS LONG, yoe AS LONG, doy AS LONG, doe AS LONG, mp AS LONG
  yy = Y
  IF M <= 2 THEN yy = yy - 1
  IF yy >= 0 THEN era = yy \ 400 ELSE era = (yy - 399) \ 400
  yoe = yy - era * 400
  IF M > 2 THEN mp = M - 3 ELSE mp = M + 9
  doy = (153 * mp + 2) \ 5 + D - 1
  doe = yoe * 365 + yoe \ 4 - yoe \ 100 + doy
  Result = era * 146097 + doe - 719468
END FUNCTION

' (how many days ago a chat was last changed, on this computer's calendar:
' its time is UTC — ISO 8601 —, the local day is DATE$'s; the offset is
' the local clock's against Repo.Now's)
FUNCTION AIDaysAgo(When AS STRING) AS INTEGER
  DIM today AS LONG, secs AS DOUBLE, local AS DOUBLE, offs AS DOUBLE, t AS DOUBLE, dl AS STRING
  IF LEN(When) < 19 THEN
    Result = 99
    EXIT FUNCTION
  END IF
  dl = DATE$
  today = AIDayNumber(VAL(MID$(dl, 7, 4)), VAL(LEFT$(dl, 2)), VAL(MID$(dl, 4, 2)))
  local = today * 86400# + TIMER
  offs = local - Repo.Now
  ' (to the quarter hour: the two clocks are read a moment apart)
  offs = INT(offs / 900 + 0.5) * 900
  t = AIDayNumber(VAL(LEFT$(When, 4)), VAL(MID$(When, 6, 2)), VAL(MID$(When, 9, 2))) * 86400# + VAL(MID$(When, 12, 2)) * 3600 + VAL(MID$(When, 15, 2)) * 60 + VAL(MID$(When, 18, 2)) + offs
  Result = today - INT(t / 86400)
END FUNCTION

SUB AIFillChats
  DIM list AS STRING, i AS INTEGER, line AS STRING, item AS STRING, q AS INTEGER, dt AS STRING, at AS INTEGER, grp AS INTEGER, was AS INTEGER, head AS STRING, n AS INTEGER
  IF AIChatProject = CHR$(1) THEN EXIT SUB
  IF LTRIM$(AIChatSearch.Text) = "" THEN
    list = AITools.ChatList(AIChatProject)
  ELSE
    list = AITools.SearchChats(AIChatProject, AIChatSearch.Text)
  END IF
  AIChatList.Clear
  AIChatIds = ""
  AIChatTitles = ""
  at = -1
  was = -1
  IF list <> "" THEN
    FOR i = 1 TO TALLY(list, CHR$(10)) + 1
      line = FIELD$(list, CHR$(10), i)
      IF line <> "" THEN
        ' (a heading row before each day's group: Today, Yesterday,
        ' Earlier — not chosen by a click or the keys)
        grp = AIDaysAgo(FIELD$(line, CHR$(9), 3))
        IF grp > 2 THEN grp = 2
        IF grp < 0 THEN grp = 0
        IF grp <> was THEN
          SELECT CASE grp
            CASE 0: head = TR$("Today")
            CASE 1: head = TR$("Yesterday")
            CASE ELSE: head = TR$("Earlier")
          END SELECT
          AIChatList.AddItems head
          AIChatList.Heading AIChatList.ItemCount - 1, 1
          AIChatIds = AIChatIds + CHR$(10)
          AIChatTitles = AIChatTitles + CHR$(10)
          was = grp
        END IF
        n = n + 1
        q = VAL(FIELD$(line, CHR$(9), 4))
        dt = MID$(FIELD$(line, CHR$(9), 3), 1, 10)
        item = FIELD$(line, CHR$(9), 2)
        item = TRN$("{title} — {n} question, {date}", "{title} — {n} questions, {date}", q, "title", item, "date", dt)
        IF FIELD$(line, CHR$(9), 5) <> "" THEN item = TR$("{item}: {q}{snippet}{q}", "item", item, "q", CHR$(34), "snippet", FIELD$(line, CHR$(9), 5))
        IF FIELD$(line, CHR$(9), 1) = AIChatId THEN
          item = TR$("{item} (open)", "item", item)
          at = AIChatList.ItemCount
        END IF
        AIChatList.AddItems item
        AIChatIds = AIChatIds + FIELD$(line, CHR$(9), 1) + CHR$(10)
        AIChatTitles = AIChatTitles + FIELD$(line, CHR$(9), 2) + CHR$(10)
      END IF
    NEXT
  END IF
  IF AIChatList.ItemCount > 0 THEN
    ' (the first row is a heading: the first chat under it)
    IF at >= 0 THEN AIChatList.ItemIndex = at ELSE AIChatList.ItemIndex = 1
  END IF
  ' (the rows' actions, in Studio's language — rename is F2, delete Del)
  AIChatList.RowActions = "rename:" + TR$("Rename (F2)") + ",delete:" + TR$("Delete (Del)") + ",more:" + TR$("More actions (Shift+F10)")
  ' (no chats yet: the one big button left; a search finding nothing says so in the list)
  AIChatsEmpty.Visible = Flag(AIChatList.ItemCount = 0 AND LTRIM$(AIChatSearch.Text) = "")
  AIChatsStartBtn.Visible = AIChatsEmpty.Visible
  AIChatList.Visible = Flag(AIChatsEmpty.Visible = 0)
  AIChatsEmpty.Caption = TR$("No chats in this project yet.")
  IF AIChatList.ItemCount = 0 THEN
    IF LTRIM$(AIChatSearch.Text) = "" THEN
      AIChatList.AccessibleDescription = TR$("No chats kept for this project yet.")
    ELSE
      AIChatList.AccessibleDescription = TR$("No chat has {q}{text}{q}.", "q", CHR$(34), "text", AIChatSearch.Text)
    END IF
  ELSE
    AIChatList.AccessibleDescription = TRN$("{n} chat", "{n} chats", n)
  END IF
END SUB

' (the chat chosen in the list: its id; "" none)
FUNCTION AIChatPicked AS STRING
  Result = ""
  IF AIChatList.ItemIndex < 0 THEN EXIT FUNCTION
  Result = FIELD$(AIChatIds, CHR$(10), AIChatList.ItemIndex + 1)
END FUNCTION

SUB AIChatsSearchChanged
  AIFillChats
END SUB

' (Down: to the list; Enter: open the first; Escape: back to the chat)
SUB AIChatsSearchKey(Key AS INTEGER, Shift AS INTEGER)
  IF Key = 40 THEN
    Key = 0
    AIChatList.SetFocus
  ELSEIF Key = 13 THEN
    Key = 0
    AIChatsOpenClick
  ELSEIF Key = 27 THEN
    Key = 0
    AIShowChats 0
  END IF
END SUB

' (CONTEXT-MENUS) A click opens the chat clicked; the arrows only move.
SUB AIChatListClick
  IF AIChatKeyed THEN
    AIChatKeyed = 0
    EXIT SUB
  END IF
  AIChatsOpenClick
END SUB

SUB AIChatListKey(Key AS INTEGER, Shift AS INTEGER)
  SELECT CASE Key
    CASE 33, 34, 35, 36, 38, 40: AIChatKeyed = 1
    CASE 13
      Key = 0
      AIChatsOpenClick
    CASE 27
      Key = 0
      AIShowChats 0
  END SELECT
END SUB

' (a row's Rename or Delete — its icon, F2, Del)
SUB AIChatRowAction(Index AS INTEGER, Action AS STRING, Sender AS RListBox)
  AIChatKeyed = 0
  Sender.ItemIndex = Index
  IF Action = "rename" THEN AIChatsRenameClick
  IF Action = "delete" THEN AIChatsDeleteClick
END SUB

' (no chats yet: Start a Chat — the chat box, empty, ready to type)
SUB AIChatsStartClick
  AIShowChats 0
  AIInput.SetFocus
END SUB

SUB AIChatsOpenClick
  DIM id AS STRING, title AS STRING
  id = AIChatPicked
  IF id = "" THEN EXIT SUB
  title = FIELD$(AIChatTitles, CHR$(10), AIChatList.ItemIndex + 1)
  IF id <> AIChatId THEN
    IF AI.Busy THEN AI.Cancel
    AISaveChat
    AIOpenChat id, title
    AIStatus.SimpleText = TR$("Opened {q}{title}{q}.", "q", CHR$(34), "title", title)
  END IF
  AIShowChats 0
END SUB

' (CONTEXT-MENUS 2b) Rename: the chat's title edited in its row (Enter
' keeps it, Esc leaves it as it was)
SUB AIChatsRenameClick
  DIM id AS STRING
  id = AIChatPicked
  IF id = "" THEN EXIT SUB
  AIRenaming = id
  AIChatUndoBar.Visible = 0
  AIChatList.SetFocus
  AIChatList.EditItem AIChatList.ItemIndex, FIELD$(AIChatTitles, CHR$(10), AIChatList.ItemIndex + 1)
END SUB

' (the title kept: renamed, the list made again; S, what the row shows
' now, is the row made for it)
SUB AIChatEdited(Index AS INTEGER, S AS STRING)
  DIM id AS STRING
  id = AIRenaming
  AIRenaming = ""
  IF id = "" THEN id = FIELD$(AIChatIds, CHR$(10), Index + 1)
  IF AIRenameChatTo(id, S) = 0 THEN
    S = AIChatList.Item(Index)
    EXIT SUB
  END IF
  AIFillChats
  AISelectChat id
  S = AIChatList.Item(Index)
END SUB

' (chat Id given a new title; 1 when it was renamed)
FUNCTION AIRenameChatTo(Id AS STRING, Title AS STRING) AS INTEGER
  DIM t AS STRING
  t = LTRIM$(RTRIM$(Title))
  IF Id = "" OR t = "" THEN EXIT FUNCTION
  IF AITools.RenameChat(AIChatProject, Id, t) THEN
    IF Id = AIChatId THEN
      AIChatName = t
      AIShowChatTitle
    END IF
    AIStatus.SimpleText = TR$("Renamed: {q}{title}{q}.", "q", CHR$(34), "title", t)
    Result = 1
  ELSE
    AIStatus.SimpleText = TR$("That chat couldn't be renamed.")
  END IF
END FUNCTION

' Copy as Markdown: the chat — its title, its questions and answers — on
' the clipboard, as Markdown.
SUB AIChatsCopyMarkdown
  DIM id AS STRING, t AS STRING
  id = AIChatPicked
  IF id = "" THEN EXIT SUB
  IF id = AIChatId THEN AISaveChat
  t = AITools.ChatMarkdown(AIChatProject, id, FIELD$(AIChatTitles, CHR$(10), AIChatList.ItemIndex + 1), TR$("You"), "RAI")
  IF t = "" THEN
    AIStatus.SimpleText = TR$("That chat couldn't be read.")
    EXIT SUB
  END IF
  Clipboard.Text = t
  AIStatus.SimpleText = TR$("Copied the chat as Markdown.")
  SetState RunState, TR$("Copied the chat as Markdown")
END SUB

' (chat Id chosen in the list, when it shows it)
SUB AISelectChat(Id AS STRING)
  DIM i AS INTEGER
  FOR i = 1 TO TALLY(AIChatIds, CHR$(10))
    IF FIELD$(AIChatIds, CHR$(10), i) = Id THEN AIChatList.ItemIndex = i - 1
  NEXT
END SUB

' Delete: the chat removed from this computer at once; Undo (beside the
' list, until another is deleted or Studio closes) puts it back.
SUB AIChatsDeleteClick
  DIM id AS STRING, title AS STRING, kept AS STRING
  id = AIChatPicked
  IF id = "" THEN EXIT SUB
  title = FIELD$(AIChatTitles, CHR$(10), AIChatList.ItemIndex + 1)
  IF id = AIChatId AND AI.Busy THEN AI.Cancel
  kept = AITools.DeleteChat(AIChatProject, id)
  IF kept = "" THEN
    AIStatus.SimpleText = TR$("That chat couldn't be deleted.")
    EXIT SUB
  END IF
  AIChatDeleted = kept
  AIChatDeletedProject = AIChatProject
  AIChatDeletedTitle = title
  ' (the one shown: a new chat in its place)
  IF id = AIChatId THEN
    AI.NewChat
    AIChatId = ""
    AIChatName = ""
    AIChat.Text = AI.Transcript
    AIShowChatTitle
  END IF
  AIChatUndoText.Caption = TR$("Deleted {q}{title}{q}.", "q", CHR$(34), "title", title)
  AIChatUndoBar.Visible = 1
  AIStatus.SimpleText = TR$("Deleted {q}{title}{q}. Undo puts it back.", "q", CHR$(34), "title", title)
  AIFillChats
  IF AIChatList.ItemCount > 0 THEN AIChatList.SetFocus ELSE AIChatUndoBtn.SetFocus
END SUB

SUB AIChatsUndoClick
  DIM id AS STRING
  IF AIChatDeleted = "" THEN EXIT SUB
  id = AITools.RestoreChat(AIChatDeletedProject, AIChatDeleted)
  AIChatUndoBar.Visible = 0
  IF id = "" THEN
    AIStatus.SimpleText = TR$("The chat couldn't be put back.")
    EXIT SUB
  END IF
  AIChatDeleted = ""
  AIStatus.SimpleText = TR$("Put back: {q}{title}{q}.", "q", CHR$(34), "title", AIChatDeletedTitle)
  ' (nothing said in the chat shown: the one put back is opened)
  IF AI.QuestionCount = 0 AND AIChatDeletedProject = AIChatProject THEN AIOpenChat id, AIChatDeletedTitle
  AIFillChats
  AIChatList.SetFocus
END SUB

' ---- edit a sent question, Retry (R2) --------------------------------------------------

' The Edit link under question Q: its text in the box, the bar saying what
' Send will do — the replies below it move to a saved copy.
SUB AIEditQuestion(Q AS INTEGER)
  IF AI.Busy THEN
    AIStatus.SimpleText = TR$("RAI is answering: wait for it, or Stop, then edit a question.")
    EXIT SUB
  END IF
  IF Q < 1 OR Q > AI.QuestionCount THEN EXIT SUB
  AIShowChats 0
  AIEditing = Q
  AIInput.Text = AI.QuestionText(Q)
  AIShowEditText
  AIEditBar.Visible = 1
  AIShowSlash
  AIInput.SetFocus
  AIStatus.SimpleText = TR$("Editing a question: Send replaces it, Escape cancels.")
END SUB

' What the bar over the box says while a question is edited: what Send
' will do, by how many questions follow the one edited.
SUB AIShowEditText
  DIM after AS INTEGER
  after = AI.QuestionCount - AIEditing
  IF after <= 0 THEN
    AIEditText.Caption = TR$("Editing your question. Send replaces it: its answer moves to a saved copy of this chat (in Chats).")
  ELSE
    AIEditText.Caption = TRN$("Editing your question. Send replaces it: its answer and the {n} question after it move to a saved copy of this chat (in Chats).", "Editing your question. Send replaces it: its answer and the {n} questions after it move to a saved copy of this chat (in Chats).", after)
  END IF
  AIEditBar.AccessibleDescription = AIEditText.Caption
END SUB

' (the chat kept as it is, under a title that says why: nothing lost —
' AfterEdit: it is the chat before an edit, else before Retry)
FUNCTION AIKeepCopy(AfterEdit AS INTEGER) AS STRING
  DIM title AS STRING
  title = AIChatName
  IF title = "" THEN title = AITools.ChatTitle(AI.QuestionText(1))
  IF AfterEdit THEN
    title = TR$("{title} (before the edit)", "title", title)
  ELSE
    title = TR$("{title} (before Retry)", "title", title)
  END IF
  Result = AITools.SaveChat(AIChatProject, "", title, AI.Model, AI.ChatState)
  IF LEFT$(Result, 6) = "error:" THEN
    OutputLine TR$("AI: the copy couldn't be kept: {error}", "error", MID$(Result, 8))
    Result = ""
  ELSE
    Result = title
  END IF
END FUNCTION

' Send while a question is edited: the chat copied, the question and what
' followed taken out, the new text asked. False when it couldn't (said).
FUNCTION AISendEdited(T AS STRING) AS INTEGER
  DIM kept AS STRING, asked AS STRING
  Result = 0
  IF AI.Busy THEN
    AIStatus.SimpleText = TR$("RAI is answering: wait for it, or Stop, then send the edited question.")
    EXIT FUNCTION
  END IF
  kept = AIKeepCopy(1)
  IF kept = "" THEN
    AIStatus.SimpleText = TR$("The question wasn't changed: the chat couldn't be copied first.")
    EXIT FUNCTION
  END IF
  asked = AI.Rewind(AIEditing)
  IF asked = "" THEN
    AIStatus.SimpleText = AI.Error
    EXIT FUNCTION
  END IF
  AIEditing = 0
  AIEditBar.Visible = 0
  AIChat.Text = AI.Transcript
  OutputLine TR$("AI: the chat before the edit is kept as {q}{title}{q} (Chats).", "q", CHR$(34), "title", kept)
  AISendNow T, 1
  Result = 1
END FUNCTION

' Retry, under the last answer: the chat copied, the last question asked
' again.
SUB AIRetry
  DIM q AS INTEGER, t AS STRING, kept AS STRING
  IF AI.Busy THEN EXIT SUB
  q = AI.QuestionCount
  IF q = 0 THEN EXIT SUB
  t = AI.QuestionText(q)
  kept = AIKeepCopy(0)
  IF kept = "" THEN
    AIStatus.SimpleText = TR$("Not retried: the chat couldn't be copied first.")
    EXIT SUB
  END IF
  IF AI.Rewind(q) = "" THEN
    AIStatus.SimpleText = AI.Error
    EXIT SUB
  END IF
  AIChat.Text = AI.Transcript
  OutputLine TR$("AI: the answer before Retry is kept in {q}{title}{q} (Chats).", "q", CHR$(34), "title", kept)
  AISendNow t, 0
END SUB

' (tests and the manual's scenes: ai:chats, ai:chatopen=N, ai:chatrename=N|name,
' ai:chatdelete=N, ai:chatundo, ai:chatsearch=text, ai:edit=N|text,
' ai:retry, ai:thought=N; True when Verb was one)
' (the list's row of the Nth chat, from 1: its headings passed over)
FUNCTION AIChatRow(N AS INTEGER) AS INTEGER
  DIM i AS INTEGER, k AS INTEGER
  Result = -1
  FOR i = 1 TO TALLY(AIChatIds, CHR$(10))
    IF FIELD$(AIChatIds, CHR$(10), i) <> "" THEN
      k = k + 1
      IF k = N THEN
        Result = i - 1
        EXIT FUNCTION
      END IF
    END IF
  NEXT
END FUNCTION

FUNCTION AIChatStep(Verb AS STRING, Arg AS STRING) AS INTEGER
  DIM id AS STRING
  Result = 1
  SELECT CASE Verb
    CASE "chats": AIShowChats 1
    CASE "chatsback": AIShowChats 0
    CASE "chatsearch"
      AIShowChats 1
      AIChatSearch.Text = Arg
      AIFillChats
    CASE "chatopen"
      AIShowChats 1
      AIChatList.ItemIndex = AIChatRow(VAL(Arg))
      AIChatsOpenClick
    CASE "chatrename"
      ' (chatrename:N|name — the Nth chat, its headings not counted)
      AIShowChats 1
      AIChatList.ItemIndex = AIChatRow(VAL(FIELD$(Arg, "|", 1)))
      id = AIChatPicked
      IF AIRenameChatTo(id, FIELD$(Arg, "|", 2)) THEN
        AIFillChats
        AISelectChat id
      END IF
    CASE "chatdelete"
      AIShowChats 1
      AIChatList.ItemIndex = AIChatRow(VAL(Arg))
      AIChatsDeleteClick
    CASE "chatundo": AIChatsUndoClick
    CASE "edit"
      AIEditQuestion VAL(FIELD$(Arg, "|", 1))
      IF INSTR(Arg, "|") > 0 THEN
        AIInput.Text = MID$(Arg, INSTR(Arg, "|") + 1)
        AISend
      END IF
    CASE "retry": AIRetry
    CASE "thought": AIChatLink "rai:thought/" + Arg
    CASE "save": AISaveChat
    CASE ELSE: Result = 0
  END SELECT
END FUNCTION

' ---- the language changed ---------------------------------------------------------------
' Studio's language was changed in Settings (ai.inc's AiLanguageChanged calls
' this): what the chats show that is made from values, not a caption that
' translates itself — the title beside Chats, the list, the note's spoken
' form, the Undo bar, the bar over the box while a question is edited.
SUB AiChatsLanguageChanged
  DIM id AS STRING
  AIShowChatTitle
  AIChatsNote.AccessibleDescription = TR$("Kept in {folder}", "folder", AITools.ChatsFolder)
  IF AIChatsPanel.Visible THEN
    id = AIChatPicked
    AIFillChats
    IF id <> "" THEN AISelectChat id
  END IF
  IF AIChatUndoBar.Visible THEN AIChatUndoText.Caption = TR$("Deleted {q}{title}{q}.", "q", CHR$(34), "title", AIChatDeletedTitle)
  IF AIEditing > 0 THEN AIShowEditText
END SUB
