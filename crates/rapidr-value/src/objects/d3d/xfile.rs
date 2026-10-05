//! DirectX `.x` model files: the format Direct3D Retained Mode's
//! `MeshBuilder.Load` and `Frame.Load` read meshes and frame hierarchies from
//! (D3DX kept it). A file is a 16-byte header — `xof `, a version (`0302`,
//! `0303`), a format and a float size — followed by data objects, each an
//! instance of a template: `Frame`, `Mesh`, `Material` and so on, nested in
//! braces, or `{ Name }` references to a named object.
//!
//! Supported: the text format (`txt `) and the binary format (`bin `, a
//! little-endian token stream) with 32- or 64-bit floats (`0032`, `0064`; a
//! text file's numbers are read as written whatever its header says). The
//! compressed formats (`tzip`, `bzip`) are refused with an error that says so.
//!
//! Read: top-level `Frame`s — their name, `FrameTransformMatrix`, meshes
//! (inline, or `{ Name }` references to a named `Mesh`) and child frames —
//! and top-level `Mesh`es; in each mesh its vertices and polygon faces,
//! `MeshNormals`, `MeshTextureCoords`, `MeshVertexColors` and
//! `MeshMaterialList`, whose `Material`s are inline or `{ Name }` references
//! to a named material anywhere in the file, each with its `TextureFilename`.
//! The standard templates' names match without regard to case
//! (`TextureFileName` is common).
//!
//! Skipped: `template` declarations (the standard templates' layouts are
//! built in; others aren't interpreted), `Header`, and every other object —
//! `AnimationSet`, `VertexDuplicationIndices`, `XSkinMeshHeader`,
//! `SkinWeights`, `DeclData`, `MeshFaceWraps` … — at any depth, by matching
//! braces.
//!
//! Tolerated, as old exporters wrote them: `,` and `;` are both just
//! separators (`;;`, `,;` and trailing commas are fine), `//` and `#`
//! comments, a material list with fewer face indices than faces (the last
//! one repeats, as Direct3D did), normals and texture coordinates whose
//! counts don't fit the mesh (see [`XMesh`]'s fields), faces of fewer than
//! three vertices (left out). Refused with an `Err`, never a panic: a damaged
//! header, an object whose data stops before the counts it declared, faces
//! that use vertices the mesh doesn't have, a file that ends inside an
//! object, binary tokens cut short.

use std::collections::{HashMap, HashSet, VecDeque};
use std::fmt::Display;

/// A material (the `Material` template).
#[derive(Debug, Clone, PartialEq)]
pub struct XMaterial {
    /// `faceColor`: red, green, blue, alpha, 0 … 1.
    pub color: [f32; 4],
    /// `power`: the specular exponent.
    pub power: f32,
    /// `specularColor`: red, green, blue.
    pub specular: [f32; 3],
    /// `emissiveColor`: red, green, blue.
    pub emissive: [f32; 3],
    /// The nested `TextureFilename`'s name, as written (usually relative to
    /// the .x file).
    pub texture: Option<String>,
}

impl Default for XMaterial {
    /// Opaque white, no shine, no texture: what faces without a material of
    /// their own get.
    fn default() -> Self {
        XMaterial { color: [1.0; 4], power: 0.0, specular: [0.0; 3], emissive: [0.0; 3], texture: None }
    }
}

/// A mesh (the `Mesh` template and the objects nested in it).
#[derive(Debug, Clone, PartialEq, Default)]
pub struct XMesh {
    pub positions: Vec<[f32; 3]>,
    /// Vertex indices per face: polygons of three or more vertices, as in the
    /// file (Direct3D's clockwise front faces), not triangulated.
    pub faces: Vec<Vec<usize>>,
    /// `MeshNormals`' normals, as stored; empty if the mesh has none, or if
    /// its normal faces don't match its faces.
    pub normals: Vec<[f32; 3]>,
    /// `MeshNormals`' normal indices per face: empty, or the same shape as
    /// `faces`.
    pub face_normals: Vec<Vec<usize>>,
    /// `MeshTextureCoords`: none, or one (u, v) per vertex (a list that is
    /// short is padded with (0, 0), one that is long cut).
    pub texcoords: Vec<[f32; 2]>,
    /// `MeshVertexColors`: none, or one RGBA per vertex (vertices the list
    /// doesn't name are white).
    pub colors: Vec<[f32; 4]>,
    /// `MeshMaterialList`'s materials, in order.
    pub materials: Vec<XMaterial>,
    /// A material index per face; empty when `materials` is.
    pub face_materials: Vec<usize>,
}

/// A frame (the `Frame` template): a coordinate system for its meshes and
/// child frames.
#[derive(Debug, Clone, PartialEq)]
pub struct XFrame {
    pub name: String,
    /// `FrameTransformMatrix`, row-major as stored. Direct3D's row vectors:
    /// `v' = v * M`, the translation in the last row. Identity if absent.
    pub transform: [f32; 16],
    pub meshes: Vec<XMesh>,
    pub children: Vec<XFrame>,
}

impl Default for XFrame {
    fn default() -> Self {
        XFrame { name: String::new(), transform: IDENTITY, meshes: Vec::new(), children: Vec::new() }
    }
}

/// What a .x file holds.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct XFile {
    /// The top-level frames.
    pub frames: Vec<XFrame>,
    /// The top-level meshes, but for those that frames use by reference
    /// (they are in those frames).
    pub meshes: Vec<XMesh>,
}

const IDENTITY: [f32; 16] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0];

/// How deep the objects read may nest (a frame in a frame in …).
const MAX_DEPTH: usize = 128;

fn bad(why: impl Display) -> String {
    format!("not a DirectX .x file RapidR can read ({why})")
}

/// Reads a .x file: its header, then its objects.
pub fn parse_x(bytes: &[u8]) -> Result<XFile, String> {
    let mut parser = Parser { lexer: lexer(bytes)?, ahead: VecDeque::new() };
    let mut root = Obj::new(Kind::Root, String::new());
    parser.body(&mut root, 0)?;
    build(&root)
}

/// Every mesh of the file in one, each transformed by its frames' matrices
/// (child's, then parent's ... : v * M_child * M_parent), normals transformed by
/// the matrices' rotation part and renormalized; indices and material indices
/// offset. What Direct3D RM's MeshBuilder.Load gives.
///
/// Top-level meshes come first, then the frames' in depth-first order. When
/// only some meshes carry normals, texture coordinates, colours or materials,
/// the others are filled in so the arrays stay aligned: a flat normal per
/// face, (0, 0), white, and a default white material.
pub fn flatten(file: &XFile) -> XMesh {
    let mut parts: Vec<(&XMesh, [f32; 16])> = file.meshes.iter().map(|mesh| (mesh, IDENTITY)).collect();
    for frame in &file.frames {
        gather(frame, &IDENTITY, &mut parts);
    }
    merge(&parts)
}

// ---------------------------------------------------------------------------
// Tokens, from either format

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    /// An identifier: a template's name, an object's name, a reference.
    Name(String),
    Str(String),
    /// A number; a binary list gives one per element.
    Num(f64),
    Open,
    Close,
    /// The `template` keyword.
    Template,
}

fn lexer(bytes: &[u8]) -> Result<Lexer<'_>, String> {
    if bytes.len() < 16 {
        return Err(bad(format!("{} bytes, too short for the 16-byte header", bytes.len())));
    }
    if !bytes.starts_with(b"xof ") {
        return Err(bad("it doesn't start with \"xof \""));
    }
    let format = &bytes[8..12];
    let src = &bytes[16..];
    match &format.to_ascii_lowercase()[..] {
        b"txt " => Ok(Lexer::Text(TextLexer { src, pos: 0 })),
        b"bin " => {
            let double = match &bytes[12..16] {
                b"0032" => false,
                b"0064" => true,
                other => return Err(bad(format!("binary with float size {:?}; only 0032 and 0064 are defined", text(other)))),
            };
            Ok(Lexer::Binary(BinaryLexer { src, pos: 0, double, list_left: 0, list_floats: false }))
        }
        b"tzip" | b"bzip" => {
            Err(bad(format!("it is compressed ({:?}); RapidR reads the uncompressed formats, so save it as \"txt \" or \"bin \"", text(format))))
        }
        _ => Err(bad(format!("unknown format {:?} in the header; expected \"txt \" or \"bin \"", text(format)))),
    }
}

