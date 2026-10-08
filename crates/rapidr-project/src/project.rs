//! The project model and its `.rrproj` format 2 (TOML) file.

use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::error::ProjectError;

/// The project file format this crate reads and writes.
pub const FORMAT: i64 = 2;

/// The first line of every `.rrproj` this crate writes.
pub const HEADER: &str = "# RapidR Studio project (format 2)";

/// What a project file is, which decides how the IDE opens and builds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FileKind {
    /// BASIC source (`.rr`, `.bas`).
    #[default]
    Module,
    /// BASIC source that defines a form (a top-level `CREATE x AS QFORM`):
    /// opens in the designer.
    Form,
    /// A file meant to be `$INCLUDE`d (`.inc`).
    Include,
    /// An icon, picture, sound or video embedded with `$RESOURCE`.
    Resource,
    /// A file shipped beside the program (anything under `assets/`).
    Asset,
    /// A data file the program reads (`.csv`, `.json`, `.db`, ...).
    Data,
}

impl FileKind {
    /// The name used in the project file (`module`, `form`, ...).
    pub fn as_str(self) -> &'static str {
        match self {
            FileKind::Module => "module",
            FileKind::Form => "form",
            FileKind::Include => "include",
            FileKind::Resource => "resource",
            FileKind::Asset => "asset",
            FileKind::Data => "data",
        }
    }
}

/// One file of the project. `path` is relative to the `.rrproj`'s folder,
/// with `/` separators.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(from = "RawProjectFile")]
pub struct ProjectFile {
    pub path: String,
    pub kind: FileKind,
}

/// A file entry as written; a missing `kind` is taken from the extension.
#[derive(Deserialize)]
struct RawProjectFile {
    #[serde(default)]
    path: String,
    #[serde(default)]
    kind: Option<FileKind>,
}

impl From<RawProjectFile> for ProjectFile {
    fn from(raw: RawProjectFile) -> Self {
        let path = normalize_path(&raw.path);
        let kind = raw.kind.unwrap_or_else(|| kind_for_path(&path));
        ProjectFile { path, kind }
    }
}

/// `[build]`: how the program is built.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Build {
    /// `CONSOLE`, `GUI`, `CGI`, `WEB`...; empty = the main file's `$APPTYPE`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub apptype: String,
    /// The program's icon (a project path): `.icns`, `.ico`, `.png` or
    /// `.svg`; empty = the main file's `$OPTION ICON`, else RapidR's.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub icon: String,
    /// The app's name (the `.app`, the Dock, Explorer); empty = the
    /// project's name.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub app_name: String,
    /// Reverse-DNS identifier (macOS' bundle ID, Linux' desktop entry);
    /// empty = `dev.rapidr.app.<name>`.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub bundle_id: String,
    /// The app's version (`1.0`, `2.3.1`); empty = 1.0.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub version: String,
    /// Who makes it (Windows' CompanyName, the copyright line).
    #[serde(skip_serializing_if = "String::is_empty")]
    pub company: String,
    /// The UI theme the program starts with.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub theme: String,
    /// What to build: `native`, `bytecode`, `web`, ...
    pub targets: Vec<String>,
    /// Build optimized (release) rather than for debugging.
    pub release: bool,
}

/// `[run]`: how the IDE runs the program.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Run {
    /// Command-line arguments (`COMMAND$`).
    pub args: Vec<String>,
    /// The working directory, relative to the project folder; empty = the
    /// project folder.
    #[serde(skip_serializing_if = "String::is_empty")]
    pub cwd: String,
    /// Run the program's forms as separate top-level windows instead of
    /// inside the IDE.
    pub separate_windows: bool,
}

/// How RapidQ-compatibility findings are reported.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CompatLevel {
    #[default]
    Warn,
    Error,
}

/// `[compat]`: whether the project must stay RapidQ-compatible.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Compat {
    /// Only RapidQ's language and components (no RapidR extensions).
    pub rapidq_compatible: bool,
    /// Whether a RapidR-only feature is a warning or an error.
    pub level: CompatLevel,
}

/// `[designer]`: the form designer's settings for this project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Designer {
    /// Grid step in pixels.
    pub grid: u32,
    /// Snap to the grid.
    pub snap: bool,
    /// Show alignment guides.
    pub guides: bool,
}

