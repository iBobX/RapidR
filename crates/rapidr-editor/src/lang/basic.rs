//! RapidQ / RapidR BASIC's keyword groups, added to `rapidq-basic.toml`'s
//! rules when it loads.
//!
//! Until the language registry (stage I0, lane L-REG) generates them, they
//! are the compiler's own lists, copied: the lexer's keywords, the
//! components (`rapidr_ast::COMPONENT_TYPES`, each also under its RapidQ `Q`
//! name) and the builtins every host implements
//! (`rapidr_bytecode::builtins::BUILTINS`). Tests check them against those
//! lists so they can't drift.

/// The lexer's control-flow keywords.
pub const CONTROL: &[&str] = &[
    "IF", "THEN", "ELSE", "ELSEIF", "END", "FOR", "TO", "STEP", "NEXT", "WHILE", "WEND", "DO", "LOOP", "UNTIL", "SELECT", "CASE", "EXIT", "RETURN", "GOTO", "GOSUB", "CALL",
];

/// The lexer's other statement keywords.
pub const STATEMENTS: &[&str] = &[
    "DIM", "AS", "SUB", "FUNCTION", "CREATE", "CONST", "TYPE", "STRUCT", "DECLARE", "LIB", "ALIAS", "WITH", "IMPORT", "PRINT", "INPUT", "DEFSTR", "DEFINT", "DEFBYTE", "DEFWORD", "DEFDWORD",
    "DEFLONG", "DEFLNG", "DEFSHORT", "DEFSNG", "DEFDBL", "DEFCUR", "EXTENDS", "PROPERTY", "SET", "BYVAL", "BYREF", "BIND", "CONSTRUCTOR", "OPEN", "CLOSE", "WRITE", "SEEK", "KILL", "RUSTSTART",
    "RUSTEND",
];

/// The lexer's type keywords (and SHORT, which DEFSHORT declares).
pub const TYPES: &[&str] = &["INTEGER", "STRING", "DOUBLE", "SINGLE", "BYTE", "WORD", "DWORD", "LONG", "SHORT", "INT64", "CURRENCY", "ROBJECT", "VARIANT"];

/// Operators written as words.
pub const OPERATORS: &[&str] = &["AND", "OR", "NOT", "XOR", "MOD"];

/// Constants every program has.
pub const CONSTANTS: &[&str] = &["TRUE", "FALSE"];

/// `rapidr_ast::COMPONENT_TYPES` (R names; the Q names are added).
pub const COMPONENTS: &[&str] = &[
    "RFORM", "RFORMMDI", "RBUTTON", "RLABEL", "REDIT", "RPANEL", "RCHECKBOX", "RRADIOBUTTON", "RCOMBOBOX", "RLISTBOX", "RFILELISTBOX", "RDIRTREE", "RTIMER", "RIMAGE", "RCANVAS", "RHEADER",
    "RRECT", "RSTRINGGRID", "RTABCONTROL", "RTREEVIEW", "RMAINMENU", "RMENUITEM", "RPOPUPMENU", "ROPENDIALOG", "RSAVEDIALOG", "RFILEDIALOG", "RCOLORDIALOG", "RFONTDIALOG", "RTOOLBAR",
    "RSTATUSBAR", "RPROGRESS", "RRICHEDIT", "RMEMO", "RSCROLLBAR", "RUPDOWN", "RDATETIMEPICKER", "RFILESTREAM", "RSTRINGLIST", "RTRACKBAR", "RPRINTER", "RREGISTRY", "RSPLITTER", "RSCROLLBOX",
    "RSQLITE", "RMYSQL", "RSOCKET", "RSERVERSOCKET", "RHTTP", "RLISTVIEW", "RPROGRESSBAR", "RNUM", "RDATAFRAME", "RPLOT", "RDESIGNSURFACE", "RCODEEDITOR", "RGROUPBOX", "RCOOLBTN", "ROVALBTN",
    "RJSON", "RDXSCREEN", "RDXIMAGELIST", "RDXTIMER", "RDXSOUND", "RDXJOYSTICK", "RD3DFRAME", "RD3DMESHBUILDER", "RD3DMESH", "RD3DFACE", "RD3DLIGHT", "RD3DTEXTURE", "RD3DVISUAL", "RD3DWRAP",
    "RD3DVECTOR", "RFONT", "RMEMORYSTREAM", "RBITMAP", "RIMAGELIST", "RNOTIFYICONDATA", "RBEVEL", "RDIGDISPLAY", "RGLASSFRAME", "RCGI", "RCOMPORT", "RDOWNLOAD", "RMIDI", "RWAVE", "RVIDEO",
    "RCDAUDIO", "RWEBVIEW", "RDOM", "RJAVASCRIPT", "RWEBSTORAGE", "RWEBAUDIO", "RWEBVIDEO", "RWEBNOTIFICATION", "RWEBGEOLOCATION", "RROUTER",
];