enum Lexer<'a> {
    Text(TextLexer<'a>),
    Binary(BinaryLexer<'a>),
}

impl Lexer<'_> {
    fn next_tok(&mut self) -> Result<Option<Tok>, String> {
        match self {
            Lexer::Text(lexer) => lexer.next_tok(),
            Lexer::Binary(lexer) => lexer.next_tok(),
        }
    }
}

/// Bytes as a string: UTF-8 if they are, else Latin-1. Trailing NULs (some
/// writers count the terminator) are dropped.
fn text(bytes: &[u8]) -> String {
    let end = bytes.iter().rposition(|&b| b != 0).map_or(0, |i| i + 1);
    let bytes = &bytes[..end];
    match std::str::from_utf8(bytes) {
        Ok(s) => s.to_owned(),
        Err(_) => bytes.iter().map(|&b| char::from(b)).collect(),
    }
}

/// The text format. Separators (`,` `;`), brackets, GUIDs and anything else
/// that isn't a brace, a word, a number or a string are passed over.
struct TextLexer<'a> {
    src: &'a [u8],
    pos: usize,
}

fn is_word_byte(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'_' | b'-' | b'+' | b'.') || c >= 0x80
}

impl TextLexer<'_> {
    /// The line `pos` is on, counting the header's as line 1.
    fn line(&self) -> usize {
        1 + self.src[..self.pos.min(self.src.len())].iter().filter(|&&c| c == b'\n').count()
    }

    fn next_tok(&mut self) -> Result<Option<Tok>, String> {
        let src = self.src;
        while let Some(&c) = src.get(self.pos) {
            let start = self.pos;
            match c {
                b'{' => {
                    self.pos += 1;
                    return Ok(Some(Tok::Open));
                }
                b'}' => {
                    self.pos += 1;
                    return Ok(Some(Tok::Close));
                }
                b'#' => self.skip_line(),
                b'/' if src.get(start + 1) == Some(&b'/') => self.skip_line(),
                b'"' => {
                    let Some(len) = src[start + 1..].iter().position(|&c| c == b'"') else {
                        return Err(bad(format!("the string on line {} has no closing quote", self.line())));
                    };
                    self.pos = start + len + 2;
                    return Ok(Some(Tok::Str(text(&src[start + 1..start + 1 + len]))));
                }
                b'<' => {
                    // A GUID, naming a template's type: nothing to read.
                    let Some(len) = src[start..].iter().position(|&c| c == b'>') else {
                        return Err(bad(format!("the <GUID> on line {} has no closing >", self.line())));
                    };
                    self.pos = start + len + 1;
                }
                _ if is_word_byte(c) => return Ok(Some(self.word())),
                _ => self.pos += 1,
            }
        }
        Ok(None)
    }

    fn skip_line(&mut self) {
        self.pos = match self.src[self.pos..].iter().position(|&c| c == b'\n') {
            Some(len) => self.pos + len + 1,
            None => self.src.len(),
        };
    }

    /// A number, a name, or the `template` keyword.
    fn word(&mut self) -> Tok {
        let src = self.src;
        let start = self.pos;
        let numeric = src[start].is_ascii_digit() || matches!(src[start], b'-' | b'+' | b'.');
        let mut end = start;
        while let Some(&c) = src.get(end) {
            if !(is_word_byte(c) || (numeric && c == b'#')) {
                break;
            }
            end += 1;
        }
        self.pos = end;
        let word = &src[start..end];
        if numeric {
            if let Some(v) = std::str::from_utf8(word).ok().and_then(|s| s.parse::<f64>().ok()) {
                return Tok::Num(v);
            }
            if word.contains(&b'#') && word.iter().any(u8::is_ascii_digit) {
                // Visual C++'s printf spelling of infinities and NaNs
                // (1.#INF00, -1.#IND00, 1.#QNAN0): no usable value.
                return Tok::Num(0.0);
            }
        }
        let word = text(word);
        if word == "template" {
            Tok::Template
        } else {
            Tok::Name(word)
        }
    }
}

/// The binary format: a u16 token, some with data after it.
struct BinaryLexer<'a> {
    src: &'a [u8],
    pos: usize,
    /// FLOAT_LIST elements are 8 bytes, not 4.
    double: bool,
    /// Numbers left in the INTEGER_LIST or FLOAT_LIST being read.
    list_left: usize,
    list_floats: bool,
}

impl<'a> BinaryLexer<'a> {
    fn take(&mut self, n: usize) -> Result<&'a [u8], String> {
        match self.pos.checked_add(n) {
            Some(end) if end <= self.src.len() => {
                let bytes = &self.src[self.pos..end];
                self.pos = end;
                Ok(bytes)
            }
            _ => Err(bad(format!("the binary data stops in the middle of a token at byte {}", self.src.len() + 16))),
        }
    }

    fn u16(&mut self) -> Result<u16, String> {
        let b = self.take(2)?;
        Ok(u16::from_le_bytes([b[0], b[1]]))
    }

    fn u32(&mut self) -> Result<u32, String> {
        let b = self.take(4)?;
        Ok(u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
    }

    /// A u32 count, then that many bytes.
    fn counted(&mut self) -> Result<&'a [u8], String> {
        let n = self.u32()? as usize;
        self.take(n)
    }

    fn next_tok(&mut self) -> Result<Option<Tok>, String> {
        loop {
            if self.list_left > 0 {
                self.list_left -= 1;
                let v = if !self.list_floats {
                    f64::from(self.u32()?)
                } else if self.double {
                    let b = self.take(8)?;
                    f64::from_le_bytes([b[0], b[1], b[2], b[3], b[4], b[5], b[6], b[7]])
                } else {
                    let b = self.take(4)?;
                    f64::from(f32::from_le_bytes([b[0], b[1], b[2], b[3]]))
                };
                return Ok(Some(Tok::Num(v)));
            }
            if self.src.len() - self.pos < 2 {
                return Ok(None); // the end, or a byte of padding
            }
            let at = self.pos + 16;
            let token = self.u16()?;
            let tok = match token {
                1 => Tok::Name(text(self.counted()?)),
                // Its terminator, a SEMICOLON or COMMA token, follows and is
                // passed over as any separator.
                2 => Tok::Str(text(self.counted()?)),
                3 => Tok::Num(f64::from(self.u32()?)),
                5 => {
                    self.take(16)?; // a GUID
                    continue;
                }
                6 | 7 => {
                    let n = self.u32()? as usize;
                    let size = if token == 7 && self.double { 8 } else { 4 };
                    match n.checked_mul(size) {
                        Some(bytes) if bytes <= self.src.len() - self.pos => {}
                        _ => return Err(bad(format!("the list at byte {at} declares {n} numbers but the file ends first"))),
                    }
                    self.list_left = n;
                    self.list_floats = token == 7;
                    continue;
                }
                10 => Tok::Open,
                11 => Tok::Close,
                31 => Tok::Template,
                // Parentheses, brackets, angle brackets, dot, comma,
                // semicolon, and the type keywords of template declarations.
                12..=20 | 40..=52 => continue,
                0 if self.src[self.pos..].iter().all(|&b| b == 0) => return Ok(None), // zero padding
                _ => return Err(bad(format!("unknown binary token {token} at byte {at}"))),
            };
            return Ok(Some(tok));
        }
    }
}

// ---------------------------------------------------------------------------
// Objects: the templates read, in a tree

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Kind {
    Root,
    Frame,
    FrameTransformMatrix,
    Mesh,
    MeshNormals,
    MeshTextureCoords,
    MeshVertexColors,
    MeshMaterialList,
    Material,
    TextureFilename,
}

const KINDS: [(&str, Kind); 9] = [
    ("Frame", Kind::Frame),
    ("FrameTransformMatrix", Kind::FrameTransformMatrix),
    ("Mesh", Kind::Mesh),
    ("MeshNormals", Kind::MeshNormals),
    ("MeshTextureCoords", Kind::MeshTextureCoords),
    ("MeshVertexColors", Kind::MeshVertexColors),
    ("MeshMaterialList", Kind::MeshMaterialList),
    ("Material", Kind::Material),
    ("TextureFilename", Kind::TextureFilename),
];

