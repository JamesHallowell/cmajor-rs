use crate::{
    ast::{decl::AliasKind, node::NodeId},
    lexer::TokenId,
};

#[derive(Debug, Clone, PartialEq)]
pub struct NamespaceDecl {
    pub keyword: TokenId,
    pub segments: Vec<TokenId>,
    pub params: Vec<NodeId>,
    pub attributes: Option<NodeId>,
    pub items: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessorDecl {
    pub keyword: TokenId,
    pub name: TokenId,
    pub params: Vec<NodeId>,
    pub attributes: Option<NodeId>,
    pub items: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphDecl {
    pub keyword: TokenId,
    pub name: TokenId,
    pub params: Vec<NodeId>,
    pub attributes: Option<NodeId>,
    pub items: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub keyword: TokenId,
    pub name: TokenId,
    pub attributes: Option<NodeId>,
    pub items: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub keyword: TokenId,
    pub name: TokenId,
    pub values: Vec<TokenId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub ty: Option<NodeId>,
    pub name: TokenId,
    pub generics: Vec<TokenId>,
    pub params: Vec<NodeId>,
    pub is_const: bool,
    pub is_event_handler: bool,
    pub attributes: Option<NodeId>,
    pub body: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub keyword: TokenId,
    pub path: Vec<TokenId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleAlias {
    pub keyword: TokenId,
    pub kind: AliasKind,
    pub name: TokenId,
    pub target: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    NamespaceDecl(NamespaceDecl),
    ProcessorDecl(ProcessorDecl),
    GraphDecl(GraphDecl),
    StructDecl(StructDecl),
    EnumDecl(EnumDecl),
    FunctionDecl(FunctionDecl),
    Import(Import),
    ModuleAlias(ModuleAlias),
}
