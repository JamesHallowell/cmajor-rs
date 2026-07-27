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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialisationParamKind {
    Using,
    Processor,
    Namespace,
    Value { ty: NodeId },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Node {
    IntLiteral {
        token: TokenId,
    },
    FloatLiteral {
        token: TokenId,
    },
    ImaginaryLiteral {
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
    Parentheses {
        paren: TokenId,
        inner: Vec<NodeId>,
    },
    Unary {
        op: TokenId,
        operand: NodeId,
    },
    PostfixUnary {
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
        index: Option<NodeId>,
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
        declarators: Vec<(TokenId, NodeId)>,
    },
    VarStmt {
        name: TokenId,
        init: Option<NodeId>,
    },
    VarDeclStmt {
        ty: NodeId,
        declarators: Vec<(TokenId, Option<NodeId>)>,
    },
    ForStmt {
        keyword: TokenId,
        init: Option<NodeId>,
        cond: Option<NodeId>,
        update: Option<NodeId>,
        body: NodeId,
    },
    IfStmt {
        keyword: TokenId,
        is_const: bool,
        cond: NodeId,
        then_branch: NodeId,
        else_branch: Option<NodeId>,
    },
    WhileStmt {
        keyword: TokenId,
        cond: NodeId,
        body: NodeId,
    },
    LoopStmt {
        keyword: TokenId,
        count: Option<NodeId>,
        body: NodeId,
    },
    ReturnStmt {
        keyword: TokenId,
        value: Option<NodeId>,
    },
    BreakStmt {
        keyword: TokenId,
    },
    ContinueStmt {
        keyword: TokenId,
    },
    TypeName {
        segments: Vec<TokenId>,
    },
    ConstType {
        keyword: TokenId,
        inner: NodeId,
    },
    Array {
        bracket: TokenId,
        element: NodeId,
        size: Option<NodeId>,
    },
    ChevronSuffix {
        angle: TokenId,
        element: NodeId,
        term: NodeId,
    },
    TypeList {
        paren: TokenId,
        types: Vec<NodeId>,
    },
    Error {
        token: TokenId,
    },
    NamespaceDecl {
        keyword: TokenId,
        segments: Vec<TokenId>,
        params: Vec<NodeId>,
        items: Vec<NodeId>,
    },
    SpecialisationParam {
        kind: SpecialisationParamKind,
        name: TokenId,
        default: Option<NodeId>,
    },
    ProcessorDecl {
        keyword: TokenId,
        name: TokenId,
        params: Vec<NodeId>,
        attributes: Option<NodeId>,
        items: Vec<NodeId>,
    },
    GraphDecl {
        keyword: TokenId,
        name: TokenId,
        params: Vec<NodeId>,
        attributes: Option<NodeId>,
        items: Vec<NodeId>,
    },
    StructDecl {
        keyword: TokenId,
        name: TokenId,
        attributes: Option<NodeId>,
        items: Vec<NodeId>,
    },
    EndpointGroup {
        direction: TokenId,
        kind: TokenId,
        endpoints: Vec<NodeId>,
    },
    EndpointWildcard {
        direction: TokenId,
        name: TokenId,
    },
    NodeDecl {
        keyword: TokenId,
        name: TokenId,
        value: NodeId,
    },
    ConnectionDecl {
        keyword: TokenId,
        connections: Vec<NodeId>,
    },
    Connection {
        interpolation: Option<TokenId>,
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
    EndpointDecl {
        ty: NodeId,
        name: TokenId,
        attributes: Option<NodeId>,
    },
    AttributeList {
        attrs: Vec<(TokenId, Option<NodeId>)>,
    },
    FunctionDecl {
        ty: NodeId,
        name: TokenId,
        generics: Vec<TokenId>,
        params: Vec<NodeId>,
        is_const: bool,
        body: NodeId,
    },
    Param {
        ty: NodeId,
        name: TokenId,
        by_ref: bool,
    },
    EventHandlerDecl {
        keyword: TokenId,
        name: TokenId,
        params: Vec<NodeId>,
        body: NodeId,
    },
    ScopeAccess {
        name: TokenId,
        base: NodeId,
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
