use crate::{
    ast::{decl::Decl, expr::Expr, graph::Graph, item::Item, stmt::Stmt},
    lexer::TokenId,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u32);

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Expr(Expr),
    Stmt(Stmt),
    Decl(Decl),
    Item(Item),
    Graph(Graph),
    Error { token: TokenId },
}

impl From<Expr> for Node {
    fn from(expr: Expr) -> Self {
        Node::Expr(expr)
    }
}

impl From<Stmt> for Node {
    fn from(stmt: Stmt) -> Self {
        Node::Stmt(stmt)
    }
}

impl From<Decl> for Node {
    fn from(decl: Decl) -> Self {
        Node::Decl(decl)
    }
}

impl From<Item> for Node {
    fn from(item: Item) -> Self {
        Node::Item(item)
    }
}

impl From<Graph> for Node {
    fn from(member: Graph) -> Self {
        Node::Graph(member)
    }
}

impl From<u32> for NodeId {
    fn from(id: u32) -> Self {
        NodeId(id)
    }
}

impl From<NodeId> for u32 {
    fn from(id: NodeId) -> Self {
        id.0
    }
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Ast {
    nodes: Vec<Node>,
}

impl Ast {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn push(&mut self, node: impl Into<Node>) -> NodeId {
        let id = NodeId::from(self.nodes.len() as u32);
        self.nodes.push(node.into());
        id
    }

    pub fn get(&self, id: NodeId) -> &Node {
        &self.nodes[u32::from(id) as usize]
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn truncate(&mut self, len: usize) {
        self.nodes.truncate(len);
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        self.nodes
            .iter()
            .any(|node| matches!(node, Node::Error { .. }))
    }

    pub fn error_tokens(&self) -> Vec<TokenId> {
        self.nodes
            .iter()
            .filter_map(|node| match node {
                Node::Error { token } => Some(*token),
                _ => None,
            })
            .collect()
    }
}
