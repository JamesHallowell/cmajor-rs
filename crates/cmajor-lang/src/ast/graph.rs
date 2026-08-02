use crate::{ast::node::NodeId, lexer::TokenId};

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
pub struct HoistedPath {
    pub segments: Vec<TokenId>,
    pub index: Option<NodeId>,
    pub target: HoistTarget,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EndpointDecl {
    pub direction: TokenId,
    pub kind: Option<TokenId>,
    pub types: Vec<NodeId>,
    pub name: Option<TokenId>,
    pub size: Option<NodeId>,
    pub hoisted: Option<HoistedPath>,
    pub attributes: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NodeDecl {
    pub keyword: TokenId,
    pub name: TokenId,
    pub processor: NodeId,
    pub array_size: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionDecl {
    pub keyword: TokenId,
    pub connections: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Connection {
    pub interpolation: Option<InterpolationKind>,
    pub sources: Vec<NodeId>,
    pub arrow: TokenId,
    pub delay: Option<NodeId>,
    pub destinations: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ConnectionIf {
    pub keyword: TokenId,
    pub cond: NodeId,
    pub then_branch: Vec<NodeId>,
    pub else_branch: Option<Vec<NodeId>>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Graph {
    EndpointDecl(EndpointDecl),
    NodeDecl(NodeDecl),
    ConnectionDecl(ConnectionDecl),
    Connection(Connection),
    ConnectionIf(ConnectionIf),
}
