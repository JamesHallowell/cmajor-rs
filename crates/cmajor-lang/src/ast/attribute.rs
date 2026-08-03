use crate::{ast::node::NodeId, lexer::TokenId};

#[derive(Debug, Clone, PartialEq)]
pub struct Attribute {
    pub key: TokenId,
    pub value: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct AttributeList {
    pub attributes: Vec<Attribute>,
}
