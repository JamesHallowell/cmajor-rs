use crate::{
    ast::{
        Ast, AttributeList, Decl, Expr, Graph, Item, Node, NodeId, Stmt,
        decl::{Alias, Var},
        expr::{
            Assign, Binary, Bracketed, Call, Field, Ident, Parentheses, PostfixUnary,
            ProcessorProperty, ScopeAccess, Ternary, TypeModifier, Unary, VectorSizeSuffix,
        },
        graph::{Connection, ConnectionDecl, ConnectionIf, EndpointDecl, NodeDecl},
        item::{
            EnumDecl, FunctionDecl, GraphDecl, Import, ModuleAlias, NamespaceDecl, ProcessorDecl,
            StructDecl,
        },
        stmt::{
            Block, BreakStmt, ContinueStmt, DeclStmt, ExprStmt, ForStmt, ForwardBranchStmt, IfStmt,
            LoopStmt, ReturnStmt, WhileStmt,
        },
    },
    lexer::TokenId,
};

pub fn visit_ast<V>(ast: &Ast, visitor: &mut V)
where
    V: Visitor + ?Sized,
{
    for &root in ast.roots() {
        visitor.visit(ast, root);
    }
}

pub trait Visitor {
    fn visit(&mut self, ast: &Ast, id: NodeId) {
        walk(ast, self, id);
    }

    fn visit_item(&mut self, ast: &Ast, id: NodeId, item: &Item) {
        walk_item(ast, self, id, item);
    }

    fn visit_namespace_decl(&mut self, ast: &Ast, id: NodeId, namespace_decl: &NamespaceDecl) {
        let _ = id;
        walk_namespace_decl(ast, self, namespace_decl);
    }

    fn visit_processor_decl(&mut self, ast: &Ast, id: NodeId, processor_decl: &ProcessorDecl) {
        let _ = id;
        walk_processor_decl(ast, self, processor_decl);
    }

    fn visit_graph_decl(&mut self, ast: &Ast, id: NodeId, graph_decl: &GraphDecl) {
        let _ = id;
        walk_graph_decl(ast, self, graph_decl);
    }

    fn visit_struct_decl(&mut self, ast: &Ast, id: NodeId, struct_decl: &StructDecl) {
        let _ = id;
        walk_struct_decl(ast, self, struct_decl);
    }

    fn visit_enum_decl(&mut self, _ast: &Ast, _id: NodeId, _enum_decl: &EnumDecl) {}

    fn visit_function_decl(&mut self, ast: &Ast, id: NodeId, function_decl: &FunctionDecl) {
        let _ = id;
        walk_function_decl(ast, self, function_decl);
    }

    fn visit_import(&mut self, _ast: &Ast, _id: NodeId, _import: &Import) {}

    fn visit_module_alias(&mut self, ast: &Ast, id: NodeId, module_alias: &ModuleAlias) {
        let _ = id;
        walk_module_alias(ast, self, module_alias);
    }

    fn visit_decl(&mut self, ast: &Ast, id: NodeId, decl: &Decl) {
        walk_decl(ast, self, id, decl);
    }

    fn visit_var(&mut self, ast: &Ast, id: NodeId, var: &Var) {
        let _ = id;
        walk_var(ast, self, var);
    }

    fn visit_alias(&mut self, ast: &Ast, id: NodeId, alias: &Alias) {
        let _ = id;
        walk_alias(ast, self, alias);
    }

    fn visit_graph(&mut self, ast: &Ast, id: NodeId, graph: &Graph) {
        walk_graph(ast, self, id, graph);
    }

    fn visit_endpoint_decl(&mut self, ast: &Ast, id: NodeId, endpoint_decl: &EndpointDecl) {
        let _ = id;
        walk_endpoint_decl(ast, self, endpoint_decl);
    }