impl Default for Designer {
    fn default() -> Self {
        Designer {
            grid: 8,
            snap: true,
            guides: true,
        }
    }
}

/// How much of the program's data the AI assistant may see.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DataAccess {
    None,
    #[default]
    Schema,
    Sample,
    Full,
}

/// `[ai]`: what the in-IDE AI may do with this project.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ai {
    /// The AI is off unless the project says otherwise.
    pub allowed: bool,
    /// Globs of project files the AI may read (empty = all).
    pub context_include: Vec<String>,
    /// Globs of project files the AI never reads.
    pub context_exclude: Vec<String>,
    pub data_access: DataAccess,
    /// Rows per table when `data_access = "sample"`.
    pub sample_rows: u32,
    /// Columns whose values are never shown to the AI.
    pub redact_columns: Vec<String>,
}

impl Default for Ai {
    fn default() -> Self {
        Ai {
            allowed: false,
            context_include: Vec::new(),
            context_exclude: Vec::new(),
            data_access: DataAccess::Schema,
            sample_rows: 20,
            redact_columns: Vec::new(),
        }
    }
}

/// A RapidR Studio project (`.rrproj`, format 2).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Project {
    pub format: i64,
    pub name: String,
    /// The main file (a project path).
    pub main: String,
    pub files: Vec<ProjectFile>,
    pub build: Build,
    pub run: Run,
    pub compat: Compat,
    pub designer: Designer,
    pub ai: Ai,
}

impl Default for Project {
    fn default() -> Self {
        Project {
            format: FORMAT,
            name: String::new(),
            main: String::new(),
            files: Vec::new(),
            build: Build::default(),
            run: Run::default(),
            compat: Compat::default(),
            designer: Designer::default(),
            ai: Ai::default(),
        }
    }
}

/// The project templates `Project::new_from_template` knows.
pub const TEMPLATES: &[&str] = &["console", "gui", "rapidq", "data", "mdi"];

/// Template sources. `{TITLE}` is replaced by the project name as a BASIC
/// string literal.
///
/// RapidQ-compatible form app: RapidQ's names and syntax only.
const TEMPLATE_RAPIDQ: &str = r#"' A RapidQ-compatible form: only RapidQ's own language and components,
' so the program also runs in the original RapidQ.
$APPTYPE GUI

' Event handlers are declared before the CREATE blocks that use them.
DECLARE SUB ShowText

CREATE Form1 AS QFORM
    Caption = {TITLE}
    Width = 360
    Height = 180
    Center
    CREATE Prompt AS QLABEL
        Caption = "Type something:"
        Left = 16: Top = 16
    END CREATE
    CREATE Entry AS QEDIT
        Text = "Hello, world"
        Left = 16: Top = 38: Width = 220
    END CREATE
    CREATE ShowBtn AS QBUTTON
        Caption = "&Show"
        Left = 248: Top = 36: Width = 80
        OnClick = ShowText
    END CREATE
    CREATE Result AS QLABEL
        Caption = "Press the button."
        Left = 16: Top = 84: Width = 312
    END CREATE
END CREATE

' Copies the edit's text into the label.
SUB ShowText
    Result.Caption = "You typed: " + Entry.Text
END SUB

Form1.ShowModal
"#;

/// Data dashboard: a grid and a chart over inline CSV text.
const TEMPLATE_DATA: &str = r#"' A small data dashboard: a table of sales in a grid and a bar chart of
' the same numbers. The data is written right here in the program, so it
' runs the same on the desktop and in a browser; to use your own, replace
' the CSV text below (first line = column names).
$APPTYPE GUI

DIM Csv AS STRING
Csv = "Region,Sales,Target" + CHR$(10) + "North,120,100" + CHR$(10) + "South,85,90" + CHR$(10) + "East,150,130" + CHR$(10) + "West,95,110"

DIM Chart AS RPlot

CREATE Form1 AS RForm
    Caption = {TITLE}
    Width = 560
    Height = 460
    Center
    CREATE Grid AS RStringGrid
        Left = 10: Top = 10: Width = 530: Height = 130
        FixedCols = 0
        DefaultColWidth = 120
    END CREATE
    CREATE Picture AS RImage
        Left = 10: Top = 150: Width = 530: Height = 230
    END CREATE
    CREATE Summary AS RLabel
        Left = 10: Top = 390: Width = 530
    END CREATE
