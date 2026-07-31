use crate::{
    ast::{decl::AliasKind, node::NodeId},
    lexer::TokenId,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Item {
    NamespaceDecl {
        keyword: TokenId,
        segments: Vec<TokenId>,
        params: Vec<NodeId>,
        attributes: Option<Vec<(TokenId, Option<NodeId>)>>,
        items: Vec<NodeId>,
    },
    ProcessorDecl {
        keyword: TokenId,
        name: TokenId,
        params: Vec<NodeId>,
        attributes: Option<Vec<(TokenId, Option<NodeId>)>>,
        items: Vec<NodeId>,
    },
    GraphDecl {
        keyword: TokenId,
        name: TokenId,
        params: Vec<NodeId>,
        attributes: Option<Vec<(TokenId, Option<NodeId>)>>,
        items: Vec<NodeId>,
    },
    StructDecl {
        keyword: TokenId,
        name: TokenId,
        attributes: Option<Vec<(TokenId, Option<NodeId>)>>,
        items: Vec<NodeId>,
    },
    EnumDecl {
        keyword: TokenId,
        name: TokenId,
        values: Vec<TokenId>,
    },
    FunctionDecl {
        ty: Option<NodeId>,
        name: TokenId,
        generics: Vec<TokenId>,
        params: Vec<NodeId>,
        is_const: bool,
        is_event_handler: bool,
        attributes: Option<Vec<(TokenId, Option<NodeId>)>>,
        body: NodeId,
    },
    Import {
        keyword: TokenId,
        path: Vec<TokenId>,
    },
    ModuleAlias {
        keyword: TokenId,
        kind: AliasKind,
        name: TokenId,
        target: NodeId,
    },
}
