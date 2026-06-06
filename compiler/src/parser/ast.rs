// AST fields are intentionally defined ahead of use —
// they will be consumed by the type checker and code generator in later phases.
#![allow(dead_code)]

pub use super::ast_ops::{BinOp, UnOp};
use crate::lexer::token::Span;

// ── Program ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub struct Program {
    pub items: Vec<Item>,
}

// ── Top-level items ───────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Item {
    Import(Import),
    Page(Page),
    Component(Component),
    Layout(Layout),
    Store(Store),
    TypeDef(TypeDef),
    EnumDef(EnumDef),
    ThemeDef(ThemeDef),
}

#[derive(Debug, Clone)]
pub struct Import {
    pub names: ImportNames,
    pub path: String,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub enum ImportNames {
    Default(String),    // import Foo from "..."
    Named(Vec<String>), // import { Foo, Bar } from "..."
}

#[derive(Debug, Clone)]
pub struct Page {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Component {
    pub name: String,
    pub params: Vec<Param>,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Layout {
    pub name: String,
    pub body: Vec<Stmt>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct Store {
    pub name: String,
    pub state: Option<StateBlock>,
    pub derived: Option<DerivedBlock>,
    pub fns: Vec<FnDef>,
    /// `api.headers { Key: value_expr }` — applied to every API call automatically.
    pub api_headers: Vec<(String, Expr)>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct TypeDef {
    pub name: String,
    pub fields: Vec<Field>,
    pub span: Span,
}

#[derive(Debug, Clone)]
pub struct EnumDef {
    pub name: String,
    pub variants: Vec<Variant>,
    pub span: Span,
}

// ── Types ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Type {
    Text,
    Number,
    Bool,
    List(Box<Type>),
    Map(Box<Type>, Box<Type>),
    Optional(Box<Type>),
    Result(Box<Type>),
    Fn(Vec<Type>, Box<Type>),
    Named(String),
    None,
}

#[derive(Debug, Clone)]
pub struct Param {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub ty: Type,
}

#[derive(Debug, Clone)]
pub struct Variant {
    pub name: String,
    pub fields: Vec<String>, // bound names: Error(msg) -> ["msg"]
}

// ── Statements ────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Stmt {
    State(StateBlock),
    Derived(DerivedBlock),
    Guard(GuardStmt),
    On(OnStmt),
    Fn(FnDef),
    Let(LetStmt),
    If(IfStmt),
    Match(MatchStmt),
    /// `item -> body` inside a list() block: param name + body stmts.
    ForEach(String, Vec<Stmt>),
    Expr(Expr),
}

#[derive(Debug, Clone)]
pub struct StateBlock {
    pub entries: Vec<StateEntry>,
}

#[derive(Debug, Clone)]
pub struct StateEntry {
    pub name: String,
    pub ty: Option<Type>,
    pub value: Expr,
    pub persist: bool,
}

#[derive(Debug, Clone)]
pub struct DerivedBlock {
    pub entries: Vec<(String, Expr)>,
}

#[derive(Debug, Clone)]
pub struct GuardStmt {
    pub condition: Expr,
    pub action: Expr,
}

#[derive(Debug, Clone)]
pub struct OnStmt {
    pub event: OnEvent,
    pub param: Option<String>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub enum OnEvent {
    Change(String),
    Mount,
    Unmount,
    Key(String),
    Responsive(Breakpoint),
}

#[derive(Debug, Clone)]
pub enum Breakpoint {
    Phone,   // < 480px
    Mobile,  // < 768px
    Tablet,  // < 1024px
    Desktop, // >= 1024px
    Wide,    // >= 1440px
}

#[derive(Debug, Clone)]
pub struct FnDef {
    pub name: String,
    pub params: Vec<Param>,
    pub return_ty: Option<Type>,
    pub body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
pub struct LetStmt {
    pub name: String,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub struct IfStmt {
    pub condition: Expr,
    pub then_body: Vec<Stmt>,
    pub else_body: Option<Vec<Stmt>>,
}

#[derive(Debug, Clone)]
pub struct MatchStmt {
    pub value: Expr,
    pub arms: Vec<MatchArm>,
}

#[derive(Debug, Clone)]
pub struct MatchArm {
    pub pattern: Pattern,
    pub body: MatchBody,
}

#[derive(Debug, Clone)]
pub enum Pattern {
    Ident(String),
    Variant(String, Vec<String>), // Error(msg)
    Wildcard,
}

#[derive(Debug, Clone)]
pub enum MatchBody {
    Expr(Expr),
    Block(Vec<Stmt>),
}

// ── Theme ─────────────────────────────────────────────────────────────────────

pub use super::ast_theme::{ThemeDef, ThemeSection, ThemeTokenValue};

// ── Expressions ───────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
pub enum Expr {
    // Literals
    Number(f64),
    Bool(bool),
    Str(Vec<StringPart>),
    Color(String),
    None,

    // Identifiers
    Ident(String),

    // Access chains
    Field(Box<Expr>, String),
    OptField(Box<Expr>, String),

    // Calls  — optional UI block for layout elements
    Call(Box<Expr>, Vec<Arg>, Option<Vec<Stmt>>),

    // Operators
    BinOp(Box<Expr>, BinOp, Box<Expr>),
    UnOp(UnOp, Box<Expr>),
    NullCoal(Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),

    // Collections
    List(Vec<Expr>),
    Object(Vec<ObjectEntry>),

    // Async
    Try(Box<Expr>),

    // Lambda: `p -> expr` or `{ p -> expr }`
    Lambda(Vec<String>, Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum StringPart {
    Lit(String),
    Interp(Box<Expr>),
}

#[derive(Debug, Clone)]
pub struct Arg {
    pub name: Option<String>,
    pub value: Expr,
}

#[derive(Debug, Clone)]
pub enum ObjectEntry {
    Field(String, Expr),
    Spread(Expr),
}