    fn visit_node_decl(&mut self, ast: &Ast, id: NodeId, node_decl: &NodeDecl) {
        let _ = id;
        walk_node_decl(ast, self, node_decl);
    }

    fn visit_connection_decl(&mut self, ast: &Ast, id: NodeId, connection_decl: &ConnectionDecl) {
        let _ = id;
        walk_connection_decl(ast, self, connection_decl);
    }

    fn visit_connection(&mut self, ast: &Ast, id: NodeId, connection: &Connection) {
        let _ = id;
        walk_connection(ast, self, connection);
    }

    fn visit_connection_if(&mut self, ast: &Ast, id: NodeId, connection_if: &ConnectionIf) {
        let _ = id;
        walk_connection_if(ast, self, connection_if);
    }

    fn visit_stmt(&mut self, ast: &Ast, id: NodeId, stmt: &Stmt) {
        walk_stmt(ast, self, id, stmt);
    }

    fn visit_block(&mut self, ast: &Ast, id: NodeId, block: &Block) {
        let _ = id;
        walk_block(ast, self, block);
    }

    fn visit_expr_stmt(&mut self, ast: &Ast, id: NodeId, expr_stmt: &ExprStmt) {
        let _ = id;
        walk_expr_stmt(ast, self, expr_stmt);
    }

    fn visit_decl_stmt(&mut self, ast: &Ast, id: NodeId, decl_stmt: &DeclStmt) {
        let _ = id;
        walk_decl_stmt(ast, self, decl_stmt);
    }

    fn visit_for_stmt(&mut self, ast: &Ast, id: NodeId, for_stmt: &ForStmt) {
        let _ = id;
        walk_for_stmt(ast, self, for_stmt);
    }

    fn visit_if_stmt(&mut self, ast: &Ast, id: NodeId, if_stmt: &IfStmt) {
        let _ = id;
        walk_if_stmt(ast, self, if_stmt);
    }

    fn visit_while_stmt(&mut self, ast: &Ast, id: NodeId, while_stmt: &WhileStmt) {
        let _ = id;
        walk_while_stmt(ast, self, while_stmt);
    }

    fn visit_loop_stmt(&mut self, ast: &Ast, id: NodeId, loop_stmt: &LoopStmt) {
        let _ = id;
        walk_loop_stmt(ast, self, loop_stmt);
    }

    fn visit_return_stmt(&mut self, ast: &Ast, id: NodeId, return_stmt: &ReturnStmt) {
        let _ = id;
        walk_return_stmt(ast, self, return_stmt);
    }

    fn visit_break_stmt(&mut self, _ast: &Ast, _id: NodeId, _break_stmt: &BreakStmt) {}

    fn visit_continue_stmt(&mut self, _ast: &Ast, _id: NodeId, _continue_stmt: &ContinueStmt) {}

    fn visit_forward_branch_stmt(
        &mut self,
        ast: &Ast,
        id: NodeId,
        forward_branch_stmt: &ForwardBranchStmt,
    ) {
        let _ = id;
        walk_forward_branch_stmt(ast, self, forward_branch_stmt);
    }

    fn visit_expr(&mut self, ast: &Ast, id: NodeId, expr: &Expr) {
        walk_expr(ast, self, id, expr);
    }

    fn visit_ident(&mut self, _ast: &Ast, _id: NodeId, _token: TokenId) {}

    fn visit_parentheses(&mut self, ast: &Ast, id: NodeId, parentheses: &Parentheses) {
        let _ = id;
        walk_parentheses(ast, self, parentheses);
    }

    fn visit_unary(&mut self, ast: &Ast, id: NodeId, unary: &Unary) {
        let _ = id;
        walk_unary(ast, self, unary);
    }

    fn visit_postfix_unary(&mut self, ast: &Ast, id: NodeId, postfix_unary: &PostfixUnary) {
        let _ = id;
        walk_postfix_unary(ast, self, postfix_unary);
    }

