use crate::{
    arena_key,
    ast::{attribute::AttributeList, decl::Decl, expr::Expr, graph::Graph, item::Item, stmt::Stmt},
    lexer::TokenId,
};

arena_key!(NodeId);

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    Expr(Expr),
    Stmt(Stmt),
    Decl(Decl),
    Item(Item),
    Graph(Graph),
    AttributeList(AttributeList),
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

impl From<AttributeList> for Node {
    fn from(attribute_list: AttributeList) -> Self {
        Node::AttributeList(attribute_list)
    }
}
