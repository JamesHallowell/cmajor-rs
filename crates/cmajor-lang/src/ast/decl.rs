use crate::{ast::node::NodeId, lexer::TokenId};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasKind {
    Using,
    Processor,
    Namespace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarRole {
    Let,
    Var,
    Typed,
    Parameter,
    SpecialisationValue,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Declarator {
    pub name: TokenId,
    pub init: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Var {
        role: VarRole,
        ty: Option<NodeId>,
        is_external: bool,
        declarators: Vec<Declarator>,
        attributes: Option<Vec<(TokenId, Option<NodeId>)>>,
    },
    Alias {
        keyword: TokenId,
        kind: AliasKind,
        name: TokenId,
        target: Option<NodeId>,
    },
}
