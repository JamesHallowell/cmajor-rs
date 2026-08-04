use crate::{
    ast::{
        Ast, AttributeList, Decl, Expr, Graph, Item, Node, NodeId, Stmt,
        decl::{Alias, Var},
        expr::{
            Assign, Binary, Bracketed, Call, Field, Ident, Parentheses, PostfixUnary,
            ProcessorProperty, ScopeAccess, Slice, Ternary, TypeModifier, Unary,
            VectorSizeSuffix,
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

pub trait Visitor {
    fn visit(&mut self, ast: &Ast, id: NodeId) {
        walk(ast, self, id);
    }

    fn visit_root(&mut self, ast: &Ast, id: NodeId) {
        assert!(ast.roots().contains(&id));
        walk(ast, self, id);
    }

    fn visit_item(&mut self, ast: &Ast, id: NodeId, item: &Item) {
        walk_item(ast, self, id, item);
    }

    fn visit_namespace_decl(&mut self, ast: &Ast, id: NodeId, namespace_decl: &NamespaceDecl) {
        let _ = id;
        namespace_decl.walk(ast, self);
    }

    fn visit_processor_decl(&mut self, ast: &Ast, id: NodeId, processor_decl: &ProcessorDecl) {
        let _ = id;
        processor_decl.walk(ast, self);
    }

    fn visit_graph_decl(&mut self, ast: &Ast, id: NodeId, graph_decl: &GraphDecl) {
        let _ = id;
        graph_decl.walk(ast, self);
    }

    fn visit_struct_decl(&mut self, ast: &Ast, id: NodeId, struct_decl: &StructDecl) {
        let _ = id;
        struct_decl.walk(ast, self);
    }

    fn visit_enum_decl(&mut self, _ast: &Ast, _id: NodeId, _enum_decl: &EnumDecl) {}

    fn visit_function_decl(&mut self, ast: &Ast, id: NodeId, function_decl: &FunctionDecl) {
        let _ = id;
        function_decl.walk(ast, self);
    }

    fn visit_import(&mut self, _ast: &Ast, _id: NodeId, _import: &Import) {}

    fn visit_module_alias(&mut self, ast: &Ast, id: NodeId, module_alias: &ModuleAlias) {
        let _ = id;
        module_alias.walk(ast, self);
    }

    fn visit_decl(&mut self, ast: &Ast, id: NodeId, decl: &Decl) {
        walk_decl(ast, self, id, decl);
    }

    fn visit_var(&mut self, ast: &Ast, id: NodeId, var: &Var) {
        let _ = id;
        var.walk(ast, self);
    }

    fn visit_alias(&mut self, ast: &Ast, id: NodeId, alias: &Alias) {
        let _ = id;
        alias.walk(ast, self);
    }

    fn visit_graph(&mut self, ast: &Ast, id: NodeId, graph: &Graph) {
        walk_graph(ast, self, id, graph);
    }

    fn visit_endpoint_decl(&mut self, ast: &Ast, id: NodeId, endpoint_decl: &EndpointDecl) {
        let _ = id;
        endpoint_decl.walk(ast, self);
    }

    fn visit_node_decl(&mut self, ast: &Ast, id: NodeId, node_decl: &NodeDecl) {
        let _ = id;
        node_decl.walk(ast, self);
    }

    fn visit_connection_decl(&mut self, ast: &Ast, id: NodeId, connection_decl: &ConnectionDecl) {
        let _ = id;
        connection_decl.walk(ast, self);
    }

    fn visit_connection(&mut self, ast: &Ast, id: NodeId, connection: &Connection) {
        let _ = id;
        connection.walk(ast, self);
    }

    fn visit_connection_if(&mut self, ast: &Ast, id: NodeId, connection_if: &ConnectionIf) {
        let _ = id;
        connection_if.walk(ast, self);
    }

    fn visit_stmt(&mut self, ast: &Ast, id: NodeId, stmt: &Stmt) {
        walk_stmt(ast, self, id, stmt);
    }

    fn visit_block(&mut self, ast: &Ast, id: NodeId, block: &Block) {
        let _ = id;
        block.walk(ast, self);
    }

    fn visit_expr_stmt(&mut self, ast: &Ast, id: NodeId, expr_stmt: &ExprStmt) {
        let _ = id;
        expr_stmt.walk(ast, self);
    }

    fn visit_decl_stmt(&mut self, ast: &Ast, id: NodeId, decl_stmt: &DeclStmt) {
        let _ = id;
        decl_stmt.walk(ast, self);
    }

    fn visit_for_stmt(&mut self, ast: &Ast, id: NodeId, for_stmt: &ForStmt) {
        let _ = id;
        for_stmt.walk(ast, self);
    }

    fn visit_if_stmt(&mut self, ast: &Ast, id: NodeId, if_stmt: &IfStmt) {
        let _ = id;
        if_stmt.walk(ast, self);
    }

    fn visit_while_stmt(&mut self, ast: &Ast, id: NodeId, while_stmt: &WhileStmt) {
        let _ = id;
        while_stmt.walk(ast, self);
    }

    fn visit_loop_stmt(&mut self, ast: &Ast, id: NodeId, loop_stmt: &LoopStmt) {
        let _ = id;
        loop_stmt.walk(ast, self);
    }

    fn visit_return_stmt(&mut self, ast: &Ast, id: NodeId, return_stmt: &ReturnStmt) {
        let _ = id;
        return_stmt.walk(ast, self);
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
        forward_branch_stmt.walk(ast, self);
    }

    fn visit_expr(&mut self, ast: &Ast, id: NodeId, expr: &Expr) {
        walk_expr(ast, self, id, expr);
    }

    fn visit_ident(&mut self, _ast: &Ast, _id: NodeId, _token: TokenId) {}

    fn visit_parentheses(&mut self, ast: &Ast, id: NodeId, parentheses: &Parentheses) {
        let _ = id;
        parentheses.walk(ast, self);
    }

    fn visit_unary(&mut self, ast: &Ast, id: NodeId, unary: &Unary) {
        let _ = id;
        unary.walk(ast, self);
    }

    fn visit_postfix_unary(&mut self, ast: &Ast, id: NodeId, postfix_unary: &PostfixUnary) {
        let _ = id;
        postfix_unary.walk(ast, self);
    }

    fn visit_binary(&mut self, ast: &Ast, id: NodeId, binary: &Binary) {
        let _ = id;
        binary.walk(ast, self);
    }

    fn visit_assign(&mut self, ast: &Ast, id: NodeId, assign: &Assign) {
        let _ = id;
        assign.walk(ast, self);
    }

    fn visit_ternary(&mut self, ast: &Ast, id: NodeId, ternary: &Ternary) {
        let _ = id;
        ternary.walk(ast, self);
    }

    fn visit_call(&mut self, ast: &Ast, id: NodeId, call: &Call) {
        let _ = id;
        call.walk(ast, self);
    }

    fn visit_bracketed(&mut self, ast: &Ast, id: NodeId, bracketed: &Bracketed) {
        let _ = id;
        bracketed.walk(ast, self);
    }

    fn visit_slice(&mut self, ast: &Ast, id: NodeId, slice: &Slice) {
        let _ = id;
        slice.walk(ast, self);
    }

    fn visit_field(&mut self, ast: &Ast, id: NodeId, field: &Field) {
        let _ = id;
        field.walk(ast, self);
    }

    fn visit_scope_access(&mut self, ast: &Ast, id: NodeId, scope_access: &ScopeAccess) {
        let _ = id;
        scope_access.walk(ast, self);
    }

    fn visit_type_modifier(&mut self, ast: &Ast, id: NodeId, type_modifier: &TypeModifier) {
        let _ = id;
        type_modifier.walk(ast, self);
    }

    fn visit_vector_size_suffix(&mut self, ast: &Ast, id: NodeId, suffix: &VectorSizeSuffix) {
        let _ = id;
        suffix.walk(ast, self);
    }

    fn visit_processor_property(&mut self, _ast: &Ast, _id: NodeId, _property: &ProcessorProperty) {
    }

    fn visit_attribute_list(&mut self, ast: &Ast, id: NodeId, attribute_list: &AttributeList) {
        let _ = id;
        attribute_list.walk(ast, self);
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

pub trait Walk<V>
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V);
}

pub fn walk<V>(ast: &Ast, visitor: &mut V, id: NodeId)
where
    V: Visitor + ?Sized,
{
    match ast.get(id) {
        Node::Item(item) => visitor.visit_item(ast, id, item),
        Node::Decl(decl) => visitor.visit_decl(ast, id, decl),
        Node::Graph(graph) => visitor.visit_graph(ast, id, graph),
        Node::Stmt(stmt) => visitor.visit_stmt(ast, id, stmt),
        Node::Expr(expr) => visitor.visit_expr(ast, id, expr),
        Node::AttributeList(attribute_list) => {
            visitor.visit_attribute_list(ast, id, attribute_list)
        }
        Node::Error { token } => visitor.visit_error(ast, id, *token),
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

pub fn walk_decl<V>(ast: &Ast, visitor: &mut V, id: NodeId, decl: &Decl)
where
    V: Visitor + ?Sized,
{
    match decl {
        Decl::Var(var) => visitor.visit_var(ast, id, var),
        Decl::Alias(alias) => visitor.visit_alias(ast, id, alias),
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
        Expr::Slice(slice) => visitor.visit_slice(ast, id, slice),
        Expr::Field(field) => visitor.visit_field(ast, id, field),
        Expr::ScopeAccess(scope_access) => visitor.visit_scope_access(ast, id, scope_access),
        Expr::TypeModifier(type_modifier) => visitor.visit_type_modifier(ast, id, type_modifier),
        Expr::VectorSizeSuffix(suffix) => visitor.visit_vector_size_suffix(ast, id, suffix),
        Expr::ProcessorProperty(property) => visitor.visit_processor_property(ast, id, property),
    }
}

impl<V> Walk<V> for AttributeList
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        for attribute in &self.attributes {
            if let Some(value) = attribute.value {
                visitor.visit(ast, value);
            }
        }
    }
}

impl<V> Walk<V> for NamespaceDecl
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        for &param in &self.params {
            visitor.visit(ast, param);
        }
        if let Some(attributes) = self.attributes {
            visitor.visit(ast, attributes);
        }
        for &member in &self.items {
            visitor.visit(ast, member);
        }
    }
}

impl<V> Walk<V> for ProcessorDecl
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        for &param in &self.params {
            visitor.visit(ast, param);
        }
        if let Some(attributes) = self.attributes {
            visitor.visit(ast, attributes);
        }
        for &member in &self.items {
            visitor.visit(ast, member);
        }
    }
}

