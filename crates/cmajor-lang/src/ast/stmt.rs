use crate::{ast::node::NodeId, lexer::TokenId};

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Block {
        brace: TokenId,
        label: Option<TokenId>,
        stmts: Vec<NodeId>,
    },
    ExprStmt {
        expr: NodeId,
    },
    DeclStmt {
        decl: NodeId,
    },
    ForStmt {
        keyword: TokenId,
        label: Option<TokenId>,
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
        label: Option<TokenId>,
        cond: NodeId,
        body: NodeId,
    },
    LoopStmt {
        keyword: TokenId,
        label: Option<TokenId>,
        count: Option<NodeId>,
        body: NodeId,
    },
    ReturnStmt {
        keyword: TokenId,
        value: Option<NodeId>,
    },
    BreakStmt {
        keyword: TokenId,
        target: Option<TokenId>,
    },
    ContinueStmt {
        keyword: TokenId,
        target: Option<TokenId>,
    },
    ForwardBranchStmt {
        keyword: TokenId,
        cond: NodeId,
        targets: Vec<TokenId>,
    },
    StaticAssertStmt {
        keyword: TokenId,
        cond: NodeId,
        message: Option<NodeId>,
    },
}
