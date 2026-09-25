//! The buttons of RapidQ's MESSAGEBOX and MESSAGEDLG (manual), shared by the
//! desktop runtime (FLTK dialogs) and the web runtime (browser dialogs).

/// A dialog button: its label and the value the call returns when chosen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Button {
    pub label: &'static str,
    pub result: i64,
}

const fn b(label: &'static str, result: i64) -> Button {
    Button { label, result }
}

// Windows MessageBox results (IDOK …), returned by MESSAGEBOX.
const IDOK: i64 = 1;
const IDCANCEL: i64 = 2;
const IDABORT: i64 = 3;
const IDRETRY: i64 = 4;
const IDIGNORE: i64 = 5;
const IDYES: i64 = 6;
const IDNO: i64 = 7;

/// `MESSAGEBOX(text, title, flags)`: the buttons for `flags` (MB_OK = 0,
/// MB_OKCANCEL = 1, MB_ABORTRETRYIGNORE = 2, MB_YESNOCANCEL = 3, MB_YESNO =
/// 4, MB_RETRYCANCEL = 5; icon and default-button bits are ignored), the
/// affirmative choice first and the negative/cancelling one second.
pub fn message_box_buttons(flags: i64) -> Vec<Button> {
    match flags & 0xF {
        1 => vec![b("OK", IDOK), b("Cancel", IDCANCEL)],
        2 => vec![b("Retry", IDRETRY), b("Abort", IDABORT), b("Ignore", IDIGNORE)],
        3 => vec![b("Yes", IDYES), b("No", IDNO), b("Cancel", IDCANCEL)],
        4 => vec![b("Yes", IDYES), b("No", IDNO)],
        5 => vec![b("Retry", IDRETRY), b("Cancel", IDCANCEL)],
        _ => vec![b("OK", IDOK)],
    }
}

/// `MESSAGEDLG(text, msgType, msgButtons, helpContext)`: the buttons for the
/// OR-ed `mb*` flags (mbYes = 1, mbNo = 2, mbOK = 4, mbCancel = 8, mbAbort =
/// 32, mbRetry = 64, mbIgnore = 128, mbAll = 256; mbHelp = 16 has no help
/// system to open and is left out), returning the `mr*` values (mrOk = 1,
/// mrCancel = 2, mrAbort = 3, mrRetry = 4, mrIgnore = 5, mrYes = 6, mrNo = 7,
/// mrAll = 8). Affirmative choices come first.
pub fn message_dlg_buttons(flags: i64) -> Vec<Button> {
    let all = [
        (1, b("Yes", 6)),
        (4, b("OK", 1)),
        (64, b("Retry", 4)),
        (256, b("All", 8)),
        (2, b("No", 7)),
        (8, b("Cancel", 2)),
        (32, b("Abort", 3)),
        (128, b("Ignore", 5)),
    ];
    let buttons: Vec<Button> = all.iter().filter(|(bit, _)| flags & bit != 0).map(|(_, button)| *button).collect();
    if buttons.is_empty() {
        vec![b("OK", 1)]
    } else {
        buttons
    }
}

/// Title of a MESSAGEDLG box for its `mt*` type (mtWarning = 0, mtError = 1,
/// mtInformation = 2, mtConfirmation = 3, mtCustom = 4).
pub fn message_dlg_title(msg_type: i64) -> &'static str {
    match msg_type {
        0 => "Warning",
        1 => "Error",
        2 => "Information",
        3 => "Confirm",
        _ => "",
    }
}

/// What closing the dialog without choosing means: Cancel/No/Abort if the
/// dialog has one, otherwise its only button.
pub fn dismissed(buttons: &[Button]) -> i64 {
    ["Cancel", "No", "Abort"]
        .iter()
        .find_map(|label| buttons.iter().find(|b| b.label == *label))
        .or(buttons.first())
        .map_or(0, |b| b.result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn buttons_and_results() {
        let yes_no = message_box_buttons(4 | 32); // MB_YESNO | MB_ICONQUESTION
        assert_eq!(yes_no.iter().map(|b| (b.label, b.result)).collect::<Vec<_>>(), [("Yes", 6), ("No", 7)]);
        assert_eq!(message_box_buttons(0), vec![b("OK", 1)]);
        let dlg = message_dlg_buttons(1 | 2 | 8); // mbYes OR mbNo OR mbCancel
        assert_eq!(dlg.iter().map(|b| b.label).collect::<Vec<_>>(), ["Yes", "No", "Cancel"]);
        assert_eq!(dismissed(&dlg), 2); // closing = Cancel
        assert_eq!(dismissed(&message_box_buttons(4)), 7); // Yes/No: closing = No
        assert_eq!(message_dlg_buttons(0), vec![b("OK", 1)]);
        assert_eq!(message_dlg_title(1), "Error");
    }
}