    fn visit_binary(&mut self, ast: &Ast, id: NodeId, binary: &Binary) {
        let _ = id;
        walk_binary(ast, self, binary);
    }

    fn visit_assign(&mut self, ast: &Ast, id: NodeId, assign: &Assign) {
        let _ = id;
        walk_assign(ast, self, assign);
    }

    fn visit_ternary(&mut self, ast: &Ast, id: NodeId, ternary: &Ternary) {
        let _ = id;
        walk_ternary(ast, self, ternary);
    }

    fn visit_call(&mut self, ast: &Ast, id: NodeId, call: &Call) {
        let _ = id;
        walk_call(ast, self, call);
    }

    fn visit_bracketed(&mut self, ast: &Ast, id: NodeId, bracketed: &Bracketed) {
        let _ = id;
        walk_bracketed(ast, self, bracketed);
    }

    fn visit_field(&mut self, ast: &Ast, id: NodeId, field: &Field) {
        let _ = id;
        walk_field(ast, self, field);
    }

    fn visit_scope_access(&mut self, ast: &Ast, id: NodeId, scope_access: &ScopeAccess) {
        let _ = id;
        walk_scope_access(ast, self, scope_access);
    }

    fn visit_type_modifier(&mut self, ast: &Ast, id: NodeId, type_modifier: &TypeModifier) {
        let _ = id;
        walk_type_modifier(ast, self, type_modifier);
    }

    fn visit_vector_size_suffix(&mut self, ast: &Ast, id: NodeId, suffix: &VectorSizeSuffix) {
        let _ = id;
        walk_vector_size_suffix(ast, self, suffix);
    }

    fn visit_processor_property(&mut self, _ast: &Ast, _id: NodeId, _property: &ProcessorProperty) {
    }

    fn visit_attribute_list(&mut self, ast: &Ast, id: NodeId, attribute_list: &AttributeList) {
        let _ = id;
        walk_attribute_list(ast, self, attribute_list);
    }

    fn visit_error(&mut self, _ast: &Ast, _id: NodeId, _token: TokenId) {}
}

pub trait ExhaustiveVisitor {
    type Output;

    fn visit_item(&mut self, ast: &Ast, item: &Item) -> Self::Output;
    fn visit_decl(&mut self, ast: &Ast, decl: &Decl) -> Self::Output;
    fn visit_graph(&mut self, ast: &Ast, graph: &Graph) -> Self::Output;
    fn visit_stmt(&mut self, ast: &Ast, stmt: &Stmt) -> Self::Output;
    fn visit_expr(&mut self, ast: &Ast, expr: &Expr) -> Self::Output;
    fn visit_attribute_list(&mut self, ast: &Ast, attribute_list: &AttributeList) -> Self::Output;
    fn visit_error(&mut self, ast: &Ast, token: TokenId) -> Self::Output;

    fn visit(&mut self, ast: &Ast, id: NodeId) -> Self::Output {
        match ast.get(id) {
            Node::Item(item) => self.visit_item(ast, item),
            Node::Decl(decl) => self.visit_decl(ast, decl),
            Node::Graph(graph) => self.visit_graph(ast, graph),
            Node::Stmt(stmt) => self.visit_stmt(ast, stmt),
            Node::Expr(expr) => self.visit_expr(ast, expr),
            Node::AttributeList(attribute_list) => self.visit_attribute_list(ast, attribute_list),
            Node::Error { token } => self.visit_error(ast, *token),
        }
    }
}

pub fn walk<V>(ast: &Ast, visitor: &mut V, id: NodeId)
where
    V: Visitor + ?Sized,
{
    match ast.get(id).clone() {
        Node::Item(item) => visitor.visit_item(ast, id, &item),
        Node::Decl(decl) => visitor.visit_decl(ast, id, &decl),
        Node::Graph(graph) => visitor.visit_graph(ast, id, &graph),
        Node::Stmt(stmt) => visitor.visit_stmt(ast, id, &stmt),
        Node::Expr(expr) => visitor.visit_expr(ast, id, &expr),
        Node::AttributeList(attribute_list) => {
            visitor.visit_attribute_list(ast, id, &attribute_list)
        }
        Node::Error { token } => visitor.visit_error(ast, id, token),
    }
}

