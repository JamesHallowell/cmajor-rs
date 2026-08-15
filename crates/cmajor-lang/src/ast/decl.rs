use crate::{
    ast::{annotation::Annotations, child::ChildList, node::NodeId},
    lexer::TokenId,
};

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
pub struct Var {
    pub role: VarRole,
    pub ty: Option<NodeId>,
    pub declarators: Vec<Declarator>,
    pub annotations: Annotations,
}

#[derive(Debug, Clone, PartialEq)]
pub struct External {
    pub ty: NodeId,
    pub names: ChildList,
    pub annotations: Annotations,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Alias {
    pub keyword: TokenId,
    pub kind: AliasKind,
    pub name: TokenId,
    pub target: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Var(Var),
    Alias(Alias),
    External(External),
}
