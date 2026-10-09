use rapidr_diagnostics::TextSpan;

pub mod numeric;
pub mod objects;
pub mod array_refs;
pub mod suffix_routines;
pub mod suffix_vars;
pub mod for_locals;
pub mod memory;
pub mod type_values;
pub mod library;
pub mod method_equals;
pub mod tray_calls;

/// A name without its type suffix (`n%` → `n`, `w??` → `w`).
pub fn strip_type_suffix(name: &str) -> &str {
    let base = name.trim_end_matches('?');
    let base = if base.len() < name.len() { base } else { name.strip_suffix(['$', '%', '&', '!', '#']).unwrap_or(name) };
    if base.is_empty() { name } else { base }
}

/// Whether `ident` names routine `routine` (both as written): the same
/// name, and no type suffix or the routine's own — `day&` inside FUNCTION
/// Day is a variable of its own, as RapidQ keeps them apart.
pub fn names_routine(ident: &str, routine: &str) -> bool {
    let (i, r) = (strip_type_suffix(ident), strip_type_suffix(routine));
    i.eq_ignore_ascii_case(r) && (i.len() == ident.len() || ident[i.len()..] == routine[r.len()..])
}

/// The type a suffix declares (RapidQ manual, data types): `?` BYTE, `??`
/// WORD, `???` DWORD, `%` SHORT, `&` LONG, `!` SINGLE, `#` DOUBLE, `$`
/// STRING.
pub fn suffix_type(name: &str) -> Option<&'static str> {
    let qs = name.len() - name.trim_end_matches('?').len();
    Some(match (qs, name.chars().last()?) {
        (1, _) => "BYTE",
        (2, _) => "WORD",
        (3, _) => "DWORD",
        (_, '%') => "SHORT",
        (_, '&') => "LONG",
        (_, '!') => "SINGLE",
        (_, '#') => "DOUBLE",
        (_, '$') => "STRING",
        _ => return None,
    })
    .filter(|_| strip_type_suffix(name).len() < name.len())
}
pub mod stream_arrays;
pub mod implicit_scope;

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub span: TextSpan,
    pub statements: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Assignment(AssignmentStatement),
    Bind(BindStatement),
    Call(CallStatement),
    Close(CloseStatement),
    Comment(CommentStatement),
    Const(ConstStatement),
    Create(CreateStatement),
    Declare(DeclareStatement),
    Dim(DimStatement),
    Directive(DirectiveStatement),
    DoLoop(DoLoopStatement),
    Exit(ExitStatement),
    For(ForStatement),
    Function(FunctionStatement),
    /// `GOSUB label`
    Gosub(JumpStatement),
    /// `GOTO label`
    Goto(JumpStatement),
    If(IfStatement),
    Import(ImportStatement),
    Input(InputStatement),
    /// `name:` or a line number at the start of a line. A bare `name:` may
    /// also be a call to a zero-argument SUB followed by `:`; code
    /// generators decide by looking the name up.
    Label(LabelStatement),
    Line(LineStatement),
    Open(OpenStatement),
    Print(PrintStatement),
    PrintHash(PrintHashStatement),
    Return(ReturnStatement),
    Seek(SeekStatement),
    SelectCase(SelectCaseStatement),
    Subroutine(SubroutineStatement),
    Type(TypeStatement),
    While(WhileStatement),
    With(WithStatement),
    WriteHash(WriteHashStatement),
    RustBlock(RustBlockStatement),
}