pub fn walk_attribute_list<V>(ast: &Ast, visitor: &mut V, attribute_list: &AttributeList)
where
    V: Visitor + ?Sized,
{
    for (_, value) in &attribute_list.attributes {
        if let Some(value) = value {
            visitor.visit(ast, *value);
        }
    }
}

pub fn walk_item<V>(ast: &Ast, visitor: &mut V, id: NodeId, item: &Item)
where
    V: Visitor + ?Sized,
{
    match item {
        Item::NamespaceDecl(namespace_decl) => {
            visitor.visit_namespace_decl(ast, id, namespace_decl)
        }
        Item::ProcessorDecl(processor_decl) => {
            visitor.visit_processor_decl(ast, id, processor_decl)
        }
        Item::GraphDecl(graph_decl) => visitor.visit_graph_decl(ast, id, graph_decl),
        Item::StructDecl(struct_decl) => visitor.visit_struct_decl(ast, id, struct_decl),
        Item::EnumDecl(enum_decl) => visitor.visit_enum_decl(ast, id, enum_decl),
        Item::FunctionDecl(function_decl) => visitor.visit_function_decl(ast, id, function_decl),
        Item::Import(import) => visitor.visit_import(ast, id, import),
        Item::ModuleAlias(module_alias) => visitor.visit_module_alias(ast, id, module_alias),
    }
}

pub fn walk_namespace_decl<V>(ast: &Ast, visitor: &mut V, namespace_decl: &NamespaceDecl)
where
    V: Visitor + ?Sized,
{
    for &param in &namespace_decl.params {
        visitor.visit(ast, param);
    }
    if let Some(attributes) = namespace_decl.attributes {
        visitor.visit(ast, attributes);
    }
    for &member in &namespace_decl.items {
        visitor.visit(ast, member);
    }
}

pub fn walk_processor_decl<V>(ast: &Ast, visitor: &mut V, processor_decl: &ProcessorDecl)
where
    V: Visitor + ?Sized,
{
    for &param in &processor_decl.params {
        visitor.visit(ast, param);
    }
    if let Some(attributes) = processor_decl.attributes {
        visitor.visit(ast, attributes);
    }
    for &member in &processor_decl.items {
        visitor.visit(ast, member);
    }
}

pub fn walk_graph_decl<V>(ast: &Ast, visitor: &mut V, graph_decl: &GraphDecl)
where
    V: Visitor + ?Sized,
{
    for &param in &graph_decl.params {
        visitor.visit(ast, param);
    }
    if let Some(attributes) = graph_decl.attributes {
        visitor.visit(ast, attributes);
    }
    for &member in &graph_decl.items {
        visitor.visit(ast, member);
    }
}

pub fn walk_struct_decl<V>(ast: &Ast, visitor: &mut V, struct_decl: &StructDecl)
where
    V: Visitor + ?Sized,
{
    if let Some(attributes) = struct_decl.attributes {
        visitor.visit(ast, attributes);
    }
    for &member in &struct_decl.items {
        visitor.visit(ast, member);
    }
}

pub fn walk_function_decl<V>(ast: &Ast, visitor: &mut V, function_decl: &FunctionDecl)
where
    V: Visitor + ?Sized,
{
    if let Some(ty) = function_decl.ty {
        visitor.visit(ast, ty);
    }
    for &param in &function_decl.params {
        visitor.visit(ast, param);
    }
    if let Some(attributes) = function_decl.attributes {
        visitor.visit(ast, attributes);
    }
    visitor.visit(ast, function_decl.body);
}

