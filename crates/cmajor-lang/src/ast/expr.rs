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
pub enum Expr {
    Literal(Literal),
    Ident {
        token: TokenId,
    },
    Parentheses {
        paren: TokenId,
        inner: Vec<NodeId>,
    },
    Unary {
        op: TokenId,
        operand: NodeId,
    },
    PostfixUnary {
        op: TokenId,
        operand: NodeId,
    },
    Binary {
        op: TokenId,
        lhs: NodeId,
        rhs: NodeId,
    },
    Assign {
        op: TokenId,
        target: NodeId,
        value: NodeId,
    },
    Ternary {
        question: TokenId,
        cond: NodeId,
        then_branch: NodeId,
        else_branch: NodeId,
    },
    Call {
        paren: TokenId,
        callee: NodeId,
        args: Vec<NodeId>,
    },
    Bracketed {
        bracket: TokenId,
        base: NodeId,
        terms: Vec<BracketTerm>,
    },
    Field {
        name: TokenId,
        base: NodeId,
    },
    ScopeAccess {
        name: TokenId,
        base: NodeId,
    },
    TypeModifier {
        source: NodeId,
        is_const: bool,
        is_ref: bool,
    },
    VectorSizeSuffix {
        angle: TokenId,
        element: NodeId,
        terms: Vec<NodeId>,
    },
    ProcessorProperty {
        name: TokenId,
    },
}