END CREATE

' Fills the grid from the CSV text, one line per row.
SUB LoadTable
    DIM lines AS INTEGER, r AS INTEGER, c AS INTEGER, row AS STRING
    lines = TALLY(Csv, CHR$(10)) + 1
    Grid.ColCount = 3
    Grid.RowCount = lines
    FOR r = 0 TO lines - 1
        row = FIELD$(Csv, CHR$(10), r + 1)
        FOR c = 0 TO 2
            Grid.Cell(c, r) = FIELD$(row, ",", c + 1)
        NEXT
    NEXT
END SUB

' Draws one bar per region from the Sales column and names the best one.
SUB DrawChart
    DIM r AS INTEGER, names AS STRING, nums AS STRING, best AS STRING
    DIM sales AS RNum
    FOR r = 1 TO Grid.RowCount - 1
        names = names + IIF(r > 1, ",", "") + Grid.Cell(0, r)
        nums = nums + IIF(r > 1, ",", "") + Grid.Cell(1, r)
    NEXT
    sales.FromList(nums)
    best = Grid.Cell(0, sales.ArgMax + 1)
    Chart.Clear
    Chart.Width = Picture.Width
    Chart.Height = Picture.Height
    Chart.Title = "Sales by region"
    Chart.Bar(names, sales, "Sales", "steelblue")
    Picture.LoadFromPlot(Chart)
    Summary.Caption = "Best region: " + best + " (" + STR$(sales.Max) + ")"
END SUB

LoadTable
DrawChart
Form1.ShowModal
"#;

/// MDI app with File and Window menus.
const TEMPLATE_MDI: &str = r#"' An MDI application: one main window that holds several child windows,
' with a File menu (New, Exit) and a Window menu (Cascade, Tile).
$APPTYPE GUI
$INCLUDE "RAPIDQ.INC"

DECLARE SUB NewDocument
DECLARE SUB QuitApp
DECLARE SUB CascadeWindows
DECLARE SUB TileWindows

CONST MaxDocs = 8
DIM Docs(MaxDocs - 1) AS RMemo   ' the text areas that become child windows
DIM DocCount AS INTEGER

CREATE Main AS RFormMDI
    Caption = {TITLE}
    Width = 720
    Height = 520
    Center
    CREATE Menu AS RMainMenu
        CREATE FileMenu AS RMenuItem
            Caption = "&File"
            CREATE NewItem AS RMenuItem
                Caption = "&New": ShortCut = "Ctrl+N": OnClick = NewDocument
            END CREATE
            CREATE Sep1 AS RMenuItem
                Caption = "-"
            END CREATE
            CREATE ExitItem AS RMenuItem
                Caption = "E&xit": OnClick = QuitApp
            END CREATE
        END CREATE
        CREATE WindowMenu AS RMenuItem
            Caption = "&Window"
            CREATE CascadeItem AS RMenuItem
                Caption = "&Cascade": OnClick = CascadeWindows
            END CREATE
            CREATE TileItem AS RMenuItem
                Caption = "&Tile": OnClick = TileWindows
            END CREATE
        END CREATE
    END CREATE
END CREATE

' The text areas start hidden; a child window is made of one when
' File > New is chosen.
DIM k AS INTEGER
FOR k = 0 TO MaxDocs - 1
    Docs(k).Parent = Main
    Docs(k).Visible = False
NEXT

SUB NewDocument
    IF DocCount < MaxDocs THEN
        DocCount = DocCount + 1
        Docs(DocCount - 1).Text = "Document " + STR$(DocCount)
        Main.AddChild(Docs(DocCount - 1).Handle, "Document " + STR$(DocCount), DocCount - 1, 0, 0, 0, 0, 1)
    END IF
END SUB

SUB QuitApp
    Main.Close
END SUB

SUB CascadeWindows
    Main.CascadeChild
END SUB

SUB TileWindows
    Main.SetVertChild
END SUB

NewDocument
Main.ShowModal
"#;

