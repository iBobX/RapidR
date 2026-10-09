//! `Obj.Member` read without parentheses. RapidQ calls a method that way —
//! `WHILE MySQL.FetchRow`, `S$ = File.ReadLine`, `IF Dlg.Execute THEN` — and
//! reads a property otherwise (`UpDown.Max`). Which one it is depends on the
//! object's type, so the runtimes decide it when the program runs
//! (`rp_comp_value`), the same way for native builds, the interpreter and
//! the web. (`Form.ShowModal` and `Dlg.Execute` are also decided by the
//! compilers, rapidr_ast::VALUE_METHODS: on the web they wait.)

/// Every component's methods.
const ANY: &[&str] = &[
    "showmodal", "close", "show", "hide", "refresh",
    "clear", "additems", "additem", "deleteitems", "deleteitem", "removeitem", "addrow", "sort", "find",
    "setfocus", "focus", "click", "selectall", "copy", "paste", "cut",
    "execute",
];

/// QSQLITE / QMYSQL (and RapidR's `DB.ClearParams`).
const DATABASE: &[&str] = &[
    "connect", "disconnect", "query", "fetchrow", "fetchfield", "fetchlengths", "fieldseek", "rowseek", "row", "rowblob",
    "escapestring", "selectdb", "createdb", "dropdb", "clearparams",
];

/// QSOCKET, QSERVERSOCKET, RHTTP.
const NETWORK: &[&str] = &[
    "connect", "write", "writeline", "read", "readline", "bind", "listen", "accept", "start", "stop", "broadcast",
    "get", "post",
];

/// QFILESTREAM, QMEMORYSTREAM (and the web's file bridge).
const STREAM: &[&str] = &["open", "readall", "eof", "readline", "readln", "readbyte", "pickfile", "download", "loadfromurl"];

const JSON: &[&str] = &["parse", "stringify", "prettify", "has", "remove", "keys", "loadfile", "savefile"];

const STRING_LIST: &[&str] = &["loadfromfile", "savetofile", "add", "delete"];

/// QCANVAS and whatever else is drawn on.
const DRAWING: &[&str] = &[
    "line", "rect", "fillrect", "circle", "ellipse", "setpixel", "getpixel", "drawtext", "loadimage", "saveimage",
];

/// QCANVAS's Get / Put (RC.EXE's member table: `X = Canvas.Get`).
const CANVAS: &[&str] = &["get", "put"];

const TREE: &[&str] = &["addroot", "addchild", "expand", "collapse"];

const MDI: &[&str] = &["closechild", "closeallchild", "cascadechild", "sethorzchild", "setvertchild", "iconarrangechild"];

/// RNUM, RDATAFRAME, RPLOT share these (native builds always called them).
const DATA_SCIENCE: &[&str] = &[
    // RNum
    "array", "zeros", "ones", "full", "arange", "linspace", "reshape", "fromlist", "from_list",
    "sum", "mean", "min", "max", "std", "var", "variance", "median", "argmin", "argmax", "count", "ptp",
    "dot", "norm", "normalize",
    "sin", "cos", "tan", "asin", "arcsin", "acos", "arccos", "atan", "arctan",
    "sqrt", "abs", "exp", "log", "ln", "log2", "log10",
    "floor", "ceil", "round", "sign", "reciprocal", "square", "negative", "neg",
    "subtract", "sub", "multiply", "mul", "divide", "div", "power", "pow", "mod", "fmod", "clip", "clamp",
    "reverse", "flip", "unique", "shuffle", "append", "concatenate", "slice",
    "cumsum", "cumprod", "diff", "any", "all", "nonzero", "searchsorted",
    "rand", "random", "randn", "random_normal", "normal", "uniform", "random_uniform", "randint", "choice",
    "tolist", "tostring", "print",
    // RDataFrame
    "readcsv", "read_csv", "loadfromcsv", "savetocsv", "to_csv", "writecsv",
    "loadfromjson", "read_json", "savetojson", "to_json",
    "head", "tail", "describe", "columns", "info", "dtypes", "shape",
    "cellbyname", "at", "setcell", "iloc", "select", "sort_values", "filter",
    "groupby", "group_by", "value_counts", "nunique", "corr", "correlation",
    "drop", "drop_column", "rename", "rename_column", "addcolumn", "add_column", "set_column",
    "fillna", "fill_null", "dropna", "drop_nulls", "sample", "nlargest", "nsmallest",
    "merge", "join", "concat", "transpose", "apply", "replace", "togrid", "to_grid", "display",
    // RPlot
    "plot", "bar", "barh", "scatter", "step", "area", "fill_between", "hist", "histogram", "pie",
    "hline", "axhline", "vline", "axvline", "annotate",
    "legend", "savefig", "save", "figsize", "xlim", "ylim", "xscale", "yscale",
];