pub fn walk_module_alias<V>(ast: &Ast, visitor: &mut V, module_alias: &ModuleAlias)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, module_alias.target);
}

pub fn walk_decl<V>(ast: &Ast, visitor: &mut V, id: NodeId, decl: &Decl)
where
    V: Visitor + ?Sized,
{
    match decl {
        Decl::Var(var) => visitor.visit_var(ast, id, var),
        Decl::Alias(alias) => visitor.visit_alias(ast, id, alias),
    }
}

pub fn walk_alias<V>(ast: &Ast, visitor: &mut V, alias: &Alias)
where
    V: Visitor + ?Sized,
{
    if let Some(target) = alias.target {
        visitor.visit(ast, target);
    }
}

pub fn walk_var<V>(ast: &Ast, visitor: &mut V, var: &Var)
where
    V: Visitor + ?Sized,
{
    let Var {
        ty,
        declarators,
        attributes,
        ..
    } = var;

    if let Some(ty) = ty {
        visitor.visit(ast, *ty);
    }
    for declarator in declarators {
        if let Some(init) = declarator.init {
            visitor.visit(ast, init);
        }
    }
    if let Some(attributes) = attributes {
        visitor.visit(ast, *attributes);
    }
}

pub fn walk_graph<V>(ast: &Ast, visitor: &mut V, id: NodeId, graph: &Graph)
where
    V: Visitor + ?Sized,
{
    match graph {
        Graph::EndpointDecl(endpoint_decl) => visitor.visit_endpoint_decl(ast, id, endpoint_decl),
        Graph::NodeDecl(node_decl) => visitor.visit_node_decl(ast, id, node_decl),
        Graph::ConnectionDecl(connection_decl) => {
            visitor.visit_connection_decl(ast, id, connection_decl)
        }
        Graph::Connection(connection) => visitor.visit_connection(ast, id, connection),
        Graph::ConnectionIf(connection_if) => visitor.visit_connection_if(ast, id, connection_if),
    }
}

pub fn walk_endpoint_decl<V>(ast: &Ast, visitor: &mut V, endpoint_decl: &EndpointDecl)
where
    V: Visitor + ?Sized,
{
    for &ty in &endpoint_decl.types {
        visitor.visit(ast, ty);
    }
    if let Some(size) = endpoint_decl.size {
        visitor.visit(ast, size);
    }
    if let Some(index) = endpoint_decl
        .hoisted
        .as_ref()
        .and_then(|hoisted| hoisted.index)
    {
        visitor.visit(ast, index);
    }
    if let Some(attributes) = endpoint_decl.attributes {
        visitor.visit(ast, attributes);
    }
}

pub fn walk_node_decl<V>(ast: &Ast, visitor: &mut V, node_decl: &NodeDecl)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, node_decl.processor);
    if let Some(array_size) = node_decl.array_size {
        visitor.visit(ast, array_size);
    }
}

pub fn walk_connection_decl<V>(ast: &Ast, visitor: &mut V, connection_decl: &ConnectionDecl)
where
    V: Visitor + ?Sized,
{
    for &connection in &connection_decl.connections {
        visitor.visit(ast, connection);
    }
}

pub fn walk_connection<V>(ast: &Ast, visitor: &mut V, connection: &Connection)
where
    V: Visitor + ?Sized,
{
    for &source in &connection.sources {
        visitor.visit(ast, source);
    }
    if let Some(delay) = connection.delay {
        visitor.visit(ast, delay);
    }
    for &destination in &connection.destinations {
        visitor.visit(ast, destination);
    }
}

pub fn walk_connection_if<V>(ast: &Ast, visitor: &mut V, connection_if: &ConnectionIf)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, connection_if.cond);
    for &then_branch in &connection_if.then_branch {
        visitor.visit(ast, then_branch);
    }
    if let Some(else_branch) = &connection_if.else_branch {
        for &else_branch in else_branch {
            visitor.visit(ast, else_branch);
        }
    }
}