impl Kind {
    fn of(word: &str) -> Option<Kind> {
        KINDS.iter().find(|(name, _)| name.eq_ignore_ascii_case(word)).map(|&(_, kind)| kind)
    }

    fn name(self) -> &'static str {
        KINDS.iter().find(|&&(_, kind)| kind == self).map_or("file", |&(name, _)| name)
    }
}

/// An object of a template this module reads: its numbers and strings in
/// order (separators gone, binary lists spread out), and what it nests.
struct Obj {
    kind: Kind,
    name: String,
    nums: Vec<f64>,
    /// Strings, and stray words that start no object.
    words: Vec<String>,
    children: Vec<Child>,
}

enum Child {
    Obj(Obj),
    /// `{ Name }`
    Ref(String),
}

impl Obj {
    fn new(kind: Kind, name: String) -> Self {
        Obj { kind, name, nums: Vec::new(), words: Vec::new(), children: Vec::new() }
    }

    /// For messages: `Mesh "box"`, or `a Mesh`.
    fn label(&self) -> String {
        if self.name.is_empty() {
            format!("a {}", self.kind.name())
        } else {
            format!("{} {:?}", self.kind.name(), self.name)
        }
    }

    fn first(&self, kind: Kind) -> Option<&Obj> {
        self.children.iter().find_map(|child| match child {
            Child::Obj(obj) if obj.kind == kind => Some(obj),
            _ => None,
        })
    }
}

struct Parser<'a> {
    lexer: Lexer<'a>,
    ahead: VecDeque<Tok>,
}

impl Parser<'_> {
    fn fill(&mut self, n: usize) -> Result<(), String> {
        while self.ahead.len() < n {
            match self.lexer.next_tok()? {
                Some(tok) => self.ahead.push_back(tok),
                None => break,
            }
        }
        Ok(())
    }

    fn next(&mut self) -> Result<Option<Tok>, String> {
        self.fill(1)?;
        Ok(self.ahead.pop_front())
    }

    fn peek_is(&mut self, i: usize, want: fn(&Tok) -> bool) -> Result<bool, String> {
        self.fill(i + 1)?;
        Ok(self.ahead.get(i).is_some_and(want))
    }

    /// An object's contents up to its closing brace (or the file's, to its
    /// end): `Type [name] { … }` objects, `{ Name }` references, numbers and
    /// strings. Objects of templates not read are skipped.
    fn body(&mut self, obj: &mut Obj, depth: usize) -> Result<(), String> {
        let top = obj.kind == Kind::Root;
        loop {
            let Some(tok) = self.next()? else {
                return if top { Ok(()) } else { Err(bad(format!("the file ends inside {}", obj.label()))) };
            };
            match tok {
                Tok::Close if top => {} // a stray closing brace between objects
                Tok::Close => return Ok(()),
                Tok::Num(v) if !top => obj.nums.push(v),
                Tok::Str(s) if !top => obj.words.push(s),
                Tok::Num(_) | Tok::Str(_) => {}
                Tok::Template => self.skip_template()?,
                Tok::Open => {
                    if let Some(name) = self.reference()? {
                        obj.children.push(Child::Ref(name));
                    }
                }
                Tok::Name(word) => {
                    let name = if self.peek_is(0, |t| matches!(t, Tok::Open))? {
                        String::new()
                    } else if self.peek_is(0, |t| matches!(t, Tok::Name(_)))? && self.peek_is(1, |t| matches!(t, Tok::Open))? {
                        match self.next()? {
                            Some(Tok::Name(name)) => name,
                            _ => String::new(),
                        }
                    } else {
                        if !top {
                            obj.words.push(word); // a word that starts no object
                        }
                        continue;
                    };
                    self.next()?; // the {
                    match Kind::of(&word) {
                        Some(_) if depth >= MAX_DEPTH => return Err(bad(format!("objects nest more than {MAX_DEPTH} deep"))),
                        Some(kind) => {
                            let mut child = Obj::new(kind, name);
                            self.body(&mut child, depth + 1)?;
                            obj.children.push(Child::Obj(child));
                        }
                        None => self.skip(&format!("the {word} object"))?,
                    }
                }
            }
        }
    }

    /// Passes over the rest of a block whose `{` has been read.
    fn skip(&mut self, what: &str) -> Result<(), String> {
        let mut depth = 1usize;
        while depth > 0 {
            match self.next()? {
                Some(Tok::Open) => depth += 1,
                Some(Tok::Close) => depth -= 1,
                Some(_) => {}
                None => return Err(bad(format!("the file ends inside {what}"))),
            }
        }
        Ok(())
    }

    /// `template Name { <GUID> members… }`, after the keyword.
    fn skip_template(&mut self) -> Result<(), String> {
        loop {
            match self.next()? {
                Some(Tok::Open) => return self.skip("a template declaration"),
                Some(_) => {}
                None => return Err(bad("the file ends inside a template declaration")),
            }
        }
    }

    /// `{ Name [<GUID>] }`, after the `{`: the name, if there is one.
    fn reference(&mut self) -> Result<Option<String>, String> {
        let mut name = None;
        loop {
            match self.next()? {
                Some(Tok::Close) => return Ok(name),
                Some(Tok::Name(n)) => {
                    name.get_or_insert(n);
                }
                Some(Tok::Open) => self.skip("a { } block")?,
                Some(_) => {}
                None => return Err(bad("the file ends inside a { reference }")),
            }
        }
    }
}

// ---------------------------------------------------------------------------
// From the tree to frames and meshes

/// Named objects, to resolve `{ Name }` references: an exact match first,
/// then one without regard to case. The first object of a name wins.
#[derive(Default)]
struct Named<'t> {
    exact: HashMap<&'t str, &'t Obj>,
    folded: HashMap<String, &'t Obj>,
}

impl<'t> Named<'t> {
    fn add(&mut self, obj: &'t Obj) {
        self.exact.entry(obj.name.as_str()).or_insert(obj);
        self.folded.entry(obj.name.to_ascii_lowercase()).or_insert(obj);
    }

    fn get(&self, name: &str) -> Option<&'t Obj> {
        self.exact.get(name).or_else(|| self.folded.get(&name.to_ascii_lowercase())).copied()
    }
}

#[derive(Default)]
struct Index<'t> {
    materials: Named<'t>,
    meshes: Named<'t>,
}

impl<'t> Index<'t> {
    fn collect(&mut self, obj: &'t Obj) {
        for child in &obj.children {
            if let Child::Obj(child) = child {
                if !child.name.is_empty() {
                    match child.kind {
                        Kind::Material => self.materials.add(child),
                        Kind::Mesh => self.meshes.add(child),
                        _ => {}
                    }
                }
                self.collect(child);
            }
        }
    }

    /// The meshes frames use by reference.
    fn referenced(&self, obj: &'t Obj, used: &mut HashSet<*const Obj>) {
        for child in &obj.children {
            match child {
                Child::Obj(child) if child.kind == Kind::Frame => self.referenced(child, used),
                Child::Ref(name) if obj.kind == Kind::Frame => {
                    if let Some(mesh) = self.meshes.get(name) {
                        used.insert(mesh);
                    }
                }
                _ => {}
            }
        }
    }

    fn frame(&self, obj: &Obj) -> Result<XFrame, String> {
        let mut frame = XFrame { name: obj.name.clone(), ..XFrame::default() };
        if let Some(matrix) = obj.first(Kind::FrameTransformMatrix) {
            frame.transform = Reader::new(matrix).floats()?;
        }
        for child in &obj.children {
            match child {
                Child::Obj(child) if child.kind == Kind::Mesh => frame.meshes.push(self.mesh(child)?),
                Child::Obj(child) if child.kind == Kind::Frame => frame.children.push(self.frame(child)?),
                Child::Ref(name) => {
                    if let Some(mesh) = self.meshes.get(name) {
                        frame.meshes.push(self.mesh(mesh)?);
                    }
                }
                Child::Obj(_) => {}
            }
        }
        Ok(frame)
    }

