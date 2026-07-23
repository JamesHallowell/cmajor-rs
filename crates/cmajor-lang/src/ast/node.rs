use crate::lexer::TokenId;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct NodeId(u32);

impl NodeId {
    pub const fn from_raw(raw: u32) -> Self {
        NodeId(raw)
    }

    pub const fn raw(self) -> u32 {
        self.0
    }

    fn index(self) -> usize {
        self.0 as usize
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    IntLiteral {
        token: TokenId,
    },
    FloatLiteral {
        token: TokenId,
    },
    StringLiteral {
        token: TokenId,
    },
    BoolLiteral {
        token: TokenId,
    },
    Ident {
        token: TokenId,
    },

    Paren {
        paren: TokenId,
        inner: NodeId,
    },
    Unary {
        op: TokenId,
        operand: NodeId,
    },
    Binary {
        op: TokenId,
        lhs: NodeId,
        rhs: NodeId,
    },
    Assign {
        op: TokenId,
        target: NodeId,
        value: NodeId,
    },
    Ternary {
        question: TokenId,
        cond: NodeId,
        then_branch: NodeId,
        else_branch: NodeId,
    },
    Call {
        paren: TokenId,
        callee: NodeId,
        args: Vec<NodeId>,
    },
    Index {
        bracket: TokenId,
        base: NodeId,
        index: NodeId,
    },
    Field {
        name: TokenId,
        base: NodeId,
    },

    Block {
        brace: TokenId,
        stmts: Vec<NodeId>,
    },
    ExprStmt {
        expr: NodeId,
    },
    LetStmt {
        name: TokenId,
        init: NodeId,
    },
    VarStmt {
        name: TokenId,
        init: Option<NodeId>,
    },
    IfStmt {
        kw: TokenId,
        cond: NodeId,
        then_branch: NodeId,
        else_branch: Option<NodeId>,
    },
    WhileStmt {
        kw: TokenId,
        cond: NodeId,
        body: NodeId,
    },
    LoopStmt {
        kw: TokenId,
        count: Option<NodeId>,
        body: NodeId,
    },
    ReturnStmt {
        kw: TokenId,
        value: Option<NodeId>,
    },
    BreakStmt {
        kw: TokenId,
    },
    ContinueStmt {
        kw: TokenId,
    },

    TypeName {
        segments: Vec<TokenId>,
    },
    TypeWrap {
        kw: TokenId,
        size: NodeId,
    },
    TypeClamp {
        kw: TokenId,
        size: NodeId,
    },
    TypeArray {
        bracket: TokenId,
        element: NodeId,
        size: Option<NodeId>,
    },
    TypeVector {
        angle: TokenId,
        element: NodeId,
        size: NodeId,
    },

    Error {
        token: TokenId,
    },
}

#[derive(Debug, Default, Clone, PartialEq)]
pub struct Ast {
    nodes: Vec<Node>,
}

impl Ast {
    pub fn new() -> Self {
        Self { nodes: Vec::new() }
    }

    pub fn push(&mut self, node: Node) -> NodeId {
        let id = NodeId::from_raw(self.nodes.len() as u32);
        self.nodes.push(node);
        id
    }

    pub fn get(&self, id: NodeId) -> &Node {
        &self.nodes[id.index()]
    }

    pub fn len(&self) -> usize {
        self.nodes.len()
    }

    pub fn is_empty(&self) -> bool {
        self.nodes.is_empty()
    }
}