const DESIGN: &[&str] = &[
    "addcomponent", "getname", "gettype", "getcompx", "getcompy", "getcompw", "getcomph",
    "setprop", "getprop", "setcompbounds", "setname", "selectcomp", "removecomponent", "clearall", "addmdichild",
];

const GRID: &[&str] = &["cell", "cells", "setcell", "setsuggestions"];

const CODE_EDITOR: &[&str] = &["getsublist", "gotosub", "gotoline", "copytext", "findnext", "findprevious", "selectnextoccurrence", "gotomatchingbracket", "getmarkers"];

const TABS: &[&str] = &["addtabs", "tab"];

/// (I1) RDOCKMANAGER's (crate::dock): `L$ = Dock.SaveLayout`.
const DOCK: &[&str] = &["savelayout", "resetlayout", "cascade", "tilehorizontal", "tilevertical", "arrangeicons", "nextdocument", "previousdocument"];

/// (I2) RDIFFVIEW's (crate::objects::diffview): `N = Diff.NextHunk`.
const DIFF: &[&str] = &["acceptall", "rejectall", "nexthunk", "previoushunk"];
/// (I1 / L-PANELS) RapidR Studio's panels' (crate::panels): `Insp.ExpandAll`,
/// `OK = Tree.Save`, `Console.FindNext`.
const PANELS: &[&str] = &["expandall", "collapseall", "clearproperties", "save", "projecttext", "clearproblems", "findnext", "selectall"];

/// (I1) RPROGRAMSESSION's: `IF Session.Start THEN`.
const SESSION: &[&str] = &["start", "stop", "pause", "continue", "stepin", "stepover", "stepout", "stacktrace", "watchvalues", "runinbrowser", "stopbrowser"];

/// (I1) RPROJECT's: `IF Project.Save THEN`.
const PROJECT: &[&str] = &["save", "build", "stopbuild", "reveal"];

/// The web's own components.
const WEB: &[&str] = &[
    "sethtml", "navigate", "appendto", "setattribute", "getattribute",
    "addclass", "removeclass", "toggleclass", "queryselector", "queryselectorall",
    "eval", "call", "set", "get", "haskey", "remove", "keys",
    "play", "pause", "seek", "fullscreen",
    "requestpermission", "getposition", "watchposition", "clearwatch",
    "addroute", "back", "forward",
];

/// The I/O and media objects' (objects::rqlib): `IF Download.LeechFile`,
/// `IF CD.Open`.
const IO: &[&str] = &["leechfile", "check", "open", "listports", "readline"];