    /// `nVertices; vertices; nFaces; faces (n; indices…)…`, then the nested
    /// objects.
    fn mesh(&self, obj: &Obj) -> Result<XMesh, String> {
        let mut r = Reader::new(obj);
        let nv = r.count("vertices", 3)?;
        let positions = (0..nv).map(|_| r.floats()).collect::<Result<Vec<[f32; 3]>, _>>()?;
        let faces = r.faces("faces", nv, "vertex")?;
        let (normals, face_normals) = match obj.first(Kind::MeshNormals) {
            Some(normals) => mesh_normals(normals, &faces)?,
            None => (Vec::new(), Vec::new()),
        };
        let texcoords = match obj.first(Kind::MeshTextureCoords) {
            Some(uv) => texture_coords(uv, nv)?,
            None => Vec::new(),
        };
        let colors = match obj.first(Kind::MeshVertexColors) {
            Some(colors) => vertex_colors(colors, nv)?,
            None => Vec::new(),
        };
        let (materials, face_materials) = match obj.first(Kind::MeshMaterialList) {
            Some(list) => self.material_list(list, faces.len())?,
            None => (Vec::new(), Vec::new()),
        };
        let mut mesh = XMesh { positions, faces, normals, face_normals, texcoords, colors, materials, face_materials };
        drop_small_faces(&mut mesh);
        Ok(mesh)
    }

    /// `nMaterials; nFaceIndexes; faceIndexes…`, then the materials: inline,
    /// or references (one that names no material gets the default).
    fn material_list(&self, obj: &Obj, nfaces: usize) -> Result<(Vec<XMaterial>, Vec<usize>), String> {
        let mut r = Reader::new(obj);
        let declared = r.index()?;
        let n = r.count("face indexes", 1)?;
        let indexes = (0..n).map(|_| r.index()).collect::<Result<Vec<usize>, _>>()?;
        let mut materials = Vec::new();
        for child in &obj.children {
            match child {
                Child::Obj(material) if material.kind == Kind::Material => materials.push(read_material(material)?),
                Child::Ref(name) => materials.push(match self.materials.get(name) {
                    Some(material) => read_material(material)?,
                    None => XMaterial::default(),
                }),
                Child::Obj(_) => {}
            }
        }
        if materials.is_empty() && declared > 0 {
            materials.push(XMaterial::default());
        }
        if materials.is_empty() {
            return Ok((materials, Vec::new()));
        }
        // Fewer indexes than faces: the last one repeats. An index past the
        // list: a default material, added once at the end.
        let listed = materials.len();
        let mut face_materials = Vec::with_capacity(nfaces);
        for face in 0..nfaces {
            let i = indexes.get(face).or(indexes.last()).copied().unwrap_or(0);
            face_materials.push(i.min(listed));
        }
        if face_materials.contains(&listed) {
            materials.push(XMaterial::default());
        }
        Ok((materials, face_materials))
    }
}

fn build(root: &Obj) -> Result<XFile, String> {
    let mut index = Index::default();
    index.collect(root);
    let mut used = HashSet::new();
    index.referenced(root, &mut used);
    let mut file = XFile::default();
    for child in &root.children {
        let Child::Obj(obj) = child else { continue };
        match obj.kind {
            Kind::Frame => file.frames.push(index.frame(obj)?),
            Kind::Mesh if !used.contains(&(obj as *const Obj)) => file.meshes.push(index.mesh(obj)?),
            _ => {}
        }
    }
    Ok(file)
}

/// An object's numbers, read in order.
struct Reader<'o> {
    obj: &'o Obj,
    at: usize,
}

impl<'o> Reader<'o> {
    fn new(obj: &'o Obj) -> Self {
        Reader { obj, at: 0 }
    }

    fn left(&self) -> usize {
        self.obj.nums.len().saturating_sub(self.at)
    }

    fn num(&mut self) -> Result<f64, String> {
        let Some(&v) = self.obj.nums.get(self.at) else {
            return Err(bad(format!("{} stops short of the values its template needs", self.obj.label())));
        };
        self.at += 1;
        Ok(v)
    }

    /// A float; infinities and NaNs read as 0.
    fn float(&mut self) -> Result<f32, String> {
        let v = self.num()? as f32;
        Ok(if v.is_finite() { v } else { 0.0 })
    }

    fn floats<const N: usize>(&mut self) -> Result<[f32; N], String> {
        let mut out = [0.0; N];
        for v in &mut out {
            *v = self.float()?;
        }
        Ok(out)
    }

    /// A count or an index: a whole number, 0 … 2³² - 1.
    fn index(&mut self) -> Result<usize, String> {
        let v = self.num()?;
        if (0.0..=f64::from(u32::MAX)).contains(&v) && v.fract() == 0.0 {
            Ok(v as usize)
        } else {
            Err(bad(format!("{} has {v} where a count or an index belongs", self.obj.label())))
        }
    }

    /// A count of items `width` values long, checked against what's left.
    fn count(&mut self, what: &str, width: usize) -> Result<usize, String> {
        let n = self.index()?;
        if n.saturating_mul(width) > self.left() {
            return Err(bad(format!("{} declares {n} {what} but only {} values follow", self.obj.label(), self.left())));
        }
        Ok(n)
    }

    /// `n; (k; i₁, …, i_k)…`, each index below `limit`.
    fn faces(&mut self, what: &str, limit: usize, item: &str) -> Result<Vec<Vec<usize>>, String> {
        let n = self.count(what, 1)?;
        let mut faces = Vec::with_capacity(n);
        for f in 0..n {
            let k = self.count("indices in a face", 1)?;
            let mut face = Vec::with_capacity(k);
            for _ in 0..k {
                let i = self.index()?;
                if i >= limit {
                    return Err(bad(format!("face {f} of {} uses {item} {i}, but there are {limit}", self.obj.label())));
                }
                face.push(i);
            }
            faces.push(face);
        }
        Ok(faces)
    }
}

/// A mesh's normals and, per face, the normal of each corner.
type Normals = (Vec<[f32; 3]>, Vec<Vec<usize>>);

/// `nNormals; normals; nFaceNormals; faces`. Normals whose faces don't match
/// the mesh's are left out (the program can work out its own); with no faces
/// at all but a normal per vertex, the normals follow the vertices.
fn mesh_normals(obj: &Obj, faces: &[Vec<usize>]) -> Result<Normals, String> {
    let mut r = Reader::new(obj);
    let n = r.count("normals", 3)?;
    let normals = (0..n).map(|_| r.floats()).collect::<Result<Vec<[f32; 3]>, _>>()?;
    let face_normals = if r.left() == 0 { Vec::new() } else { r.faces("normal faces", usize::MAX, "normal")? };
    let used = faces.iter().flatten().max().map_or(0, |&i| i + 1);
    if face_normals.is_empty() && !normals.is_empty() && used <= normals.len() {
        return Ok((normals, faces.to_vec()));
    }
    let fits = face_normals.len() == faces.len() && face_normals.iter().zip(faces).all(|(n, f)| n.len() == f.len() && n.iter().all(|&i| i < normals.len()));
    Ok(if fits { (normals, face_normals) } else { (Vec::new(), Vec::new()) })
}

/// `nTextureCoords; (u; v)…`, fitted to the vertex count.
fn texture_coords(obj: &Obj, nv: usize) -> Result<Vec<[f32; 2]>, String> {
    let mut r = Reader::new(obj);
    let n = r.count("texture coordinates", 2)?;
    let mut uv = (0..n).map(|_| r.floats()).collect::<Result<Vec<[f32; 2]>, _>>()?;
    if !uv.is_empty() {
        uv.resize(nv, [0.0; 2]);
    }
    Ok(uv)
}

/// `nVertexColors; (index; r; g; b; a)…`: a colour per vertex, white where
/// the list names none.
fn vertex_colors(obj: &Obj, nv: usize) -> Result<Vec<[f32; 4]>, String> {
    let mut r = Reader::new(obj);
    let n = r.count("vertex colors", 5)?;
    let mut colors = if n == 0 { Vec::new() } else { vec![[1.0; 4]; nv] };
    for _ in 0..n {
        let i = r.index()?;
        let color = r.floats()?;
        if let Some(slot) = colors.get_mut(i) {
            *slot = color;
        }
    }
    Ok(colors)
}