#[derive(Debug, Clone, PartialEq)]
pub struct AssignmentStatement {
    pub span: TextSpan,
    pub target: Expression,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CallStatement {
    pub span: TextSpan,
    pub callee: Expression,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConstStatement {
    pub span: TextSpan,
    pub name: String,
    pub declared_type: Option<String>,
    pub value: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DimStatement {
    pub span: TextSpan,
    pub declarators: Vec<VariableDeclarator>,
    pub type_name: String,
    /// `AS STRING * 20`: the fixed length (stores are cut to it).
    pub fixed_len: Option<usize>,
    /// `STATIC x AS T` inside a SUB/FUNCTION: one variable shared by every
    /// call (and recursion) of that procedure, initialised once.
    pub is_static: bool,
    /// `REDIM a(n) AS T`: resize keeping the data (`rapidr_value::redim`).
    pub is_redim: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariableDeclarator {
    pub span: TextSpan,
    pub name: String,
    pub dimensions: Vec<ArrayDimension>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum ArrayDimension {
    Single(Expression),
    Range { start: Expression, end: Expression },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectiveStatement {
    pub span: TextSpan,
    pub name: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ImportStatement {
    pub span: TextSpan,
    pub module_name: String,
    pub alias: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LineStatement {
    pub span: TextSpan,
    pub kind: LineStatementKind,
    pub text: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PrintStatement {
    pub span: TextSpan,
    pub items: Vec<Expression>,
    /// `zones[i]`: item i moves to the next 14-column print zone after it
    /// (QBasic's `,`). The parser leaves them all false: in RapidQ the comma
    /// and the semicolon have the same effect.
    pub zones: Vec<bool>,
    /// False when the statement ends with `;` or `,` (stay on the line).
    pub append_newline: bool,
}

// --- Block constructs ---

#[derive(Debug, Clone, PartialEq)]
pub struct IfStatement {
    pub span: TextSpan,
    pub condition: Expression,
    pub then_body: Vec<Statement>,
    pub elseif_branches: Vec<ElseIfBranch>,
    pub else_body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElseIfBranch {
    pub span: TextSpan,
    pub condition: Expression,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStatement {
    pub span: TextSpan,
    pub variable: String,
    pub start: Expression,
    pub end: Expression,
    pub step: Option<Expression>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileStatement {
    pub span: TextSpan,
    pub condition: Expression,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DoLoopStatement {
    pub span: TextSpan,
    pub condition: Option<Expression>,
    pub pre_condition: bool,
    pub is_until: bool,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SelectCaseStatement {
    pub span: TextSpan,
    pub expression: Expression,
    pub cases: Vec<CaseBranch>,
    pub case_else: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseBranch {
    pub span: TextSpan,
    /// The branch matches when any of these matches (`CASE 1, 5 TO 9, IS > 20`).
    pub values: Vec<CaseValue>,
    pub body: Vec<Statement>,
}

/// One item of a `CASE` list, tested against the SELECT expression.
#[derive(Debug, Clone, PartialEq)]
pub enum CaseValue {
    /// `CASE 3` — equal to the value.
    Value(Expression),
    /// `CASE 1 TO 5` — between the bounds, inclusive.
    Range(Expression, Expression),
    /// `CASE IS > 10` — the comparison holds (`=`, `<>`, `<`, `<=`, `>`, `>=`).
    Is(BinaryOperator, Expression),
    /// `CASE IS = "l" AND MID$(s, i, 1) = "d"` — the comparison, then AND /
    /// OR / XOR with more conditions, left to right: RapidQ's compiler reads
    /// the IS comparison as the first operand of a logical expression (the
    /// corpus' printf.bas).
    IsLogic(BinaryOperator, Expression, Vec<(BinaryOperator, Expression)>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct SubroutineStatement {
    pub span: TextSpan,
    pub name: String,
    pub params: Vec<Parameter>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionStatement {
    pub span: TextSpan,
    pub name: String,
    pub params: Vec<Parameter>,
    pub return_type: Option<String>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parameter {
    pub span: TextSpan,
    pub name: String,
    pub type_name: String,
    pub by_ref: bool,
    /// `list() AS STRING`: an array parameter (arrays are shared with the caller).
    pub is_array: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeStatement {
    pub span: TextSpan,
    pub name: String,
    pub extends: Option<String>,
    /// `EXTENDS` (or the manual's `AS`) was written, QOBJECT included —
    /// `extends` leaves QOBJECT out (a plain TYPE with methods), but RapidQ
    /// takes its data-type objects (QRECT …) as fields only in an object.
    pub object_base: bool,
    pub fields: Vec<TypeField>,
    pub methods: Vec<Statement>,
    pub constructor: Vec<Statement>,
    /// `EVENT OnClick … END EVENT`: handlers every instance gets bound to.
    pub events: Vec<TypeEvent>,
    /// A template's parameters (`TYPE NewClass<DataType, Size>`; see
    /// [`templates`]); empty for an ordinary TYPE.
    pub template_params: Vec<String>,
}

/// An EVENT block inside TYPE … EXTENDS: the handler for one event of every
/// instance, with the instance available as `This`.
#[derive(Debug, Clone, PartialEq)]
pub struct TypeEvent {
    pub span: TextSpan,
    pub name: String,
    pub params: Vec<Parameter>,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeField {
    pub span: TextSpan,
    pub name: String,
    pub type_name: String,
    /// `Name AS STRING * 20`: the fixed length (stores are cut to it).
    pub fixed_len: Option<usize>,
    /// Upper bound of an array field (`Names(2)` or `Colors(1 TO 16)`).
    pub array_size: Option<Expression>,
    /// Lower bound of an array field, when written (`Colors(1 TO 16)`); 0 otherwise.
    pub array_lower: Option<Expression>,
    /// The further dimensions of a multi-dimensional array field
    /// (`vertex(0 TO 9, 2)`): (lower, upper) of the 2nd, 3rd, …
    pub more_dims: Vec<(Expression, Expression)>,
    /// `Focus AS LONG PROPERTY SET Set_Focus`: assigning the field from
    /// outside the setter calls the `PROPERTY SET Set_Focus (v AS LONG)`
    /// method instead (which stores the value itself).
    pub setter: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CreateStatement {
    pub span: TextSpan,
    pub name: String,
    pub type_name: String,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WithStatement {
    pub span: TextSpan,
    pub object: Expression,
    pub body: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExitStatement {
    pub span: TextSpan,
    pub exit_type: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LabelStatement {
    pub span: TextSpan,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JumpStatement {
    pub span: TextSpan,
    pub label: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStatement {
    pub span: TextSpan,
    pub value: Option<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct InputStatement {
    pub span: TextSpan,
    pub prompt: Option<Expression>,
    pub target: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BindStatement {
    pub span: TextSpan,
    pub target: Expression,
    pub handler: Expression,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclareStatement {
    pub span: TextSpan,
    pub is_function: bool,
    pub name: String,
    pub lib: Option<String>,
    pub alias: Option<String>,
    pub params: Vec<Parameter>,
    pub return_type: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CommentStatement {
    pub span: TextSpan,
    pub text: String,
}

/// OPEN "filename" FOR mode AS #n
#[derive(Debug, Clone, PartialEq)]
pub struct OpenStatement {
    pub span: TextSpan,
    pub filename: Expression,
    pub mode: String,
    pub file_number: Expression,
}

/// CLOSE #n
#[derive(Debug, Clone, PartialEq)]
pub struct CloseStatement {
    pub span: TextSpan,
    pub file_number: Expression,
}

/// PRINT #n, items...
#[derive(Debug, Clone, PartialEq)]
pub struct PrintHashStatement {
    pub span: TextSpan,
    pub file_number: Expression,
    pub items: Vec<Expression>,
}

/// WRITE #n, items...
#[derive(Debug, Clone, PartialEq)]
pub struct WriteHashStatement {
    pub span: TextSpan,
    pub file_number: Expression,
    pub items: Vec<Expression>,
}

/// SEEK #n, position
#[derive(Debug, Clone, PartialEq)]
pub struct SeekStatement {
    pub span: TextSpan,
    pub file_number: Expression,
    pub position: Expression,
}

/// Raw Rust code block: RUSTSTART ... RUSTEND
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RustBlockStatement {
    pub span: TextSpan,
    pub code: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LineStatementKind {
    AssignmentOrCall,
    Bind,
    Case,
    Close,
    Const,
    Create,
    Declare,
    Dim,
    Do,
    Else,
    ElseIf,
    End,
    Exit,
    For,
    Function,
    If,
    Import,
    Loop,
    Next,
    Open,
    Print,
    Return,
    Seek,
    Select,
    Sub,
    Type,
    While,
    Wend,
    With,
    Write,
    Unknown,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expression {
    ArrayAccess(ArrayAccessExpression),
    Binary(BinaryExpression),
    FunctionCall(FunctionCallExpression),
    Identifier(Identifier),
    Literal(Literal),
    MemberAccess(MemberAccessExpression),
    MethodCall(MethodCallExpression),
    Unary(UnaryExpression),
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArrayAccessExpression {
    pub span: TextSpan,
    pub array: Box<Expression>,
    pub indices: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BinaryExpression {
    pub span: TextSpan,
    pub left: Box<Expression>,
    pub operator: BinaryOperator,
    pub right: Box<Expression>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BinaryOperator {
    Add,
    And,
    Concat,
    Divide,
    Equal,
    GreaterThan,
    GreaterThanOrEqual,
    IntegerDivide,
    LessThan,
    LessThanOrEqual,
    Modulo,
    Multiply,
    NotEqual,
    Or,
    Power,
    Subtract,
    Xor,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionCallExpression {
    pub span: TextSpan,
    pub callee: Box<Expression>,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Identifier {
    pub span: TextSpan,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MemberAccessExpression {
    pub span: TextSpan,
    pub object: Box<Expression>,
    pub member: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MethodCallExpression {
    pub span: TextSpan,
    pub object: Box<Expression>,
    pub method: String,
    pub args: Vec<Expression>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum LiteralValue {
    Integer(i64),
    Float(f64),
    String(String),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Literal {
    pub span: TextSpan,
    pub value: LiteralValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct UnaryExpression {
    pub span: TextSpan,
    pub operator: UnaryOperator,
    pub operand: Box<Expression>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UnaryOperator {
    Negate,
    Not,
    Positive,
    /// `@x` as an argument: pass `x` by reference (RapidQ manual 3.5), as if
    /// the parameter were declared BYREF.
    Ref,
}

/// RapidQ's `INC x [, n]` / `DEC x [, n]` as the assignment `x = x ± n`
/// (None if this call isn't one, e.g. a user SUB named Inc). Used by both
/// backends so they accept exactly the same forms.
pub fn inc_dec_assignment(c: &CallStatement, is_user_routine: impl Fn(&str) -> bool) -> Option<AssignmentStatement> {
    let Expression::Identifier(id) = &c.callee else { return None };
    let op = match id.name.to_ascii_lowercase().as_str() {
        "inc" => BinaryOperator::Add,
        "dec" => BinaryOperator::Subtract,
        _ => return None,
    };
    if is_user_routine(&id.name) || c.args.is_empty() || c.args.len() > 2 {
        return None;
    }
    let target = c.args[0].clone();
    if !matches!(target, Expression::Identifier(_) | Expression::ArrayAccess(_) | Expression::FunctionCall(_) | Expression::MemberAccess(_)) {
        return None;
    }
    let amount = c.args.get(1).cloned().unwrap_or(Expression::Literal(Literal {
        span: c.span,
        value: LiteralValue::Integer(1),
    }));
    Some(AssignmentStatement {
        span: c.span,
        target: target.clone(),
        value: Expression::Binary(BinaryExpression {
            span: c.span,
            left: Box::new(target),
            operator: op,
            right: Box::new(amount),
        }),
    })
}

/// RapidQ's templates (manual 10.8): `TYPE NewClass<DataType, Size> …
/// END TYPE` is made anew for each set of arguments a program uses
/// (`DIM x AS NewClass<INTEGER, 10>` → a TYPE `NewClass__INTEGER_10`), its
/// parameters replaced in its fields' types and sizes and in its code: a
/// type name where a type goes, the argument itself (`10`, a CONST) where
/// a value goes. The template itself is dropped (both backends).
pub fn templates(program: &Program) -> Program {
    use std::collections::HashMap;
    let defs: HashMap<String, TypeStatement> = program
        .statements
        .iter()
        .filter_map(|s| match s {
            Statement::Type(t) if !t.template_params.is_empty() => Some((t.name.to_ascii_uppercase(), t.clone())),
            _ => None,
        })
        .collect();
    if defs.is_empty() {
        return program.clone();
    }
    // `Name<A,B>` → (Name, [A, B]) when Name is a template.
    let split = |t: &str| -> Option<(String, Vec<String>)> {
        let (base, rest) = t.split_once('<')?;
        let args: Vec<String> = rest.strip_suffix('>')?.split(',').map(|a| a.trim().to_string()).collect();
        defs.contains_key(&base.trim().to_ascii_uppercase()).then(|| (base.trim().to_string(), args))
    };
    let mangle = |base: &str, args: &[String]| -> String {
        let parts: Vec<String> = args.iter().map(|a| a.chars().map(|c| if c.is_ascii_alphanumeric() { c } else { '_' }).collect()).collect();
        format!("{base}__{}", parts.join("_"))
    };
    // Renames every `Name<A,B>` type in `stmts`, noting the instantiations.
    let rename = |stmts: &mut Vec<Statement>, wanted: &mut Vec<(String, String, Vec<String>)>| {
        let mut fix = |t: &mut String| {
            if let Some((base, args)) = split(t) {
                let name = mangle(&base, &args);
                if !wanted.iter().any(|(n, _, _)| n.eq_ignore_ascii_case(&name)) {
                    wanted.push((name.clone(), base, args));
                }
                *t = name;
            }
        };
        walk_statements_mut(stmts, &mut |st| match st {
            Statement::Dim(d) => fix(&mut d.type_name),
            Statement::Create(c) => fix(&mut c.type_name),
            Statement::Subroutine(r) => r.params.iter_mut().for_each(|p| fix(&mut p.type_name)),
            Statement::Function(r) => r.params.iter_mut().for_each(|p| fix(&mut p.type_name)),
            Statement::Type(t) => t.fields.iter_mut().for_each(|f| fix(&mut f.type_name)),
            _ => {}
        });
    };
    let mut statements: Vec<Statement> = program.statements.iter().filter(|s| !matches!(s, Statement::Type(t) if !t.template_params.is_empty())).cloned().collect();
    let mut wanted = Vec::new();
    rename(&mut statements, &mut wanted);
    // Each instantiation (those inside them too), in the order first used.
    let mut made: Vec<Statement> = Vec::new();
    let mut done = 0;
    while done < wanted.len() && made.len() < 256 {
        let (name, base, args) = wanted[done].clone();
        done += 1;
        let Some(def) = defs.get(&base.to_ascii_uppercase()) else { continue };
        let mut t = TypeStatement { name, template_params: Vec::new(), ..def.clone() };
        let arg_of = |n: &str| def.template_params.iter().position(|p| p.eq_ignore_ascii_case(n)).and_then(|i| args.get(i).cloned());
        let as_type = |ty: &mut String| {
            if let Some(a) = arg_of(ty) {
                *ty = canonical_type_name(&a);
            }
        };
        let as_value = |e: &mut Expression| {
            if let Expression::Identifier(id) = e {
                if let Some(a) = arg_of(&id.name) {
                    let span = id.span;
                    *e = if let Ok(n) = a.parse::<i64>() {
                        Expression::Literal(Literal { span, value: LiteralValue::Integer(n) })
                    } else if let Ok(x) = a.parse::<f64>() {
                        Expression::Literal(Literal { span, value: LiteralValue::Float(x) })
                    } else if let Some(text) = a.strip_prefix('"').and_then(|x| x.strip_suffix('"')) {
                        Expression::Literal(Literal { span, value: LiteralValue::String(text.to_string()) })
                    } else {
                        Expression::Identifier(Identifier { span, name: a })
                    };
                }
            }
        };
        for f in &mut t.fields {
            as_type(&mut f.type_name);
            for e in [f.array_size.as_mut(), f.array_lower.as_mut()].into_iter().flatten() {
                walk_expression_mut(e, &mut |x| as_value(x));
            }
            for (lo, hi) in &mut f.more_dims {
                walk_expression_mut(lo, &mut |x| as_value(x));
                walk_expression_mut(hi, &mut |x| as_value(x));
            }
        }
        let mut wrapped = vec![Statement::Type(t)];
        walk_statements_mut(&mut wrapped, &mut |st| match st {
            Statement::Dim(d) => as_type(&mut d.type_name),
            Statement::Subroutine(r) => r.params.iter_mut().for_each(|p| as_type(&mut p.type_name)),
            Statement::Function(r) => {
                r.params.iter_mut().for_each(|p| as_type(&mut p.type_name));
                if let Some(rt) = r.return_type.as_mut() {
                    as_type(rt);
                }
            }
            _ => {}
        });
        if let Some(Statement::Type(t)) = wrapped.first_mut() {
            walk_expressions_mut(&mut t.methods, true, &mut |x| as_value(x));
            walk_expressions_mut(&mut t.constructor, true, &mut |x| as_value(x));
            for ev in &mut t.events {
                walk_expressions_mut(&mut ev.body, true, &mut |x| as_value(x));
            }
        }
        rename(&mut wrapped, &mut wanted);
        made.extend(wrapped);
    }
    // (made types first: before the code that DIMs them)
    made.extend(statements);
    Program { statements: made, ..program.clone() }
}

/// RapidQ's dotted TYPE fields — `hdr.hwndFrom AS LONG`, `hdr.code AS
/// LONG`, `Table.Name(150) AS STRING` — a record inside the record: the
/// TYPE gets one field `hdr` of a TYPE made for it (`NMHDR2__hdr`, declared
/// just before), holding `hwndFrom` and `code`; `N.hdr.hwndFrom` is then an
/// ordinary nested member (both backends).
pub fn dotted_fields(program: &Program) -> Program {
    fn split(t: &TypeStatement, out: &mut Vec<Statement>) -> TypeStatement {
        let mut fields: Vec<TypeField> = Vec::new();
        let mut groups: Vec<(String, Vec<TypeField>)> = Vec::new();
        for f in &t.fields {
            match f.name.split_once('.') {
                Some((head, rest)) => {
                    let inner = TypeField { name: rest.to_string(), ..f.clone() };
                    match groups.iter_mut().find(|(h, _)| h.eq_ignore_ascii_case(head)) {
                        Some((_, g)) => g.push(inner),
                        None => {
                            groups.push((head.to_string(), vec![inner]));
                            // (the nested record's place among the fields)
                            fields.push(TypeField {
                                name: head.to_string(),
                                type_name: String::new(),
                                fixed_len: None,
                                array_size: None,
                                array_lower: None,
                                more_dims: Vec::new(),
                                setter: None,
                                ..f.clone()
                            });
                        }
                    }
                }
                None => fields.push(f.clone()),
            }
        }
        for (head, group) in groups {
            let nested_name = format!("{}__{}", t.name, head);
            let nested = TypeStatement {
                span: t.span,
                name: nested_name.clone(),
                extends: None,
                object_base: false,
                fields: group,
                methods: Vec::new(),
                constructor: Vec::new(),
                events: Vec::new(),
                template_params: Vec::new(),
            };
            // (its own dotted fields nest further)
            let nested = split(&nested, out);
            out.push(Statement::Type(nested));
            if let Some(f) = fields.iter_mut().find(|f| f.name.eq_ignore_ascii_case(&head) && f.type_name.is_empty()) {
                f.type_name = nested_name;
            }
        }
        TypeStatement { fields, ..t.clone() }
    }
    if !program.statements.iter().any(|s| matches!(s, Statement::Type(t) if t.fields.iter().any(|f| f.name.contains('.')))) {
        return program.clone();
    }
    let mut statements = Vec::with_capacity(program.statements.len());
    for s in &program.statements {
        match s {
            Statement::Type(t) if t.fields.iter().any(|f| f.name.contains('.')) => {
                let t = split(t, &mut statements);
                statements.push(Statement::Type(t));
            }
            _ => statements.push(s.clone()),
        }
    }
    Program { statements, ..program.clone() }
}

/// The type RapidQ's `$OPTION DIM BYTE|WORD|…|STRING|VARIANT` gives what
/// isn't declared (RapidQ manual, chapter 3); `None` without one.
pub fn option_dim_type(statements: &[Statement]) -> Option<String> {
    let mut found = None;
    for s in statements {
        if let Statement::Directive(d) = s {
            if d.name.eq_ignore_ascii_case("$OPTION") {
                let v = d.value.as_deref().unwrap_or("").trim();
                let mut words = v.split_whitespace();
                if words.next().is_some_and(|w| w.eq_ignore_ascii_case("DIM")) {
                    if let Some(t) = words.next() {
                        found = Some(canonical_type_name(t));
                    }
                }
            }
        }
    }
    found
}

/// `$OPTION DIM INTEGER`: a variable the program never declares — `n = 7 / 2`,
/// `FOR i = …`, `INPUT a`, or one only read (`PRINT zz` is 0, not empty) —
/// is of that type, not RapidR's VARIANT (it is declared at the top, global
/// as undeclared names are). Names with a type suffix keep theirs; CREATE
/// and TYPE bodies set properties, not variables; `is_builtin`: names the
/// runtimes answer themselves (`TIMER`, `PI`). Both backends.
pub fn option_dim(program: &Program, is_builtin: &dyn Fn(&str) -> bool) -> Program {
    use std::collections::HashSet;
    // First, which SUB's undeclared names are its own (implicit_scope):
    // whatever the main program declares, or RapidQ/RapidR answers itself,
    // isn't a variable of a SUB.
    let program = &{
        let mut top: HashSet<String> = HashSet::new();
        let main: Vec<Statement> = program.statements.iter().filter(|s| !matches!(s, Statement::Subroutine(_) | Statement::Function(_))).cloned().collect();
        walk(
            &main,
            &mut |s| match s {
                Statement::Dim(d) => top.extend(d.declarators.iter().map(|v| strip_type_suffix(&v.name).to_ascii_lowercase())),
                Statement::Const(c) => { top.insert(strip_type_suffix(&c.name).to_ascii_lowercase()); }
                Statement::Create(c) => { top.insert(c.name.to_ascii_lowercase()); }
                Statement::Declare(d) => { top.insert(strip_type_suffix(&d.name).to_ascii_lowercase()); }
                Statement::Type(t) => { top.insert(t.name.to_ascii_lowercase()); }
                Statement::Label(l) => { top.insert(l.name.to_ascii_lowercase()); }
                _ => {}
            },
            &mut |_| {},
        );
        for s in &program.statements {
            match s {
                Statement::Subroutine(r) => { top.insert(strip_type_suffix(&r.name).to_ascii_lowercase()); }
                Statement::Function(f) => { top.insert(strip_type_suffix(&f.name).to_ascii_lowercase()); }
                _ => {}
            }
        }
        implicit_scope::lower(program, &|k| {
            top.contains(k)
                || is_builtin(k)
                || rapidr_constant(k).is_some()
                || matches!(k, "true" | "false" | "vttrue" | "vtfalse" | "result" | "me" | "this" | "sender" | "byte" | "word" | "dword" | "short" | "integer" | "long" | "single" | "double" | "string" | "variant" | "currency" | "int64")
                || is_component_type_name(&canonical_type_name(k))
        })
    };
    // (RapidQ's default: a variable with no suffix and no declaration is
    // a DOUBLE)
    let ty = option_dim_type(&program.statements).unwrap_or_else(|| "DOUBLE".to_string());
    if ty == "VARIANT" {
        return program.clone();
    }
    let key = |n: &str| n.to_ascii_lowercase();
    let mut declared: HashSet<String> = HashSet::new();
    for s in &program.statements {
        match s {
            Statement::Dim(d) => declared.extend(d.declarators.iter().map(|v| key(&v.name))),
            Statement::Const(c) => { declared.insert(key(&c.name)); }
            Statement::Subroutine(r) => { declared.insert(key(&r.name)); }
            Statement::Function(f) => { declared.insert(key(&f.name)); }
            Statement::Declare(d) => { declared.insert(key(&d.name)); }
            Statement::Create(c) => { declared.insert(key(&c.name)); }
            Statement::Type(t) => { declared.insert(key(&t.name)); }
            _ => {}
        }
    }
    fn targets(stmts: &[Statement], out: &mut Vec<String>, locals: &mut HashSet<String>) {
        for s in stmts {
            let name_of = |e: &Expression, out: &mut Vec<String>| {
                if let Expression::Identifier(i) = e {
                    out.push(i.name.clone());
                }
            };
            match s {
                Statement::Assignment(a) => name_of(&a.target, out),
                Statement::Input(i) => name_of(&i.target, out),
                Statement::For(f) => {
                    out.push(f.variable.clone());
                    targets(&f.body, out, locals);
                }
                Statement::Dim(d) => locals.extend(d.declarators.iter().map(|v| v.name.to_ascii_lowercase())),
                Statement::If(i) => {
                    targets(&i.then_body, out, locals);
                    for b in &i.elseif_branches {
                        targets(&b.body, out, locals);
                    }
                    targets(&i.else_body, out, locals);
                }
                Statement::DoLoop(d) => targets(&d.body, out, locals),
                Statement::While(w) => targets(&w.body, out, locals),
                Statement::SelectCase(c) => {
                    for case in &c.cases {
                        targets(&case.body, out, locals);
                    }
                    targets(&c.case_else, out, locals);
                }
                _ => {}
            }
        }
    }
    let mut wanted: Vec<String> = Vec::new();
    let add = |names: Vec<String>, locals: &HashSet<String>, wanted: &mut Vec<String>| {
        for n in names {
            let k = n.to_ascii_lowercase();
            // (`__swap_tmp` and the like: the parser's own temporaries)
            if n.starts_with("__") || suffix_type(&n).is_some() || n.contains('.') || declared.contains(&k) || locals.contains(&k) {
                continue;
            }
            if !wanted.iter().any(|w| w.eq_ignore_ascii_case(&n)) {
                wanted.push(n);
            }
        }
    };
    // Names only read: every declaration anywhere (nested CREATEs, local
    // DIMs, parameters) counts, and names used as objects or called don't.
    let mut everywhere: HashSet<String> = HashSet::new();
    walk(
        &program.statements,
        &mut |s| match s {
            Statement::Dim(d) => everywhere.extend(d.declarators.iter().map(|v| strip_type_suffix(&v.name).to_ascii_lowercase())),
            Statement::Const(c) => { everywhere.insert(strip_type_suffix(&c.name).to_ascii_lowercase()); }
            Statement::Create(c) => { everywhere.insert(c.name.to_ascii_lowercase()); }
            Statement::Subroutine(r) => {
                everywhere.insert(strip_type_suffix(&r.name).to_ascii_lowercase());
                everywhere.extend(r.params.iter().map(|p| strip_type_suffix(&p.name).to_ascii_lowercase()));
            }
            Statement::Function(f) => {
                everywhere.insert(strip_type_suffix(&f.name).to_ascii_lowercase());
                everywhere.extend(f.params.iter().map(|p| strip_type_suffix(&p.name).to_ascii_lowercase()));
            }
            Statement::Declare(d) => { everywhere.insert(strip_type_suffix(&d.name).to_ascii_lowercase()); }
            Statement::Type(t) => { everywhere.insert(t.name.to_ascii_lowercase()); }
            Statement::Label(l) => { everywhere.insert(l.name.to_ascii_lowercase()); }
            _ => {}
        },
        &mut |_| {},
    );
    fn reads(stmts: &[Statement], out: &mut Vec<String>, not_vars: &mut HashSet<String>) {
        for s in stmts {
            match s {
                Statement::Type(_) | Statement::Create(_) => {}
                _ => {
                    let mut called = Vec::new();
                    walk(
                    std::slice::from_ref(s),
                    &mut |x| {
                        if let Statement::Call(c) = x {
                            if let Expression::Identifier(i) = &c.callee {
                                called.push(i.name.to_ascii_lowercase());
                            }
                        }
                    },
                    &mut |e| match e {
                        Expression::MemberAccess(m) => {
                            if let Expression::Identifier(i) = m.object.as_ref() {
                                not_vars.insert(i.name.to_ascii_lowercase());
                            }
                        }
                        Expression::FunctionCall(c) => {
                            if let Expression::Identifier(i) = c.callee.as_ref() {
                                not_vars.insert(i.name.to_ascii_lowercase());
                            }
                        }
                        Expression::ArrayAccess(a) => {
                            if let Expression::Identifier(i) = a.array.as_ref() {
                                not_vars.insert(i.name.to_ascii_lowercase());
                            }
                        }
                        Expression::Identifier(i) => out.push(i.name.clone()),
                        _ => {}
                    },
                    );
                    not_vars.extend(called);
                }
            }
        }
    }
    let mut read_names = Vec::new();
    let mut not_vars = HashSet::new();
    reads(&program.statements, &mut read_names, &mut not_vars);
    // (walk goes into CREATE bodies below other blocks: those reads are the
    // component's properties — create_property_reads)
    let mut in_creates = Vec::new();
    walk(
        &program.statements,
        &mut |s| {
            if let Statement::Create(c) = s {
                walk(&c.body, &mut |_| {}, &mut |e| {
                    if let Expression::Identifier(i) = e {
                        in_creates.push(i.name.to_ascii_lowercase());
                    }
                });
            }
        },
        &mut |_| {},
    );
    let only_read: Vec<String> = read_names
        .into_iter()
        .filter(|n| {
            let k = strip_type_suffix(n).to_ascii_lowercase();
            !everywhere.contains(&k)
                && !not_vars.contains(&n.to_ascii_lowercase())
                && !in_creates.contains(&n.to_ascii_lowercase())
                && !is_builtin(&k)
                // (RapidR's own constants, `akLeft` …)
                && rapidr_constant(&k).is_none()
                && !matches!(k.as_str(), "true" | "false" | "vttrue" | "vtfalse" | "result" | "byte" | "word" | "dword" | "short" | "integer" | "long" | "single" | "double" | "string" | "variant" | "currency" | "int64")
                && !is_component_type_name(&canonical_type_name(&k))
        })
        .collect();
    let mut top = Vec::new();
    let mut top_locals = HashSet::new();
    targets(&program.statements, &mut top, &mut top_locals);
    top.extend(only_read);
    add(top, &top_locals, &mut wanted);
    for s in &program.statements {
        let (name, params, body) = match s {
            Statement::Subroutine(r) => (&r.name, &r.params, &r.body),
            Statement::Function(f) => (&f.name, &f.params, &f.body),
            _ => continue,
        };
        let mut locals: HashSet<String> = params.iter().map(|p| key(&p.name)).collect();
        locals.insert(key(name));
        if matches!(s, Statement::Function(_)) {
            // (RapidQ's RESULT = value: the function's return value)
            locals.insert("result".to_string());
        }
        let mut names = Vec::new();
        targets(body, &mut names, &mut locals);
        add(names, &locals, &mut wanted);
    }
    if wanted.is_empty() {
        return program.clone();
    }
    let span = program.span;
    let mut statements: Vec<Statement> = wanted
        .into_iter()
        .map(|name| {
            Statement::Dim(DimStatement {
                span,
                declarators: vec![VariableDeclarator { span, name, dimensions: Vec::new() }],
                type_name: ty.clone(),
                fixed_len: None,
                is_static: false,
                is_redim: false,
            })
        })
        .collect();
    statements.extend(program.statements.iter().cloned());
    Program { statements, ..program.clone() }
}

/// RapidQ's compile-time type check: a value that is certainly a string —
/// `"text"`, `a$`, a STRING variable or constant, `MID$(…)`, a FUNCTION AS
/// STRING, or a sum starting with one — stored into a variable that is
/// certainly a number (`DIM n AS LONG`, `n%`, undeclared under RapidQ's
/// default DOUBLE) is an error, in RapidQ's words: `Type mismatch,
/// expecting type DOUBLE, but got STRING`. Uncertain cases (VARIANTs,
/// objects, properties) pass. Run after [`option_dim`].
pub fn type_mismatches(program: &Program) -> Vec<(TextSpan, String)> {
    use std::collections::HashMap;
    const NUMERIC: &[&str] = &["BYTE", "WORD", "DWORD", "SHORT", "INTEGER", "LONG", "SINGLE", "DOUBLE"];
    type Types = HashMap<String, String>;
    fn scalars(stmts: &[Statement], into: &mut Types) {
        for s in stmts {
            match s {
                Statement::Dim(d) if !d.is_redim => {
                    for v in &d.declarators {
                        if v.dimensions.is_empty() {
                            into.insert(v.name.to_ascii_lowercase(), d.type_name.to_ascii_uppercase());
                        } else {
                            into.remove(&v.name.to_ascii_lowercase());
                        }
                    }
                }
                Statement::Const(c) => {
                    if matches!(c.value, Expression::Literal(Literal { value: LiteralValue::String(_), .. })) {
                        into.insert(c.name.to_ascii_lowercase(), "STRING".to_string());
                    }
                }
                Statement::If(i) => {
                    scalars(&i.then_body, into);
                    for b in &i.elseif_branches {
                        scalars(&b.body, into);
                    }
                    scalars(&i.else_body, into);
                }
                Statement::For(f) => scalars(&f.body, into),
                Statement::While(w) => scalars(&w.body, into),
                Statement::DoLoop(d) => scalars(&d.body, into),
                Statement::SelectCase(c) => {
                    for case in &c.cases {
                        scalars(&case.body, into);
                    }
                    scalars(&c.case_else, into);
                }
                _ => {}
            }
        }
    }
    struct Ctx<'a> {
        globals: &'a Types,
        locals: Types,
        string_functions: &'a [String],
    }
    impl Ctx<'_> {
        fn type_of(&self, name: &str) -> Option<&str> {
            if let Some(t) = suffix_type(name) {
                return Some(t);
            }
            let k = name.to_ascii_lowercase();
            self.locals.get(&k).or_else(|| self.globals.get(&k)).map(String::as_str)
        }
        fn is_string(&self, e: &Expression) -> bool {
            match e {
                Expression::Literal(Literal { value: LiteralValue::String(_), .. }) => true,
                Expression::Identifier(i) => self.type_of(&i.name) == Some("STRING"),
                Expression::FunctionCall(c) => match c.callee.as_ref() {
                    Expression::Identifier(i) => {
                        i.name.ends_with('$') || self.string_functions.iter().any(|f| f.eq_ignore_ascii_case(&i.name))
                    }
                    _ => false,
                },
                Expression::Binary(b) => matches!(b.operator, BinaryOperator::Add | BinaryOperator::Concat) && self.is_string(&b.left),
                _ => false,
            }
        }
    }
    fn check(stmts: &[Statement], ctx: &Ctx, out: &mut Vec<(TextSpan, String)>) {
        for s in stmts {
            match s {
                Statement::Assignment(a) => {
                    if let Expression::Identifier(i) = &a.target {
                        if let Some(t) = ctx.type_of(&i.name).filter(|t| NUMERIC.contains(t)) {
                            if ctx.is_string(&a.value) {
                                out.push((a.span, format!("Type mismatch, expecting type {t}, but got STRING")));
                            }
                        }
                    }
                }
                Statement::If(i) => {
                    check(&i.then_body, ctx, out);
                    for b in &i.elseif_branches {
                        check(&b.body, ctx, out);
                    }
                    check(&i.else_body, ctx, out);
                }
                Statement::For(f) => check(&f.body, ctx, out),
                Statement::While(w) => check(&w.body, ctx, out),
                Statement::DoLoop(d) => check(&d.body, ctx, out),
                Statement::With(w) => check(&w.body, ctx, out),
                Statement::SelectCase(c) => {
                    for case in &c.cases {
                        check(&case.body, ctx, out);
                    }
                    check(&c.case_else, ctx, out);
                }
                _ => {}
            }
        }
    }
    let mut globals = Types::new();
    scalars(&program.statements, &mut globals);
    let string_functions: Vec<String> = program
        .statements
        .iter()
        .filter_map(|s| match s {
            Statement::Function(f) if f.return_type.as_deref().is_some_and(|t| t.eq_ignore_ascii_case("STRING")) => Some(f.name.clone()),
            _ => None,
        })
        .collect();
    let mut out = Vec::new();
    check(&program.statements, &Ctx { globals: &globals, locals: Types::new(), string_functions: &string_functions }, &mut out);
    for s in &program.statements {
        let (name, params, body, is_function) = match s {
            Statement::Subroutine(r) => (&r.name, &r.params, &r.body, false),
            Statement::Function(f) => (&f.name, &f.params, &f.body, true),
            _ => continue,
        };
        let mut locals = Types::new();
        for p in params {
            locals.insert(p.name.to_ascii_lowercase(), if p.is_array { String::new() } else { p.type_name.to_ascii_uppercase() });
        }
        if is_function {
            // (the function's own name and RESULT hold its return value)
            locals.insert(name.to_ascii_lowercase(), String::new());
            locals.insert("result".to_string(), String::new());
        }
        scalars(body, &mut locals);
        check(body, &Ctx { globals: &globals, locals, string_functions: &string_functions }, &mut out);
    }
    out
}

/// RapidQ's `$TYPECHECK ON` (and `$OPTION EXPLICIT`, "same as using
/// TYPECHECK ON"): from there until `$TYPECHECK OFF`, a variable stored
/// into (`x = …`, `FOR x`, `INPUT x`) must have been declared — DIM,
/// CONST, a parameter — or it is RapidQ's `Undeclared identifier x`; a
/// name read anywhere else (an argument — `List.AddItems itme` —, an
/// operand, a condition, what PRINT prints) is RC.EXE's `Undefined symbol
/// ITME` (the name in upper case, its suffix kept). Run on the program as
/// written (before [`hoist_routines`]), so each SUB is checked with the
/// setting where it stands. `is_builtin`: RapidQ's own names
/// (`NViewLibPresent = 2` sets one); RapidR's own constants and
/// True / False are known too (its additions). A name stored into while
/// the check is off is a variable from there on, as RapidQ makes it one.
pub fn typecheck_errors(program: &Program, is_builtin: &dyn Fn(&str) -> bool) -> Vec<(TextSpan, String)> {
    use std::collections::HashSet;
    fn switch(s: &Statement, on: &mut bool) {
        if let Statement::Directive(d) = s {
            let v = d.value.as_deref().unwrap_or("").trim().to_ascii_uppercase();
            if d.name.eq_ignore_ascii_case("$TYPECHECK") {
                *on = v.starts_with("ON");
            } else if d.name.eq_ignore_ascii_case("$OPTION") && v.starts_with("EXPLICIT") {
                *on = true;
            }
        }
    }
    fn declared_in(stmts: &[Statement], into: &mut HashSet<String>) {
        for s in stmts {
            match s {
                Statement::Dim(d) => into.extend(d.declarators.iter().map(|v| strip_type_suffix(&v.name).to_ascii_lowercase())),
                Statement::Const(c) => { into.insert(strip_type_suffix(&c.name).to_ascii_lowercase()); }
                // (a component and the ones CREATEd inside it)
                Statement::Create(c) => {
                    into.insert(strip_type_suffix(&c.name).to_ascii_lowercase());
                    declared_in(&c.body, into);
                }
                Statement::If(i) => {
                    declared_in(&i.then_body, into);
                    for b in &i.elseif_branches {
                        declared_in(&b.body, into);
                    }
                    declared_in(&i.else_body, into);
                }
                Statement::For(f) => declared_in(&f.body, into),
                Statement::While(w) => declared_in(&w.body, into),
                Statement::DoLoop(d) => declared_in(&d.body, into),
                Statement::With(w) => declared_in(&w.body, into),
                Statement::SelectCase(c) => {
                    for case in &c.cases {
                        declared_in(&case.body, into);
                    }
                    declared_in(&c.case_else, into);
                }
                _ => {}
            }
        }
    }
    // (`OutVal%` is the `outval` DIM declared)
    let key = |n: &str| strip_type_suffix(n).to_ascii_lowercase();
    /// What every routine sees: the program's declarations, and the
    /// variables its main program made by storing into them unchecked.
    struct Shared<'a> {
        globals: HashSet<String>,
        is_builtin: &'a dyn Fn(&str) -> bool,
    }
    struct Check {
        /// A routine's parameters and DIMs (and what it stored into
        /// unchecked); `None` for the main program, whose are the globals.
        locals: Option<HashSet<String>>,
        reported: HashSet<String>,
    }
    impl Check {
        fn known(&self, k: &str, shared: &Shared) -> bool {
            self.locals.as_ref().is_some_and(|l| l.contains(k))
                || shared.globals.contains(k)
                || (shared.is_builtin)(k)
                || rapidr_constant(k).is_some()
                || matches!(k, "true" | "false" | "vttrue" | "vtfalse")
        }
        fn skipped(name: &str) -> bool {
            name.starts_with("__") || name.contains('.') || name.eq_ignore_ascii_case("_with_")
        }
        fn store(&mut self, name: &str, span: TextSpan, on: bool, shared: &mut Shared, out: &mut Vec<(TextSpan, String)>) {
            let k = strip_type_suffix(name).to_ascii_lowercase();
            if Self::skipped(name) || self.known(&k, shared) {
                return;
            }
            if !on {
                // (the variable RapidQ makes of it)
                match self.locals.as_mut() {
                    Some(l) => l.insert(k),
                    None => shared.globals.insert(k),
                };
                return;
            }
            if self.reported.insert(k) {
                out.push((span, format!("Undeclared identifier {name}")));
            }
        }
        /// The names `e` reads: a bare name is checked; the object of a
        /// member or method and a called routine or array are not
        /// (RapidR's other checks name those).
        fn read(&mut self, e: &Expression, on: bool, shared: &Shared, out: &mut Vec<(TextSpan, String)>) {
            if !on {
                return;
            }
            match e {
                Expression::Identifier(i) => {
                    let k = strip_type_suffix(&i.name).to_ascii_lowercase();
                    if Self::skipped(&i.name) || self.known(&k, shared) {
                        return;
                    }
                    if self.reported.insert(k) {
                        out.push((i.span, format!("Undefined symbol {}", i.name.to_ascii_uppercase())));
                    }
                }
                Expression::Literal(_) => {}
                Expression::Binary(b) => {
                    self.read(&b.left, on, shared, out);
                    self.read(&b.right, on, shared, out);
                }
                Expression::Unary(u) => self.read(&u.operand, on, shared, out),
                Expression::MemberAccess(m) => self.read_callee(&m.object, on, shared, out),
                Expression::MethodCall(m) => {
                    self.read_callee(&m.object, on, shared, out);
                    self.read_all(&m.args, on, shared, out);
                }
                Expression::FunctionCall(f) => {
                    self.read_callee(&f.callee, on, shared, out);
                    self.read_all(&f.args, on, shared, out);
                }
                Expression::ArrayAccess(a) => {
                    self.read_callee(&a.array, on, shared, out);
                    self.read_all(&a.indices, on, shared, out);
                }
            }
        }
        fn read_all(&mut self, es: &[Expression], on: bool, shared: &Shared, out: &mut Vec<(TextSpan, String)>) {
            for e in es {
                self.read(e, on, shared, out);
            }
        }
        /// An object, a routine or an array: not a bare name read, but what
        /// leads to it may be (`a(i).x`).
        fn read_callee(&mut self, e: &Expression, on: bool, shared: &Shared, out: &mut Vec<(TextSpan, String)>) {
            if !matches!(e, Expression::Identifier(_)) {
                self.read(e, on, shared, out);
            }
        }
        /// What a store's target reads: an element's indices.
        fn target(&mut self, e: &Expression, on: bool, shared: &Shared, out: &mut Vec<(TextSpan, String)>) {
            if !matches!(e, Expression::Identifier(_)) {
                self.read_callee(e, on, shared, out);
            }
        }
        fn walk(&mut self, stmts: &[Statement], on: &mut bool, shared: &mut Shared, out: &mut Vec<(TextSpan, String)>) {
            for s in stmts {
                switch(s, on);
                match s {
                    Statement::Assignment(a) => {
                        self.read(&a.value, *on, shared, out);
                        self.target(&a.target, *on, shared, out);
                        if let Expression::Identifier(i) = &a.target {
                            self.store(&i.name, a.span, *on, shared, out);
                        }
                    }
                    Statement::Call(c) => {
                        self.read_callee(&c.callee, *on, shared, out);
                        self.read_all(&c.args, *on, shared, out);
                    }
                    Statement::Print(p) => self.read_all(&p.items, *on, shared, out),
                    Statement::Input(i) => {
                        if let Some(p) = &i.prompt {
                            self.read(p, *on, shared, out);
                        }
                        self.target(&i.target, *on, shared, out);
                        if let Expression::Identifier(t) = &i.target {
                            self.store(&t.name, i.span, *on, shared, out);
                        }
                    }
                    Statement::For(f) => {
                        self.read(&f.start, *on, shared, out);
                        self.read(&f.end, *on, shared, out);
                        if let Some(step) = &f.step {
                            self.read(step, *on, shared, out);
                        }
                        self.store(&f.variable, f.span, *on, shared, out);
                        self.walk(&f.body, on, shared, out);
                    }
                    Statement::If(i) => {
                        self.read(&i.condition, *on, shared, out);
                        self.walk(&i.then_body, on, shared, out);
                        for b in &i.elseif_branches {
                            self.read(&b.condition, *on, shared, out);
                            self.walk(&b.body, on, shared, out);
                        }
                        self.walk(&i.else_body, on, shared, out);
                    }
                    Statement::While(w) => {
                        self.read(&w.condition, *on, shared, out);
                        self.walk(&w.body, on, shared, out);
                    }
                    Statement::DoLoop(d) => {
                        if let Some(c) = &d.condition {
                            self.read(c, *on, shared, out);
                        }
                        self.walk(&d.body, on, shared, out);
                    }
                    Statement::With(w) => {
                        self.read_callee(&w.object, *on, shared, out);
                        self.walk(&w.body, on, shared, out);
                    }
                    Statement::SelectCase(c) => {
                        self.read(&c.expression, *on, shared, out);
                        for case in &c.cases {
                            for v in &case.values {
                                match v {
                                    CaseValue::Value(e) | CaseValue::Is(_, e) => self.read(e, *on, shared, out),
                                    CaseValue::Range(a, b) => {
                                        self.read(a, *on, shared, out);
                                        self.read(b, *on, shared, out);
                                    }
                                    CaseValue::IsLogic(_, e, rest) => {
                                        self.read(e, *on, shared, out);
                                        for (_, x) in rest {
                                            self.read(x, *on, shared, out);
                                        }
                                    }
                                }
                            }
                            self.walk(&case.body, on, shared, out);
                        }
                        self.walk(&c.case_else, on, shared, out);
                    }
                    _ => {}
                }
            }
        }
    }
    if !program.statements.iter().any(|s| {
        let mut on = false;
        switch(s, &mut on);
        on
    }) {
        return Vec::new();
    }
    let mut globals: HashSet<String> = HashSet::new();
    declared_in(&program.statements, &mut globals);
    for s in &program.statements {
        match s {
            Statement::Subroutine(r) => { globals.insert(key(&r.name)); }
            Statement::Function(f) => { globals.insert(key(&f.name)); }
            Statement::Declare(d) => { globals.insert(key(&d.name)); }
            Statement::Type(t) => { globals.insert(key(&t.name)); }
            _ => {}
        }
    }
    let mut shared = Shared { globals, is_builtin };
    let mut out = Vec::new();
    let mut on = false;
    let mut main = Check { locals: None, reported: HashSet::new() };
    for s in &program.statements {
        let routine = match s {
            Statement::Subroutine(r) => Some((&r.name, &r.params, &r.body, false)),
            Statement::Function(f) => Some((&f.name, &f.params, &f.body, true)),
            _ => None,
        };
        match routine {
            Some((name, params, body, is_function)) => {
                let mut locals: HashSet<String> = params.iter().map(|p| key(&p.name)).collect();
                if is_function {
                    locals.insert(key(name));
                    locals.insert("result".to_string());
                }
                declared_in(body, &mut locals);
                let mut c = Check { locals: Some(locals), reported: HashSet::new() };
                // (a $TYPECHECK inside the SUB holds on after it)
                c.walk(body, &mut on, &mut shared, &mut out);
            }
            None => main.walk(std::slice::from_ref(s), &mut on, &mut shared, &mut out),
        }
    }
    out
}

/// A property RC.EXE refuses to set with its `X.P is a read-only value.`:
/// QDXJOYSTICK's state (and RapidR's additions to it).
pub fn is_read_only_value(type_name: &str, property: &str) -> bool {
    let read_only: &[&str] = match canonical_type_name(type_name).to_ascii_uppercase().as_str() {
        "RDXJOYSTICK" => &["IsLeft", "IsRight", "IsUp", "IsDown", "Connected", "Name", "X", "Y", "Z", "R", "U", "V", "Buttons", "POV"],
        // (RC.EXE: `N.CBSIZE is a read-only value.`, `G.HANDLE is …`)
        "RNOTIFYICONDATA" => &["cbSize"],
        "RGLASSFRAME" => &["Handle"],
        // (`M.MAXY is a read-only value.`)
        "RD3DMESH" => &["MaxY", "MinY"],
        // (QCOMPORT's: `C.CONNECTED is a read-only value.`)
        "RCOMPORT" => &["Connected", "Handle", "InQue", "OutQue", "PendingIO"],
        _ => &[],
    };
    read_only.iter().any(|p| p.eq_ignore_ascii_case(property))
}

/// The members RapidQ's compiler knows for its data types QRECT and
/// QNOTIFYICONDATA, for QGLASSFRAME and for QD3DANIMATION /
/// QD3DANIMATIONSET (RC.EXE's own member table; any
/// other is its `Member X not part of class Y`), lowercase. QGLASSFRAME
/// also takes RapidR's additions every visible component has
/// (AccessibleName, AccessibleDescription, Anchors).
pub fn fixed_members(type_name: &str) -> Option<&'static [&'static str]> {
    Some(match canonical_type_name(type_name).to_ascii_uppercase().as_str() {
        "RRECT" => &["left", "top", "right", "bottom"],
        "RNOTIFYICONDATA" => &["cbsize", "hwnd", "uid", "uflags", "ucallbackmessage", "hicon", "sztip"],
        // (RC.EXE's QD3DANIMATION and QD3DANIMATIONSET: Parent alone)
        "RD3DANIMATION" | "RD3DANIMATIONSET" => &["parent"],
        "RGLASSFRAME" => &[
            "left", "top", "width", "height", "clientwidth", "clientheight", "color", "enabled", "visible", "showhint", "hint",
            "popupmenu", "cursor", "handle", "align", "moveable", "transparency", "transparentcolor", "onclick", "ondblclick",
            "onmousedown", "onmousemove", "onmouseup", "parent",
            // (RapidR's)
            "accessiblename", "accessibledescription", "anchors",
        ],
        _ => return None,
    })
}

/// `= x` with nothing before the `=`: what RapidQ's compiler makes of it
/// (RC.EXE, probes 2026-10-08) — a number is itself (`x = = 5` stores 5,
/// `v.Bar = 2 * 3, 4` passes 6), text is "" (`s$ = = "hi"` stores "",
/// `L.AddItems = "a"` adds an empty line). It is how `Obj.Method = a, b`
/// passes its first argument. A call of `__lone_equals` (rapidr_value).
pub fn lone_equals(span: TextSpan, operand: Expression) -> Expression {
    Expression::FunctionCall(FunctionCallExpression {
        span,
        callee: Box::new(Expression::Identifier(Identifier { span, name: "__lone_equals".into() })),
        args: vec![operand],
    })
}

/// The TYPEs a program makes objects of (their names, uppercase). RapidQ's
/// compiler compiles a TYPE where a DIM (or CREATE) makes one — its errors
/// name the section `V.FOO` after the variable — so a TYPE no DIM makes is
/// never compiled at all: its methods may call routines nobody declared,
/// its fields may be QRECTs in a plain TYPE (RC.EXE, probes 2026-10-08;
/// RapidQ's `direct3d/Lights_pyramid.bas` carries such a TYPE). A parameter
/// `p AS T` makes none. Counted generously: a type named by any DIM or
/// CREATE, by any TYPE's field or EXTENDS, or used as a template.
pub fn instantiated_types(program: &Program) -> std::collections::HashSet<String> {
    let mut out = std::collections::HashSet::new();
    let base = |t: &str| t.split('<').next().unwrap_or("").trim().to_ascii_uppercase();
    walk(
        &program.statements,
        &mut |s| match s {
            Statement::Dim(d) => {
                out.insert(base(&d.type_name));
            }
            Statement::Create(c) => {
                out.insert(base(&c.type_name));
            }
            Statement::Type(t) => {
                out.extend(t.fields.iter().map(|f| base(&f.type_name)));
                if let Some(e) = &t.extends {
                    out.insert(base(e));
                }
            }
            _ => {}
        },
        &mut |_| {},
    );
    out
}

/// An object's name used as a type (RC.EXE, probes 2026-10-08): after `DIM
/// Lst AS QSTRINGLIST`, `DIM x AS Lst` makes a new QSTRINGLIST and `SUB S(p
/// AS Lst)` takes one — the object's own type, as if it were written
/// (RapidQ's `graphics/Choosecolor.bas` has `SUB ButtonClick (Sender AS
/// BUTTON)` after `CREATE Button AS QBUTTON`). A variable of a plain type
/// isn't one (`SUB S(p AS N)` after `DIM n AS INTEGER`: `Unknown type N`).
/// Rewritten here, before both backends and [`rapidq_checks`] read types:
/// the main program's objects (DIM or CREATE of a component or a TYPE), a
/// name no type has.
pub fn object_name_types(program: &Program) -> Program {
    use std::collections::{HashMap, HashSet};
    let types: HashSet<String> = program
        .statements
        .iter()
        .filter_map(|s| match s {
            Statement::Type(t) => Some(t.name.to_ascii_uppercase()),
            _ => None,
        })
        .collect();
    let is_type = |t: &str| {
        let base = t.split('<').next().unwrap_or("").trim();
        !base.is_empty() && (rapidr_lang::type_name(base).is_some() || rapidr_lang::resolve_component(base).is_some() || types.contains(&base.to_ascii_uppercase()))
    };
    let is_object_type = |t: &str| {
        let base = t.split('<').next().unwrap_or("").trim();
        rapidr_lang::resolve_component(base).is_some() || types.contains(&base.to_ascii_uppercase())
    };
    // (the main program's objects, CREATEs nested in CREATEs too)
    let mut objects: HashMap<String, String> = HashMap::new();
    fn collect(stmts: &[Statement], objects: &mut HashMap<String, String>, is_object_type: &dyn Fn(&str) -> bool) {
        for s in stmts {
            match s {
                Statement::Dim(d) if is_object_type(&d.type_name) => {
                    for v in d.declarators.iter().filter(|v| v.dimensions.is_empty()) {
                        objects.entry(strip_type_suffix(&v.name).to_ascii_uppercase()).or_insert_with(|| d.type_name.clone());
                    }
                }
                Statement::Create(c) => {
                    if is_object_type(&c.type_name) {
                        objects.entry(c.name.to_ascii_uppercase()).or_insert_with(|| c.type_name.clone());
                    }
                    collect(&c.body, objects, is_object_type);
                }
                _ => {}
            }
        }
    }
    collect(&program.statements, &mut objects, &is_object_type);
    let mut program = program.clone();
    if objects.is_empty() {
        return program;
    }
    let resolve = |t: &mut String| {
        if !is_type(t) {
            if let Some(real) = objects.get(&t.trim().to_ascii_uppercase()) {
                *t = real.clone();
            }
        }
    };
    walk_statements_mut(&mut program.statements, &mut |s| match s {
        Statement::Dim(d) => resolve(&mut d.type_name),
        Statement::Subroutine(r) => r.params.iter_mut().for_each(|p| resolve(&mut p.type_name)),
        Statement::Function(f) => {
            f.params.iter_mut().for_each(|p| resolve(&mut p.type_name));
            if let Some(t) = f.return_type.as_mut() {
                resolve(t);
            }
        }
        Statement::Declare(d) => d.params.iter_mut().for_each(|p| resolve(&mut p.type_name)),
        Statement::Type(t) => t.fields.iter_mut().for_each(|f| resolve(&mut f.type_name)),
        _ => {}
    });
    program
}

/// RapidQ's errors for a type name nothing defines (RC.EXE, probes
/// 2026-10-08), each in its compiler's words:
/// - `DIM b AS QBITMAPEX` (in a SUB too, arrays too, and a field of a TYPE
///   EXTENDS QOBJECT): `Unknown data type QBITMAPEX`;
/// - a SUB's or FUNCTION's parameter: `Unknown type FOOBAR`;
/// - a FUNCTION's result: `FOOBAR is not a valid data type for your FUNCTION`;
/// - `CREATE q AS FOOBAR`: `Create Method only works for QObjects, not FOOBAR`;
/// - `TYPE T EXTENDS FOOBAR`: `You can only extend QObjects`;
/// - a field of a plain TYPE: `Datatype FOOBAR not supported in STRUCT`.
///
/// A name is known when it is a built-in type (RapidR's own INT64 & co.
/// too: additions), a component of the registry (RapidQ's, RapidR's, the
/// planned ones) or a TYPE of the program; a template's TYPE is checked
/// where it is used. A TYPE no DIM makes isn't compiled by RapidQ (see
/// [`instantiated_types`]): its fields and code aren't checked. Not copied:
/// RC.EXE also refuses `DIM v AS T` above `TYPE T` (one pass); RapidR
/// knows every TYPE of the program wherever it stands.
fn unknown_type_checks(program: &Program, outside_types: &[Statement]) -> Vec<(TextSpan, String)> {
    let mut user: std::collections::HashSet<String> = std::collections::HashSet::new();
    for s in &program.statements {
        if let Statement::Type(t) = s {
            user.insert(t.name.to_ascii_uppercase());
        }
    }
    let known = |t: &str| -> bool {
        let base = t.split('<').next().unwrap_or("").trim();
        base.is_empty()
            || rapidr_lang::type_name(base).is_some()
            || rapidr_lang::resolve_component(base).is_some()
            || user.contains(&base.to_ascii_uppercase())
    };
    let upper = |t: &str| t.trim().to_ascii_uppercase();
    let mut out = Vec::new();
    let made = instantiated_types(program);
    let check_params = |params: &[Parameter], out: &mut Vec<(TextSpan, String)>| {
        for p in params.iter().filter(|p| !known(&p.type_name)) {
            out.push((p.span, format!("Unknown type {}", upper(&p.type_name))));
        }
    };
    let on_stmt = |s: &Statement, out: &mut Vec<(TextSpan, String)>| match s {
        Statement::Dim(d) if !known(&d.type_name) => out.push((d.span, format!("Unknown data type {}", upper(&d.type_name)))),
        Statement::Create(c) if !known(&c.type_name) => out.push((c.span, format!("Create Method only works for QObjects, not {}", upper(&c.type_name)))),
        Statement::Subroutine(r) => check_params(&r.params, out),
        Statement::Function(f) => {
            check_params(&f.params, out);
            if let Some(t) = f.return_type.as_deref().filter(|t| !known(t)) {
                out.push((f.span, format!("{} is not a valid data type for your FUNCTION", upper(t))));
            }
        }
        _ => {}
    };
    walk(outside_types, &mut |s| on_stmt(s, &mut out), &mut |_| {});
    for s in &program.statements {
        let Statement::Type(t) = s else { continue };
        if !t.template_params.is_empty() || !made.contains(&t.name.to_ascii_uppercase()) {
            continue;
        }
        if t.extends.as_deref().is_some_and(|b| !known(b)) {
            out.push((t.span, "You can only extend QObjects".to_string()));
        }
        let object = t.extends.is_some() || t.object_base;
        // (a field `OnFoo AS EVENT(Template)`: RapidQ's custom events)
        for f in t.fields.iter().filter(|f| !known(&f.type_name) && !f.type_name.eq_ignore_ascii_case("EVENT")) {
            let message = if object { format!("Unknown data type {}", upper(&f.type_name)) } else { format!("Datatype {} not supported in STRUCT", upper(&f.type_name)) };
            out.push((f.span, message));
        }
        walk(&t.methods, &mut |s| on_stmt(s, &mut out), &mut |_| {});
        walk(&t.constructor, &mut |s| on_stmt(s, &mut out), &mut |_| {});
    }
    out
}

/// RapidQ's errors about its data types and fixed-member objects
/// ([`fixed_members`]), in its compiler's words:
/// - `Member WIDTH not part of class R` — a member it doesn't have (`R` the
///   object, the member's whole dotted path: `FONT.NAME`);
/// - `Component assignment is not yet supported.` — `R2 = R` of a QRECT or
///   QNOTIFYICONDATA;
/// - `Datatype QRECT not supported in STRUCT` — one in a TYPE or STRUCT
///   without EXTENDS (RC.EXE refuses every component there; RapidR's own
///   programs keep composing TYPEs of components, so only these two are
///   refused). With `EXTENDS QOBJECT` (or any base) it is a field like
///   any object's.
fn fixed_member_checks(program: &Program, outside_types: &[Statement], component_types: &std::collections::HashMap<String, String>) -> Vec<(TextSpan, String)> {
    use std::collections::HashMap as Map;
    let mut out = Vec::new();
    let is_record = |t: &str| matches!(canonical_type_name(t).to_ascii_uppercase().as_str(), "RRECT" | "RNOTIFYICONDATA");
    let rapidq_name = |t: &str| {
        let c = canonical_type_name(t).to_ascii_uppercase();
        c.strip_prefix('R').map_or(c.clone(), |rest| format!("Q{rest}"))
    };
    let made = instantiated_types(program);
    for s in &program.statements {
        if let Statement::Type(t) = s {
            if t.extends.is_none() && !t.object_base && made.contains(&t.name.to_ascii_uppercase()) {
                for f in t.fields.iter().filter(|f| is_record(&f.type_name)) {
                    out.push((f.span, format!("Datatype {} not supported in STRUCT", rapidq_name(&f.type_name))));
                }
            }
        }
    }
    // The QRECT / QNOTIFYICONDATA fields of TYPEs (`TYPE T EXTENDS QOBJECT
    // … R AS QRECT`) and the variables DIMmed as those TYPEs: `v.R` is a
    // QRECT as `r` is (RC.EXE: `v.R = r2` and `v.R.Width` are refused alike).
    let mut record_fields: Map<(String, String), String> = Map::new();
    for s in &program.statements {
        if let Statement::Type(t) = s {
            for f in t.fields.iter().filter(|f| is_record(&f.type_name) && f.array_size.is_none()) {
                record_fields.insert((t.name.to_ascii_uppercase(), f.name.to_ascii_lowercase()), f.type_name.clone());
            }
        }
    }
    let mut typed_vars: Map<String, String> = Map::new();
    if !record_fields.is_empty() {
        walk(
            outside_types,
            &mut |s| {
                if let Statement::Dim(d) = s {
                    let t = d.type_name.trim().to_ascii_uppercase();
                    if record_fields.keys().any(|(ty, _)| *ty == t) {
                        for v in &d.declarators {
                            typed_vars.insert(strip_type_suffix(&v.name).to_ascii_lowercase(), t.clone());
                        }
                    }
                }
            },
            &mut |_| {},
        );
    }
    // (the record a member access names: `r`, or `v.R` of such a TYPE)
    let record_of = |e: &Expression| -> Option<String> {
        match e {
            Expression::Identifier(i) => component_types.get(&i.name.to_ascii_lowercase()).filter(|k| is_record(k)).cloned(),
            Expression::MemberAccess(m) => {
                let Expression::Identifier(root) = m.object.as_ref() else { return None };
                let t = typed_vars.get(&strip_type_suffix(&root.name).to_ascii_lowercase())?;
                record_fields.get(&(t.clone(), m.member.to_ascii_lowercase())).cloned()
            }
            _ => None,
        }
    };
    // (the longest member chain on each use of an object: `g.Font.Name`)
    let mut chains: Map<(usize, usize), (TextSpan, String, String)> = Map::new();
    walk(
        outside_types,
        &mut |s| {
            if let Statement::Assignment(a) = s {
                let kind = |n: &str| component_types.get(&n.to_ascii_lowercase());
                let value_is_object = match &a.value {
                    Expression::Identifier(v) => kind(&v.name).is_some(),
                    other => record_of(other).is_some(),
                };
                if record_of(&a.target).is_some() && value_is_object {
                    out.push((a.span, "Component assignment is not yet supported.".to_string()));
                }
            }
        },
        &mut |e| {
            let Expression::MemberAccess(m) = e else { return };
            let mut path = vec![m.member.clone()];
            let mut object = m.object.as_ref();
            while let Expression::MemberAccess(inner) = object {
                path.push(inner.member.clone());
                object = inner.object.as_ref();
            }
            let Expression::Identifier(root) = object else { return };
            path.reverse();
            // (a record field's members: `v.R.Width` — the first is the field)
            let (members, skip) = match component_types.get(&root.name.to_ascii_lowercase()).and_then(|t| fixed_members(t)) {
                Some(members) => (members, 0),
                None => {
                    let Some(t) = typed_vars.get(&strip_type_suffix(&root.name).to_ascii_lowercase()) else { return };
                    let Some(field_type) = record_fields.get(&(t.clone(), path[0].to_ascii_lowercase())) else { return };
                    if path.len() < 2 {
                        return;
                    }
                    let Some(members) = fixed_members(field_type) else { return };
                    (members, 1)
                }
            };
            let joined = path.join(".");
            let key = (root.span.start, root.span.end);
            if chains.get(&key).is_none_or(|(_, _, p)| p.len() < joined.len()) {
                let first_known = members.contains(&path[skip].to_ascii_lowercase().as_str());
                // (a known member's own members — `G.Parent.Caption` — are the
                // other object's business)
                if !first_known {
                    chains.insert(key, (m.span, root.name.clone(), joined));
                } else {
                    chains.remove(&key);
                }
            }
        },
    );
    let mut found: Vec<_> = chains.into_values().collect();
    found.sort_by_key(|(span, _, _)| (span.start, span.end));
    for (span, root, path) in found {
        out.push((span, format!("Member {} not part of class {}", path.to_ascii_uppercase(), root.to_ascii_uppercase())));
    }
    out
}

const CATCH_ALL_NAMES: &[&str] = &[
    "showmodal", "show", "close", "hide", "repaint", "refresh", "update", "paint", "center", "setparent", "clear", "additems", "additem",
    "deleteitems", "deleteitem", "removeitem", "setfocus", "focus", "click", "addbordericons", "delbordericons", "selectall", "copy", "paste", "cut", "execute",
    "line", "rect", "fillrect", "circle", "ellipse", "setpixel", "getpixel", "drawtext", "loadimage", "saveimage",
];

/// RapidQ's `Member X not part of class Y` for a component's member the
/// language registry (crates/rapidr-lang) doesn't have — RapidQ's members
/// (those RapidR doesn't answer yet too: RC.EXE compiles them) and RapidR's
/// own: `Btn.Click` on a QBUTTON is RC.EXE's error, and RapidR's. Checked on
/// a component the program names (CREATE, DIM AS Q…) and its first member
/// (`Form.Font.Name`: Font); a TYPE EXTENDS one is the TYPE's business, and
/// so are an include library's components (`from`: their members are the
/// include's TYPE's) and the OLE objects (their members are the server's).
fn registry_member_checks(program: &Program, outside_types: &[Statement], component_types: &std::collections::HashMap<String, String>) -> Vec<(TextSpan, String)> {
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    let user_types: std::collections::HashSet<String> =
        program.statements.iter().filter_map(|s| if let Statement::Type(t) = s { Some(canonical_type_name(&t.name).to_ascii_uppercase()) } else { None }).collect();
    walk(outside_types, &mut |_| {}, &mut |e| {
        let Expression::MemberAccess(m) = e else { return };
        let Expression::Identifier(root) = m.object.as_ref() else { return };
        let Some(t) = component_types.get(&root.name.to_ascii_lowercase()) else { return };
        if fixed_members(t).is_some() || user_types.contains(&canonical_type_name(t).to_ascii_uppercase()) {
            return;
        }
        let Some(c) = rapidr_lang::component(&canonical_type_name(t)) else { return };
        // (a Q name RapidQ doesn't have — QToolBar — is an include's TYPE)
        let foreign_q = t.to_ascii_uppercase().starts_with('Q')
            && !c.rapidq.is_some_and(|q| q.eq_ignore_ascii_case(t))
            && !c.aliases.iter().any(|a| a.eq_ignore_ascii_case(t));
        if c.from.is_some() || foreign_q || matches!(c.name, "QOLEOBJECT" | "QOLECONTAINER") {
            return;
        }
        // (`Form.WndProc = F` binds its OnWndProc)
        let event = m.member.get(2..).filter(|_| m.member.to_ascii_lowercase().starts_with("on"));
        if !CATCH_ALL_NAMES.iter().any(|n| n.eq_ignore_ascii_case(&m.member)) {
            return;
        }
        if c.has_member(&m.member) || event.is_some_and(|e| c.event(e).is_some()) || !seen.insert((m.span.start, m.span.end)) {
            return;
        }
        out.push((m.span, format!("Member {} not part of class {}", m.member.to_ascii_uppercase(), root.name.to_ascii_uppercase())));
    });
    out
}

/// The properties RapidQ's manual lists as read-only (R) for its own
/// components (Appendix A); assigning one is RapidQ's `Property X of Y is
/// read-only.`
pub fn is_read_only_property(type_name: &str, property: &str) -> bool {
    const TABLE: &[(&str, &[&str])] = &[
        ("QBUTTON", &["Handle"]),
        ("QCHECKBOX", &["Handle"]),
        ("QCOMBOBOX", &["Handle", "ItemCount"]),
        ("QCOMPORT", &["BytesNotRead", "BytesNotWritten", "Connected", "Handle"]),
        ("QEDIT", &["Handle", "IsMasked"]),
        ("QFILELISTBOX", &["ItemCount", "SelCount"]),
        ("QFILESTREAM", &["EOF", "Handle", "LineCount", "Size"]),
        ("QFORM", &["Handle"]),
        ("QGROUPBOX", &["Handle"]),
        ("QHEADER", &["SectionsCount"]),
        ("QIMAGELIST", &["Count"]),
        ("QLABEL", &["Handle"]),
        ("QLISTBOX", &["Handle", "ItemCount", "SelCount"]),
        ("QLISTVIEW", &["ColumnsCount", "Handle"]),
        ("QMAINMENU", &["Handle"]),
        ("QMEMORYSTREAM", &["LineCount"]),
        ("QMENUITEM", &["Count", "Handle"]),
        ("QOUTLINE", &["Handle"]),
        ("QPOPUPMENU", &["Handle"]),
        ("QRADIOBUTTON", &["Handle"]),
        ("QRICHEDIT", &["Handle", "WhereX", "WhereY"]),
        ("QSCROLLBAR", &["Handle"]),
        ("QSCROLLBOX", &["Handle"]),
        ("QSTATUSBAR", &["Handle"]),
        ("QSTRINGGRID", &["Handle", "VisibleColCount", "VisibleRowCount"]),
        ("QSTRINGLIST", &["ItemCount"]),
        ("QTRACKBAR", &["Handle"]),
    ];
    TABLE.iter().any(|(t, props)| {
        (t.eq_ignore_ascii_case(type_name) || canonical_type_name(t).eq_ignore_ascii_case(type_name))
            && props.iter().any(|p| p.eq_ignore_ascii_case(property))
    })
}

/// More of RapidQ's compile-time checks, in its compiler's words (run on
/// the program as written):
/// - a SUB/FUNCTION called with more or fewer arguments than it declares:
///   `Too many actual parameters for S` / `Too few parameters for S`;
/// - a name DIMmed twice in one scope: `Identifier a already used, try
///   another name` (`i%` and `i$` are two: RapidQ's examples DIM both);
/// - `RESULT = …` outside a FUNCTION: `Trying to assign return value while
///   not in FUNCTION`;
/// - a read-only property assigned (`List.ItemCount = 3`, or `Handle = …`
///   in its CREATE): `Property ItemCount of List is read-only.`
pub fn rapidq_checks(program: &Program) -> Vec<(TextSpan, String)> {
    use std::collections::{HashMap, HashSet};
    let key = |n: &str| strip_type_suffix(n).to_ascii_lowercase();
    // (a DECLARE SUB without LIB is the routine's signature too: RapidQ's
    // examples call `DECLARE SUB Check_Window ()` with none though the SUB
    // takes a Sender)
    let mut arity: HashMap<String, (String, usize, Vec<usize>)> = HashMap::new();
    for s in &program.statements {
        match s {
            Statement::Subroutine(r) => { arity.entry(key(&r.name)).or_insert((r.name.clone(), r.params.len(), Vec::new())).1 = r.params.len(); }
            Statement::Function(f) => { arity.entry(key(&f.name)).or_insert((f.name.clone(), f.params.len(), Vec::new())).1 = f.params.len(); }
            _ => {}
        }
    }
    for s in &program.statements {
        if let Statement::Declare(d) = s {
            if d.lib.is_none() {
                if let Some(a) = arity.get_mut(&key(&d.name)) {
                    a.2.push(d.params.len());
                }
            }
        }
    }
    let mut out = Vec::new();
    let count = |name_expr: &Expression, argc: usize, span: TextSpan, out: &mut Vec<(TextSpan, String)>| {
        let Expression::Identifier(i) = name_expr else { return };
        if let Some((name, n, declared)) = arity.get(&key(&i.name)) {
            if declared.contains(&argc) {
            } else if argc > *n {
                out.push((span, format!("Too many actual parameters for {name}")));
            } else if argc < *n {
                out.push((span, format!("Too few parameters for {name}")));
            }
        }
    };
    // (TYPE bodies call their own methods by bare name: not these routines)
    let outside_types: Vec<Statement> = program.statements.iter().filter(|s| !matches!(s, Statement::Type(_))).cloned().collect();
    walk(
        &outside_types,
        &mut |s| {
            if let Statement::Call(c) = s {
                count(&c.callee, c.args.len(), c.span, &mut out);
            }
        },
        &mut |_| {},
    );
    let mut calls: Vec<(Expression, usize, TextSpan)> = Vec::new();
    walk(
        &outside_types,
        &mut |_| {},
        &mut |e| {
            if let Expression::FunctionCall(c) = e {
                calls.push(((*c.callee).clone(), c.args.len(), c.span));
            }
        },
    );
    for (callee, argc, span) in calls {
        count(&callee, argc, span, &mut out);
    }
    // A FUNCTION with parameters naming itself without them, other than to
    // set its result (`F = F + x`): RapidQ reads the name as a call, so it
    // wants the arguments — `Expected ( but got "+"` (RC.EXE; RESULT reads
    // the result). Without parameters the name is a call of itself, which
    // both compilers make.
    for s in &outside_types {
        let Statement::Function(f) = s else { continue };
        if f.params.is_empty() {
            continue;
        }
        let mut not_reads: HashSet<*const Expression> = HashSet::new();
        let mut set_or_called: HashSet<*const Expression> = HashSet::new();
        let mut next_token: HashMap<*const Expression, &'static str> = HashMap::new();
        let mut reads: Vec<(*const Expression, TextSpan)> = Vec::new();
        walk(
            &f.body,
            &mut |st| match st {
                Statement::Assignment(a) => {
                    set_or_called.insert(&a.target as *const Expression);
                }
                Statement::Call(c) => {
                    set_or_called.insert(&c.callee as *const Expression);
                }
                _ => {}
            },
            &mut |e| match e {
                Expression::FunctionCall(c) => {
                    not_reads.insert(&*c.callee as *const Expression);
                }
                Expression::ArrayAccess(a) => {
                    not_reads.insert(&*a.array as *const Expression);
                }
                Expression::Binary(b) => {
                    use BinaryOperator as B;
                    let op = match b.operator {
                        B::Add => "+",
                        B::Subtract => "-",
                        B::Multiply => "*",
                        B::Divide => "/",
                        B::IntegerDivide => "\\",
                        B::Power => "^",
                        B::Concat => "&",
                        B::Equal => "=",
                        B::NotEqual => "<>",
                        B::LessThan => "<",
                        B::LessThanOrEqual => "<=",
                        B::GreaterThan => ">",
                        B::GreaterThanOrEqual => ">=",
                        B::Modulo => "MOD",
                        B::And => "AND",
                        B::Or => "OR",
                        B::Xor => "XOR",
                    };
                    next_token.insert(&*b.left as *const Expression, op);
                }
                Expression::Identifier(i) if names_routine(&i.name, &f.name) => reads.push((e as *const Expression, i.span)),
                _ => {}
            },
        );
        for (at, span) in reads.into_iter().filter(|(at, _)| !not_reads.contains(at) && !set_or_called.contains(at)) {
            let got = next_token.get(&at).map_or("end-of-line".to_string(), |t| format!("\"{t}\""));
            out.push((span, format!("Expected ( but got {got}")));
        }
    }
    fn dims(stmts: &[Statement], seen: &mut HashSet<String>, out: &mut Vec<(TextSpan, String)>) {
        for s in stmts {
            match s {
                Statement::Dim(d) if !d.is_redim && !d.is_static => {
                    for v in &d.declarators {
                        if !seen.insert(v.name.to_ascii_lowercase()) {
                            out.push((v.span, format!("Identifier {} already used, try another name", v.name)));
                        }
                    }
                }
                Statement::If(i) => {
                    dims(&i.then_body, seen, out);
                    for b in &i.elseif_branches {
                        dims(&b.body, seen, out);
                    }
                    dims(&i.else_body, seen, out);
                }
                Statement::For(f) => dims(&f.body, seen, out),
                Statement::While(w) => dims(&w.body, seen, out),
                Statement::DoLoop(d) => dims(&d.body, seen, out),
                Statement::SelectCase(c) => {
                    for case in &c.cases {
                        dims(&case.body, seen, out);
                    }
                    dims(&c.case_else, seen, out);
                }
                _ => {}
            }
        }
    }
    fn result_outside(stmts: &[Statement], out: &mut Vec<(TextSpan, String)>) {
        walk(
            stmts,
            &mut |s| {
                if let Statement::Assignment(a) = s {
                    if matches!(&a.target, Expression::Identifier(i) if i.name.eq_ignore_ascii_case("result")) {
                        out.push((a.span, "Trying to assign return value while not in FUNCTION".to_string()));
                    }
                }
            },
            &mut |_| {},
        );
    }
    // Objects RapidQ has no arrays of (its compiler's own message, its names).
    walk(
        &program.statements,
        &mut |s| {
            if let Statement::Dim(d) = s {
                // (the parser keeps RapidR's name of the type: QREGISTRY's RREGISTRY)
                const NAMES: [&str; 17] = [
                    "QMainMenu", "QFileStream", "QSOCKET", "QDIRTREE", "QDXSCREEN", "QOBJECT", "QCOMPORT", "QRECT", "QNOTIFYICONDATA", "QREGISTRY",
                    "QDXTIMER", "QDXIMAGELIST", "QDXJOYSTICK", "QD3DMESH", "QD3DTEXTURE", "QD3DWRAP", "QD3DVISUAL",
                ];
                let Some(name) = NAMES.iter().find(|n| canonical_type_name(n).eq_ignore_ascii_case(&d.type_name) || n.eq_ignore_ascii_case(&d.type_name)) else { return };
                for v in d.declarators.iter().filter(|v| !v.dimensions.is_empty()) {
                    out.push((v.span, format!("Array of {name} is not supported!")));
                }
            }
        },
        &mut |_| {},
    );
    // Read-only properties: the components the program names (CREATE, DIM
    // AS Q…) and the bare properties inside each CREATE.
    let mut component_types: HashMap<String, String> = HashMap::new();
    walk(
        &outside_types,
        &mut |s| match s {
            Statement::Create(c) => {
                component_types.insert(c.name.to_ascii_lowercase(), c.type_name.clone());
            }
            Statement::Dim(d) if is_rapidq_object_type(&d.type_name) || is_component_type_name(&d.type_name) => {
                for v in &d.declarators {
                    component_types.insert(v.name.to_ascii_lowercase(), d.type_name.clone());
                }
            }
            _ => {}
        },
        &mut |_| {},
    );
    walk(
        &outside_types,
        &mut |s| match s {
            Statement::Assignment(a) => {
                if let Expression::MemberAccess(m) = &a.target {
                    if let Expression::Identifier(o) = m.object.as_ref() {
                        let t = component_types.get(&o.name.to_ascii_lowercase());
                        // (RC.EXE's own message first: `J.ISLEFT is a read-only
                        // value.`; then the manual's read-only properties)
                        if t.is_some_and(|t| is_read_only_value(t, &m.member)) {
                            out.push((a.span, format!("{}.{} is a read-only value.", o.name.to_ascii_uppercase(), m.member.to_ascii_uppercase())));
                        } else if t.is_some_and(|t| is_read_only_property(t, &m.member)) {
                            out.push((a.span, format!("Property {} of {} is read-only.", m.member, o.name)));
                        }
                    }
                }
            }
            Statement::Create(c) => {
                for b in &c.body {
                    if let Statement::Assignment(a) = b {
                        if let Expression::Identifier(p) = &a.target {
                            if is_read_only_property(&c.type_name, &p.name) {
                                out.push((a.span, format!("Property {} of {} is read-only.", p.name, c.name)));
                            }
                        }
                    }
                }
            }
            _ => {}
        },
        &mut |_| {},
    );
    // (`L.Sorted = 1, 2`: the parser made it a call — `Obj.Method = a, b`
    // — but a property takes one value: RC.EXE's `Expected end-of-line but
    // got ,`)
    walk(
        &outside_types,
        &mut |s| {
            let Statement::Call(c) = s else { return };
            let Expression::MemberAccess(m) = &c.callee else { return };
            let Expression::Identifier(root) = m.object.as_ref() else { return };
            let lone = matches!(c.args.first(), Some(Expression::FunctionCall(f)) if matches!(f.callee.as_ref(), Expression::Identifier(i) if i.name == "__lone_equals"));
            if c.args.len() < 2 || !lone {
                return;
            }
            let Some(t) = component_types.get(&root.name.to_ascii_lowercase()) else { return };
            if rapidr_lang::component(&canonical_type_name(t)).is_some_and(|k| k.property(&m.member).is_some() && k.method(&m.member).is_none()) {
                out.push((c.span, "Expected end-of-line but got ,".to_string()));
            }
        },
        &mut |_| {},
    );
    // (`L.Clear = 1`: a method that takes nothing can't be "assigned")
    walk(
        &outside_types,
        &mut |s| {
            let Statement::Assignment(a) = s else { return };
            let Expression::MemberAccess(m) = &a.target else { return };
            let Expression::Identifier(root) = m.object.as_ref() else { return };
            let Some(t) = component_types.get(&root.name.to_ascii_lowercase()) else { return };
            if method_equals::method_params(program, t, &m.member) == Some(0) {
                out.push((a.span, "Expected end-of-line but got =".to_string()));
            }
        },
        &mut |_| {},
    );
    out.extend(unknown_type_checks(program, &outside_types));
    out.extend(fixed_member_checks(program, &outside_types, &component_types));
    out.extend(registry_member_checks(program, &outside_types, &component_types));
    let mut global_dims = HashSet::new();
    let main: Vec<Statement> = outside_types.iter().filter(|s| !matches!(s, Statement::Subroutine(_) | Statement::Function(_))).cloned().collect();
    dims(&main, &mut global_dims, &mut out);
    let result_declared = global_dims.contains("result");
    if !result_declared {
        let plain: Vec<Statement> = main.iter().filter(|s| !matches!(s, Statement::Create(_) | Statement::With(_))).cloned().collect();
        result_outside(&plain, &mut out);
    }
    for s in &program.statements {
        match s {
            Statement::Subroutine(r) => {
                let mut seen = HashSet::new();
                dims(&r.body, &mut seen, &mut out);
                if !result_declared && !seen.contains("result") {
                    result_outside(&r.body, &mut out);
                }
            }
            Statement::Function(f) => dims(&f.body, &mut HashSet::new(), &mut out),
            _ => {}
        }
    }
    out
}

/// RapidQ's `QUICKSORT(A(first), A(last), ASCEND|DESCEND)` (also without
/// parentheses): `__quicksort(A, descend, first indices…, last indices…)`,
/// which sorts those elements in place (rapidr_value::builtins). Both ends
/// must be elements of one array (RapidQ's "span sorting" across arrays
/// DIMmed side by side relies on its memory layout).
pub fn quicksort(program: &Program) -> Program {
    let mut program = program.clone();
    let element = |e: &Expression| -> Option<(String, Vec<Expression>)> {
        match e {
            Expression::FunctionCall(c) => match c.callee.as_ref() {
                Expression::Identifier(i) => Some((i.name.clone(), c.args.clone())),
                _ => None,
            },
            Expression::ArrayAccess(a) => match a.array.as_ref() {
                Expression::Identifier(i) => Some((i.name.clone(), a.indices.clone())),
                _ => None,
            },
            _ => None,
        }
    };
    walk_statements_mut(&mut program.statements, &mut |s| {
        let Statement::Call(c) = s else { return };
        if !matches!(&c.callee, Expression::Identifier(i) if i.name.eq_ignore_ascii_case("QUICKSORT")) || c.args.len() != 3 {
            return;
        }
        let (Some((a, first)), Some((b, last))) = (element(&c.args[0]), element(&c.args[1])) else { return };
        if !a.eq_ignore_ascii_case(&b) || first.len() != last.len() {
            return;
        }
        let span = c.span;
        let descend = match &c.args[2] {
            Expression::Identifier(i) if i.name.eq_ignore_ascii_case("ASCEND") => Expression::Literal(Literal { span, value: LiteralValue::Integer(0) }),
            Expression::Identifier(i) if i.name.eq_ignore_ascii_case("DESCEND") => Expression::Literal(Literal { span, value: LiteralValue::Integer(1) }),
            other => other.clone(),
        };
        let mut args = vec![Expression::Identifier(Identifier { span, name: a }), descend];
        args.extend(first);
        args.extend(last);
        c.callee = Expression::Identifier(Identifier { span, name: "__quicksort".to_string() });
        c.args = args;
    });
    program
}

/// RapidQ's `INITARRAY(A, v1, v2, …)`: `A(LBOUND(A)) = v1`,
/// `A(LBOUND(A) + 1) = v2`, … — the first elements get the values (both
/// backends; not when the program has its own routine of that name).
pub fn init_arrays(program: &Program) -> Program {
    let own = program.statements.iter().any(|s| match s {
        Statement::Subroutine(r) => r.name.eq_ignore_ascii_case("initarray"),
        Statement::Function(f) => f.name.eq_ignore_ascii_case("initarray"),
        _ => false,
    });
    let mut program = program.clone();
    if own {
        return program;
    }
    let mut lower = |block: &mut Vec<Statement>| {
        if !block.iter().any(|s| matches!(s, Statement::Call(c) if matches!(&c.callee, Expression::Identifier(id) if id.name.eq_ignore_ascii_case("initarray")))) {
            return;
        }
        let mut out = Vec::with_capacity(block.len());
        for s in block.drain(..) {
            match s {
                Statement::Call(c) if matches!(&c.callee, Expression::Identifier(id) if id.name.eq_ignore_ascii_case("initarray")) && c.args.len() >= 2 => {
                    let span = c.span;
                    let array = c.args[0].clone();
                    let lbound = Expression::FunctionCall(FunctionCallExpression {
                        span,
                        callee: Box::new(Expression::Identifier(Identifier { span, name: "LBOUND".into() })),
                        args: vec![array.clone()],
                    });
                    for (i, value) in c.args[1..].iter().enumerate() {
                        let index = Expression::Binary(BinaryExpression {
                            span,
                            left: Box::new(lbound.clone()),
                            operator: BinaryOperator::Add,
                            right: Box::new(Expression::Literal(Literal { span, value: LiteralValue::Integer(i as i64) })),
                        });
                        let target = Expression::FunctionCall(FunctionCallExpression { span, callee: Box::new(array.clone()), args: vec![index] });
                        out.push(Statement::Assignment(AssignmentStatement { span, target, value: value.clone() }));
                    }
                }
                other => out.push(other),
            }
        }
        *block = out;
    };
    for s in &mut program.statements {
        match s {
            Statement::Subroutine(r) => for_each_block_mut(&mut r.body, &mut lower),
            Statement::Function(f) => for_each_block_mut(&mut f.body, &mut lower),
            Statement::Type(t) => {
                for m in &mut t.methods {
                    match m {
                        Statement::Subroutine(r) => for_each_block_mut(&mut r.body, &mut lower),
                        Statement::Function(f) => for_each_block_mut(&mut f.body, &mut lower),
                        _ => {}
                    }
                }
                for_each_block_mut(&mut t.constructor, &mut lower);
            }
            _ => {}
        }
    }
    for_each_block_mut(&mut program.statements, &mut lower);
    program
}

/// `INPUT [prompt,] var` as the assignment
/// `var = __input_value(INPUT(prompt), var, suffix)`: `INPUT(prompt)` prints
/// the prompt and reads a line, and `__input_value` stores it as text or a
/// number for the variable (`rapidr_value::input_value`). The Rust backend
/// lowers INPUT this way; the VM does the same with its INPUT opcode.
pub fn input_assignment(i: &InputStatement) -> AssignmentStatement {
    let span = i.span;
    let call = |name: &str, args: Vec<Expression>| {
        Expression::FunctionCall(FunctionCallExpression {
            span,
            callee: Box::new(Expression::Identifier(Identifier { span, name: name.into() })),
            args,
        })
    };
    let text = |s: &str| Expression::Literal(Literal { span, value: LiteralValue::String(s.into()) });
    let prompt = i.prompt.clone().unwrap_or_else(|| text(""));
    AssignmentStatement {
        span,
        target: i.target.clone(),
        value: call("__input_value", vec![call("input", vec![prompt]), i.target.clone(), text(input_suffix(&i.target))]),
    }
}

/// Component types both backends can create (RapidR's upper-case names):
/// the language registry's (crates/rapidr-lang/data/components), the single
/// source for "is this DIM/CREATE type a GUI/system component?".
pub const COMPONENT_TYPES: &[&str] = rapidr_lang::COMPONENT_TYPES;

/// RapidQ's built-in objects (its manual's component list) that RapidR has
/// no component for yet. Fields and variables of these types are objects
/// (generic property bags at run time; unimplemented methods warn), so
/// programs using them compile instead of failing on the type name.
/// Methods RapidQ programs call without parentheses for their result —
/// `IF Form.ShowModal THEN`, `IF OpenDialog.Execute THEN` — so that in an
/// expression `Obj.Member` is a call, not a property read (both backends).
pub const VALUE_METHODS: &[&str] = &["showmodal", "execute", "leechfile"];

/// RapidR's own constants, for its extensions (RapidQ's are RAPIDQ.INC's,
/// which `rapidr_preprocessor` supplies): there without an include, and a
/// program's own variable or constant of the same name wins (both
/// backends). Anchors' akLeft … akBottom (`rapidr_value::layout::AK_LEFT`
/// …).
pub const RAPIDR_CONSTANTS: &[(&str, i64)] = &[("akleft", 1), ("aktop", 2), ("akright", 4), ("akbottom", 8)];

/// The value of RapidR constant `name` (any case), if it is one.
pub fn rapidr_constant(name: &str) -> Option<i64> {
    RAPIDR_CONSTANTS.iter().find(|(n, _)| n.eq_ignore_ascii_case(name)).map(|(_, v)| *v)
}

// (QDIRLISTVIEW, QDOCKFORM: RapidR's own libraries, rapidr_preprocessor::
// RAPIDR_LIBRARIES — TYPEs a program that names them gets)
pub const RAPIDQ_OBJECTS_NOT_YET_IMPLEMENTED: &[&str] = &["QOLECONTAINER", "QOLEOBJECT"];

/// The type suffix of an INPUT variable (`name$` → "$"), or "": with the
/// variable's current value it decides whether INPUT stores text or a number
/// (`rapidr_value::input_value`).
pub fn input_suffix(target: &Expression) -> &'static str {
    let name = match target {
        Expression::Identifier(id) => &id.name,
        Expression::ArrayAccess(a) => match a.array.as_ref() {
            Expression::Identifier(id) => &id.name,
            _ => return "",
        },
        Expression::FunctionCall(f) => match f.callee.as_ref() {
            Expression::Identifier(id) => &id.name,
            _ => return "",
        },
        _ => return "",
    };
    match name.chars().last() {
        Some('$') => "$",
        Some('%') => "%",
        Some('&') => "&",
        Some('!') => "!",
        Some('#') => "#",
        _ => "",
    }
}

/// `Stream.Read(var)` (QFILESTREAM / QMEMORYSTREAM, manual: "Generic Read,
/// determines the storage space and saves data in variable") as the
/// assignment `var = Stream.__read(var)`: the stream reads as many bytes as
/// the variable's current value takes. `is_stream` says whether the object
/// is a stream.
pub fn stream_read_assignment(c: &CallStatement, is_stream: &dyn Fn(&Expression) -> bool) -> Option<AssignmentStatement> {
    let Expression::MemberAccess(m) = &c.callee else { return None };
    let [target] = c.args.as_slice() else { return None };
    if !m.member.eq_ignore_ascii_case("read")
        || !matches!(target, Expression::Identifier(_) | Expression::ArrayAccess(_) | Expression::FunctionCall(_) | Expression::MemberAccess(_))
        || !is_stream(&m.object)
    {
        return None;
    }
    let callee = Expression::MemberAccess(MemberAccessExpression { span: m.span, object: m.object.clone(), member: "__read".into() });
    Some(AssignmentStatement {
        span: c.span,
        target: target.clone(),
        value: Expression::FunctionCall(FunctionCallExpression { span: c.span, callee: Box::new(callee), args: vec![target.clone()] }),
    })
}

/// Inside `CREATE SB AS QSTATUSBAR … END CREATE`, `Panel(0).Width = 100`
/// means `SB.Panel(0).Width = 100`: one of the component's indexed
/// sub-objects. Returns the body with such statements (assignments and calls
/// whose object is `Name(args)` for a `Name` that `is_known` doesn't claim
/// as a variable, array or routine) written with `obj` explicitly.
pub fn qualify_create_body(body: &[Statement], obj: &str, type_name: &str, is_known: &dyn Fn(&str) -> bool) -> Vec<Statement> {
    let own = component_indexed_members(type_name);
    let qualify = |e: &Expression| -> Option<Expression> {
        let Expression::MemberAccess(m) = e else { return None };
        let Expression::FunctionCall(fc) = m.object.as_ref() else { return None };
        let Expression::Identifier(sub) = fc.callee.as_ref() else { return None };
        // (the component's own `Panel(i)` even when the program has a
        // `Panel` array of its own: inside the CREATE it's the object's)
        if is_known(&sub.name) && !own.iter().any(|o| o.eq_ignore_ascii_case(&sub.name)) {
            return None;
        }
        let owner = Expression::Identifier(Identifier { span: sub.span, name: obj.to_string() });
        let callee = Expression::MemberAccess(MemberAccessExpression { span: sub.span, object: Box::new(owner), member: sub.name.clone() });
        let object = Expression::FunctionCall(FunctionCallExpression { span: fc.span, callee: Box::new(callee), args: fc.args.clone() });
        Some(Expression::MemberAccess(MemberAccessExpression { span: m.span, object: Box::new(object), member: m.member.clone() }))
    };
    body.iter()
        .map(|s| match s {
            Statement::Assignment(a) => match qualify(&a.target) {
                Some(target) => Statement::Assignment(AssignmentStatement { target, ..a.clone() }),
                None => s.clone(),
            },
            Statement::Call(c) => match qualify(&c.callee) {
                Some(callee) => Statement::Call(CallStatement { callee, ..c.clone() }),
                None => s.clone(),
            },
            _ => s.clone(),
        })
        .collect()
}

/// RapidQ reads a component's properties bare inside its CREATE — `Left =
/// (Screen.Width - Width) \ 2`, `PRINT ItemCount` — as the object's, when
/// the program has no variable, constant, routine or component of that name
/// (`Left` is LEFT$'s name too, but a bare `Left` can't be a call). Both
/// backends; assignment targets are the backends' own property sets.
pub fn create_property_reads(program: &Program) -> Program {
    use std::collections::HashSet;
    fn declared(stmts: &[Statement], into: &mut HashSet<String>) {
        walk(
            stmts,
            &mut |s| match s {
                Statement::Dim(d) => into.extend(d.declarators.iter().map(|v| strip_type_suffix(&v.name).to_ascii_lowercase())),
                Statement::Const(c) => { into.insert(strip_type_suffix(&c.name).to_ascii_lowercase()); }
                Statement::Create(c) => { into.insert(c.name.to_ascii_lowercase()); }
                Statement::Subroutine(r) => { into.insert(strip_type_suffix(&r.name).to_ascii_lowercase()); }
                Statement::Function(f) => { into.insert(strip_type_suffix(&f.name).to_ascii_lowercase()); }
                Statement::Declare(d) => { into.insert(strip_type_suffix(&d.name).to_ascii_lowercase()); }
                _ => {}
            },
            &mut |_| {},
        );
    }
    fn rewrite(stmts: &mut [Statement], known: &HashSet<String>) {
        for s in stmts.iter_mut() {
            match s {
                Statement::Subroutine(_) | Statement::Function(_) => {
                    let (name, params, body) = match s {
                        Statement::Subroutine(r) => (r.name.clone(), r.params.clone(), &mut r.body),
                        Statement::Function(f) => (f.name.clone(), f.params.clone(), &mut f.body),
                        _ => unreachable!(),
                    };
                    let mut locals = known.clone();
                    locals.extend(params.iter().map(|p| strip_type_suffix(&p.name).to_ascii_lowercase()));
                    locals.insert(strip_type_suffix(&name).to_ascii_lowercase());
                    declared(body, &mut locals);
                    rewrite(body, &locals);
                }
                Statement::Create(c) => {
                    let obj = c.name.clone();
                    let read = &mut |e: &mut Expression| {
                        if let Expression::Identifier(id) = e {
                            if !known.contains(&id.name.to_ascii_lowercase()) && is_readable_property(&id.name) {
                                let owner = Expression::Identifier(Identifier { span: id.span, name: obj.clone() });
                                *e = Expression::MemberAccess(MemberAccessExpression { span: id.span, object: Box::new(owner), member: id.name.clone() });
                            }
                        }
                    };
                    for b in c.body.iter_mut() {
                        match b {
                            Statement::Create(_) => {}
                            Statement::Assignment(a) => {
                                walk_expression_mut(&mut a.value, read);
                                if !matches!(a.target, Expression::Identifier(_)) {
                                    walk_expression_mut(&mut a.target, read);
                                }
                            }
                            Statement::Call(call) => {
                                for arg in &mut call.args {
                                    walk_expression_mut(arg, read);
                                }
                                // (`Font.AddStyles(fsBold)`: the object's Font)
                                if let Expression::MemberAccess(m) = &mut call.callee {
                                    walk_expression_mut(&mut m.object, read);
                                }
                            }
                            other => walk_expressions_mut(std::slice::from_mut(other), false, read),
                        }
                    }
                    for b in c.body.iter_mut() {
                        if matches!(b, Statement::Create(_)) {
                            rewrite(std::slice::from_mut(b), known);
                        }
                    }
                }
                _ => {
                    for body in child_bodies_mut(s) {
                        rewrite(body, known);
                    }
                }
            }
        }
    }
    let mut has_create = false;
    walk(&program.statements, &mut |x| has_create |= matches!(x, Statement::Create(_)), &mut |_| {});
    if !has_create {
        return program.clone();
    }
    let mut known = HashSet::new();
    declared(&program.statements, &mut known);
    let mut program = program.clone();
    rewrite(&mut program.statements, &known);
    program
}

/// Properties a RapidQ program reads bare inside a component's CREATE
/// (`Width`, `ItemCount`, …; see [`qualify_create_body`]).
fn is_readable_property(name: &str) -> bool {
    const PROPS: &[&str] = &[
        "left", "top", "width", "height", "caption", "text", "color", "visible", "enabled", "hint", "showhint", "tag",
        "cursor", "align", "font", "clientwidth", "clientheight", "tabstop", "taborder", "handle", "itemcount",
        "itemindex", "checked", "position", "max", "min", "rowcount", "colcount", "selcount", "linecount", "selstart",
        "sellength", "seltext", "interval", "borderstyle", "formstyle", "windowstate", "autosize", "alignment",
        "readonly", "maxlength", "passwordchar", "sorted", "multiselect", "columns", "row", "col", "fixedrows",
        "fixedcols", "defaultrowheight", "defaultcolwidth", "pageindex", "tabindex", "value", "transparent", "picture",
        "bmp", "icon", "wordwrap", "scrollbars", "layout", "flat", "down", "allowallup", "groupindex",
    ];
    PROPS.contains(&name.to_ascii_lowercase().as_str())
}

/// A component's indexed members (`Panel(i).Width` of a QSTATUSBAR,
/// `Column(i).Caption` of a QLISTVIEW, …): inside its CREATE they're the
/// object's, whatever else the program calls by those names.
pub fn component_indexed_members(type_name: &str) -> &'static [&'static str] {
    match canonical_type_name(type_name).to_ascii_uppercase().as_str() {
        "RSTATUSBAR" => &["panel", "panels"],
        "RLISTVIEW" => &["item", "column", "subitem", "selected"],
        "RTREEVIEW" => &["item"],
        "RHEADER" => &["sections", "section"],
        "RSTRINGGRID" => &["cell", "colwidths", "rowheights", "columnstyle", "columnlist"],
        "RLISTBOX" | "RCOMBOBOX" | "RFILELISTBOX" => &["item", "selected"],
        "RTABCONTROL" => &["tab", "tabs"],
        _ => &[],
    }
}

/// A component RapidR implements, or one of RapidQ's objects it doesn't yet.
pub fn is_rapidq_object_type(type_name: &str) -> bool {
    is_component_type_name(&canonical_type_name(type_name))
        || RAPIDQ_OBJECTS_NOT_YET_IMPLEMENTED.contains(&type_name.to_ascii_uppercase().as_str())
}

/// A timer the runtimes tick while the program waits (QTIMER, QDXTIMER):
/// both backends register it when it's made.
pub fn is_timer_type(type_name: &str) -> bool {
    // (QDXJOYSTICK: its events looked for at each tick)
    matches!(canonical_type_name(type_name).to_ascii_uppercase().as_str(), "RTIMER" | "RDXTIMER" | "RDXJOYSTICK" | "RCOMPORT" | "RMIDI" | "RWAVE" | "RVIDEO" | "RCDAUDIO")
}

pub fn is_component_type_name(type_name: &str) -> bool {
    COMPONENT_TYPES.contains(&type_name.to_ascii_uppercase().as_str())
}

/// RapidQ's components that its include libraries define as TYPEs —
/// QBevel.inc's QBEVEL, QDigDisplay.inc's QDIGDISPLAY — with RapidR's
/// built-in for each. A program that includes the library (so defines the
/// TYPE) gets its own; one that doesn't (the manual's own examples don't)
/// gets the built-in. `canonical_type_name` leaves these names alone: the
/// parser decides (`component_type_reference`).
pub const INCLUDE_LIBRARY_COMPONENTS: &[(&str, &str)] = &[("QBEVEL", "RBEVEL"), ("QDIGDISPLAY", "RDIGDISPLAY")];

/// Whether `name` is one of [`INCLUDE_LIBRARY_COMPONENTS`]' RapidQ names.
pub fn is_include_library_component(name: &str) -> bool {
    INCLUDE_LIBRARY_COMPONENTS.iter().any(|(q, _)| q.eq_ignore_ascii_case(name))
}

/// A type named in a program (`AS QBEVEL`, `EXTENDS QPANEL`): RapidR's
/// name for a RapidQ component (`canonical_type_name`), an include
/// library's component the built-in unless the program defines that TYPE
/// itself (`own_types`, upper case).
pub fn component_type_reference(name: &str, own_types: &[String]) -> String {
    let upper = name.to_ascii_uppercase();
    match INCLUDE_LIBRARY_COMPONENTS.iter().find(|(q, _)| *q == upper) {
        Some(_) if own_types.contains(&upper) => name.to_string(),
        Some((_, r)) => (*r).to_string(),
        None => canonical_type_name(name),
    }
}

thread_local! {
    /// The TYPEs of the program being compiled whose names RapidR would
    /// otherwise take for one of its components (upper case):
    /// [`set_program_types`].
    static PROGRAM_TYPES: std::cell::RefCell<Vec<String>> = const { std::cell::RefCell::new(Vec::new()) };
}

/// The program being parsed defines these TYPEs (the parser, before it
/// parses a program): a name among them that RapidR's Q-to-R rule would
/// make a RapidR component (`TYPE QToolBar EXTENDS QPanel`, RapidQ's
/// QToolbar example — RapidQ has no QTOOLBAR) stays the program's TYPE.
/// RapidQ's own component names can't be TYPE names in RapidQ, and the
/// include libraries' (QBEVEL, QDIGDISPLAY) are decided by the parser.
pub fn set_program_types(names: &[String]) {
    let mine: Vec<String> = names
        .iter()
        .map(|n| n.to_ascii_uppercase())
        .filter(|n| !is_include_library_component(n) && n.strip_prefix('Q').is_some_and(|rest| rapidr_lang::component(&format!("R{rest}")).is_some_and(|c| c.rapidq.is_none())))
        .collect();
    PROGRAM_TYPES.with(|t| *t.borrow_mut() = mine);
}

fn is_program_type(upper: &str) -> bool {
    PROGRAM_TYPES.with(|t| t.borrow().iter().any(|n| n == upper))
}

/// RapidQ names its components QForm, QButton, …; RapidR's are RForm,
/// RButton, …. Maps a RapidQ component name to RapidR's, and RapidR's in any
/// case to the same (upper case: `QPANEL`, `RPanel` → `RPANEL`), and leaves
/// every other type name unchanged.
pub fn canonical_type_name(type_name: &str) -> String {
    let upper = type_name.to_ascii_uppercase();
    // (RAPIDQ2.INC's `$DEFINE QCOMPORT COMPORT`: rapidr_ast::library)
    if upper == "COMPORT" {
        return "RCOMPORT".into();
    }
    // (an include library's component: the parser decided already)
    if is_include_library_component(&upper) {
        return type_name.to_string();
    }
    // (the program's own TYPE QTOOLBAR — RapidQ has none, RapidR's prefix
    // rule would make it RTOOLBAR: the program's TYPE, as in RapidQ)
    if is_program_type(&upper) {
        return type_name.to_string();
    }
    if let Some(rest) = upper.strip_prefix('Q') {
        // RapidQ components whose RapidR counterpart has another name.
        if rest == "GAUGE" {
            return "RPROGRESSBAR".into();
        }
        // QOUTLINE (Windows 3.1's tree): a tree view with its own methods
        // (rapidr_value::objects::tree).
        if rest == "OUTLINE" {
            return "RTREEVIEW".into();
        }
        let r_name = format!("R{rest}");
        if is_component_type_name(&r_name) {
            return r_name;
        }
    }
    // (RapidR's own name in any case — `RPanel`, `rpanel` — is the same
    // component as RapidQ's QPANEL, spelled as the compilers spell it)
    if is_component_type_name(&upper) {
        return upper;
    }
    type_name.to_string()
}

// ---------------------------------------------------------------------------
// Walking the tree
// ---------------------------------------------------------------------------

/// Every statement list inside `stmts` (itself included), innermost first.
pub(crate) fn for_each_block_mut(stmts: &mut Vec<Statement>, f: &mut dyn FnMut(&mut Vec<Statement>)) {
    for s in stmts.iter_mut() {
        match s {
            Statement::If(i) => {
                for_each_block_mut(&mut i.then_body, f);
                for b in &mut i.elseif_branches {
                    for_each_block_mut(&mut b.body, f);
                }
                for_each_block_mut(&mut i.else_body, f);
            }
            Statement::For(x) => for_each_block_mut(&mut x.body, f),
            Statement::While(x) => for_each_block_mut(&mut x.body, f),
            Statement::DoLoop(x) => for_each_block_mut(&mut x.body, f),
            Statement::With(x) => for_each_block_mut(&mut x.body, f),
            Statement::SelectCase(c) => {
                for b in &mut c.cases {
                    for_each_block_mut(&mut b.body, f);
                }
                for_each_block_mut(&mut c.case_else, f);
            }
            _ => {}
        }
    }
    f(stmts);
}

/// A FUNCTION RapidQ lets a program name with a dot (`FUNCTION
/// Calc.Twice`) returns what its body assigns to that name (`Calc.Twice =
/// N * 2`, as to any function's name): the assignment's target is the
/// function's own name, not a member `Twice` of an object `Calc`.
pub fn dotted_function_results(program: &Program) -> Program {
    let mut out = program.clone();
    walk_statements_mut(&mut out.statements, &mut |s| {
        let Statement::Function(f) = s else { return };
        if !f.name.contains('.') {
            return;
        }
        let name = f.name.clone();
        walk_statements_mut(&mut f.body, &mut |s| {
            let Statement::Assignment(a) = s else { return };
            let Expression::MemberAccess(m) = &a.target else { return };
            let Expression::Identifier(o) = m.object.as_ref() else { return };
            if format!("{}.{}", o.name, m.member).eq_ignore_ascii_case(&name) {
                a.target = Expression::Identifier(Identifier { span: m.span, name: name.clone() });
            }
        });
    });
    out
}

/// `DIM x AS QRegistry` (a RapidQ object RapidR has no component for) inside
/// a SUB, FUNCTION or TYPE method: `x` refers to an object of its own name,
/// as it does in the main program, in both backends — so its properties
/// are kept and its methods warn that they aren't implemented, instead of
/// the interpreter stopping ("not an object") while native builds go on.
pub fn routine_objects(program: &Program) -> Program {
    let user_types: std::collections::HashSet<String> = program
        .statements
        .iter()
        .filter_map(|s| match s {
            Statement::Type(t) => Some(t.name.to_ascii_uppercase()),
            _ => None,
        })
        .collect();
    let object_type = |t: &str| {
        let upper = t.trim().to_ascii_uppercase();
        !user_types.contains(&upper)
            && !is_component_type_name(&canonical_type_name(&upper))
            && (RAPIDQ_OBJECTS_NOT_YET_IMPLEMENTED.contains(&upper.as_str()) || (upper.starts_with('Q') && upper.len() > 1))
    };
    let mut add_objects = |body: &mut Vec<Statement>| {
        let mut out = Vec::with_capacity(body.len());
        for s in body.drain(..) {
            let extra: Vec<Statement> = match &s {
                Statement::Dim(d) if !d.is_redim && object_type(&d.type_name) => d
                    .declarators
                    .iter()
                    .filter(|v| v.dimensions.is_empty())
                    .flat_map(|v| {
                        let span = d.span;
                        let text = |x: &str| Expression::Literal(Literal { span, value: LiteralValue::String(x.into()) });
                        [
                            Statement::Call(CallStatement {
                                span,
                                callee: Expression::Identifier(Identifier { span, name: "__objcreate".into() }),
                                args: vec![text(&v.name), text(&d.type_name.trim().to_ascii_uppercase())],
                            }),
                            Statement::Assignment(AssignmentStatement {
                                span,
                                target: Expression::Identifier(Identifier { span, name: v.name.clone() }),
                                value: text(&v.name),
                            }),
                        ]
                    })
                    .collect(),
                _ => Vec::new(),
            };
            out.push(s);
            out.extend(extra);
        }
        *body = out;
    };
    let mut program = program.clone();
    for s in &mut program.statements {
        match s {
            Statement::Subroutine(sub) => for_each_block_mut(&mut sub.body, &mut add_objects),
            Statement::Function(f) => for_each_block_mut(&mut f.body, &mut add_objects),
            Statement::Type(t) => {
                for m in &mut t.methods {
                    match m {
                        Statement::Subroutine(sub) => for_each_block_mut(&mut sub.body, &mut add_objects),
                        Statement::Function(f) => for_each_block_mut(&mut f.body, &mut add_objects),
                        _ => {}
                    }
                }
            }
            _ => {}
        }
    }
    program
}

/// SUBs and FUNCTIONs written inside another SUB/FUNCTION (RapidQ accepts
/// them, e.g. event handlers next to the code that binds them) moved to the
/// top level, after their enclosing routine: every routine is global, so
/// both backends see them the same way.
pub fn hoist_routines(program: &Program) -> Program {
    fn take_nested(body: &mut Vec<Statement>, out: &mut Vec<Statement>) {
        let mut kept = Vec::with_capacity(body.len());
        for s in body.drain(..) {
            match s {
                Statement::Subroutine(_) | Statement::Function(_) => hoist(s, out),
                other => kept.push(other),
            }
        }
        *body = kept;
    }
    fn hoist(mut s: Statement, out: &mut Vec<Statement>) {
        let mut nested = Vec::new();
        match &mut s {
            Statement::Subroutine(sub) => take_nested(&mut sub.body, &mut nested),
            Statement::Function(f) => take_nested(&mut f.body, &mut nested),
            _ => {}
        }
        out.push(s);
        out.extend(nested);
    }
    let mut program = program.clone();
    let mut out = Vec::with_capacity(program.statements.len());
    for s in program.statements.drain(..) {
        hoist(s, &mut out);
    }
    program.statements = out;
    program
}

/// Calls `on_stmt` for every statement and `on_expr` for every expression in
/// `stmts`, depth first, including nested blocks, SUB/FUNCTION bodies and
/// TYPE methods/events/constructors.
pub fn walk(stmts: &[Statement], on_stmt: &mut dyn FnMut(&Statement), on_expr: &mut dyn FnMut(&Expression)) {
    for stmt in stmts {
        walk_statement(stmt, on_stmt, on_expr);
    }
}

fn walk_statement(stmt: &Statement, on_stmt: &mut dyn FnMut(&Statement), on_expr: &mut dyn FnMut(&Expression)) {
    on_stmt(stmt);
    let mut exprs: Vec<&Expression> = Vec::new();
    let mut bodies: Vec<&[Statement]> = Vec::new();
    match stmt {
        Statement::Assignment(a) => exprs.extend([&a.target, &a.value]),
        Statement::Bind(b) => exprs.extend([&b.target, &b.handler]),
        Statement::Call(c) => {
            exprs.push(&c.callee);
            exprs.extend(&c.args);
        }
        Statement::Close(c) => exprs.push(&c.file_number),
        Statement::Const(c) => exprs.push(&c.value),
        Statement::Create(c) => bodies.push(&c.body),
        Statement::Dim(d) => {
            for decl in &d.declarators {
                for dim in &decl.dimensions {
                    match dim {
                        ArrayDimension::Single(e) => exprs.push(e),
                        ArrayDimension::Range { start, end } => exprs.extend([start, end]),
                    }
                }
            }
        }
        Statement::DoLoop(d) => {
            exprs.extend(&d.condition);
            bodies.push(&d.body);
        }
        Statement::For(f) => {
            exprs.extend([&f.start, &f.end]);
            exprs.extend(&f.step);
            bodies.push(&f.body);
        }
        Statement::Function(f) => bodies.push(&f.body),
        Statement::Subroutine(s) => bodies.push(&s.body),
        Statement::If(i) => {
            exprs.push(&i.condition);
            bodies.push(&i.then_body);
            for b in &i.elseif_branches {
                exprs.push(&b.condition);
                bodies.push(&b.body);
            }
            bodies.push(&i.else_body);
        }
        Statement::Input(i) => {
            exprs.extend(&i.prompt);
            exprs.push(&i.target);
        }
        Statement::Open(o) => exprs.extend([&o.filename, &o.file_number]),
        Statement::Print(p) => exprs.extend(&p.items),
        Statement::PrintHash(p) => {
            exprs.push(&p.file_number);
            exprs.extend(&p.items);
        }
        Statement::WriteHash(w) => {
            exprs.push(&w.file_number);
            exprs.extend(&w.items);
        }
        Statement::Return(r) => exprs.extend(&r.value),
        Statement::Seek(s) => exprs.extend([&s.file_number, &s.position]),
        Statement::SelectCase(s) => {
            exprs.push(&s.expression);
            for c in &s.cases {
                for v in &c.values {
                    match v {
                        CaseValue::Value(e) | CaseValue::Is(_, e) => exprs.push(e),
                        CaseValue::Range(a, b) => exprs.extend([a, b]),
                        CaseValue::IsLogic(_, e, rest) => {
                            exprs.push(e);
                            exprs.extend(rest.iter().map(|(_, x)| x));
                        }
                    }
                }
                bodies.push(&c.body);
            }
            bodies.push(&s.case_else);
        }
        Statement::Type(t) => {
            bodies.push(&t.methods);
            bodies.push(&t.constructor);
            for e in &t.events {
                bodies.push(&e.body);
            }
            for f in &t.fields {
                exprs.extend(&f.array_size);
                exprs.extend(&f.array_lower);
            }
        }
        Statement::While(w) => {
            exprs.push(&w.condition);
            bodies.push(&w.body);
        }
        Statement::With(w) => {
            exprs.push(&w.object);
            bodies.push(&w.body);
        }
        Statement::Comment(_)
        | Statement::Declare(_)
        | Statement::Directive(_)
        | Statement::Exit(_)
        | Statement::Gosub(_)
        | Statement::Goto(_)
        | Statement::Import(_)
        | Statement::Label(_)
        | Statement::Line(_)
        | Statement::RustBlock(_) => {}
    }
    for e in exprs {
        walk_expression(e, on_expr);
    }
    for body in bodies {
        walk(body, on_stmt, on_expr);
    }
}

/// Calls `on_expr` for `expr` and every sub-expression, parents first.
pub fn walk_expression(expr: &Expression, on_expr: &mut dyn FnMut(&Expression)) {
    on_expr(expr);
    match expr {
        Expression::ArrayAccess(a) => {
            walk_expression(&a.array, on_expr);
            for i in &a.indices {
                walk_expression(i, on_expr);
            }
        }
        Expression::Binary(b) => {
            walk_expression(&b.left, on_expr);
            walk_expression(&b.right, on_expr);
        }
        Expression::FunctionCall(f) => {
            walk_expression(&f.callee, on_expr);
            for a in &f.args {
                walk_expression(a, on_expr);
            }
        }
        Expression::MemberAccess(m) => walk_expression(&m.object, on_expr),
        Expression::MethodCall(m) => {
            walk_expression(&m.object, on_expr);
            for a in &m.args {
                walk_expression(a, on_expr);
            }
        }
        Expression::Unary(u) => walk_expression(&u.operand, on_expr),
        Expression::Identifier(_) | Expression::Literal(_) => {}
    }
}

/// For each routine that some call site passes `@x` to, the argument
/// positions passed that way (lowercase routine name → positions).
pub fn ref_argument_positions(stmts: &[Statement]) -> std::collections::HashMap<String, Vec<usize>> {
    let mut out: std::collections::HashMap<String, Vec<usize>> = std::collections::HashMap::new();
    let mut note = |callee: &Expression, args: &[Expression]| {
        let Expression::Identifier(id) = callee else { return };
        for (i, arg) in args.iter().enumerate() {
            if matches!(arg, Expression::Unary(u) if u.operator == UnaryOperator::Ref) {
                let entry = out.entry(id.name.to_ascii_lowercase()).or_default();
                if !entry.contains(&i) {
                    entry.push(i);
                }
            }
        }
    };
    let mut stmt_calls: Vec<(Expression, Vec<Expression>)> = Vec::new();
    let mut expr_calls: Vec<(Expression, Vec<Expression>)> = Vec::new();
    walk(
        stmts,
        &mut |s| {
            if let Statement::Call(c) = s {
                stmt_calls.push((c.callee.clone(), c.args.clone()));
            }
        },
        &mut |e| {
            if let Expression::FunctionCall(f) = e {
                expr_calls.push(((*f.callee).clone(), f.args.clone()));
            }
        },
    );
    for (callee, args) in stmt_calls.iter().chain(&expr_calls) {
        note(callee, args);
    }
    out
}

/// Calls `on_expr` on every expression in `stmts` (children before parents),
/// allowing it to replace them. Nested WITH blocks are visited only when
/// `into_with_bodies` is true (their target expression always is).
pub fn walk_expressions_mut(stmts: &mut [Statement], into_with_bodies: bool, on_expr: &mut dyn FnMut(&mut Expression)) {
    for stmt in stmts {
        walk_statement_mut(stmt, into_with_bodies, on_expr);
    }
}

fn walk_statement_mut(stmt: &mut Statement, into_with: bool, f: &mut dyn FnMut(&mut Expression)) {
    let (exprs, bodies) = statement_parts_mut(stmt, into_with);
    for e in exprs {
        walk_expression_mut(e, f);
    }
    for body in bodies {
        walk_expressions_mut(body, into_with, f);
    }
}

/// A statement's own expressions and its nested blocks (a WITH's body only
/// when `into_with`).
pub(crate) fn statement_parts_mut(stmt: &mut Statement, into_with: bool) -> (Vec<&mut Expression>, Vec<&mut Vec<Statement>>) {
    let mut exprs: Vec<&mut Expression> = Vec::new();
    let mut bodies: Vec<&mut Vec<Statement>> = Vec::new();
    match stmt {
        Statement::Assignment(a) => exprs.extend([&mut a.target, &mut a.value]),
        Statement::Bind(b) => exprs.extend([&mut b.target, &mut b.handler]),
        Statement::Call(c) => {
            exprs.push(&mut c.callee);
            exprs.extend(c.args.iter_mut());
        }
        Statement::Close(c) => exprs.push(&mut c.file_number),
        Statement::Const(c) => exprs.push(&mut c.value),
        Statement::Create(c) => bodies.push(&mut c.body),
        Statement::Dim(d) => {
            for decl in &mut d.declarators {
                for dim in &mut decl.dimensions {
                    match dim {
                        ArrayDimension::Single(e) => exprs.push(e),
                        ArrayDimension::Range { start, end } => exprs.extend([start, end]),
                    }
                }
            }
        }
        Statement::DoLoop(d) => {
            exprs.extend(d.condition.as_mut());
            bodies.push(&mut d.body);
        }
        Statement::For(fs) => {
            exprs.extend([&mut fs.start, &mut fs.end]);
            exprs.extend(fs.step.as_mut());
            bodies.push(&mut fs.body);
        }
        Statement::Function(fs) => bodies.push(&mut fs.body),
        Statement::Subroutine(s) => bodies.push(&mut s.body),
        Statement::If(i) => {
            exprs.push(&mut i.condition);
            bodies.push(&mut i.then_body);
            for b in &mut i.elseif_branches {
                exprs.push(&mut b.condition);
                bodies.push(&mut b.body);
            }
            bodies.push(&mut i.else_body);
        }
        Statement::Input(i) => {
            exprs.extend(i.prompt.as_mut());
            exprs.push(&mut i.target);
        }
        Statement::Open(o) => exprs.extend([&mut o.filename, &mut o.file_number]),
        Statement::Print(p) => exprs.extend(p.items.iter_mut()),
        Statement::PrintHash(p) => {
            exprs.push(&mut p.file_number);
            exprs.extend(p.items.iter_mut());
        }
        Statement::WriteHash(w) => {
            exprs.push(&mut w.file_number);
            exprs.extend(w.items.iter_mut());
        }
        Statement::Return(r) => exprs.extend(r.value.as_mut()),
        Statement::Seek(s) => exprs.extend([&mut s.file_number, &mut s.position]),
        Statement::SelectCase(s) => {
            exprs.push(&mut s.expression);
            for c in &mut s.cases {
                for v in &mut c.values {
                    match v {
                        CaseValue::Value(e) | CaseValue::Is(_, e) => exprs.push(e),
                        CaseValue::Range(a, b) => exprs.extend([a, b]),
                        CaseValue::IsLogic(_, e, rest) => {
                            exprs.push(e);
                            exprs.extend(rest.iter_mut().map(|(_, x)| x));
                        }
                    }
                }
                bodies.push(&mut c.body);
            }
            bodies.push(&mut s.case_else);
        }
        Statement::Type(t) => {
            bodies.push(&mut t.methods);
            bodies.push(&mut t.constructor);
            for e in &mut t.events {
                bodies.push(&mut e.body);
            }
        }
        Statement::While(w) => {
            exprs.push(&mut w.condition);
            bodies.push(&mut w.body);
        }
        Statement::With(w) => {
            exprs.push(&mut w.object);
            if into_with {
                bodies.push(&mut w.body);
            }
        }
        _ => {}
    }
    (exprs, bodies)
}

pub(crate) fn walk_expression_mut(expr: &mut Expression, f: &mut dyn FnMut(&mut Expression)) {
    match expr {
        Expression::ArrayAccess(a) => {
            walk_expression_mut(&mut a.array, f);
            for i in &mut a.indices {
                walk_expression_mut(i, f);
            }
        }
        Expression::Binary(b) => {
            walk_expression_mut(&mut b.left, f);
            walk_expression_mut(&mut b.right, f);
        }
        Expression::FunctionCall(c) => {
            walk_expression_mut(&mut c.callee, f);
            for a in &mut c.args {
                walk_expression_mut(a, f);
            }
        }
        Expression::MemberAccess(m) => walk_expression_mut(&mut m.object, f),
        Expression::MethodCall(m) => {
            walk_expression_mut(&mut m.object, f);
            for a in &mut m.args {
                walk_expression_mut(a, f);
            }
        }
        Expression::Unary(u) => walk_expression_mut(&mut u.operand, f),
        Expression::Identifier(_) | Expression::Literal(_) => {}
    }
    f(expr);
}

/// The body of `WITH obj … END WITH` with every `.Member` (parsed as a
/// member of the `_with_` placeholder) pointing at `obj` instead. Nested
/// WITH blocks keep their own placeholder (but their target, e.g.
/// `WITH .Font`, is resolved against `obj`).
pub fn resolve_with_body(body: &[Statement], obj: &Expression) -> Vec<Statement> {
    let mut body = body.to_vec();
    walk_expressions_mut(&mut body, false, &mut |e| {
        if matches!(e, Expression::Identifier(id) if id.name == "_with_") {
            *e = obj.clone();
        }
    });
    body
}

/// Calls `f` on every statement in `stmts` and in every nested block
/// (parents first), allowing it to change them.
pub fn walk_statements_mut(stmts: &mut [Statement], f: &mut dyn FnMut(&mut Statement)) {
    for stmt in stmts {
        f(stmt);
        for body in child_bodies_mut(stmt) {
            walk_statements_mut(body, f);
        }
    }
}

pub(crate) fn child_bodies_mut(stmt: &mut Statement) -> Vec<&mut Vec<Statement>> {
    match stmt {
        Statement::Create(c) => vec![&mut c.body],
        Statement::DoLoop(d) => vec![&mut d.body],
        Statement::For(f) => vec![&mut f.body],
        Statement::Function(f) => vec![&mut f.body],
        Statement::Subroutine(s) => vec![&mut s.body],
        Statement::If(i) => {
            let mut v = vec![&mut i.then_body];
            v.extend(i.elseif_branches.iter_mut().map(|b| &mut b.body));
            v.push(&mut i.else_body);
            v
        }
        Statement::SelectCase(s) => {
            let mut v: Vec<&mut Vec<Statement>> = s.cases.iter_mut().map(|c| &mut c.body).collect();
            v.push(&mut s.case_else);
            v
        }
        Statement::Type(t) => {
            let mut v = vec![&mut t.methods, &mut t.constructor];
            v.extend(t.events.iter_mut().map(|e| &mut e.body));
            v
        }
        Statement::While(w) => vec![&mut w.body],
        Statement::With(w) => vec![&mut w.body],
        _ => Vec::new(),
    }
}

/// The items of a `DATA` line as written (RapidQ manual, DATA): split at
/// commas outside quotes; quoted text is a string (quotes removed), a number
/// is numeric, anything else is its trimmed text
/// (`DATA my dog ate my homework, "oh, boy!", 34.4`).
pub fn data_items(raw: &str) -> Vec<LiteralValue> {
    let mut parts = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    for c in raw.chars() {
        match c {
            '"' => {
                in_quotes = !in_quotes;
                current.push(c);
            }
            ',' if !in_quotes => parts.push(std::mem::take(&mut current)),
            _ => current.push(c),
        }
    }
    parts.push(current);
    if parts.len() == 1 && parts[0].trim().is_empty() {
        return Vec::new();
    }
    parts
        .iter()
        .map(|part| {
            let t = part.trim();
            if let Some(inner) = t.strip_prefix('"') {
                LiteralValue::String(inner.strip_suffix('"').unwrap_or(inner).to_string())
            } else if let Ok(n) = t.parse::<i64>() {
                LiteralValue::Integer(n)
            } else if let (Ok(f), true) = (t.parse::<f64>(), t.starts_with(|c: char| c.is_ascii_digit() || matches!(c, '-' | '+' | '.'))) {
                LiteralValue::Float(f)
            } else {
                LiteralValue::String(t.to_string())
            }
        })
        .collect()
}

/// Name of the identifier standing for a left-out argument (`COLOR , 1`,
/// `INSTR(, a, b)`); code generators pass Null for it.
pub const OMITTED_ARGUMENT: &str = "__omitted";
