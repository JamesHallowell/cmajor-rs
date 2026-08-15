use crate::{
    ast::{child::ChildList, node::NodeId},
    lexer::TokenId,
};

#[derive(Debug, Clone, PartialEq)]
pub struct Annotation {
    pub key: TokenId,
    pub value: Option<NodeId>,
}

#[derive(Debug, Default, Copy, Clone, PartialEq)]
pub struct Annotations(Option<ChildList>);

impl From<ChildList> for Annotations {
    fn from(value: ChildList) -> Self {
        Self(Some(value))
    }
}

impl From<Option<ChildList>> for Annotations {
    fn from(value: Option<ChildList>) -> Self {
        Self(value)
    }
}

impl Annotations {
    pub fn get(&self) -> Option<ChildList> {
        self.0
    }
}