/// The components' RapidQ names (`Q` + an R name's rest) and the older
/// names `rapidr_ast::canonical_type_name` maps.
pub const Q_COMPONENTS: &[&str] = &[
    "QFORM", "QFORMMDI", "QBUTTON", "QLABEL", "QEDIT", "QPANEL", "QCHECKBOX", "QRADIOBUTTON", "QCOMBOBOX", "QLISTBOX", "QFILELISTBOX", "QDIRTREE", "QTIMER", "QIMAGE", "QCANVAS", "QHEADER",
    "QRECT", "QSTRINGGRID", "QTABCONTROL", "QTREEVIEW", "QMAINMENU", "QMENUITEM", "QPOPUPMENU", "QOPENDIALOG", "QSAVEDIALOG", "QFILEDIALOG", "QCOLORDIALOG", "QFONTDIALOG", "QTOOLBAR",
    "QSTATUSBAR", "QPROGRESS", "QRICHEDIT", "QMEMO", "QSCROLLBAR", "QUPDOWN", "QDATETIMEPICKER", "QFILESTREAM", "QSTRINGLIST", "QTRACKBAR", "QPRINTER", "QREGISTRY", "QSPLITTER", "QSCROLLBOX",
    "QSQLITE", "QMYSQL", "QSOCKET", "QSERVERSOCKET", "QHTTP", "QLISTVIEW", "QPROGRESSBAR", "QNUM", "QDATAFRAME", "QPLOT", "QDESIGNSURFACE", "QCODEEDITOR", "QGROUPBOX", "QCOOLBTN", "QOVALBTN",
    "QJSON", "QDXSCREEN", "QDXIMAGELIST", "QDXTIMER", "QDXSOUND", "QDXJOYSTICK", "QD3DFRAME", "QD3DMESHBUILDER", "QD3DMESH", "QD3DFACE", "QD3DLIGHT", "QD3DTEXTURE", "QD3DVISUAL", "QD3DWRAP",
    "QD3DVECTOR", "QFONT", "QMEMORYSTREAM", "QBITMAP", "QIMAGELIST", "QNOTIFYICONDATA", "QBEVEL", "QDIGDISPLAY", "QGLASSFRAME", "QCGI", "QCOMPORT", "QDOWNLOAD", "QMIDI", "QWAVE", "QVIDEO",
    "QCDAUDIO", "QWEBVIEW", "QDOM", "QJAVASCRIPT", "QWEBSTORAGE", "QWEBAUDIO", "QWEBVIDEO", "QWEBNOTIFICATION", "QWEBGEOLOCATION", "QROUTER", "QGAUGE", "QOUTLINE", "COMPORT",
];