impl<V> Walk<V> for GraphDecl
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        for &param in &self.params {
            visitor.visit(ast, param);
        }
        if let Some(attributes) = self.attributes {
            visitor.visit(ast, attributes);
        }
        for &member in &self.items {
            visitor.visit(ast, member);
        }
    }
}

impl<V> Walk<V> for StructDecl
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        if let Some(attributes) = self.attributes {
            visitor.visit(ast, attributes);
        }
        for &member in &self.items {
            visitor.visit(ast, member);
        }
    }
}

impl<V> Walk<V> for FunctionDecl
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        if let Some(ty) = self.ty {
            visitor.visit(ast, ty);
        }
        for &param in &self.params {
            visitor.visit(ast, param);
        }
        if let Some(attributes) = self.attributes {
            visitor.visit(ast, attributes);
        }
        visitor.visit(ast, self.body);
    }
}

impl<V> Walk<V> for ModuleAlias
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.target);
    }
}

impl<V> Walk<V> for Alias
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        if let Some(target) = self.target {
            visitor.visit(ast, target);
        }
    }
}

impl<V> Walk<V> for Var
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        let Var {
            ty,
            declarators,
            attributes,
            ..
        } = self;

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
}

impl<V> Walk<V> for EndpointDecl
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        for &ty in &self.types {
            visitor.visit(ast, ty);
        }
        if let Some(size) = self.size {
            visitor.visit(ast, size);
        }
        if let Some(index) = self.hoisted.as_ref().and_then(|hoisted| hoisted.index) {
            visitor.visit(ast, index);
        }
        if let Some(attributes) = self.attributes {
            visitor.visit(ast, attributes);
        }
    }
}

