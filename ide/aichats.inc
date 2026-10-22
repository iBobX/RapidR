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
DECLARE SUB AIChatsUndoClick
DECLARE SUB AIChatsSearchChanged
DECLARE SUB AIChatsSearchKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB AIRenameOkClick
DECLARE SUB AIRenameCancelClick
DECLARE SUB AIRenameKey(Key AS INTEGER, Shift AS INTEGER)
DECLARE SUB AIFillChats
DECLARE SUB AIOpenChat(Id AS STRING, Title AS STRING)
DECLARE FUNCTION AIChatPicked AS STRING
DECLARE SUB AISelectChat(Id AS STRING)

SUB SetUpAIChats
  AIChatProject = CHR$(1)
  AIChatsPanel.Parent = AIPanel
  AIChatsPanel.Align = 5
  AIChatsPanel.BevelOuter = 0
  AIChatsPanel.Visible = 0
  AIChatsPanel.AccessibleName = "Chats"
  ' (the search box on top)
  AIChatsHead.Parent = AIChatsPanel
  AIChatsHead.Align = 1
  AIChatsHead.Height = 40
  AIChatsHead.BevelOuter = 0
  AIChatSearchLabel.Parent = AIChatsHead
  AIChatSearchLabel.Caption = "Search:"
  AIChatSearchLabel.AutoSize = 0
  AIChatSearch.Parent = AIChatsHead
  AIChatSearch.AccessibleName = "Search chats"
  AIChatSearch.AccessibleDescription = "Their titles and what was said; Down goes to the list, Escape back to the chat"
  AIChatSearch.Hint = "Search chats"
  AIChatSearch.OnChange = AIChatsSearchChanged
  AIChatSearch.OnKeyDown = AIChatsSearchKey
  ' (Open, Rename, Delete at the bottom — Back is the Chats button's, at the
  ' top —; Undo after a delete; the new name while renaming)
  AIChatsFoot.Parent = AIChatsPanel
  AIChatsFoot.Align = 2
  AIChatsFoot.Height = 96
  AIChatsFoot.BevelOuter = 0
  AIChatsOpenBtn.Parent = AIChatsFoot
  AIChatsOpenBtn.Caption = "Open"
  AIChatsOpenBtn.AccessibleName = "Open the chat"
  AIChatsOpenBtn.OnClick = AIChatsOpenClick
  AIChatsRenameBtn.Parent = AIChatsFoot
  AIChatsRenameBtn.Caption = "Rename"
  AIChatsRenameBtn.AccessibleName = "Rename the chat"
  AIChatsRenameBtn.OnClick = AIChatsRenameClick
  AIChatsDeleteBtn.Parent = AIChatsFoot
  AIChatsDeleteBtn.Caption = "Delete"
  AIChatsDeleteBtn.AccessibleName = "Delete the chat"
  AIChatsDeleteBtn.AccessibleDescription = "It is removed from this computer; Undo puts it back"
  AIChatsDeleteBtn.OnClick = AIChatsDeleteClick
  AIChatUndoBar.Parent = AIChatsPanel
  AIChatUndoBar.Align = 2
  AIChatUndoBar.Height = 36
  AIChatUndoBar.BevelOuter = 0
  AIChatUndoBar.Visible = 0
  AIChatUndoBar.AccessibleName = "Chat deleted"
  AIChatUndoText.Parent = AIChatUndoBar
  AIChatUndoText.AutoSize = 0: AIChatUndoText.WordWrap = 0
  AIChatUndoBtn.Parent = AIChatUndoBar
  AIChatUndoBtn.Caption = "Undo"
  AIChatUndoBtn.AccessibleName = "Undo: put the chat back"
  AIChatUndoBtn.OnClick = AIChatsUndoClick
  AIRenameBar.Parent = AIChatsPanel
  AIRenameBar.Align = 2
  AIRenameBar.Height = 40
  AIRenameBar.BevelOuter = 0
  AIRenameBar.Visible = 0
  AIRenameBar.AccessibleName = "Rename the chat"
  AIRenameEdit.Parent = AIRenameBar
  AIRenameEdit.AccessibleName = "The chat's new name"
  AIRenameEdit.AccessibleDescription = "Enter keeps it, Escape leaves the name as it was"
  AIRenameEdit.OnKeyDown = AIRenameKey
  AIRenameOk.Parent = AIRenameBar
  AIRenameOk.Caption = "Save"
  AIRenameOk.AccessibleName = "Save the new name"
  AIRenameOk.OnClick = AIRenameOkClick
  AIRenameCancel.Parent = AIRenameBar
  AIRenameCancel.Caption = "Cancel"
  AIRenameCancel.AccessibleName = "Keep the name as it was"
  AIRenameCancel.OnClick = AIRenameCancelClick
  ' (where they are kept, said under the list: personal data, plainly)
  AIChatsNote.Parent = AIChatsFoot
  AIChatsNote.AutoSize = 0: AIChatsNote.WordWrap = 1
  AIChatsNote.AccessibleName = "Where chats are kept"
  AIChatList.Parent = AIChatsPanel
  AIChatList.Align = 5
  AIChatList.AccessibleName = "This project's chats, the newest first"
  AIChatList.OnDblClick = AIChatsOpenClick
  AIShowChatTitle