pub fn walk_stmt<V>(ast: &Ast, visitor: &mut V, id: NodeId, stmt: &Stmt)
where
    V: Visitor + ?Sized,
{
    match stmt {
        Stmt::Block(block) => visitor.visit_block(ast, id, block),
        Stmt::ExprStmt(expr_stmt) => visitor.visit_expr_stmt(ast, id, expr_stmt),
        Stmt::DeclStmt(decl_stmt) => visitor.visit_decl_stmt(ast, id, decl_stmt),
        Stmt::ForStmt(for_stmt) => visitor.visit_for_stmt(ast, id, for_stmt),
        Stmt::IfStmt(if_stmt) => visitor.visit_if_stmt(ast, id, if_stmt),
        Stmt::WhileStmt(while_stmt) => visitor.visit_while_stmt(ast, id, while_stmt),
        Stmt::LoopStmt(loop_stmt) => visitor.visit_loop_stmt(ast, id, loop_stmt),
        Stmt::ReturnStmt(return_stmt) => visitor.visit_return_stmt(ast, id, return_stmt),
        Stmt::BreakStmt(break_stmt) => visitor.visit_break_stmt(ast, id, break_stmt),
        Stmt::ContinueStmt(continue_stmt) => visitor.visit_continue_stmt(ast, id, continue_stmt),
        Stmt::ForwardBranchStmt(forward_branch_stmt) => {
            visitor.visit_forward_branch_stmt(ast, id, forward_branch_stmt)
        }
    }
}

pub fn walk_block<V>(ast: &Ast, visitor: &mut V, block: &Block)
where
    V: Visitor + ?Sized,
{
    for &stmt in &block.stmts {
        visitor.visit(ast, stmt);
    }
}

pub fn walk_expr_stmt<V>(ast: &Ast, visitor: &mut V, expr_stmt: &ExprStmt)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, expr_stmt.expr);
}

pub fn walk_decl_stmt<V>(ast: &Ast, visitor: &mut V, decl_stmt: &DeclStmt)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, decl_stmt.decl);
}

pub fn walk_for_stmt<V>(ast: &Ast, visitor: &mut V, for_stmt: &ForStmt)
where
    V: Visitor + ?Sized,
{
    if let Some(cond) = for_stmt.cond {
        visitor.visit(ast, cond);
    }
    if let Some(update) = for_stmt.update {
        visitor.visit(ast, update);
    }
    visitor.visit(ast, for_stmt.body);
}

pub fn walk_if_stmt<V>(ast: &Ast, visitor: &mut V, if_stmt: &IfStmt)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, if_stmt.cond);
    visitor.visit(ast, if_stmt.then_branch);
    if let Some(else_branch) = if_stmt.else_branch {
        visitor.visit(ast, else_branch);
    }
}

pub fn walk_while_stmt<V>(ast: &Ast, visitor: &mut V, while_stmt: &WhileStmt)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, while_stmt.cond);
    visitor.visit(ast, while_stmt.body);
}

pub fn walk_loop_stmt<V>(ast: &Ast, visitor: &mut V, loop_stmt: &LoopStmt)
where
    V: Visitor + ?Sized,
{
    if let Some(count) = loop_stmt.count {
        visitor.visit(ast, count);
    }
    visitor.visit(ast, loop_stmt.body);
}

pub fn walk_return_stmt<V>(ast: &Ast, visitor: &mut V, return_stmt: &ReturnStmt)
where
    V: Visitor + ?Sized,
{
    if let Some(value) = return_stmt.value {
        visitor.visit(ast, value);
    }
}

pub fn walk_forward_branch_stmt<V>(
    ast: &Ast,
    visitor: &mut V,
    forward_branch_stmt: &ForwardBranchStmt,
) where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, forward_branch_stmt.cond);
}