/// `faceColor (RGBA); power; specularColor (RGB); emissiveColor (RGB);`, and
/// a nested `TextureFilename { "name"; }`.
fn read_material(obj: &Obj) -> Result<XMaterial, String> {
    let mut r = Reader::new(obj);
    let color = r.floats()?;
    let power = r.float()?;
    let specular = r.floats()?;
    let emissive = r.floats()?;
    let texture = obj
        .children
        .iter()
        .find_map(|child| match child {
            Child::Obj(t) if t.kind == Kind::TextureFilename => t.words.first().cloned(),
            _ => None,
        })
        .filter(|name| !name.is_empty());
    Ok(XMaterial { color, power, specular, emissive, texture })
}

/// Leaves out faces of fewer than three vertices, with their normals and
/// materials.
fn drop_small_faces(mesh: &mut XMesh) {
    if mesh.faces.iter().all(|face| face.len() >= 3) {
        return;
    }
    let keep: Vec<bool> = mesh.faces.iter().map(|face| face.len() >= 3).collect();
    retain_by(&mut mesh.faces, &keep);
    retain_by(&mut mesh.face_normals, &keep);
    retain_by(&mut mesh.face_materials, &keep);
}

fn retain_by<T>(items: &mut Vec<T>, keep: &[bool]) {
    let mut i = 0;
    items.retain(|_| {
        i += 1;
        keep.get(i - 1).copied().unwrap_or(true)
    });
}

// ---------------------------------------------------------------------------
// Flattening

fn gather<'f>(frame: &'f XFrame, parent: &[f32; 16], parts: &mut Vec<(&'f XMesh, [f32; 16])>) {
    let world = mul(&frame.transform, parent);
    for mesh in &frame.meshes {
        parts.push((mesh, world));
    }
    for child in &frame.children {
        gather(child, &world, parts);
    }
}

/// `a * b`, row-major.
fn mul(a: &[f32; 16], b: &[f32; 16]) -> [f32; 16] {
    let mut out = [0.0; 16];
    for (i, cell) in out.iter_mut().enumerate() {
        let (row, col) = (i / 4, i % 4);
        *cell = (0..4).map(|k| a[row * 4 + k] * b[k * 4 + col]).sum();
    }
    out
}

/// `[x y z w] * m`'s x, y, z: w = 1 for a point, 0 for a direction.
fn apply(m: &[f32; 16], v: [f32; 3], w: f32) -> [f32; 3] {
    [
        v[0] * m[0] + v[1] * m[4] + v[2] * m[8] + w * m[12],
        v[0] * m[1] + v[1] * m[5] + v[2] * m[9] + w * m[13],
        v[0] * m[2] + v[1] * m[6] + v[2] * m[10] + w * m[14],
    ]
}