impl Project {
    /// A project named `name` whose main file is `main` (added as a file,
    /// its kind taken from the extension).
    pub fn new(name: impl Into<String>, main: impl Into<String>) -> Self {
        let main = normalize_path(&main.into());
        let mut project = Project {
            name: name.into(),
            main: main.clone(),
            ..Project::default()
        };
        if !main.is_empty() {
            project.add_file(&main, kind_for_path(&main));
        }
        project
    }

    /// A new project from a template (see [`TEMPLATES`]: `console`, `gui`,
    /// `rapidq`, `data`, `mdi`), with the files to write as
    /// `(project path, contents)`. The main file is `main.rr`, or `main.bas`
    /// for the `rapidq` template (which also turns RapidQ compatibility on).
    pub fn new_from_template(
        name: &str,
        template: &str,
    ) -> Result<(Project, Vec<(String, String)>), ProjectError> {
        let title = basic_string(name);
        let mut rapidq = false;
        let mut main = "main.rr".to_string();
        let main_text = match template.to_ascii_lowercase().as_str() {
            "console" => format!(
                "$APPTYPE CONSOLE\n\nPRINT {}\n",
                basic_string(&format!("Hello from {name}!"))
            ),
            "gui" => format!(
                "$APPTYPE GUI\n\n\
                 CREATE Form1 AS RForm\n    \
                 Caption = {title}\n    \
                 Width = 480\n    \
                 Height = 320\n\
                 END CREATE\n\n\
                 Form1.ShowModal\n"
            ),
            "rapidq" => {
                rapidq = true;
                main = "main.bas".to_string();
                TEMPLATE_RAPIDQ.replace("{TITLE}", &title)
            }
            "data" => TEMPLATE_DATA.replace("{TITLE}", &title),
            "mdi" => TEMPLATE_MDI.replace("{TITLE}", &title),
            _ => return Err(ProjectError::UnknownTemplate(template.to_string())),
        };
        let mut project = Project {
            name: name.to_string(),
            main: main.clone(),
            ..Project::default()
        };
        project.compat.rapidq_compatible = rapidq;
        project.add_file(&main, kind_for_source(&main, &main_text));
        Ok((project, vec![(main, main_text)]))
    }

    /// Reads a format 2 project. A missing `format` is "not a RapidR
    /// project"; any format other than 2 is refused with a clear message.
    /// Unknown keys are ignored and missing sections take their defaults.
    pub fn from_toml(text: &str) -> Result<Project, ProjectError> {
        let table: toml::Table =
            toml::from_str(text).map_err(|e| ProjectError::Toml(e.to_string()))?;
        match table.get("format") {
            None => return Err(ProjectError::NotAProject),
            Some(toml::Value::Integer(n)) if *n == FORMAT => {}
            Some(toml::Value::Integer(n)) => return Err(ProjectError::UnsupportedFormat(*n)),
            Some(other) => {
                return Err(ProjectError::Toml(format!(
                    "`format` must be a number, found {}",
                    other.type_str()
                )))
            }
        }
        let mut project: Project = toml::Value::Table(table)
            .try_into()
            .map_err(|e: toml::de::Error| ProjectError::Toml(e.to_string()))?;
        project.main = normalize_path(&project.main);
        Ok(project)
    }

    /// The project as TOML, in a stable, readable order, starting with
    /// [`HEADER`].
    pub fn to_toml(&self) -> String {
        let body = toml::to_string(self).expect("a project always serializes to TOML");
        format!("{HEADER}\n\n{body}")
    }

    /// Reads a `.rrproj` (format 2) from disk.
    pub fn load(path: &Path) -> Result<Project, ProjectError> {
        let text = std::fs::read_to_string(path).map_err(|e| ProjectError::io(path, e))?;
        Project::from_toml(&text)
    }

    /// Writes the project to `path` as TOML.
    pub fn save(&self, path: &Path) -> Result<(), ProjectError> {
        std::fs::write(path, self.to_toml()).map_err(|e| ProjectError::io(path, e))
    }

    /// Adds a file (its path normalized to `/` separators). Returns false
    /// (and changes nothing) when the project already has it.
    pub fn add_file(&mut self, path: &str, kind: FileKind) -> bool {
        let path = normalize_path(path);
        if path.is_empty() || self.position(&path).is_some() {
            return false;
        }
        self.files.push(ProjectFile { path, kind });
        true
    }

