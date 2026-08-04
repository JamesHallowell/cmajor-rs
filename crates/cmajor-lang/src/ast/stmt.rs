use crate::{arena_key, ast::node::NodeId, lexer::TokenId};

arena_key!(StmtId(pub(super) NodeId));

#[derive(Debug, Clone, PartialEq)]
pub struct Block {
    pub label: Option<TokenId>,
    pub stmts: Vec<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExprStmt {
    pub expr: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DeclStmt {
    pub decl: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForStmt {
    pub label: Option<TokenId>,
    pub init: Option<NodeId>,
    pub cond: Option<NodeId>,
    pub update: Option<NodeId>,
    pub body: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfConstStmt {
    pub cond: NodeId,
    pub then_branch: NodeId,
    pub else_branch: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct IfStmt {
    pub cond: NodeId,
    pub then_branch: NodeId,
    pub else_branch: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WhileStmt {
    pub label: Option<TokenId>,
    pub cond: NodeId,
    pub body: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LoopStmt {
    pub label: Option<TokenId>,
    pub count: Option<NodeId>,
    pub body: NodeId,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ReturnStmt {
    pub value: Option<NodeId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct BreakStmt {
    pub target: Option<TokenId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContinueStmt {
    pub target: Option<TokenId>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ForwardBranchStmt {
    pub cond: NodeId,
    pub targets: Vec<TokenId>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Block(Block),
    ExprStmt(ExprStmt),
    DeclStmt(DeclStmt),
    ForStmt(ForStmt),
    IfStmt(IfStmt),
    IfConstStmt(IfConstStmt),
    WhileStmt(WhileStmt),
    LoopStmt(LoopStmt),
    ReturnStmt(ReturnStmt),
    BreakStmt(BreakStmt),
    ContinueStmt(ContinueStmt),
    ForwardBranchStmt(ForwardBranchStmt),
}
