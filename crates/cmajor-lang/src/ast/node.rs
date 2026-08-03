use {
    crate::{
        arena_key,
        ast::{
            attribute::AttributeList,
            decl::Decl,
            expr::Expr,
            graph::Graph,
            item::Item,
            stmt::{Stmt, StmtId},
        },
        lexer::TokenId,
        utils::arena::{Arena, SecondaryArena},
    },
    std::range::Range,
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

#[derive(Debug, Default, Clone)]
pub struct Ast {
    nodes: Arena<NodeId, Node>,
    roots: Vec<NodeId>,
    spans: SecondaryArena<NodeId, Range<TokenId>>,
}

impl Ast {
    pub fn new(
        nodes: Arena<NodeId, Node>,
        roots: Vec<NodeId>,
        spans: SecondaryArena<NodeId, Range<TokenId>>,
    ) -> Self {
        let ast = Self {
            nodes,
            roots,
            spans,
        };

        #[cfg(test)]
        assert_spans_contain_children(&ast);

        ast
    }

    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    pub fn push(&mut self, node: impl Into<Node>) -> NodeId {
        self.nodes.push(node.into())
    }

    pub fn get(&self, id: NodeId) -> &Node {
        &self.nodes[id]
    }

    pub fn span(&self, id: NodeId) -> Range<TokenId> {
        self.spans[id]
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

    pub fn as_stmt(&self, node: NodeId) -> Option<StmtId> {
        if let Node::Stmt(_) = &self.nodes[node] {
            Some(StmtId(node))
        } else {
            None
        }
    }
}

impl std::ops::Index<NodeId> for Ast {
    type Output = Node;

    fn index(&self, id: NodeId) -> &Self::Output {
        &self.nodes[id]
    }
}

#[cfg(test)]
pub(crate) fn assert_spans_contain_children(ast: &Ast) {
    use crate::ast::visit::{Visitor, Walk, walk};

    struct SpanChecker {
        ancestors: Vec<NodeId>,
        violations: Vec<String>,
    }

    impl Visitor for SpanChecker {
        fn visit(&mut self, ast: &Ast, id: NodeId) {
            let span = ast.span(id);

            if let Some(&parent) = self.ancestors.last() {
                let parent_span = ast.span(parent);
                if span.start < parent_span.start || span.end > parent_span.end {
                    self.violations.push(format!(
                        "{id:?} span {span:?} is not contained by parent {parent:?} span {parent_span:?}"
                    ));
                }
            }

            self.ancestors.push(id);
            walk(ast, self, id);
            self.ancestors.pop();
        }
    }

    let mut checker = SpanChecker {
        ancestors: Vec::new(),
        violations: Vec::new(),
    };
    for &root in ast.roots() {
        checker.visit(ast, root);
    }

    assert!(
        checker.violations.is_empty(),
        "span containment violated:\n{}",
        checker.violations.join("\n")
    );
}
