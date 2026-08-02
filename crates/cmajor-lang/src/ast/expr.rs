use crate::{ast::node::NodeId, lexer::TokenId};

#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Int32 { token: TokenId },
    Int64 { token: TokenId },
    Float32 { token: TokenId },
    Float64 { token: TokenId },
    Imaginary32 { token: TokenId },
    Imaginary64 { token: TokenId },
    String { token: TokenId },
    Bool { token: TokenId },
}

impl Literal {
    pub fn token(&self) -> &TokenId {
        match self {
            Literal::Int32 { token } => token,
            Literal::Int64 { token } => token,
            Literal::Float32 { token } => token,
            Literal::Float64 { token } => token,
            Literal::Imaginary32 { token } => token,
            Literal::Imaginary64 { token } => token,
            Literal::String { token } => token,
            Literal::Bool { token } => token,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BracketTerm {
    pub start: Option<NodeId>,
    pub end: Option<NodeId>,
    pub is_range: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ident {
    pub token: TokenId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Parentheses {
    pub paren: TokenId,
    pub inner: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Unary {
    pub op: TokenId,
    pub operand: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PostfixUnary {
    pub op: TokenId,
    pub operand: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Binary {
    pub op: TokenId,
    pub lhs: NodeId,
    pub rhs: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Assign {
    pub op: TokenId,
    pub target: NodeId,
    pub value: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Ternary {
    pub question: TokenId,
    pub cond: NodeId,
    pub then_branch: NodeId,
    pub else_branch: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Call {
    pub paren: TokenId,
    pub callee: NodeId,
    pub args: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bracketed {
    pub bracket: TokenId,
    pub base: NodeId,
    pub terms: Vec<BracketTerm>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Field {
    pub name: TokenId,
    pub base: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ScopeAccess {
    pub name: TokenId,
    pub base: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypeModifier {
    pub source: NodeId,
    pub is_const: bool,
    pub is_ref: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct VectorSizeSuffix {
    pub angle: TokenId,
    pub element: NodeId,
    pub terms: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessorProperty {
    pub name: TokenId,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Literal(Literal),
    Ident(Ident),
    Parentheses(Parentheses),
    Unary(Unary),
    PostfixUnary(PostfixUnary),
    Binary(Binary),
    Assign(Assign),
    Ternary(Ternary),
    Call(Call),
    Bracketed(Bracketed),
    Field(Field),
    ScopeAccess(ScopeAccess),
    TypeModifier(TypeModifier),
    VectorSizeSuffix(VectorSizeSuffix),
    ProcessorProperty(ProcessorProperty),
}
