use crate::{
    ast::{
        Ast, ChildList, Decl, Node,
        annotation::Annotations,
        decl::{AliasKind, EnumValue},
        node::NodeId,
    },
    lexer::TokenId,
};

#[derive(Debug, Clone, PartialEq)]
pub struct NamespaceDecl {
    pub segments: Vec<TokenId>,
    pub params: Vec<NodeId>,
    pub annotations: Annotations,
    pub items: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ProcessorDecl {
    pub name: TokenId,
    pub params: Vec<NodeId>,
    pub annotations: Annotations,
    pub items: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct GraphDecl {
    pub name: TokenId,
    pub params: Vec<NodeId>,
    pub annotations: Annotations,
    pub items: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StructDecl {
    pub name: TokenId,
    pub annotations: Annotations,
    pub items: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EnumDecl {
    pub name: TokenId,
    pub values: ChildList,
}

#[derive(Debug, Clone, PartialEq)]
pub struct FunctionDecl {
    pub returns: NodeId,
    pub name: TokenId,
    pub generics: Vec<TokenId>,
    pub params: Vec<NodeId>,
    pub is_const: bool,
    pub annotations: Annotations,
    pub body: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventHandlerDecl {
    pub name: TokenId,
    pub generics: Vec<TokenId>,
    pub params: Vec<NodeId>,
    pub is_const: bool,
    pub annotations: Annotations,
    pub body: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Import {
    pub path: Vec<TokenId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ModuleAlias {
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
    EventHandlerDecl(EventHandlerDecl),
    Import(Import),
    ModuleAlias(ModuleAlias),
}

impl EnumDecl {
    pub fn values<'ast>(&self, ast: &'ast Ast) -> impl Iterator<Item = (NodeId, &'ast EnumValue)> {
        self.values.iter(ast).map(|id| match ast.get(id) {
            Node::Decl(Decl::EnumValue(value)) => (id, value),
            _ => unreachable!("Expected EnumValue node"),
        })
    }
}
