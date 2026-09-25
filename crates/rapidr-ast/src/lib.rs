use rapidr_diagnostics::TextSpan;

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
    If(IfStatement),
    Import(ImportStatement),
    Input(InputStatement),
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
    /// `zones[i]`: item i is followed by `,` (move to the next 14-column
    /// print zone). Items followed by `;` or nothing are printed directly.
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
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeStatement {
    pub span: TextSpan,
    pub name: String,
    pub extends: Option<String>,
    pub fields: Vec<TypeField>,
    pub methods: Vec<Statement>,
    pub constructor: Vec<Statement>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeField {
    pub span: TextSpan,
    pub name: String,
    pub type_name: String,
    pub array_size: Option<Expression>,
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