END SUB

SUB LayOutAIChats
  DIM w AS INTEGER, bh AS INTEGER, bw AS INTEGER
  w = AIPanel.ClientWidth
  IF w < 200 THEN w = 200
  IF ThemeName = "classic" THEN bh = 23 ELSE bh = 28
  AIChatSearchLabel.Left = 8: AIChatSearchLabel.Top = 11: AIChatSearchLabel.Width = 56: AIChatSearchLabel.Height = 18
  AIChatSearch.Left = 66: AIChatSearch.Top = 6: AIChatSearch.Width = w - 72
  ' (three buttons across the bottom)
  bw = (w - 12 - 2 * 4) \ 3
  AIChatsNote.Left = 8: AIChatsNote.Top = 6: AIChatsNote.Width = w - 16: AIChatsNote.Height = 40
  AIChatsOpenBtn.Left = 6: AIChatsOpenBtn.Top = 54: AIChatsOpenBtn.Width = bw: AIChatsOpenBtn.Height = bh
  AIChatsRenameBtn.Left = 6 + (bw + 4): AIChatsRenameBtn.Top = 54: AIChatsRenameBtn.Width = bw: AIChatsRenameBtn.Height = bh
  AIChatsDeleteBtn.Left = 6 + 2 * (bw + 4): AIChatsDeleteBtn.Top = 54: AIChatsDeleteBtn.Width = bw: AIChatsDeleteBtn.Height = bh
  IF ThemeName = "classic" THEN AIChatList.ItemHeight = 16 ELSE AIChatList.ItemHeight = 26
  AIChatUndoBtn.Left = w - 6 - 72: AIChatUndoBtn.Top = (36 - bh) \ 2: AIChatUndoBtn.Width = 72: AIChatUndoBtn.Height = bh
  AIChatUndoText.Left = 8: AIChatUndoText.Top = 9: AIChatUndoText.Width = w - 16 - 76: AIChatUndoText.Height = 18
  AIRenameCancel.Left = w - 6 - 70: AIRenameCancel.Top = (40 - bh) \ 2: AIRenameCancel.Width = 70: AIRenameCancel.Height = bh
  AIRenameOk.Left = w - 6 - 70 - 4 - 64: AIRenameOk.Top = (40 - bh) \ 2: AIRenameOk.Width = 64: AIRenameOk.Height = bh
  AIRenameEdit.Left = 6: AIRenameEdit.Top = 8: AIRenameEdit.Width = w - 6 - 70 - 4 - 64 - 4 - 6
END SUB

SUB AIChatsFonts
  AIChatSearch.Font = UIFont
  AIChatSearchLabel.Font = UIFont
  AIChatSearchLabel.Font.Color = TC("page.text")
  AIChatList.Font = UIFont
  AIChatsOpenBtn.Font = UIFont
  AIChatsRenameBtn.Font = UIFont
  AIChatsDeleteBtn.Font = UIFont
  AIChatUndoText.Font = UIFont
  AIChatUndoText.Font.Color = TC("page.text")
  AIChatUndoBtn.Font = UIFont
  AIRenameEdit.Font = UIFont
  AIRenameOk.Font = UIFont
  AIRenameCancel.Font = UIFont
  AIChatsNote.Font = DimFont
  AIChatsNote.Font.Color = TC("page.dim")
