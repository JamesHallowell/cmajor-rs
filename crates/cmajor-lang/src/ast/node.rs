use crate::{
    arena_key,
    ast::{decl::Decl, expr::Expr, graph::Graph, item::Item, stmt::Stmt},
    lexer::TokenId,
    utils::arena::Arena,
};

arena_key!(NodeId);

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

#[derive(Debug, Default, Clone)]
pub struct Ast {
    nodes: Arena<NodeId, Node>,
}

impl Ast {
    pub fn new() -> Self {
        Self {
            nodes: Arena::default(),
        }
    }

    pub fn push(&mut self, node: impl Into<Node>) -> NodeId {
        self.nodes.push(node.into())
    }

    pub fn get(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }

    pub fn has_errors(&self) -> bool {
        self.nodes
            .values()
            .any(|node| matches!(node, Node::Error { .. }))
    }

    pub fn error_tokens(&self) -> Vec<TokenId> {
        self.nodes
            .values()
            .filter_map(|node| match node {
                Node::Error { token } => Some(token),
                _ => None,
            })
            .copied()
            .collect()
    }
}