impl<V> Walk<V> for NodeDecl
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.processor);
        if let Some(array_size) = self.array_size {
            visitor.visit(ast, array_size);
        }
    }
}

impl<V> Walk<V> for ConnectionDecl
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        for &connection in &self.connections {
            visitor.visit(ast, connection);
        }
    }
}

impl<V> Walk<V> for Connection
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        for &source in &self.sources {
            visitor.visit(ast, source);
        }
        if let Some(delay) = self.delay {
            visitor.visit(ast, delay);
        }
        for &destination in &self.destinations {
            visitor.visit(ast, destination);
        }
    }
}

impl<V> Walk<V> for ConnectionIf
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.cond);
        for &then_branch in &self.then_branch {
            visitor.visit(ast, then_branch);
        }
        if let Some(else_branch) = &self.else_branch {
            for &else_branch in else_branch {
                visitor.visit(ast, else_branch);
            }
        }
    }
}

impl<V> Walk<V> for Block
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        for &stmt in &self.stmts {
            visitor.visit(ast, stmt);
        }
    }
}

impl<V> Walk<V> for ExprStmt
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.expr);
    }
}

impl<V> Walk<V> for DeclStmt
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.decl);
    }
}

impl<V> Walk<V> for ForStmt
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        if let Some(cond) = self.cond {
            visitor.visit(ast, cond);
        }
        if let Some(update) = self.update {
            visitor.visit(ast, update);
        }
        visitor.visit(ast, self.body);
    }
}