    /// Removes a file; returns whether the project had it. Removing the main
    /// file also clears `main`.
    pub fn remove_file(&mut self, path: &str) -> bool {
        let path = normalize_path(path);
        match self.position(&path) {
            Some(i) => {
                self.files.remove(i);
                if self.main == path {
                    self.main.clear();
                }
                true
            }
            None => false,
        }
    }

    /// Whether the project has this file.
    pub fn contains(&self, path: &str) -> bool {
        self.position(&normalize_path(path)).is_some()
    }

    fn position(&self, normalized: &str) -> Option<usize> {
        self.files.iter().position(|f| f.path == normalized)
    }

    /// The number of files (RProject's `FileCount`).
    pub fn file_count(&self) -> usize {
        self.files.len()
    }

    /// The `i`th file, 0-based (RProject's `File(i)`).
    pub fn file(&self, i: usize) -> Option<&ProjectFile> {
        self.files.get(i)
    }

    /// The files of one kind, in project order.
    pub fn files_of_kind(&self, kind: FileKind) -> Vec<&ProjectFile> {
        self.files.iter().filter(|f| f.kind == kind).collect()
    }

    /// `"rapidq"` when the project must stay RapidQ-compatible, else `""`
    /// (RProject's `CompatMode`).
    pub fn compat_mode(&self) -> &'static str {
        if self.compat.rapidq_compatible {
            "rapidq"
        } else {
            ""
        }
    }
}

/// A project path in the file's form: `/` separators, no `./` steps, no
/// empty steps. Leading `..` steps are kept (a file beside the project).
pub fn normalize_path(path: &str) -> String {
    let unified = path.trim().replace('\\', "/");
    let absolute = unified.starts_with('/');
    let mut parts: Vec<&str> = Vec::new();
    for part in unified.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if matches!(parts.last(), Some(p) if *p != "..") {
                    parts.pop();
                } else if !absolute {
                    parts.push("..");
                }
            }
            p => parts.push(p),
        }
    }
    let joined = parts.join("/");
    if absolute {
        format!("/{joined}")
    } else {
        joined
    }
}

fn extension(path: &str) -> String {
    let name = path.rsplit('/').next().unwrap_or(path);
    match name.rfind('.') {
        Some(i) if i > 0 => name[i + 1..].to_ascii_lowercase(),
        _ => String::new(),
    }
}

/// The kind of a file by its path alone (`.rr` / `.bas` are `Module`; see
/// [`kind_for_source`] to recognize forms).
pub fn kind_for_path(path: &str) -> FileKind {
    let path = normalize_path(path);
    let in_assets = path
        .split('/')
        .next()
        .is_some_and(|first| first.eq_ignore_ascii_case("assets"))
        && path.contains('/');
    match extension(&path).as_str() {
        // (RapidQ's `.rqw` window programs, `.rqb` / `.rq` libraries)
        "rr" | "bas" | "rqw" | "rqb" | "rq" => FileKind::Module,
        "inc" => FileKind::Include,
        "ico" | "bmp" | "png" | "jpg" | "jpeg" | "gif" | "wav" | "mid" | "midi" | "avi" => {
            if in_assets {
                FileKind::Asset
            } else {
                FileKind::Resource
            }
        }
        "csv" | "json" | "txt" | "db" | "sqlite" | "parquet" => FileKind::Data,
        _ => FileKind::Asset,
    }
}

/// The kind of a file from its path and text: a `.rr` / `.bas` file that
/// creates a form at the top level is a `Form`.
pub fn kind_for_source(path: &str, text: &str) -> FileKind {
    match kind_for_path(path) {
        FileKind::Module if defines_form(text) => FileKind::Form,
        kind => kind,
    }
}

/// The kind of a file on disk (reads `.rr` / `.bas` files to recognize
/// forms; a file that cannot be read is taken by its extension).
pub fn kind_for_file(path: &Path) -> FileKind {
    let name = path.to_string_lossy();
    match kind_for_path(&name) {
        FileKind::Module => match std::fs::read(path) {
            Ok(bytes) => kind_for_source(&name, &String::from_utf8_lossy(&bytes)),
            Err(_) => FileKind::Module,
        },
        kind => kind,
    }
}

