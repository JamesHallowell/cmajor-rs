use crate::{
    ast::{
        Ast,
        annotation::Annotations,
        child::ChildList,
        node::{Node, NodeId},
    },
    lexer::TokenId,
    static_assert_size,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AliasKind {
    Using,
    Processor,
    Namespace,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum VarKind {
    Let,
    Var,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Declarator {
    pub name: TokenId,
    pub init: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Var {
    pub kind: VarKind,
    pub declarators: ChildList,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TypedDecl {
    pub ty: NodeId,
    pub declarators: ChildList,
    pub annotations: Annotations,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Param {
    pub ty: NodeId,
    pub name: TokenId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpecialisationValue {
    pub ty: NodeId,
    pub name: TokenId,
    pub init: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct External {
    pub ty: NodeId,
    pub names: ChildList,
    pub annotations: Annotations,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Alias {
    pub kind: AliasKind,
    pub name: TokenId,
    pub target: Option<NodeId>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct EnumValue {
    pub name: TokenId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GenericParam {
    pub name: TokenId,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Decl {
    Var(Var),
    TypedDecl(TypedDecl),
    Param(Param),
    SpecialisationValue(SpecialisationValue),
    Alias(Alias),
    External(External),
    Declarator(Declarator),
    EnumValue(EnumValue),
    GenericParam(GenericParam),
}

static_assert_size!(Decl, 24);

macro_rules! declarators_method {
    ($ty:ty) => {
        impl $ty {
            pub fn declarators<'ast>(
                &self,
                ast: &'ast Ast,
            ) -> impl Iterator<Item = (NodeId, &'ast Declarator)> {
                ast.children(self.declarators)
                    .iter()
                    .map(|&id| match ast.get(id) {
                        Node::Decl(Decl::Declarator(declarator)) => (id, declarator),
                        _ => unreachable!("expected Declarator nodes"),
                    })
            }
        }
    };
}

declarators_method!(Var);
declarators_method!(TypedDecl);
