mod annotation;
#[allow(clippy::module_inception)]
mod ast;
mod child;
mod decl;
mod dump;
mod expr;
mod graph;
mod item;
mod node;
mod stmt;
pub mod visit;

pub use {
    annotation::{Annotation, Annotations},
    ast::Ast,
    child::{Checkpoint, ChildId, ChildList, ChildPool},
    decl::{
        Alias, AliasKind, Decl, Declarator, EnumValue, External, GenericParam, Param,
        SpecialisationValue, TypedDecl, Var, VarKind,
    },
    dump::dump,
    expr::{
        Assign, Binary, Bracketed, Call, Expr, Field, Ident, Literal, Parentheses, PostfixUnary,
        ProcessorProperty, ScopeAccess, Slice, Ternary, TypeModifier, Unary, VectorSizeSuffix,
    },
    graph::{
        Connection, ConnectionDecl, ConnectionIf, EndpointDeclaration, Graph, HoistTarget,
        HoistedEndpointDeclaration, InterpolationKind, NodeDecl,
    },
    item::{
        EnumDecl, EventHandlerDecl, FunctionDecl, GraphDecl, Import, Item, ModuleAlias,
        NamespaceDecl, ProcessorDecl, StructDecl,
    },
    node::{Node, NodeId},
    stmt::{
        Block, BreakStmt, ContinueStmt, DeclStmt, ExprStmt, ForStmt, ForwardBranchStmt,
        IfConstStmt, IfStmt, LoopStmt, ReturnStmt, Stmt, WhileStmt,
    },
};
