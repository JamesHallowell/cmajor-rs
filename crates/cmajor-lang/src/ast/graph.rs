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
pub enum Graph {
    EndpointDecl {
        direction: TokenId,
        kind: Option<TokenId>,
        types: Vec<NodeId>,
        name: Option<TokenId>,
        size: Option<NodeId>,
        hoisted: Option<HoistedPath>,
        attributes: Option<Vec<(TokenId, Option<NodeId>)>>,
    },
    NodeDecl {
        keyword: TokenId,
        name: TokenId,
        processor: NodeId,
        array_size: Option<NodeId>,
    },
    ConnectionDecl {
        keyword: TokenId,
        connections: Vec<NodeId>,
    },
    Connection {
        interpolation: Option<InterpolationKind>,
        sources: Vec<NodeId>,
        arrow: TokenId,
        delay: Option<NodeId>,
        destinations: Vec<NodeId>,
    },
    ConnectionIf {
        keyword: TokenId,
        cond: NodeId,
        then_branch: Vec<NodeId>,
        else_branch: Option<Vec<NodeId>>,
    },
}