/// The methods of type `t` (RapidR's uppercase `R…` name) beyond [`ANY`].
fn methods_of(t: &str) -> &'static [&'static [&'static str]] {
    match t {
        "RSQLITE" | "RMYSQL" => &[DATABASE],
        "RSOCKET" | "RSERVERSOCKET" | "RHTTP" => &[NETWORK],
        "RFILESTREAM" | "RMEMORYSTREAM" => &[STREAM],
        "RJSON" => &[JSON],
        "RSTRINGLIST" => &[STRING_LIST],
        "RCANVAS" => &[DRAWING, CANVAS],
        "RFORM" | "RBITMAP" | "RIMAGE" => &[DRAWING],
        "RTREEVIEW" => &[TREE],
        "RFORMMDI" => &[MDI, DRAWING],
        "RNUM" | "RDATAFRAME" | "RPLOT" => &[DATA_SCIENCE],
        "RDESIGNSURFACE" => &[DESIGN],
        "RPROGRAMSESSION" => &[SESSION],
        "RPROJECT" => &[PROJECT],
        "RSTRINGGRID" => &[GRID],
        "RCODEEDITOR" => &[CODE_EDITOR],
        "RTABCONTROL" => &[TABS],
        "RDOCKMANAGER" => &[DOCK],
        "RDIFFVIEW" => &[DIFF],
        "RPROPERTYINSPECTOR" | "RTOOLBOX" | "RPROJECTTREE" | "ROUTPUTCONSOLE" | "RCOMMANDPALETTE" | "RTOOLBAR" | "RMARKDOWNVIEW" => &[PANELS],
        "RDOWNLOAD" | "RCDAUDIO" | "RCOMPORT" | "RMIDI" | "RWAVE" | "RVIDEO" => &[IO],
        "RWEBVIEW" | "RDOM" | "RJAVASCRIPT" | "RWEBSTORAGE" | "RWEBAUDIO" | "RWEBVIDEO" | "RWEBNOTIFICATION"
        | "RWEBGEOLOCATION" | "RROUTER" => &[WEB],
        _ => &[],
    }
}

/// Whether `member` (lowercase) is any type's method read without
/// parentheses — the quick test before the type is looked up.
pub fn is_value_method_name(member: &str) -> bool {
    [ANY, DATABASE, NETWORK, STREAM, JSON, STRING_LIST, DRAWING, CANVAS, TREE, MDI, DATA_SCIENCE, DESIGN, GRID, CODE_EDITOR, TABS, WEB, IO, DOCK, DIFF, SESSION, PROJECT, PANELS]
        .iter()
        .any(|list| list.contains(&member))
}

/// Whether reading `member` (lowercase) of an object of type `type_name`
/// (`RMYSQL` or RapidQ's `QMYSQL`, any case; "" for no component) calls the
/// method rather than reading a property.
pub fn is_value_method(type_name: &str, member: &str) -> bool {
    if type_name.is_empty() || !is_value_method_name(member) {
        return false;
    }
    let upper = type_name.to_ascii_uppercase();
    let t = match upper.strip_prefix('Q') {
        Some(rest) => format!("R{rest}"),
        None => upper,
    };
    ANY.contains(&member) || methods_of(&t).iter().any(|list| list.contains(&member))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn methods_by_type() {
        // (RapidQ's idioms)
        assert!(is_value_method("RMYSQL", "fetchrow"));
        assert!(is_value_method("QSQLITE", "fetchrow"));
        assert!(is_value_method("RSQLITE", "clearparams"));
        assert!(is_value_method("QFILESTREAM", "readline"));
        assert!(is_value_method("RFILESTREAM", "eof"));
        assert!(is_value_method("RFORM", "showmodal"));
        assert!(is_value_method("ROPENDIALOG", "execute"));
        // (a property of these types, a method of others)
        assert!(!is_value_method("RUPDOWN", "max"));
        assert!(!is_value_method("QTRACKBAR", "min"));
        assert!(!is_value_method("RPROGRESSBAR", "max"));
        assert!(!is_value_method("RLISTBOX", "columns"));
        assert!(is_value_method("RNUM", "max"));
        assert!(is_value_method("RDATAFRAME", "columns"));
        // (no component, or not a method at all)
        assert!(!is_value_method("", "fetchrow"));
        assert!(!is_value_method("RMYSQL", "rowcount"));
        assert!(!is_value_method("RFORM", "center"));
    }
}