END SUB

' (the chat's title beside Chats: its name, or "New chat")
SUB AIShowChatTitle
  DIM t AS STRING, w AS INTEGER
  IF AIChatName = "" THEN t = "New chat" ELSE t = AIChatName
  ' (as much as fits, then "…": the whole title in its hint and its name)
  AIChatTitle.Hint = t
  AIChatTitle.ShowHint = 1
  AIChatTitle.AccessibleName = "This chat: " + t
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
    OutputLine "AI: this chat couldn't be kept: " + MID$(r, 8)
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
    AIStatus.SimpleText = "That chat can't be read any more (removed outside Studio?)."
    EXIT SUB
  END IF
  AI.ChatState = state
  AIChatId = Id
  AIChatName = Title
  AIEditing = 0
  AIEditBar.Visible = 0
  AIChat.Text = AI.Transcript
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
    AIStatus.SimpleText = "A new chat. The last one is kept: Chats lists it."
  ELSE
    AIStatus.SimpleText = "A new chat."
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
    AIChatsNote.Caption = "Kept on this computer, in Studio's own folder — never in the project. Delete removes a chat for good."
    AIChatsNote.Hint = AITools.ChatsFolder
    AIChatsNote.ShowHint = 1
    AIChatsNote.AccessibleDescription = "Kept in " + AITools.ChatsFolder
    ' (the list gets the pane: the box and its buttons come back with Back)
    AIChat.Visible = 0
    AIBottom.Visible = 0
    AIChatsPanel.Visible = 1
    AIChatsBtn.Caption = "Back"
    AIChatsBtn.AccessibleName = "Back to the chat"
    LayOutAIChats
    AIFillChats
    AIChatSearch.SetFocus
  ELSE
    IF AIChatsPanel.Visible = 0 THEN EXIT SUB
    AIChatsPanel.Visible = 0
    AIBottom.Visible = 1
    AIChat.Visible = 1
    AIChatsBtn.Caption = "Chats"
    AIChatsBtn.AccessibleName = "Chats"
    AIInput.SetFocus
  END IF
END SUB

' (the list, as the search box says: the newest first, this chat marked)
SUB AIFillChats
  DIM list AS STRING, i AS INTEGER, line AS STRING, item AS STRING, q AS INTEGER, dt AS STRING, at AS INTEGER
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
  IF list <> "" THEN
    FOR i = 1 TO TALLY(list, CHR$(10)) + 1
      line = FIELD$(list, CHR$(10), i)
      IF line <> "" THEN
        q = VAL(FIELD$(line, CHR$(9), 4))
        dt = MID$(FIELD$(line, CHR$(9), 3), 1, 10)
        item = FIELD$(line, CHR$(9), 2)
        IF q = 1 THEN item = item + " — 1 question, " + dt ELSE item = item + " — " + LTRIM$(STR$(q)) + " questions, " + dt
        IF FIELD$(line, CHR$(9), 5) <> "" THEN item = item + ": " + CHR$(34) + FIELD$(line, CHR$(9), 5) + CHR$(34)
        IF FIELD$(line, CHR$(9), 1) = AIChatId THEN
          item = item + " (open)"
          at = AIChatList.ItemCount
        END IF
        AIChatList.AddItems item
        AIChatIds = AIChatIds + FIELD$(line, CHR$(9), 1) + CHR$(10)
        AIChatTitles = AIChatTitles + FIELD$(line, CHR$(9), 2) + CHR$(10)
      END IF
    NEXT
  END IF
  IF AIChatList.ItemCount > 0 THEN
    IF at >= 0 THEN AIChatList.ItemIndex = at ELSE AIChatList.ItemIndex = 0
  END IF
  AIChatsOpenBtn.Enabled = Flag(AIChatList.ItemCount > 0)
  AIChatsRenameBtn.Enabled = Flag(AIChatList.ItemCount > 0)
  AIChatsDeleteBtn.Enabled = Flag(AIChatList.ItemCount > 0)
  IF AIChatList.ItemCount = 0 THEN
    IF LTRIM$(AIChatSearch.Text) = "" THEN
      AIChatList.AccessibleDescription = "No chats kept for this project yet."
    ELSE
      AIChatList.AccessibleDescription = "No chat has " + CHR$(34) + AIChatSearch.Text + CHR$(34) + "."
    END IF
  ELSE
    AIChatList.AccessibleDescription = LTRIM$(STR$(AIChatList.ItemCount)) + " chats"
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

