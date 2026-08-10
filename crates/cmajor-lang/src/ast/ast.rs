use {
    crate::{
        ast::{
            child::{ChildList, ChildPool},
            node::{Node, NodeId},
            visit::Visitor,
        },
        lexer::TokenId,
        utils::arena::{Arena, SecondaryArena, SparseSecondaryArena},
    },
    std::range::Range,
};

#[derive(Debug, Default, Clone)]
pub struct Ast {
    nodes: Arena<NodeId, Node>,
    roots: Vec<NodeId>,
    spans: SecondaryArena<NodeId, Range<TokenId>>,
    children: ChildPool,
    labels: SparseSecondaryArena<NodeId, TokenId>,
}

impl Ast {
    pub fn new(
        nodes: Arena<NodeId, Node>,
        roots: Vec<NodeId>,
        spans: SecondaryArena<NodeId, Range<TokenId>>,
        child_pool: ChildPool,
        labels: SparseSecondaryArena<NodeId, TokenId>,
    ) -> Self {
        let ast = Self {
            nodes,
            roots,
            spans,
            children: child_pool,
            labels,
        };

        #[cfg(test)]
        assert_spans_contain_children(&ast);

        ast
    }

    pub fn roots(&self) -> &[NodeId] {
        &self.roots
    }

    pub fn children(&self, children: ChildList) -> &[NodeId] {
        self.children.get(children)
    }

    pub fn label(&self, node: NodeId) -> Option<TokenId> {
        self.labels.get(node).copied()
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

    pub fn visit<V>(&self, visitor: &mut V)
    where
        V: Visitor + ?Sized,
    {
        for &root in self.roots() {
            visitor.visit_root(self, root);
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
    use crate::ast::visit::{Visitor, walk};

    struct Violation {
        node: (NodeId, Range<TokenId>),
        parent: (NodeId, Range<TokenId>),
    }

    #[derive(Default)]
    struct SpanChecker {
        ancestors: Vec<NodeId>,
        violations: Vec<Violation>,
    }

    impl std::fmt::Debug for Violation {
        fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
            write!(
                f,
                "{:?} span {:?} is not contained within parent {:?} span {:?}",
                self.node.0, self.node.1, self.parent.0, self.parent.1
            )
        }
    }

    impl Visitor for SpanChecker {
        fn visit(&mut self, ast: &Ast, id: NodeId) {
            let span = ast.span(id);

            if let Some(&parent) = self.ancestors.last() {
                let parent_span = ast.span(parent);

                if span.start < parent_span.start || span.end > parent_span.end {
                    self.violations.push(Violation {
                        node: (id, span),
                        parent: (parent, parent_span),
                    });
                }
            }

            self.ancestors.push(id);
            walk(ast, self, id);
            self.ancestors.pop();
        }
    }

    let mut checker = SpanChecker::default();
    ast.visit(&mut checker);

    assert!(
        checker.violations.is_empty(),
        "span containment violated:\n{:#?}",
        checker.violations
    );
}