pub fn walk_expr<V>(ast: &Ast, visitor: &mut V, id: NodeId, expr: &Expr)
where
    V: Visitor + ?Sized,
{
    match expr {
        Expr::Literal(_) => {}
        &Expr::Ident(Ident { token }) => visitor.visit_ident(ast, id, token),
        Expr::Parentheses(parentheses) => visitor.visit_parentheses(ast, id, parentheses),
        Expr::Unary(unary) => visitor.visit_unary(ast, id, unary),
        Expr::PostfixUnary(postfix_unary) => visitor.visit_postfix_unary(ast, id, postfix_unary),
        Expr::Binary(binary) => visitor.visit_binary(ast, id, binary),
        Expr::Assign(assign) => visitor.visit_assign(ast, id, assign),
        Expr::Ternary(ternary) => visitor.visit_ternary(ast, id, ternary),
        Expr::Call(call) => visitor.visit_call(ast, id, call),
        Expr::Bracketed(bracketed) => visitor.visit_bracketed(ast, id, bracketed),
        Expr::Field(field) => visitor.visit_field(ast, id, field),
        Expr::ScopeAccess(scope_access) => visitor.visit_scope_access(ast, id, scope_access),
        Expr::TypeModifier(type_modifier) => visitor.visit_type_modifier(ast, id, type_modifier),
        Expr::VectorSizeSuffix(suffix) => visitor.visit_vector_size_suffix(ast, id, suffix),
        Expr::ProcessorProperty(property) => visitor.visit_processor_property(ast, id, property),
    }
}

pub fn walk_parentheses<V>(ast: &Ast, visitor: &mut V, parentheses: &Parentheses)
where
    V: Visitor + ?Sized,
{
    for &expr in &parentheses.inner {
        visitor.visit(ast, expr);
    }
}

pub fn walk_unary<V>(ast: &Ast, visitor: &mut V, unary: &Unary)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, unary.operand);
}

pub fn walk_postfix_unary<V>(ast: &Ast, visitor: &mut V, postfix_unary: &PostfixUnary)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, postfix_unary.operand);
}

pub fn walk_binary<V>(ast: &Ast, visitor: &mut V, binary: &Binary)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, binary.lhs);
    visitor.visit(ast, binary.rhs);
}

pub fn walk_assign<V>(ast: &Ast, visitor: &mut V, assign: &Assign)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, assign.target);
    visitor.visit(ast, assign.value);
}

pub fn walk_ternary<V>(ast: &Ast, visitor: &mut V, ternary: &Ternary)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, ternary.cond);
    visitor.visit(ast, ternary.then_branch);
    visitor.visit(ast, ternary.else_branch);
}

pub fn walk_call<V>(ast: &Ast, visitor: &mut V, call: &Call)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, call.callee);
    for &arg in &call.args {
        visitor.visit(ast, arg);
    }
}

pub fn walk_bracketed<V>(ast: &Ast, visitor: &mut V, bracketed: &Bracketed)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, bracketed.base);
    for term in &bracketed.terms {
        if let Some(start) = term.start {
            visitor.visit(ast, start);
        }
        if let Some(end) = term.end {
            visitor.visit(ast, end);
        }
    }
}

pub fn walk_field<V>(ast: &Ast, visitor: &mut V, field: &Field)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, field.base);
}

pub fn walk_scope_access<V>(ast: &Ast, visitor: &mut V, scope_access: &ScopeAccess)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, scope_access.base);
}

pub fn walk_type_modifier<V>(ast: &Ast, visitor: &mut V, type_modifier: &TypeModifier)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, type_modifier.source);
}

pub fn walk_vector_size_suffix<V>(ast: &Ast, visitor: &mut V, suffix: &VectorSizeSuffix)
where
    V: Visitor + ?Sized,
{
    visitor.visit(ast, suffix.element);
    for &term in &suffix.terms {
        visitor.visit(ast, term);
    }
}