SUB AIChatsOpenClick
  DIM id AS STRING, title AS STRING
  id = AIChatPicked
  IF id = "" THEN EXIT SUB
  title = FIELD$(AIChatTitles, CHR$(10), AIChatList.ItemIndex + 1)
  IF id <> AIChatId THEN
    IF AI.Busy THEN AI.Cancel
    AISaveChat
    AIOpenChat id, title
    AIStatus.SimpleText = "Opened " + CHR$(34) + title + CHR$(34) + "."
  END IF
  AIShowChats 0
END SUB

SUB AIChatsRenameClick
  IF AIChatPicked = "" THEN EXIT SUB
  AIRenameEdit.Text = FIELD$(AIChatTitles, CHR$(10), AIChatList.ItemIndex + 1)
  AIChatUndoBar.Visible = 0
  AIRenameBar.Visible = 1
  AIRenameEdit.SetFocus
END SUB

SUB AIRenameKey(Key AS INTEGER, Shift AS INTEGER)
  IF Key = 13 THEN
    Key = 0
    AIRenameOkClick
  ELSEIF Key = 27 THEN
    Key = 0
    AIRenameCancelClick
  END IF
END SUB

SUB AIRenameOkClick
  DIM id AS STRING, t AS STRING
  id = AIChatPicked
  t = LTRIM$(RTRIM$(AIRenameEdit.Text))
  IF id = "" OR t = "" THEN
    AIRenameCancelClick
    EXIT SUB
  END IF
  IF AITools.RenameChat(AIChatProject, id, t) THEN
    IF id = AIChatId THEN
      AIChatName = t
      AIShowChatTitle
    END IF
    AIStatus.SimpleText = "Renamed: " + CHR$(34) + t + CHR$(34) + "."
  ELSE
    AIStatus.SimpleText = "That chat couldn't be renamed."
  END IF
  AIRenameBar.Visible = 0
  AIFillChats
  AISelectChat id
  AIChatList.SetFocus
END SUB

' (chat Id chosen in the list, when it shows it)
SUB AISelectChat(Id AS STRING)
  DIM i AS INTEGER
  FOR i = 1 TO TALLY(AIChatIds, CHR$(10))
    IF FIELD$(AIChatIds, CHR$(10), i) = Id THEN AIChatList.ItemIndex = i - 1
  NEXT
END SUB

SUB AIRenameCancelClick
  AIRenameBar.Visible = 0
  AIChatList.SetFocus
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
    AIStatus.SimpleText = "That chat couldn't be deleted."
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
  AIRenameBar.Visible = 0
  AIChatUndoText.Caption = "Deleted " + CHR$(34) + title + CHR$(34) + "."
  AIChatUndoBar.Visible = 1
  AIStatus.SimpleText = "Deleted " + CHR$(34) + title + CHR$(34) + ". Undo puts it back."
  AIFillChats
  IF AIChatList.ItemCount > 0 THEN AIChatList.SetFocus ELSE AIChatUndoBtn.SetFocus
END SUB

SUB AIChatsUndoClick
  DIM id AS STRING
  IF AIChatDeleted = "" THEN EXIT SUB
  id = AITools.RestoreChat(AIChatDeletedProject, AIChatDeleted)
  AIChatUndoBar.Visible = 0
  IF id = "" THEN
    AIStatus.SimpleText = "The chat couldn't be put back."
    EXIT SUB
  END IF
  AIChatDeleted = ""
  AIStatus.SimpleText = "Put back: " + CHR$(34) + AIChatDeletedTitle + CHR$(34) + "."
  ' (nothing said in the chat shown: the one put back is opened)
  IF AI.QuestionCount = 0 AND AIChatDeletedProject = AIChatProject THEN AIOpenChat id, AIChatDeletedTitle
  AIFillChats
  AIChatList.SetFocus
