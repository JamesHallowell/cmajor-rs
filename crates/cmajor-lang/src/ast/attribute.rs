use crate::{ast::node::NodeId, lexer::TokenId};

#[derive(Debug, Clone, PartialEq)]
pub struct AttributeList {
    pub attributes: Vec<(TokenId, Option<NodeId>)>,
}