/// `rapidr_bytecode::builtins::BUILTINS` as programs write them: without the
/// compiler's internal `__` calls and its `_func` / `_hash` / `_field`
/// variants.
pub const BUILTINS: &[&str] = &[
    "ATAN", "TAB", "GET", "SETCONSOLETITLE", "CHDRIVE", "MEMCPY", "MEMSET", "MEMCMP", "LFLUSH", "ABS", "ACOS", "ASC", "ASIN", "ATN", "BEEP", "BIN", "CBOOL", "CDBL", "CEIL", "CHDIR", "CHR", "CINT",
    "CLS", "CLNG", "CLOSE", "COLOR", "COMMAND", "COMMANDCOUNT", "CONVBASE", "COS", "CSNG", "CSRLIN", "CURDIR", "DATE", "DELETE", "DIR", "DIREXISTS", "DOEVENTS", "INKEY", "E", "END", "ENVIRON", "EOF", "EXP",
    "EXTRACTRESOURCE", "FIELD", "FILEEXISTS", "FILELEN", "FIX", "FLOOR", "FORMAT", "FRAC", "FREEFILE", "HEX", "HEXTODEC", "IIF", "INPUT", "INSERT", "INSTR", "INT", "INV", "ISNUMERIC", "KILL",
    "LBOUND", "LCASE", "LEFT", "LEN", "LOCATE", "LOF", "LOG", "LTRIM", "MESSAGEBOX", "MESSAGEDLG", "MID", "MKDIR", "MOUSEX", "MOUSEY", "MSGBOX", "OCT", "OPEN", "PI", "PLAYSOUND", "PLAYWAV",
    "POS", "PRINT", "PRINTLN", "RANDOMIZE", "RENAME", "RESOURCE", "RESOURCECOUNT", "REPLACE", "REPLACESUBSTR", "REVERSE", "RGB", "RIGHT", "RINSTR", "RMDIR", "RND", "ROUND", "RTRIM", "RUN",
    "SEEK", "SGN", "SHELL", "SHELLWAIT", "SHL", "SHR", "SHOWMESSAGE", "SIN", "SLEEP", "SOUND", "SPACE", "SQR", "STR", "STRF", "STRING", "TALLY", "TAN", "TIME", "TIMER", "TRIM", "UBOUND",
    "UCASE", "VAL", "VARTYPE",
];

/// The groups, as `Language::from_toml_with` takes them (the first group
/// that has a word decides its kind).
pub const KEYWORD_GROUPS: &[(&str, &[&str])] = &[
    ("keyword.control", CONTROL),
    ("keyword", STATEMENTS),
    ("type", TYPES),
    ("keyword.operator", OPERATORS),
    ("constant", CONSTANTS),
    ("type.component", COMPONENTS),
    ("type.component", Q_COMPONENTS),
    ("function", BUILTINS),
];

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn lexes_as_keyword(word: &str) -> bool {
        let tokens = rapidr_lexer::Lexer::new(&format!("{word} "), None).tokenize().expect("lexes");
        !matches!(tokens.first().map(|t| t.kind), Some(rapidr_lexer::TokenType::Identifier) | None)
    }

    #[test]
    fn the_lexers_keywords() {
        for w in CONTROL.iter().chain(STATEMENTS).chain(OPERATORS).chain(TYPES.iter().filter(|t| **t != "SHORT")) {
            assert!(lexes_as_keyword(w), "{w} is not a keyword of rapidr-lexer");
        }
        assert!(!lexes_as_keyword("SHORT"));
    }

    #[test]
    fn the_components() {
        let ours: BTreeSet<&str> = COMPONENTS.iter().copied().collect();
        let theirs: BTreeSet<&str> = rapidr_ast::COMPONENT_TYPES.iter().copied().collect();
        assert_eq!(ours, theirs, "COMPONENTS drifted from rapidr_ast::COMPONENT_TYPES");
        for r in COMPONENTS {
            let q = format!("Q{}", &r[1..]);
            assert!(Q_COMPONENTS.contains(&q.as_str()), "{q} missing");
            // (an include library's component keeps its Q name)
            let c = rapidr_ast::canonical_type_name(&q).to_ascii_uppercase();
            assert!(c == *r || c == q, "{q} is {c}");
        }
    }

    #[test]
    fn the_builtins() {
        let ours: BTreeSet<String> = BUILTINS.iter().map(|b| b.to_ascii_lowercase()).collect();
        let internal = |b: &str| b.starts_with("__") || b.contains('.') || b.ends_with("_func") || b.ends_with("_hash") || b.ends_with("_field") || b == "line_input" || b == "rapidr__waitkey";
        let theirs: BTreeSet<String> = rapidr_bytecode::builtins::BUILTINS.iter().filter(|b| !internal(b)).map(|b| b.to_string()).collect();
        assert_eq!(ours, theirs, "BUILTINS drifted from rapidr_bytecode::builtins::BUILTINS");
    }
}