impl<V> Walk<V> for IfStmt
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.cond);
        visitor.visit(ast, self.then_branch);
        if let Some(else_branch) = self.else_branch {
            visitor.visit(ast, else_branch);
        }
    }
}

impl<V> Walk<V> for WhileStmt
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.cond);
        visitor.visit(ast, self.body);
    }
}

impl<V> Walk<V> for LoopStmt
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        if let Some(count) = self.count {
            visitor.visit(ast, count);
        }
        visitor.visit(ast, self.body);
    }
}

impl<V> Walk<V> for ReturnStmt
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        if let Some(value) = self.value {
            visitor.visit(ast, value);
        }
    }
}

impl<V> Walk<V> for ForwardBranchStmt
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.cond);
    }
}

impl<V> Walk<V> for Parentheses
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        if let Some(children) = self.inner {
            for &child in ast.children(children) {
                visitor.visit(ast, child);
            }
        }
    }
}

impl<V> Walk<V> for Unary
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.operand);
    }
}

impl<V> Walk<V> for PostfixUnary
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.operand);
    }
}

impl<V> Walk<V> for Binary
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.lhs);
        visitor.visit(ast, self.rhs);
    }
}

impl<V> Walk<V> for Assign
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.target);
        visitor.visit(ast, self.value);
    }
}

impl<V> Walk<V> for Ternary
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.cond);
        visitor.visit(ast, self.then_branch);
        visitor.visit(ast, self.else_branch);
    }
}

impl<V> Walk<V> for Call
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.callee);
        for &arg in &self.args {
            visitor.visit(ast, arg);
        }
    }
}

impl<V> Walk<V> for Bracketed
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.base);
        if let Some(terms) = self.terms {
            for &term in ast.children(terms) {
                visitor.visit(ast, term);
            }
        }
    }
}

impl<V> Walk<V> for Slice
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        if let Some(start) = self.start {
            visitor.visit(ast, start);
        }
        if let Some(end) = self.end {
            visitor.visit(ast, end);
        }
    }
}

impl<V> Walk<V> for Field
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.base);
    }
}

impl<V> Walk<V> for ScopeAccess
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.base);
    }
}

impl<V> Walk<V> for TypeModifier
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.source);
    }
}

impl<V> Walk<V> for VectorSizeSuffix
where
    V: Visitor + ?Sized,
{
    fn walk(&self, ast: &Ast, visitor: &mut V) {
        visitor.visit(ast, self.element);
        for &term in &self.terms {
            visitor.visit(ast, term);
        }
    }
}