END SUB

' ---- edit a sent question, Retry (R2) --------------------------------------------------

' The Edit link under question Q: its text in the box, the bar saying what
' Send will do — the replies below it move to a saved copy.
SUB AIEditQuestion(Q AS INTEGER)
  DIM after AS INTEGER
  IF AI.Busy THEN
    AIStatus.SimpleText = "RAI is answering: wait for it, or Stop, then edit a question."
    EXIT SUB
  END IF
  IF Q < 1 OR Q > AI.QuestionCount THEN EXIT SUB
  AIShowChats 0
  after = AI.QuestionCount - Q
  AIEditing = Q
  AIInput.Text = AI.QuestionText(Q)
  IF after = 0 THEN
    AIEditText.Caption = "Editing your question. Send replaces it: its answer moves to a saved copy of this chat (in Chats)."
  ELSEIF after = 1 THEN
    AIEditText.Caption = "Editing your question. Send replaces it: its answer and the 1 question after it move to a saved copy of this chat (in Chats)."
  ELSE
    AIEditText.Caption = "Editing your question. Send replaces it: its answer and the " + LTRIM$(STR$(after)) + " questions after it move to a saved copy of this chat (in Chats)."
  END IF
  AIEditBar.AccessibleDescription = AIEditText.Caption
  AIEditBar.Visible = 1
  AIShowSlash
  AIInput.SetFocus
  AIStatus.SimpleText = "Editing a question: Send replaces it, Escape cancels."
END SUB

' (the chat kept as it is, under a title that says why: nothing lost)
FUNCTION AIKeepCopy(Why AS STRING) AS STRING
  DIM title AS STRING
  title = AIChatName
  IF title = "" THEN title = AITools.ChatTitle(AI.QuestionText(1))
  title = title + " (" + Why + ")"
  Result = AITools.SaveChat(AIChatProject, "", title, AI.Model, AI.ChatState)
  IF LEFT$(Result, 6) = "error:" THEN
    OutputLine "AI: the copy couldn't be kept: " + MID$(Result, 8)
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
    AIStatus.SimpleText = "RAI is answering: wait for it, or Stop, then send the edited question."
    EXIT FUNCTION
  END IF
  kept = AIKeepCopy("before the edit")
  IF kept = "" THEN
    AIStatus.SimpleText = "The question wasn't changed: the chat couldn't be copied first."
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
  OutputLine "AI: the chat before the edit is kept as " + CHR$(34) + kept + CHR$(34) + " (Chats)."
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
  kept = AIKeepCopy("before Retry")
  IF kept = "" THEN
    AIStatus.SimpleText = "Not retried: the chat couldn't be copied first."
    EXIT SUB
  END IF
  IF AI.Rewind(q) = "" THEN
    AIStatus.SimpleText = AI.Error
    EXIT SUB
  END IF
  AIChat.Text = AI.Transcript
  OutputLine "AI: the answer before Retry is kept in " + CHR$(34) + kept + CHR$(34) + " (Chats)."
  AISendNow t, 0
END SUB

' (tests and the manual's scenes: ai:chats, ai:chatopen=N, ai:chatrename=N|name,
' ai:chatdelete=N, ai:chatundo, ai:chatsearch=text, ai:edit=N|text,
' ai:retry, ai:thought=N; True when Verb was one)
FUNCTION AIChatStep(Verb AS STRING, Arg AS STRING) AS INTEGER
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
      AIChatList.ItemIndex = VAL(Arg) - 1
      AIChatsOpenClick
    CASE "chatrename"
      AIShowChats 1
      AIChatList.ItemIndex = VAL(FIELD$(Arg, "|", 1)) - 1
      AIChatsRenameClick
      AIRenameEdit.Text = FIELD$(Arg, "|", 2)
      AIRenameOkClick
    CASE "chatdelete"
      AIShowChats 1
      AIChatList.ItemIndex = VAL(Arg) - 1
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