/// Whether the source creates a form at the top level
/// (`CREATE x AS QFORM | RFORM | QFORMMDI | RFORMMDI`, any case), by a
/// simple line scan that tracks `CREATE` / `END CREATE` nesting.
pub fn defines_form(text: &str) -> bool {
    let mut depth = 0usize;
    for line in text.lines() {
        let mut words = line.split_whitespace();
        let Some(first) = words.next() else { continue };
        if first.eq_ignore_ascii_case("CREATE") {
            if depth == 0 {
                let _name = words.next();
                let as_kw = words.next();
                let ty = words.next().map(|t| {
                    t.chars()
                        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_')
                        .collect::<String>()
                        .to_ascii_uppercase()
                });
                if as_kw.is_some_and(|w| w.eq_ignore_ascii_case("AS"))
                    && matches!(
                        ty.as_deref(),
                        Some("QFORM" | "RFORM" | "QFORMMDI" | "RFORMMDI")
                    )
                {
                    return true;
                }
            }
            depth += 1;
        } else if first.eq_ignore_ascii_case("END")
            && words
                .next()
                .is_some_and(|w| w.eq_ignore_ascii_case("CREATE"))
        {
            depth = depth.saturating_sub(1);
        }
    }
    false
}

/// A BASIC string literal for `s`. RapidQ has no `""` escape, so a quote
/// is joined in as `CHR$(34)`: `"Say " + CHR$(34) + "hi" + CHR$(34) + ""`.
pub fn basic_string(s: &str) -> String {
    s.split('"')
        .map(|p| format!("\"{p}\""))
        .collect::<Vec<_>>()
        .join(" + CHR$(34) + ")
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Project {
        let mut p = Project::new("Demo", "Demo.rr");
        p.add_file("Form1.rr", FileKind::Form);
        p.add_file("lib\\util.inc", FileKind::Include);
        p.add_file("assets/logo.png", FileKind::Asset);
        p.add_file("data/people.csv", FileKind::Data);
        p.build.apptype = "GUI".into();
        p.build.icon = "app.ico".into();
        p.build.app_name = "Demo App".into();
        p.build.bundle_id = "com.example.demo".into();
        p.build.version = "2.1".into();
        p.build.company = "Example Ltd".into();
        p.build.targets = vec!["native".into(), "web".into()];
        p.build.release = true;
        p.run.args = vec!["-v".into(), "a b".into()];
        p.run.cwd = "data".into();
        p.run.separate_windows = true;
        p.compat.rapidq_compatible = true;
        p.compat.level = CompatLevel::Error;
        p.designer.grid = 4;
        p.designer.snap = false;
        p.ai.allowed = true;
        p.ai.context_exclude = vec!["secrets/**".into()];
        p.ai.data_access = DataAccess::Sample;
        p.ai.sample_rows = 5;
        p.ai.redact_columns = vec!["password".into()];
        p
    }

    #[test]
    fn toml_round_trip() {
        let p = sample();
        let text = p.to_toml();
        assert!(text.starts_with("# RapidR Studio project (format 2)\n"));
        assert!(text.contains("format = 2"));
        assert!(text.contains("path = \"lib/util.inc\""));
        assert!(text.contains("kind = \"include\""));
        assert!(text.contains("data_access = \"sample\""));
        assert!(text.contains("level = \"error\""));
        let back = Project::from_toml(&text).unwrap();
        assert_eq!(back, p);
        assert_eq!(back.to_toml(), text, "stable output");
        // format, name and main come first.
        let first_keys: Vec<&str> = text
            .lines()
            .filter(|l| !l.starts_with('#') && !l.is_empty())
            .take(3)
            .collect();
        assert_eq!(
            first_keys,
            ["format = 2", "name = \"Demo\"", "main = \"Demo.rr\""]
        );
    }

    #[test]
    fn defaults_when_sections_missing() {
        let p = Project::from_toml("format = 2\nname = \"X\"\nmain = \"x.rr\"\n").unwrap();
        assert_eq!(p.name, "X");
        assert!(p.files.is_empty());
        assert_eq!(p.build, Build::default());
        assert_eq!(p.designer.grid, 8);
        assert!(p.designer.snap && p.designer.guides);
        assert!(!p.ai.allowed);
        assert_eq!(p.ai.data_access, DataAccess::Schema);
        assert_eq!(p.ai.sample_rows, 20);
        assert_eq!(p.compat.level, CompatLevel::Warn);
        assert_eq!(p.compat_mode(), "");

        // A partial section keeps the other defaults.
        let p = Project::from_toml("format = 2\n[designer]\ngrid = 10\n[ai]\nallowed = true\n")
            .unwrap();
        assert_eq!(p.designer.grid, 10);
        assert!(p.designer.snap);
        assert!(p.ai.allowed);
        assert_eq!(p.ai.sample_rows, 20);
    }

    #[test]
    fn unknown_keys_ignored() {
        let text = "format = 2\nname = \"X\"\nfuture = 1\n\
                    [[files]]\npath = \"a.rr\"\nkind = \"module\"\ncolor = \"red\"\n\
                    [[files]]\npath = \"pic.bmp\"\n\
                    [build]\nrelease = true\nturbo = true\n\
                    [plugins]\nx = 1\n";
        let p = Project::from_toml(text).unwrap();
        assert_eq!(p.file_count(), 2);
        assert_eq!(
            p.file(1).unwrap().kind,
            FileKind::Resource,
            "kind from extension"
        );
        assert!(p.build.release);
    }

    #[test]
    fn format_checks() {
        let err = Project::from_toml("format = 3\nname = \"X\"\n").unwrap_err();
        assert_eq!(err, ProjectError::UnsupportedFormat(3));
        assert!(err.to_string().contains("newer RapidR Studio"), "{err}");
        let err = Project::from_toml("name = \"X\"\n").unwrap_err();
        assert_eq!(err, ProjectError::NotAProject);
        assert!(err.to_string().contains("not a RapidR project"));
        assert!(matches!(
            Project::from_toml("format = 1\n").unwrap_err(),
            ProjectError::UnsupportedFormat(1)
        ));
        assert!(matches!(
            Project::from_toml("format = \"2\"\n").unwrap_err(),
            ProjectError::Toml(_)
        ));
        assert!(matches!(
            Project::from_toml("format = = 2").unwrap_err(),
            ProjectError::Toml(_)
        ));
    }

    #[test]
    fn file_list_api() {
        let mut p = sample();
        assert_eq!(p.file_count(), 5);
        assert_eq!(p.file(0).unwrap().path, "Demo.rr");
        assert!(!p.add_file("./Form1.rr", FileKind::Form), "already there");
        assert_eq!(p.files_of_kind(FileKind::Include)[0].path, "lib/util.inc");
        assert!(p.remove_file("lib/util.inc"));
        assert!(!p.remove_file("lib/util.inc"));
        assert!(p.files_of_kind(FileKind::Include).is_empty());
        assert!(p.remove_file("Demo.rr"));
        assert_eq!(p.main, "");
        assert_eq!(p.compat_mode(), "rapidq");
    }

    #[test]
    fn kinds() {
        assert_eq!(kind_for_path("a.rr"), FileKind::Module);
        assert_eq!(kind_for_path("A.BAS"), FileKind::Module);
        assert_eq!(kind_for_path("x.inc"), FileKind::Include);
        assert_eq!(kind_for_path("icon.ico"), FileKind::Resource);
        assert_eq!(kind_for_path("assets/icon.ico"), FileKind::Asset);
        assert_eq!(kind_for_path("Assets\\snd.WAV"), FileKind::Asset);
        assert_eq!(kind_for_path("people.csv"), FileKind::Data);
        assert_eq!(kind_for_path("x.parquet"), FileKind::Data);
        let form = "' a form\n$APPTYPE GUI\ncreate Form1 as qform\n  caption = \"x\"\nend create\n";
        assert!(defines_form(form));
        assert_eq!(kind_for_source("f.bas", form), FileKind::Form);
        assert!(defines_form("CREATE Main AS RFormMDI\nEND CREATE"));
        assert!(defines_form("CREATE F AS QFORM ' comment"));
        assert!(!defines_form("CREATE B AS QBUTTON\nEND CREATE"));
        assert!(!defines_form(
            "CREATE P AS QPANEL\n CREATE F AS QFORM\n END CREATE\nEND CREATE"
        ));
        assert!(!defines_form("' CREATE F AS QFORM\nPRINT 1"));
        assert_eq!(kind_for_source("x.inc", form), FileKind::Include);
    }

    #[test]
    fn paths_normalize() {
        assert_eq!(normalize_path("a\\b\\..\\c.rr"), "a/c.rr");
        assert_eq!(normalize_path("./x//y.rr"), "x/y.rr");
        assert_eq!(normalize_path("../common/x.inc"), "../common/x.inc");
    }

    #[test]
    fn templates() {
        let (p, files) = Project::new_from_template("Hello", "console").unwrap();
        assert_eq!(p.main, "main.rr");
        assert_eq!(p.file_count(), 1);
        assert_eq!(files[0].0, "main.rr");
        assert!(files[0].1.contains("PRINT \"Hello from Hello!\""));

        let (p, files) = Project::new_from_template("My \"App\"", "GUI").unwrap();
        let text = &files[0].1;
        assert!(text.starts_with("$APPTYPE GUI\n"));
        assert!(text.contains("CREATE Form1 AS RForm"));
        assert!(text.contains("Caption = \"My \" + CHR$(34) + \"App\" + CHR$(34) + \"\""));
        assert!(text.contains("Width = 480") && text.contains("Height = 320"));
        assert!(text.trim_end().ends_with("Form1.ShowModal"));
        assert_eq!(p.file(0).unwrap().kind, FileKind::Form);

        for id in TEMPLATES {
            let (p, files) = Project::new_from_template("T", id).unwrap();
            assert!(!p.main.is_empty() && p.contains(&p.main), "{id}");
            assert!(files.iter().any(|(path, _)| *path == p.main), "{id}");
            let expected = if *id == "rapidq" { "main.bas" } else { "main.rr" };
            assert_eq!(p.main, expected, "{id}");
            assert_eq!(p.compat_mode(), if *id == "rapidq" { "rapidq" } else { "" });
        }

        // The RapidQ template sticks to RapidQ: no RapidR-only names, no
        // `""` escapes, no line continuations, SUBs declared before use.
        let (p, files) = Project::new_from_template("My \"App\"", "rapidq").unwrap();
        let text = &files[0].1;
        assert_eq!(p.file(0).unwrap().kind, FileKind::Form);
        assert!(text.contains("CREATE Form1 AS QFORM") && text.contains("QEDIT"));
        assert!(text.contains("Caption = \"My \" + CHR$(34) + \"App\" + CHR$(34) + \"\""));
        assert!(!text.contains("RFORM") && !text.contains(" _\n") && !text.contains("\"\"\""));
        assert!(text.find("DECLARE SUB ShowText") < text.find("CREATE Form1"));

        // The other form templates make a form (so the project tree shows
        // it as one); the data template has a grid and a chart, the MDI one
        // an MDI parent with a main menu.
        let (p, files) = Project::new_from_template("Dash", "data").unwrap();
        assert_eq!(p.file(0).unwrap().kind, FileKind::Form);
        assert!(files[0].1.contains("AS RStringGrid") && files[0].1.contains("AS RPlot"));
        let (p, files) = Project::new_from_template("Docs", "mdi").unwrap();
        assert_eq!(p.file(0).unwrap().kind, FileKind::Form);
        assert!(files[0].1.contains("AS RFormMDI") && files[0].1.contains("AS RMainMenu"));
        // (RapidR's names in every template but the RapidQ one: R-NAMES)
        for id in TEMPLATES.iter().filter(|t| **t != "rapidq") {
            let (_, files) = Project::new_from_template("T", id).unwrap();
            assert!(!files[0].1.contains(" AS Q"), "{id}: {}", files[0].1);
        }
        assert!(!files[0].1.contains("{TITLE}"));

        assert_eq!(
            Project::new_from_template("x", "wizard").unwrap_err(),
            ProjectError::UnknownTemplate("wizard".into())
        );
    }

    #[test]
    fn save_and_load() {
        let dir = std::env::temp_dir().join(format!("rapidr-project-save-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("Demo.rrproj");
        let p = sample();
        p.save(&path).unwrap();
        assert_eq!(Project::load(&path).unwrap(), p);
        let missing = Project::load(&dir.join("nope.rrproj")).unwrap_err();
        assert!(matches!(missing, ProjectError::Io { .. }));
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