fn normalize(v: [f32; 3]) -> [f32; 3] {
    let len = (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt();
    if len > 0.0 && len.is_finite() {
        v.map(|c| c / len)
    } else {
        v
    }
}

/// A polygon's normal (Newell's method), facing the side its vertices run
/// clockwise on, as Direct3D's front faces do.
fn face_normal(positions: &[[f32; 3]], face: &[usize], base: usize) -> [f32; 3] {
    let mut n = [0.0f32; 3];
    for (k, &i) in face.iter().enumerate() {
        let j = face[(k + 1) % face.len()];
        let (Some(a), Some(b)) = (positions.get(base.saturating_add(i)), positions.get(base.saturating_add(j))) else {
            continue;
        };
        n[0] += (a[1] - b[1]) * (a[2] + b[2]);
        n[1] += (a[2] - b[2]) * (a[0] + b[0]);
        n[2] += (a[0] - b[0]) * (a[1] + b[1]);
    }
    normalize(n)
}

/// The mesh's normals line up with its faces.
fn normals_fit(mesh: &XMesh) -> bool {
    !mesh.normals.is_empty()
        && mesh.face_normals.len() == mesh.faces.len()
        && mesh.face_normals.iter().zip(&mesh.faces).all(|(n, f)| n.len() == f.len() && n.iter().all(|&i| i < mesh.normals.len()))
}

fn merge(parts: &[(&XMesh, [f32; 16])]) -> XMesh {
    let any_normals = parts.iter().any(|(mesh, _)| normals_fit(mesh));
    let any_uv = parts.iter().any(|(mesh, _)| !mesh.texcoords.is_empty());
    let any_colors = parts.iter().any(|(mesh, _)| !mesh.colors.is_empty());
    let any_materials = parts.iter().any(|(mesh, _)| !mesh.materials.is_empty());
    let mut out = XMesh::default();
    for (mesh, m) in parts {
        let base = out.positions.len();
        out.positions.extend(mesh.positions.iter().map(|&p| apply(m, p, 1.0)));
        out.faces.extend(mesh.faces.iter().map(|face| face.iter().map(|&i| i.saturating_add(base)).collect::<Vec<_>>()));
        if any_normals {
            let nbase = out.normals.len();
            if normals_fit(mesh) {
                out.normals.extend(mesh.normals.iter().map(|&n| normalize(apply(m, n, 0.0))));
                out.face_normals.extend(mesh.face_normals.iter().map(|face| face.iter().map(|&i| i + nbase).collect::<Vec<_>>()));
            } else {
                for (k, face) in mesh.faces.iter().enumerate() {
                    out.normals.push(face_normal(&out.positions, face, base));
                    out.face_normals.push(vec![nbase + k; face.len()]);
                }
            }
        }
        if any_uv {
            out.texcoords.extend((0..mesh.positions.len()).map(|i| mesh.texcoords.get(i).copied().unwrap_or([0.0; 2])));
        }
        if any_colors {
            out.colors.extend((0..mesh.positions.len()).map(|i| mesh.colors.get(i).copied().unwrap_or([1.0; 4])));
        }
        if any_materials {
            let mbase = out.materials.len();
            if mesh.materials.is_empty() {
                out.materials.push(XMaterial::default());
                out.face_materials.resize(out.face_materials.len() + mesh.faces.len(), mbase);
            } else {
                out.materials.extend(mesh.materials.iter().cloned());
                let last = mesh.materials.len() - 1;
                out.face_materials
                    .extend((0..mesh.faces.len()).map(|f| mbase + mesh.face_materials.get(f).or(mesh.face_materials.last()).copied().unwrap_or(0).min(last)));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A frame holding a frame holding a mesh with normals, texture
    /// coordinates, vertex colours and two materials (one a reference); a
    /// top-level material and mesh; templates, a header and an animation set
    /// to skip.
    const TEXT: &str = r#"xof 0303txt 0032
// A comment.
# Another.
template Header {
 <3D82AB43-62DA-11cf-AB39-0020AF71E433>
 WORD major;
 WORD minor;
 DWORD flags;
}

template Mesh {
 <3D82AB44-62DA-11cf-AB39-0020AF71E433>
 DWORD nVertices;
 array Vector vertices[nVertices];
 DWORD nFaces;
 array MeshFace faces[nFaces];
 [...]
}

Header { 1; 0; 1; }

Material Red {
  1.0; 0.0; 0.0; 1.0;;
  8.0;
  0.5; 0.5; 0.5;;
  0.0; 0.0; 0.25;;
}

Frame Root {
  FrameTransformMatrix {
    1.0, 0.0, 0.0, 0.0,
    0.0, 1.0, 0.0, 0.0,
    0.0, 0.0, 1.0, 0.0,
    10.0, 0.0, 0.0, 1.0;;
  }
  Frame Child {
    FrameTransformMatrix {
      2.0, 0.0, 0.0, 0.0,
      0.0, 2.0, 0.0, 0.0,
      0.0, 0.0, 2.0, 0.0,
      0.0, 5.0, 0.0, 1.0;;
    }
    Mesh Quad {
      5;
      0.0; 0.0; 0.0;,
      1.0; 0.0; 0.0;,
      1.0; 1.0; 0.0;,
      0.0; 1.0; 0.0;,
      0.5; 2.0; 0.0;;
      3;
      4; 0, 3, 2, 1;,
      3; 3, 4, 2;,
      3; 0, 2, 1;;
      MeshNormals {
        1;
        0.0; 0.0; -1.0;;
        3;
        4; 0, 0, 0, 0;,
        3; 0, 0, 0;,
        3; 0, 0, 0;;
      }
      MeshTextureCoords {
        5;
        0.0; 1.0;,
        1.0; 1.0;,
        1.0; 0.0;,
        0.0; 0.0;,
        0.5; -1.0;;
      }
      MeshVertexColors {
        2;
        0; 1.0; 0.0; 0.0; 1.0;,
        4; 0.0; 0.0; 1.0; 0.5;;
      }
      MeshMaterialList {
        2;
        2;
        1,
        0;;
        { Red }
        Material {
          1.0; 1.0; 1.0; 1.0;;
          0.0;
          0.0; 0.0; 0.0;;
          0.0; 0.0; 0.0;;
          TextureFileName { "wood.bmp"; }
        }
      }
      VertexDuplicationIndices { 5; 5; 0, 1, 2, 3, 4; }
    }
  }
}

Mesh Floor {
  4;
  -1.0; 0.0; -1.0;, -1.0; 0.0; 1.0;, 1.0; 0.0; 1.0;, 1.0; 0.0; -1.0;;
  1;
  4; 0, 1, 2, 3;;
  MeshMaterialList { 1; 1; 0;; { Red <35FF44E0-6C7C-11cf-8F52-0040333594A3> } }
}

AnimationSet Walk {
  Animation {
    { Root }
    AnimationKey { 4; 1; 0; 16; 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0;;; }
  }
}
"#;

    const T_ROOT: [f64; 16] = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 10.0, 0.0, 0.0, 1.0];
    const T_CHILD: [f64; 16] = [2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 0.0, 0.0, 2.0, 0.0, 0.0, 5.0, 0.0, 1.0];
    const QUAD: [f64; 15] = [0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 1.0, 1.0, 0.0, 0.0, 1.0, 0.0, 0.5, 2.0, 0.0];
    const QUAD_UV: [f64; 10] = [0.0, 1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.5, -1.0];
    const FLOOR: [f64; 12] = [-1.0, 0.0, -1.0, -1.0, 0.0, 1.0, 1.0, 0.0, 1.0, 1.0, 0.0, -1.0];
    const RED: [f64; 11] = [1.0, 0.0, 0.0, 1.0, 8.0, 0.5, 0.5, 0.5, 0.0, 0.0, 0.25];
    const WHITE: [f64; 11] = [1.0, 1.0, 1.0, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];

    /// Builds a binary .x token by token.
    struct Bin {
        bytes: Vec<u8>,
        double: bool,
    }

    impl Bin {
        fn new(double: bool) -> Self {
            let mut bytes = b"xof 0303bin ".to_vec();
            bytes.extend_from_slice(if double { b"0064" } else { b"0032" });
            Bin { bytes, double }
        }
        fn tok(&mut self, token: u16) -> &mut Self {
            self.bytes.extend_from_slice(&token.to_le_bytes());
            self
        }
        fn u32(&mut self, v: u32) -> &mut Self {
            self.bytes.extend_from_slice(&v.to_le_bytes());
            self
        }
        fn name(&mut self, s: &str) -> &mut Self {
            self.tok(1).u32(s.len() as u32);
            self.bytes.extend_from_slice(s.as_bytes());
            self
        }
        fn string(&mut self, s: &str) -> &mut Self {
            self.tok(2).u32(s.len() as u32);
            self.bytes.extend_from_slice(s.as_bytes());
            self.tok(20)
        }
        fn int(&mut self, v: u32) -> &mut Self {
            self.tok(3).u32(v).tok(20)
        }
        fn guid(&mut self) -> &mut Self {
            self.tok(5);
            self.bytes.extend_from_slice(&[0x5a; 16]);
            self
        }
        fn ints(&mut self, v: &[u32]) -> &mut Self {
            self.tok(6).u32(v.len() as u32);
            for &x in v {
                self.u32(x);
            }
            self
        }
        fn floats(&mut self, v: &[f64]) -> &mut Self {
            self.tok(7).u32(v.len() as u32);
            for &x in v {
                if self.double {
                    self.bytes.extend_from_slice(&x.to_le_bytes());
                } else {
                    self.bytes.extend_from_slice(&(x as f32).to_le_bytes());
                }
            }
            self
        }
        fn open(&mut self, kind: &str, name: Option<&str>) -> &mut Self {
            self.name(kind);
            if let Some(name) = name {
                self.name(name);
            }
            self.tok(10)
        }
        fn close(&mut self) -> &mut Self {
            self.tok(11)
        }
        fn reference(&mut self, name: &str, guid: bool) -> &mut Self {
            self.tok(10).name(name);
            if guid {
                self.guid();
            }
            self.tok(11)
        }
    }

    /// `TEXT`, in the binary format.
    fn binary(double: bool) -> Vec<u8> {
        let mut b = Bin::new(double);
        // template Header { <GUID> WORD major; WORD minor; DWORD flags; }
        b.tok(31).name("Header").tok(10).guid();
        b.tok(40).name("major").tok(20).tok(40).name("minor").tok(20).tok(41).name("flags").tok(20).tok(11);
        // template Mesh { <GUID> DWORD nVertices; array Vector vertices[nVertices]; … [...] }
        b.tok(31).name("Mesh").tok(10).guid();
        b.tok(41).name("nVertices").tok(20).tok(52).name("Vector").name("vertices").tok(14).name("nVertices").tok(15).tok(20);
        b.tok(41).name("nFaces").tok(20).tok(52).name("MeshFace").name("faces").tok(14).name("nFaces").tok(15).tok(20);
        b.tok(14).tok(18).tok(18).tok(18).tok(15).tok(11);
        b.open("Header", None).int(1).int(0).int(1).close();
        b.open("Material", Some("Red")).floats(&RED).close();
        b.open("Frame", Some("Root"));
        b.open("FrameTransformMatrix", None).floats(&T_ROOT).close();
        b.open("Frame", Some("Child"));
        b.open("FrameTransformMatrix", None).floats(&T_CHILD).close();
        b.open("Mesh", Some("Quad")).ints(&[5]).floats(&QUAD).ints(&[3, 4, 0, 3, 2, 1, 3, 3, 4, 2, 3, 0, 2, 1]);
        b.open("MeshNormals", None).ints(&[1]).floats(&[0.0, 0.0, -1.0]).ints(&[3, 4, 0, 0, 0, 0, 3, 0, 0, 0, 3, 0, 0, 0]).close();
        b.open("MeshTextureCoords", None).ints(&[5]).floats(&QUAD_UV).close();
        b.open("MeshVertexColors", None).ints(&[2, 0]).floats(&[1.0, 0.0, 0.0, 1.0]).ints(&[4]).floats(&[0.0, 0.0, 1.0, 0.5]).close();
        b.open("MeshMaterialList", None).ints(&[2, 2, 1, 0]).reference("Red", false);
        b.open("Material", None).floats(&WHITE).open("TextureFileName", None).string("wood.bmp").close().close();
        b.close(); // MeshMaterialList
        b.open("VertexDuplicationIndices", None).ints(&[5, 5, 0, 1, 2, 3, 4]).close();
        b.close().close().close(); // Quad, Child, Root
        b.open("Mesh", Some("Floor")).ints(&[4]).floats(&FLOOR).ints(&[1, 4, 0, 1, 2, 3]);
        b.open("MeshMaterialList", None).ints(&[1, 1, 0]).reference("Red", true).close();
        b.close();
        b.open("AnimationSet", Some("Walk")).open("Animation", None).reference("Root", false);
        b.open("AnimationKey", None).ints(&[4, 1, 0, 16]).floats(&T_ROOT).close();
        b.close().close();
        b.bytes
    }

    fn f32s<const N: usize>(v: &[f64]) -> [f32; N] {
        let mut out = [0.0; N];
        for (o, &x) in out.iter_mut().zip(v) {
            *o = x as f32;
        }
        out
    }

    fn triples(v: &[f64]) -> Vec<[f32; 3]> {
        v.chunks(3).map(f32s::<3>).collect()
    }

    fn red() -> XMaterial {
        XMaterial { color: [1.0, 0.0, 0.0, 1.0], power: 8.0, specular: [0.5; 3], emissive: [0.0, 0.0, 0.25], texture: None }
    }

    fn wood() -> XMaterial {
        XMaterial { texture: Some("wood.bmp".into()), ..XMaterial::default() }
    }

    fn expected() -> XFile {
        let quad = XMesh {
            positions: triples(&QUAD),
            faces: vec![vec![0, 3, 2, 1], vec![3, 4, 2], vec![0, 2, 1]],
            normals: vec![[0.0, 0.0, -1.0]],
            face_normals: vec![vec![0; 4], vec![0; 3], vec![0; 3]],
            texcoords: QUAD_UV.chunks(2).map(f32s::<2>).collect(),
            colors: vec![[1.0, 0.0, 0.0, 1.0], [1.0; 4], [1.0; 4], [1.0; 4], [0.0, 0.0, 1.0, 0.5]],
            materials: vec![red(), wood()],
            face_materials: vec![1, 0, 0],
        };
        let floor = XMesh { positions: triples(&FLOOR), faces: vec![vec![0, 1, 2, 3]], materials: vec![red()], face_materials: vec![0], ..XMesh::default() };
        let child = XFrame { name: "Child".into(), transform: f32s(&T_CHILD), meshes: vec![quad], children: vec![] };
        let root = XFrame { name: "Root".into(), transform: f32s(&T_ROOT), meshes: vec![], children: vec![child] };
        XFile { frames: vec![root], meshes: vec![floor] }
    }

    #[test]
    fn reads_text() {
        assert_eq!(parse_x(TEXT.as_bytes()).unwrap(), expected());
    }

    #[test]
    fn reads_binary_32_and_64_bit() {
        assert_eq!(parse_x(&binary(false)).unwrap(), expected());
        assert_eq!(parse_x(&binary(true)).unwrap(), expected());
    }

    #[test]
    fn flatten_applies_frames_child_first() {
        let flat = flatten(&parse_x(TEXT.as_bytes()).unwrap());
        // Floor as it is, then Quad scaled by 2 and moved by (0, 5, 0) (Child), then by (10, 0, 0) (Root).
        let mut positions = triples(&FLOOR);
        positions.extend([[10.0, 5.0, 0.0], [12.0, 5.0, 0.0], [12.0, 7.0, 0.0], [10.0, 7.0, 0.0], [11.0, 9.0, 0.0]]);
        assert_eq!(flat.positions, positions);
        assert_eq!(flat.faces, vec![vec![0, 1, 2, 3], vec![4, 7, 6, 5], vec![7, 8, 6], vec![4, 6, 5]]);
        // Floor has no normals: a flat one, up. Quad's is renormalized after the scale.
        assert_eq!(flat.normals, vec![[0.0, 1.0, 0.0], [0.0, 0.0, -1.0]]);
        assert_eq!(flat.face_normals, vec![vec![0; 4], vec![1; 4], vec![1; 3], vec![1; 3]]);
        assert_eq!(flat.texcoords.len(), 9);
        assert_eq!(flat.texcoords[..4], [[0.0; 2]; 4]);
        assert_eq!(flat.texcoords[8], [0.5, -1.0]);
        assert_eq!(flat.colors.len(), 9);
        assert_eq!(flat.colors[4], [1.0, 0.0, 0.0, 1.0]);
        assert_eq!(flat.colors[8], [0.0, 0.0, 1.0, 0.5]);
        assert_eq!(flat.materials, vec![red(), red(), wood()]);
        assert_eq!(flat.face_materials, vec![0, 2, 1, 1]);
    }

    #[test]
    fn flatten_rotates_normals() {
        // A quarter turn about x (y goes to z) and a scale of 3, around a frame moved by (0, 0, 1).
        let turn = [3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 3.0, 0.0, 0.0, -3.0, 0.0, 0.0, 0.0, 0.0, 0.0, 1.0];
        let moved = [1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0, 1.0];
        let mesh = XMesh {
            positions: vec![[0.0, 1.0, 0.0], [1.0, 1.0, 0.0], [0.0, 1.0, 1.0]],
            faces: vec![vec![0, 1, 2]],
            normals: vec![[0.0, 1.0, 0.0]],
            face_normals: vec![vec![0, 0, 0]],
            ..XMesh::default()
        };
        let child = XFrame { name: "turned".into(), transform: turn, meshes: vec![mesh], children: vec![] };
        let file = XFile { frames: vec![XFrame { transform: moved, children: vec![child], ..XFrame::default() }], meshes: vec![] };
        let flat = flatten(&file);
        assert_eq!(flat.positions, vec![[0.0, 0.0, 4.0], [3.0, 0.0, 4.0], [0.0, -3.0, 4.0]]);
        assert_eq!(flat.normals, vec![[0.0, 0.0, 1.0]]);
        assert!(flat.materials.is_empty() && flat.face_materials.is_empty() && flat.texcoords.is_empty() && flat.colors.is_empty());
    }

    #[test]
    fn sloppy_separators() {
        let file = parse_x(
            b"xof 0302txt 0064\n\
              Mesh {\n 3;;\n 0.0;0.0;0.0;,,\n 1.000000e+000,0,-0.5e-1,;\n 0.0 1.0 0.0;;;\n\
              1;\n 3;0,1,2,;;,\n\
              MeshTextureCoords { 3;; 0,0,; 1;0;;; 0;1,,; }\n}\n",
        )
        .unwrap();
        let mesh = &file.meshes[0];
        assert_eq!(mesh.positions, vec![[0.0, 0.0, 0.0], [1.0, 0.0, -0.05], [0.0, 1.0, 0.0]]);
        assert_eq!(mesh.faces, vec![vec![0, 1, 2]]);
        assert_eq!(mesh.texcoords, vec![[0.0, 0.0], [1.0, 0.0], [0.0, 1.0]]);
    }

    #[test]
    fn material_list_details() {
        let file = parse_x(
            b"xof 0302txt 0032\n\
              Mesh m { 4; 0;0;0;, 1;0;0;, 0;1;0;, 1;1;0;; 4; 3;0,1,2;, 3;1,3,2;, 2;0,1;, 3;0,2,3;;\n\
                MeshMaterialList { 3; 2; 0, 5;; { Missing } { m2 } }\n\
                MeshNormals { 1; 0;0;1;; 2; 3;0,0,0;, 3;0,0,0;; }\n\
                MeshVertexColors { 1; 9; 1;0;0;1;; }\n\
                MeshTextureCoords { 2; 0;0;, 1;1;; }\n\
              }\n\
              material M2 { 0;1;0;1;; 1; 0;0;0;; 0;0;0;; texturefilename { \"g.png\"; } }\n",
        )
        .unwrap();
        let mesh = &file.meshes[0];
        // The two-vertex face is left out; the material list's last index repeats and is past the list.
        assert_eq!(mesh.faces, vec![vec![0, 1, 2], vec![1, 3, 2], vec![0, 2, 3]]);
        let green = XMaterial { color: [0.0, 1.0, 0.0, 1.0], power: 1.0, texture: Some("g.png".into()), ..XMaterial::default() };
        assert_eq!(mesh.materials, vec![XMaterial::default(), green, XMaterial::default()]);
        assert_eq!(mesh.face_materials, vec![0, 2, 2]);
        // Normal faces that don't match the mesh's are dropped; a colour for no vertex changes none.
        assert!(mesh.normals.is_empty() && mesh.face_normals.is_empty());
        assert_eq!(mesh.colors, vec![[1.0; 4]; 4]);
        assert_eq!(mesh.texcoords, vec![[0.0, 0.0], [1.0, 1.0], [0.0, 0.0], [0.0, 0.0]]);
    }

    #[test]
    fn frame_mesh_references() {
        let file = parse_x(
            b"xof 0303txt 0032\n\
              Mesh Tri { 3; 0;0;0;, 1;0;0;, 0;1;0;; 1; 3;0,1,2;; }\n\
              Mesh Other { 3; 0;0;0;, 2;0;0;, 0;2;0;; 1; 3;0,1,2;; }\n\
              Frame A { { Tri } Frame B { FrameTransformMatrix { 1,0,0,0, 0,1,0,0, 0,0,1,0, 0,0,7,1;; } { tri } } }\n",
        )
        .unwrap();
        assert_eq!(file.meshes.len(), 1); // Tri belongs to the frames now
        assert_eq!(file.meshes[0].positions[1], [2.0, 0.0, 0.0]);
        assert_eq!(file.frames[0].meshes.len(), 1);
        assert_eq!(file.frames[0].children[0].meshes.len(), 1);
        assert_eq!(flatten(&file).positions.len(), 9);
        assert_eq!(flatten(&file).positions[8], [0.0, 1.0, 7.0]);
    }

    #[test]
    fn mesh_normals_without_faces_follow_vertices() {
        let file = parse_x(b"xof 0302txt 0032 Mesh { 3; 0;0;0;, 1;0;0;, 0;1;0;; 1; 3;0,1,2;; MeshNormals { 3; 0;0;1;, 0;0;1;, 0;0;1;; 0;; } }").unwrap();
        assert_eq!(file.meshes[0].normals.len(), 3);
        assert_eq!(file.meshes[0].face_normals, vec![vec![0, 1, 2]]);
    }

    #[test]
    fn skips_unknown_objects_at_any_depth() {
        let mut deep = String::from("xof 0302txt 0032\n");
        for _ in 0..1000 {
            deep.push_str("Wrapper { 1; 2; { Ref } ");
        }
        deep.push_str(&"}".repeat(1000));
        deep.push_str(" Mesh { 3; 0;0;0;, 1;0;0;, 0;1;0;; 1; 3;0,1,2;; XSkinMeshHeader { 1; 2; 3; } DeclData { 1; 2; 3; } }");
        let file = parse_x(deep.as_bytes()).unwrap();
        assert_eq!(file.meshes.len(), 1);
        assert_eq!(file.meshes[0].faces.len(), 1);
    }

    #[test]
    fn refuses_what_it_cannot_read() {
        let err = |bytes: &[u8]| parse_x(bytes).unwrap_err();
        for message in [
            err(b"hello"),
            err(b"JUNK0302txt 0032 Mesh { }"),
            err(b"xof 0302tzip0032\x00\x01\x02"),
            err(b"xof 0303bzip0032\x00\x01\x02"),
            err(b"xof 0302abc 0032"),
            err(b"xof 0302bin 0016"),
        ] {
            assert!(message.starts_with("not a DirectX .x file RapidR can read ("), "{message}");
        }
        assert!(err(b"hello").contains("too short"));
        assert!(err(b"xof 0302tzip0032\x00\x01\x02").contains("compressed"));
        assert!(err(b"xof 0303bzip0032\x00\x01\x02").contains("compressed"));
        assert!(err(b"xof 0302abc 0032").contains("unknown format"));
        assert!(err(b"xof 0302bin 0016").contains("float size"));
        assert!(err(b"xof 0302txt 0032 Mesh { 100; 1;2;3;; }").contains("declares 100 vertices"));
        assert!(err(b"xof 0302txt 0032 Mesh { 3; 0;0;0;, 1;0;0;, 0;1;0;; 1; 3;0,1,3;; }").contains("uses vertex 3"));
        assert!(err(b"xof 0302txt 0032 Mesh { 3; 0;0;0;, 1;0;0;, 0;1;0;; 1; 3;0,1,-2;; }").contains("count or an index"));
        assert!(err(b"xof 0302txt 0032 Mesh { 1; 0;0;").contains("ends inside a Mesh"));
        assert!(err(b"xof 0302txt 0032 Frame F { FrameTransformMatrix { 1, 0, 0; } }").contains("stops short"));
        assert!(err(b"xof 0302txt 0032\nMaterial { 1;1;1;1;; 0; 0;0;0;; 0;0;0;; TextureFilename { \"a.bmp; } }").contains("line 2"));
        assert!(err(b"xof 0302txt 0032\n\nHeader { <3D82AB43-62DA 1; }").contains("no closing >"));
        assert!(err(b"xof 0302txt 0032 AnimationSet { Animation {").contains("AnimationSet"));
        let nested = format!("xof 0302txt 0032 {}", "Frame { ".repeat(MAX_DEPTH + 2));
        assert!(err(nested.as_bytes()).contains("nest"));
        // Binary: a list longer than the file, an unknown token.
        let mut b = Bin::new(false);
        b.open("Mesh", None).tok(6).u32(u32::MAX);
        assert!(err(&b.bytes).contains("declares 4294967295 numbers"));
        let mut b = Bin::new(true);
        b.open("Mesh", None).tok(7).u32(1 << 30).u32(0);
        assert!(err(&b.bytes).contains("file ends first"));
        let mut b = Bin::new(false);
        b.tok(99);
        assert!(err(&b.bytes).contains("unknown binary token 99"));
    }

    #[test]
    fn truncated_binary_is_an_error_never_a_panic() {
        for double in [false, true] {
            let bytes = binary(double);
            for len in 0..bytes.len() {
                let _ = parse_x(&bytes[..len]);
            }
            for len in [bytes.len() - 1, bytes.len() - 2, bytes.len() - 3, bytes.len() / 2, 40] {
                assert!(parse_x(&bytes[..len]).is_err(), "{len} of {} bytes", bytes.len());
            }
        }
        for len in 0..TEXT.len() {
            if let Ok(file) = parse_x(&TEXT.as_bytes()[..len]) {
                flatten(&file);
            }
        }
    }

    #[test]
    fn damaged_files_never_panic() {
        let mut seed = 0x2545_f491_4f6c_dd1du64;
        let mut random = move || {
            seed ^= seed << 13;
            seed ^= seed >> 7;
            seed ^= seed << 17;
            seed
        };
        for source in [TEXT.as_bytes().to_vec(), binary(false), binary(true)] {
            for _ in 0..3000 {
                let mut bytes = source.clone();
                for _ in 0..1 + random() % 4 {
                    let at = 16 + (random() as usize) % (bytes.len() - 16);
                    bytes[at] = match random() % 4 {
                        0 => 0,
                        1 => 0xff,
                        2 => bytes[at] ^ (1 << (random() % 8)),
                        _ => random() as u8,
                    };
                }
                if let Ok(file) = parse_x(&bytes) {
                    flatten(&file);
                }
            }
        }
    }

    fn collect_x_files(dir: &std::path::Path, out: &mut Vec<std::path::PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else { return };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_x_files(&path, out);
            } else if path.extension().is_some_and(|e| e.eq_ignore_ascii_case("x")) {
                out.push(path);
            }
        }
    }

    /// Every .x model of RapidQ's examples (`RAPIDQ_DIR`, else
    /// ~/Downloads/Rapidq) reads, with something to draw.
    #[test]
    #[ignore = "reads the RapidQ example models from outside the repository"]
    fn rapidq_corpus() {
        let dir = std::env::var("RAPIDQ_DIR").unwrap_or_else(|_| format!("{}/Downloads/Rapidq", std::env::var("HOME").unwrap_or_default()));
        let dir = std::path::Path::new(&dir);
        let mut files = Vec::new();
        collect_x_files(dir, &mut files);
        files.sort();
        assert!(!files.is_empty(), "no .x files under {}", dir.display());
        let mut failures = Vec::new();
        for path in &files {
            let name = path.strip_prefix(dir).unwrap_or(path).display().to_string();
            let bytes = std::fs::read(path).unwrap();
            let format = String::from_utf8_lossy(&bytes[8.min(bytes.len())..16.min(bytes.len())]).into_owned();
            match parse_x(&bytes) {
                Ok(file) => {
                    let flat = flatten(&file);
                    let textures: std::collections::BTreeSet<&str> = flat.materials.iter().filter_map(|m| m.texture.as_deref()).collect();
                    println!(
                        "{name:58} {format} frames {:2} meshes {:2} | vertices {:6} faces {:6} normals {:6} uv {:6} colors {:5} materials {:3} | {}",
                        file.frames.len(),
                        file.meshes.len(),
                        flat.positions.len(),
                        flat.faces.len(),
                        flat.normals.len(),
                        flat.texcoords.len(),
                        flat.colors.len(),
                        flat.materials.len(),
                        textures.into_iter().collect::<Vec<_>>().join(" "),
                    );
                    if flat.positions.is_empty() || flat.faces.is_empty() {
                        failures.push(format!("{name}: an empty mesh"));
                    }
                }
                Err(e) => {
                    println!("{name:58} {format} ERROR {e}");
                    failures.push(format!("{name}: {e}"));
                }
            }
        }
        println!("{} files", files.len());
        assert!(failures.is_empty(), "{failures:#?}");
    }
}
