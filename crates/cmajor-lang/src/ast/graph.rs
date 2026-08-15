use crate::{
    ast::{annotation::Annotations, child::ChildList, node::NodeId},
    lexer::TokenId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InterpolationKind {
    None,
    Latch,
    Linear,
    Sinc,
    Fast,
    Best,
}

#[derive(Debug, Clone, PartialEq)]
pub enum HoistTarget {
    Name(TokenId),
    Wildcard { prefix: Option<TokenId> },
}

#[derive(Debug, Clone, PartialEq)]
pub struct EndpointDeclaration {
    pub direction: TokenId,
    pub kind: TokenId,
    pub types: Vec<NodeId>,
    pub name: TokenId,
    pub size: Option<NodeId>,
    pub annotations: Annotations,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HoistedEndpointDeclaration {
    pub direction: TokenId,
    pub segments: Vec<TokenId>,
    pub index: Option<NodeId>,
    pub target: HoistTarget,
    pub name: Option<TokenId>,
    pub annotations: Annotations,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeDecl {
    pub name: TokenId,
    pub processor: NodeId,
    pub array_size: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionDecl {
    pub connections: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Connection {
    pub interpolation: Option<InterpolationKind>,
    pub sources: ChildList,
    pub arrow: TokenId,
    pub delay: Option<NodeId>,
    pub destinations: ChildList,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionIf {
    pub cond: NodeId,
    pub then_branch: Vec<NodeId>,
    pub else_branch: Option<Vec<NodeId>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Graph {
    EndpointDeclaration(EndpointDeclaration),
    HoistedEndpointDeclaration(HoistedEndpointDeclaration),
    NodeDecl(NodeDecl),
    ConnectionDecl(ConnectionDecl),
    Connection(Connection),
    ConnectionIf(ConnectionIf),
}
