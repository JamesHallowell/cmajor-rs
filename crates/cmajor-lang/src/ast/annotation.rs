use crate::{
    ast::{
        Ast,
        child::{ChildList, ChildListExt},
        node::{Node, NodeId},
    },
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

impl From<Annotations> for Option<ChildList> {
    fn from(value: Annotations) -> Self {
        value.0
    }
}

impl Annotations {
    pub fn iter(&self, ast: &Ast) -> impl Iterator<Item = NodeId> {
        self.0.children(ast)
    }

    pub fn get<'ast>(&self, ast: &'ast Ast) -> impl Iterator<Item = (NodeId, &'ast Annotation)> {
        self.iter(ast).map(|id| match ast.get(id) {
            Node::Annotation(annotation) => (id, annotation),
            _ => unreachable!("Expected Annotation node"),
        })
    }
}
