mod scope;
mod symbol;
pub mod unit;
#[cfg(test)]
mod view;

pub use {
    scope::{Scope, ScopeId},
    symbol::{Symbol, SymbolId, SymbolKind, SymbolOrigin, SymbolTable},
    unit::UnitId,
};

use crate::{
    ast::{
        Alias, Ast, Block, EndpointDeclaration, EnumDecl, EnumValue, EventHandlerDecl, Expr,
        ForStmt, FunctionDecl, GraphDecl, HoistedEndpointDeclaration, Ident, IfStmt, LoopStmt,
        ModuleAlias, NamespaceDecl, Node, NodeDecl, NodeId, Param, ProcessorDecl, ScopeAccess,
        SpecialisationValue, Stmt, StructDecl, TypedDecl, Var, WhileStmt,
        visit::{Visitor, Walk},
    },
    lexer::{TokenId, TokenStream},
    resolver::unit::Anchor,
    utils::{
        arena::{Arena, SecondaryArena, SparseSecondaryArena},
        source::{Source, SourceLocation},
        span::Span,
    },
};

pub struct Resolution {
    pub symbols: SymbolTable,
    pub diagnostics: Vec<ResolvedDiagnostic>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ResolvedDiagnostic {
    pub unit: UnitId,
    pub location: Span<SourceLocation>,
    pub message: String,
}

impl std::fmt::Display for ResolvedDiagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}: {}", self.location.start, self.message)
    }
}

#[derive(Clone, Copy)]
pub struct Unit<'a> {
    pub name: &'a str,
    pub source: &'a str,
    pub tokens: &'a TokenStream,
    pub ast: &'a Ast,
}

pub fn resolve(source: &str, tokens: &TokenStream, ast: &Ast) -> Resolution {
    resolve_all(&[Unit {
        name: "",
        source,
        tokens,
        ast,
    }])
}

pub fn resolve_all(units: &[Unit<'_>]) -> Resolution {
    let mut state = State::new(units);
    let global_scope = state.symbols.global_scope();
    let unit_ids: Vec<UnitId> = (&state.units).into_iter().map(|(id, _)| id).collect();

    for &unit_id in &unit_ids {
        state.current_unit = Some(unit_id);
        state.current_scope = global_scope;
        let ast = state.ast();
        ast.visit(&mut Declare { state: &mut state });
    }

    for &unit_id in &unit_ids {
        state.current_unit = Some(unit_id);
        state.current_scope = global_scope;
        let ast = state.ast();
        ast.visit(&mut Resolve { state: &mut state });
    }

    Resolution {
        symbols: state.symbols,
        diagnostics: state.diagnostics,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("undeclared identifier '{name}'")]
    UndeclaredIdentifier { name: String },

    #[error("redefinition of '{name}' (previously declared at {})", location.map_or_else(|| "<builtin>".to_string(), |location| location.to_string()))]
    RedefinedIdentifier {
        name: String,
        location: Option<SourceLocation>,
    },

    #[error("unexpected node")]
    UnexpectedNode,

    #[error("'{name}' is not a namespace, processor, graph, struct, or enum")]
    NotAScope { name: String },
}

type Result<T> = std::result::Result<T, (Span<SourceLocation>, Error)>;

struct State<'a> {
    units: Arena<UnitId, Unit<'a>>,
    current_unit: Option<UnitId>,
    symbols: SymbolTable,
    diagnostics: Vec<ResolvedDiagnostic>,
    current_scope: ScopeId,
    symbols_scope: SparseSecondaryArena<SymbolId, ScopeId>,
    node_scope: SecondaryArena<UnitId, SparseSecondaryArena<NodeId, ScopeId>>,
    declared: SecondaryArena<UnitId, SparseSecondaryArena<NodeId, SymbolId>>,
}

impl<'a> State<'a> {
    pub fn new(units: &[Unit<'a>]) -> Self {
        let mut symbols = SymbolTable::new();
        let global_scope = symbols.global_scope();

        const BUILTINS: &[(&str, SymbolKind)] = &[
            ("void", SymbolKind::Primitive),
            ("bool", SymbolKind::Primitive),
            ("int", SymbolKind::Primitive),
            ("int32", SymbolKind::Primitive),
            ("int64", SymbolKind::Primitive),
            ("float", SymbolKind::Primitive),
            ("float32", SymbolKind::Primitive),
            ("float64", SymbolKind::Primitive),
            ("complex", SymbolKind::Primitive),
            ("complex32", SymbolKind::Primitive),
            ("complex64", SymbolKind::Primitive),
            ("wrap", SymbolKind::Primitive),
            ("clamp", SymbolKind::Primitive),
            ("abs", SymbolKind::Function),
            ("sqrt", SymbolKind::Function),
            ("pow", SymbolKind::Function),
            ("fmod", SymbolKind::Function),
            ("remainder", SymbolKind::Function),
            ("roundToInt", SymbolKind::Function),
            ("floor", SymbolKind::Function),
            ("ceil", SymbolKind::Function),
            ("rint", SymbolKind::Function),
            ("log10", SymbolKind::Function),
            ("log", SymbolKind::Function),
            ("exp", SymbolKind::Function),
            ("sin", SymbolKind::Function),
            ("sinh", SymbolKind::Function),
            ("asin", SymbolKind::Function),
            ("asinh", SymbolKind::Function),
            ("cos", SymbolKind::Function),
            ("cosh", SymbolKind::Function),
            ("acos", SymbolKind::Function),
            ("acosh", SymbolKind::Function),
            ("tan", SymbolKind::Function),
            ("tanh", SymbolKind::Function),
            ("atan", SymbolKind::Function),
            ("atanh", SymbolKind::Function),
            ("atan2", SymbolKind::Function),
            ("max", SymbolKind::Function),
            ("min", SymbolKind::Function),
            ("select", SymbolKind::Function),
            ("lerp", SymbolKind::Function),
            ("addModulo2Pi", SymbolKind::Function),
            ("nan", SymbolKind::Variable),
            ("inf", SymbolKind::Variable),
            ("pi", SymbolKind::Variable),
            ("twoPi", SymbolKind::Variable),
            ("static_assert", SymbolKind::Function),
        ];

        for &(name, kind) in BUILTINS {
            symbols.declare(SymbolOrigin::Builtin { name }, kind, global_scope);
        }

        let units: Arena<UnitId, Unit<'a>> = units.iter().copied().collect();

        let node_scope: SecondaryArena<UnitId, _> = units
            .into_iter()
            .map(|(id, _)| (id, SparseSecondaryArena::default()))
            .collect();
        let declared: SecondaryArena<UnitId, _> = units
            .into_iter()
            .map(|(id, _)| (id, SparseSecondaryArena::default()))
            .collect();

        let current_unit = if units.is_empty() {
            None
        } else {
            Some(units.first().0)
        };

        State {
            units,
            current_unit,
            symbols,
            diagnostics: Vec::new(),
            current_scope: global_scope,
            symbols_scope: SparseSecondaryArena::default(),
            node_scope,
            declared,
        }
    }

    fn current_unit(&self) -> UnitId {
        self.current_unit
            .expect("current_unit is set before any unit-scoped resolver operation runs")
    }

    fn ast(&self) -> &'a Ast {
        self.units[self.current_unit()].ast
    }

    fn source(&self) -> Source<'a> {
        Source::new(self.units[self.current_unit()].source)
    }

    fn anchor(&self, token: TokenId) -> Anchor {
        Anchor {
            unit: self.current_unit(),
            token,
        }
    }

    fn error(&mut self, location: Span<SourceLocation>, err: Error) {
        self.diagnostics.push(ResolvedDiagnostic {
            unit: self.current_unit(),
            location,
            message: err.to_string(),
        });
    }

    fn text_in(&self, unit: UnitId, token: TokenId) -> &str {
        let data = &self.units[unit];
        let span = data.tokens.span(token);
        &data.source[span.to_range()]
    }

    fn text(&self, token: TokenId) -> &str {
        self.text_in(self.current_unit(), token)
    }

    fn location_in(&self, unit: UnitId, token: TokenId) -> Span<SourceLocation> {
        let data = &self.units[unit];
        data.tokens
            .span(token)
            .to_source_location_span(&Source::new(data.source))
    }

    fn location(&self, token: TokenId) -> Span<SourceLocation> {
        self.location_in(self.current_unit(), token)
    }

    fn symbol_location(&self, symbol: SymbolId) -> Option<Span<SourceLocation>> {
        let symbol = self.symbols.symbol(symbol);
        match symbol.origin {
            SymbolOrigin::Builtin { name: _ } => None,
            SymbolOrigin::Source { unit, name, .. } => Some(self.location_in(unit, name)),
        }
    }

    fn has_matching_name(&self, name: TokenId) -> impl FnMut(&Symbol) -> bool {
        let query = self.text(name).to_string();
        move |symbol: &Symbol| match symbol.origin {
            SymbolOrigin::Builtin { name: symbol_name } => query == *symbol_name,
            SymbolOrigin::Source {
                unit,
                name: symbol_name,
                ..
            } => query == self.text_in(unit, symbol_name),
        }
    }

    fn declare(
        &mut self,
        scope: ScopeId,
        name: TokenId,
        kind: SymbolKind,
        node: NodeId,
    ) -> SymbolId {
        let unit = self.current_unit();

        if let Some(&symbol) = self.declared[unit].get(node) {
            return symbol;
        }

        if !kind.allows_duplicates()
            && let Some(existing) = self.symbols.find_local(scope, self.has_matching_name(name))
        {
            let existing_location = self
                .symbol_location(existing)
                .map(|location| location.start);

            self.error(
                self.location(name),
                Error::RedefinedIdentifier {
                    name: self.text(name).to_string(),
                    location: existing_location,
                },
            );
        }

        let symbol = self
            .symbols
            .declare(SymbolOrigin::Source { unit, name, node }, kind, scope);
        self.declared[unit].insert(node, symbol);
        symbol
    }
}

struct Declare<'s, 'a> {
    state: &'s mut State<'a>,
}

impl<'s, 'a> Declare<'s, 'a> {
    fn with_scope<R>(&mut self, inner: ScopeId, f: impl FnOnce(&mut Self) -> R) -> R {
        let parent = self.state.current_scope;
        self.state.current_scope = inner;
        let result = f(self);
        self.state.current_scope = parent;
        result
    }

    fn declare_container(
        &mut self,
        ast: &Ast,
        name: TokenId,
        kind: SymbolKind,
        node: NodeId,
        builtins: &[(&'static str, SymbolKind)],
        items: &[NodeId],
    ) -> SymbolId {
        let symbol = self
            .state
            .declare(self.state.current_scope, name, kind, node);

        let scope_start = self.state.anchor(self.state.ast().span(node).start);

        let container_scope = self
            .state
            .symbols
            .new_scope(self.state.current_scope, scope_start);
        self.state.symbols_scope.insert(symbol, container_scope);
        let unit = self.state.current_unit();
        self.state.node_scope[unit].insert(node, container_scope);

        self.with_scope(container_scope, |this| {
            for &(name, kind) in builtins {
                this.state.symbols.declare(
                    SymbolOrigin::Builtin { name },
                    kind,
                    this.state.current_scope,
                );
            }

            for &member in items {
                this.visit(ast, member);
            }
        });

        symbol
    }

    fn declare_or_reuse_namespace(
        &mut self,
        scope: ScopeId,
        name: TokenId,
        node: NodeId,
    ) -> ScopeId {
        if let Some(existing_id) = self
            .state
            .symbols
            .find_local(scope, self.state.has_matching_name(name))
        {
            let existing = self.state.symbols.symbol(existing_id);
            if existing.kind == SymbolKind::Namespace {
                return self
                    .state
                    .symbols_scope
                    .get(existing_id)
                    .copied()
                    .expect("namespace symbol always has an inner scope");
            }

            let error_location = self.state.location(name);

            self.state.error(
                error_location,
                Error::RedefinedIdentifier {
                    name: self.state.text(name).to_string(),
                    location: self
                        .state
                        .symbol_location(existing_id)
                        .map(|location| location.start),
                },
            );
        }

        let symbol = self.state.symbols.declare(
            SymbolOrigin::Source {
                unit: self.state.current_unit(),
                name,
                node,
            },
            SymbolKind::Namespace,
            scope,
        );
        let scope_start = self.state.anchor(self.state.ast().span(node).start);

        let inner = self.state.symbols.new_scope(scope, scope_start);
        self.state.symbols_scope.insert(symbol, inner);
        inner
    }
}

impl<'s, 'a> Visitor for Declare<'s, 'a> {
    fn visit_namespace_decl(&mut self, ast: &Ast, id: NodeId, namespace_decl: &NamespaceDecl) {
        let scope = self.state.current_scope;
        let inner = namespace_decl
            .segments
            .iter()
            .fold(scope, |scope, &segment| {
                self.declare_or_reuse_namespace(scope, segment, id)
            });
        let unit = self.state.current_unit();
        self.state.node_scope[unit].insert(id, inner);

        self.with_scope(inner, |this| {
            for &member in &namespace_decl.items {
                this.visit(ast, member);
            }
        });
    }

    fn visit_processor_decl(&mut self, ast: &Ast, id: NodeId, processor_decl: &ProcessorDecl) {
        self.declare_container(
            ast,
            processor_decl.name,
            SymbolKind::Processor,
            id,
            &[
                ("advance", SymbolKind::Function),
                ("processor", SymbolKind::Variable),
                ("console", SymbolKind::Endpoint),
            ],
            &processor_decl.items,
        );
    }

    fn visit_graph_decl(&mut self, ast: &Ast, id: NodeId, graph_decl: &GraphDecl) {
        self.declare_container(
            ast,
            graph_decl.name,
            SymbolKind::Graph,
            id,
            &[],
            &graph_decl.items,
        );
    }

    fn visit_struct_decl(&mut self, ast: &Ast, id: NodeId, struct_decl: &StructDecl) {
        self.declare_container(
            ast,
            struct_decl.name,
            SymbolKind::Struct,
            id,
            &[("this", SymbolKind::Variable)],
            &struct_decl.items,
        );
    }

    fn visit_enum_decl(&mut self, ast: &Ast, id: NodeId, enum_decl: &EnumDecl) {
        self.declare_container(
            ast,
            enum_decl.name,
            SymbolKind::Enum,
            id,
            &[],
            ast.children(enum_decl.values),
        );
    }

    fn visit_enum_value(&mut self, _ast: &Ast, id: NodeId, enum_value: &EnumValue) {
        self.state.declare(
            self.state.current_scope,
            enum_value.name,
            SymbolKind::EnumValue,
            id,
        );
    }

    fn visit_function_decl(&mut self, _ast: &Ast, id: NodeId, function_decl: &FunctionDecl) {
        self.state.declare(
            self.state.current_scope,
            function_decl.name,
            SymbolKind::Function,
            id,
        );
    }

    fn visit_event_handler_decl(
        &mut self,
        _ast: &Ast,
        _id: NodeId,
        _event_handler_decl: &EventHandlerDecl,
    ) {
    }

    fn visit_module_alias(&mut self, _ast: &Ast, id: NodeId, module_alias: &ModuleAlias) {
        self.state.declare(
            self.state.current_scope,
            module_alias.name,
            SymbolKind::Alias,
            id,
        );
    }

    fn visit_var(&mut self, ast: &Ast, _id: NodeId, var: &Var) {
        for (id, declarator) in var.declarators(ast) {
            self.state.declare(
                self.state.current_scope,
                declarator.name,
                SymbolKind::Variable,
                id,
            );
        }
    }

    fn visit_typed_decl(&mut self, ast: &Ast, _id: NodeId, typed_decl: &TypedDecl) {
        for (id, declarator) in typed_decl.declarators(ast) {
            self.state.declare(
                self.state.current_scope,
                declarator.name,
                SymbolKind::Variable,
                id,
            );
        }
    }

    fn visit_alias(&mut self, _ast: &Ast, id: NodeId, alias: &Alias) {
        self.state
            .declare(self.state.current_scope, alias.name, SymbolKind::Alias, id);
    }

    fn visit_endpoint_declaration(
        &mut self,
        _ast: &Ast,
        id: NodeId,
        endpoint_declaration: &EndpointDeclaration,
    ) {
        self.state.declare(
            self.state.current_scope,
            endpoint_declaration.name,
            SymbolKind::Endpoint,
            id,
        );
    }

    fn visit_hoisted_endpoint_declaration(
        &mut self,
        _ast: &Ast,
        id: NodeId,
        hoisted_endpoint_declaration: &HoistedEndpointDeclaration,
    ) {
        if let Some(name) = hoisted_endpoint_declaration.name {
            self.state
                .declare(self.state.current_scope, name, SymbolKind::Endpoint, id);
        }
    }

    fn visit_node_decl(&mut self, _ast: &Ast, id: NodeId, node_decl: &NodeDecl) {
        self.state.declare(
            self.state.current_scope,
            node_decl.name,
            SymbolKind::Node,
            id,
        );
    }
}

struct Resolve<'s, 'a> {
    state: &'s mut State<'a>,
}

impl<'s, 'a> Resolve<'s, 'a> {
    fn with_new_scope_at<R>(
        &mut self,
        anchor: TokenId,
        f: impl FnOnce(&mut Self, ScopeId) -> R,
    ) -> R {
        let anchor = self.state.anchor(anchor);
        let inner = self
            .state
            .symbols
            .new_scope(self.state.current_scope, anchor);
        self.with_scope(inner, |this| f(this, inner))
    }

    fn with_scope<R>(&mut self, inner: ScopeId, f: impl FnOnce(&mut Self) -> R) -> R {
        let parent = self.state.current_scope;
        self.state.current_scope = inner;
        let result = f(self);
        self.state.current_scope = parent;
        result
    }

    fn resolve_container(&mut self, ast: &Ast, id: NodeId, items: &[NodeId]) {
        let unit = self.state.current_unit();
        let inner = *self.state.node_scope[unit]
            .get(id)
            .expect("container scope is created during the declare pass");

        self.with_scope(inner, |this| {
            for &member in items {
                this.visit(ast, member);
            }
        });
    }

    fn resolve_scope_path(&self, node: NodeId) -> Result<SymbolId> {
        match self.state.ast().get(node) {
            Node::Expr(Expr::Ident(ident)) => {
                let symbol = self
                    .state
                    .symbols
                    .find_visible(
                        self.state.current_scope,
                        self.state.has_matching_name(ident.token),
                    )
                    .ok_or((
                        self.state.location(ident.token),
                        Error::UndeclaredIdentifier {
                            name: self.state.text(ident.token).to_owned(),
                        },
                    ))?;

                Ok(symbol)
            }
            Node::Expr(Expr::ScopeAccess(scope_access)) => {
                let base = self.resolve_scope_path(scope_access.base)?;

                let base_scope = *self.state.symbols_scope.get(base).ok_or_else(|| {
                    let base_location = self
                        .state
                        .location(self.state.ast().span(scope_access.base).start);

                    (
                        base_location,
                        Error::NotAScope {
                            name: self.state.source()[base_location].to_owned(),
                        },
                    )
                })?;

                let Node::Expr(Expr::Ident(name)) = self.state.ast().get(scope_access.name) else {
                    return Err((
                        self.state
                            .location(self.state.ast().span(scope_access.name).start),
                        Error::UnexpectedNode,
                    ));
                };

                let member = self
                    .state
                    .symbols
                    .find_local(base_scope, self.state.has_matching_name(name.token))
                    .ok_or((
                        self.state.location(name.token),
                        Error::UndeclaredIdentifier {
                            name: self.state.text(name.token).to_owned(),
                        },
                    ))?;

                Ok(member)
            }
            Node::Expr(Expr::Call(call)) => self.resolve_scope_path(call.callee),
            _ => {
                let location = self.state.location(self.state.ast().span(node).start);
                Err((location, Error::UnexpectedNode))
            }
        }
    }
}

impl<'s, 'a> Visitor for Resolve<'s, 'a> {
    fn visit_namespace_decl(&mut self, ast: &Ast, id: NodeId, namespace_decl: &NamespaceDecl) {
        self.resolve_container(ast, id, &namespace_decl.items);
    }

    fn visit_processor_decl(&mut self, ast: &Ast, id: NodeId, processor_decl: &ProcessorDecl) {
        self.resolve_container(ast, id, &processor_decl.items);
    }

    fn visit_graph_decl(&mut self, ast: &Ast, id: NodeId, graph_decl: &GraphDecl) {
        self.resolve_container(ast, id, &graph_decl.items);
    }

    fn visit_struct_decl(&mut self, ast: &Ast, id: NodeId, struct_decl: &StructDecl) {
        self.resolve_container(ast, id, &struct_decl.items);
    }

    fn visit_function_decl(&mut self, ast: &Ast, _id: NodeId, function_decl: &FunctionDecl) {
        self.with_new_scope_at(function_decl.name, |this, _| {
            this.visit(ast, function_decl.returns);
            for &param in &function_decl.params {
                this.visit(ast, param);
            }
            function_decl.annotations.walk(ast, this);

            let Node::Stmt(Stmt::Block(body)) = ast.get(function_decl.body) else {
                unreachable!("function body is always a block")
            };
            body.walk(ast, this);
        });
    }

    fn visit_event_handler_decl(
        &mut self,
        ast: &Ast,
        _id: NodeId,
        event_handler: &EventHandlerDecl,
    ) {
        self.with_new_scope_at(event_handler.name, |this, _| {
            for &param in &event_handler.params {
                this.visit(ast, param);
            }
            event_handler.annotations.walk(ast, this);

            let Node::Stmt(Stmt::Block(body)) = ast.get(event_handler.body) else {
                unreachable!("event handler body is always a block")
            };
            body.walk(ast, this);
        });
    }

    fn visit_var(&mut self, ast: &Ast, _id: NodeId, var: &Var) {
        for (id, declarator) in var.declarators(ast) {
            self.state.declare(
                self.state.current_scope,
                declarator.name,
                SymbolKind::Variable,
                id,
            );
        }
        var.walk(ast, self);
    }

    fn visit_typed_decl(&mut self, ast: &Ast, _id: NodeId, typed_decl: &TypedDecl) {
        for (id, declarator) in typed_decl.declarators(ast) {
            self.state.declare(
                self.state.current_scope,
                declarator.name,
                SymbolKind::Variable,
                id,
            );
        }
        typed_decl.walk(ast, self);
    }

    fn visit_param(&mut self, ast: &Ast, id: NodeId, param: &Param) {
        self.state.declare(
            self.state.current_scope,
            param.name,
            SymbolKind::Variable,
            id,
        );

        param.walk(ast, self);
    }

    fn visit_specialisation_value(
        &mut self,
        ast: &Ast,
        id: NodeId,
        specialisation_value: &SpecialisationValue,
    ) {
        self.state.declare(
            self.state.current_scope,
            specialisation_value.name,
            SymbolKind::Variable,
            id,
        );

        specialisation_value.walk(ast, self);
    }

    fn visit_block(&mut self, ast: &Ast, id: NodeId, block: &Block) {
        let scope_start = ast.span(id).start;
        self.with_new_scope_at(scope_start, |this, _| block.walk(ast, this));
    }

    fn visit_for_stmt(&mut self, ast: &Ast, id: NodeId, for_stmt: &ForStmt) {
        let scope_start = ast.span(id).start;
        self.with_new_scope_at(scope_start, |this, _| {
            if let Some(init) = for_stmt.init {
                this.visit(ast, init);
            }
            if let Some(cond) = for_stmt.cond {
                this.visit(ast, cond);
            }
            if let Some(update) = for_stmt.update {
                this.visit(ast, update);
            }

            this.with_new_scope_at(ast.span(for_stmt.body).start, |this, _| {
                match ast.get(for_stmt.body) {
                    Node::Stmt(Stmt::Block(block)) => block.walk(ast, this),
                    _ => this.visit(ast, for_stmt.body),
                }
            });
        });
    }

    fn visit_if_stmt(&mut self, ast: &Ast, id: NodeId, if_stmt: &IfStmt) {
        let scope_start = ast.span(id).start;
        self.with_new_scope_at(scope_start, |this, _| if_stmt.walk(ast, this));
    }

    fn visit_while_stmt(&mut self, ast: &Ast, id: NodeId, while_stmt: &WhileStmt) {
        let scope_start = ast.span(id).start;
        self.with_new_scope_at(scope_start, |this, _| while_stmt.walk(ast, this));
    }

    fn visit_loop_stmt(&mut self, ast: &Ast, id: NodeId, loop_stmt: &LoopStmt) {
        let scope_start = ast.span(id).start;
        self.with_new_scope_at(scope_start, |this, _| loop_stmt.walk(ast, this));
    }

    fn visit_ident(&mut self, _ast: &Ast, _id: NodeId, ident: &Ident) {
        if self
            .state
            .symbols
            .find_visible(
                self.state.current_scope,
                self.state.has_matching_name(ident.token),
            )
            .is_none()
        {
            self.state.error(
                self.state.location(ident.token),
                Error::UndeclaredIdentifier {
                    name: self.state.text(ident.token).to_owned(),
                },
            );
        }
    }

    fn visit_scope_access(&mut self, _ast: &Ast, id: NodeId, _scope_access: &ScopeAccess) {
        if let Err((location, err)) = self.resolve_scope_path(id) {
            self.state.error(location, err);
        }
    }
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        crate::{lexer, parser},
        indoc::indoc,
        view::ResolutionView,
    };

    fn expect_resolution(source: &'_ str) -> ResolutionView<'_> {
        let tokens = lexer::tokenize(source);
        let (ast, _) = parser::parse(source, &tokens);
        assert!(!ast.has_errors(), "source failed to parse: {source}");
        let resolution = resolve(source, &tokens, &ast);
        let source = Source::new(source);
        ResolutionView::new(resolution, tokens, source)
    }

    macro_rules! assert_resolution {
        ($program:expr, @$resolution:literal) => {
            insta::assert_yaml_snapshot!(expect_resolution($program), @$resolution);
        };
    }

    #[test]
    fn declares_top_level_processor() {
        assert_resolution!(
            indoc! {"
                processor P
                {
                    output stream int out;
                    void main() {}
                }
            "},
            @r#"
        symbols:
          - name: P
            kind: Processor
            location: "1:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: out
                kind: Endpoint
                location: "3:23"
              - name: main
                kind: Function
                location: "4:10"
            scopes:
              - location: "4:10"
        "#
        );
    }

    #[test]
    fn resolve_symbols_in_nested_scopes() {
        assert_resolution!(
            indoc! {"
                processor P
                {
                    int helper() { return 42; }
                    void main() { int x = helper(); }
                }
            "},
            @r#"
        symbols:
          - name: P
            kind: Processor
            location: "1:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: helper
                kind: Function
                location: "3:9"
              - name: main
                kind: Function
                location: "4:10"
            scopes:
              - location: "3:9"
              - location: "4:10"
                symbols:
                  - name: x
                    kind: Variable
                    location: "4:23"
        "#
        );
    }

    #[test]
    fn duplicate_processor_names_are_reported() {
        assert_resolution!(
            indoc! {"
                processor P
                {
                    output stream int out;
                }

                processor P
                {
                    output stream int out;
                }
            "},
            @r#"
        symbols:
          - name: P
            kind: Processor
            location: "1:11"
          - name: P
            kind: Processor
            location: "6:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: out
                kind: Endpoint
                location: "3:23"
          - location: "6:1"
            symbols:
              - name: out
                kind: Endpoint
                location: "8:23"
        diagnostics:
          - "6:11: redefinition of 'P' (previously declared at 1:11)"
        "#
        );
    }

    #[test]
    fn function_overloads_do_not_conflict() {
        assert_resolution!(
            indoc! {"
                processor P
                {
                    void f(int x) {}
                    void f(float x) {}
                    output stream int out;
                }
            "},
            @r#"
        symbols:
          - name: P
            kind: Processor
            location: "1:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: f
                kind: Function
                location: "3:10"
              - name: f
                kind: Function
                location: "4:10"
              - name: out
                kind: Endpoint
                location: "5:23"
            scopes:
              - location: "3:10"
                symbols:
                  - name: x
                    kind: Variable
                    location: "3:16"
              - location: "4:10"
                symbols:
                  - name: x
                    kind: Variable
                    location: "4:18"
        "#
        );
    }

    #[test]
    fn local_shadowing_a_parameter_is_reported() {
        assert_resolution!(
            indoc! {"
                processor P
                {
                    void f(bool a) { let a = false; }
                    output stream int out;
                }
            "},
            @r#"
        symbols:
          - name: P
            kind: Processor
            location: "1:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: f
                kind: Function
                location: "3:10"
              - name: out
                kind: Endpoint
                location: "4:23"
            scopes:
              - location: "3:10"
                symbols:
                  - name: a
                    kind: Variable
                    location: "3:17"
                  - name: a
                    kind: Variable
                    location: "3:26"
        diagnostics:
          - "3:26: redefinition of 'a' (previously declared at 3:17)"
        "#
        );
    }

    #[test]
    fn reopened_namespaces_share_a_scope() {
        assert_resolution!(
            indoc! {"
                namespace n
                {
                    processor A { output stream int out; }
                }

                namespace n
                {
                    processor B { output stream int out; }
                }
            "},
            @r#"
        symbols:
          - name: n
            kind: Namespace
            location: "1:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: A
                kind: Processor
                location: "3:15"
              - name: B
                kind: Processor
                location: "8:15"
            scopes:
              - location: "3:5"
                symbols:
                  - name: out
                    kind: Endpoint
                    location: "3:37"
              - location: "8:5"
                symbols:
                  - name: out
                    kind: Endpoint
                    location: "8:37"
        "#
        );
    }

    #[test]
    fn duplicate_state_variables_are_reported() {
        assert_resolution!(
            indoc! {"
                processor P
                {
                    int x;
                    int x;
                    output stream int out;
                    void main() {}
                }
            "},
            @r#"
        symbols:
          - name: P
            kind: Processor
            location: "1:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: x
                kind: Variable
                location: "3:9"
              - name: x
                kind: Variable
                location: "4:9"
              - name: out
                kind: Endpoint
                location: "5:23"
              - name: main
                kind: Function
                location: "6:10"
            scopes:
              - location: "6:10"
        diagnostics:
          - "4:9: redefinition of 'x' (previously declared at 3:9)"
        "#
        );
    }

    #[test]
    fn duplicate_node_names_in_a_graph_are_reported() {
        assert_resolution!(
            indoc! {"
                processor P
                {
                    output stream int out;
                }

                graph G
                {
                    node a = P;
                    node a = P;
                    output stream int out;
                }
            "},
            @r#"
        symbols:
          - name: P
            kind: Processor
            location: "1:11"
          - name: G
            kind: Graph
            location: "6:7"
        scopes:
          - location: "1:1"
            symbols:
              - name: out
                kind: Endpoint
                location: "3:23"
          - location: "6:1"
            symbols:
              - name: a
                kind: Node
                location: "8:10"
              - name: a
                kind: Node
                location: "9:10"
              - name: out
                kind: Endpoint
                location: "10:23"
        diagnostics:
          - "9:10: redefinition of 'a' (previously declared at 8:10)"
        "#
        );
    }

    #[test]
    fn undeclared_identifier() {
        assert_resolution!(
            indoc!{"
                processor P
                {
                    void main()
                    {
                        int x = qty;
                    }
                }
            "},
        @r#"
        symbols:
          - name: P
            kind: Processor
            location: "1:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: main
                kind: Function
                location: "3:10"
            scopes:
              - location: "3:10"
                symbols:
                  - name: x
                    kind: Variable
                    location: "5:13"
        diagnostics:
          - "5:17: undeclared identifier 'qty'"
        "#);
    }

    #[test]
    fn struct_method_forward_references_enclosing_processor_field() {
        assert_resolution!(
            indoc! {"
                processor P
                {
                    struct Note
                    {
                        void play() { out <- volume; }
                    }

                    output stream int out;
                    let volume = 1;
                }
            "},
        @r#"
        symbols:
          - name: P
            kind: Processor
            location: "1:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: Note
                kind: Struct
                location: "3:12"
              - name: out
                kind: Endpoint
                location: "8:23"
              - name: volume
                kind: Variable
                location: "9:9"
            scopes:
              - location: "3:5"
                symbols:
                  - name: play
                    kind: Function
                    location: "5:14"
                scopes:
                  - location: "5:14"
        "#);
    }

    #[test]
    fn namespace_member_forward_references_sibling_function() {
        assert_resolution!(
            indoc! {"
                namespace n
                {
                    int f() { return g(); }
                    int g() { return 1; }
                }
            "},
        @r#"
        symbols:
          - name: n
            kind: Namespace
            location: "1:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: f
                kind: Function
                location: "3:9"
              - name: g
                kind: Function
                location: "4:9"
            scopes:
              - location: "3:9"
              - location: "4:9"
        "#);
    }

    #[test]
    fn top_level_forward_reference_resolves() {
        assert_resolution!(
            indoc! {"
                processor P
                {
                    void main() { let x = helper(); }
                }
                int helper() { return 1; }
            "},
        @r#"
        symbols:
          - name: P
            kind: Processor
            location: "1:11"
          - name: helper
            kind: Function
            location: "5:5"
        scopes:
          - location: "1:1"
            symbols:
              - name: main
                kind: Function
                location: "3:10"
            scopes:
              - location: "3:10"
                symbols:
                  - name: x
                    kind: Variable
                    location: "3:23"
          - location: "5:5"
        "#);
    }

    #[test]
    fn qualified_namespace() {
        assert_resolution!(
            indoc!{"
                namespace Utils
                {
                    int square (int x)
                    {
                        return x * x;
                    }
                }
                processor P
                {
                    void main()
                    {
                        int x = Utils::square (4);
                    }
                }
            "},
        @r#"
        symbols:
          - name: Utils
            kind: Namespace
            location: "1:11"
          - name: P
            kind: Processor
            location: "8:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: square
                kind: Function
                location: "3:9"
            scopes:
              - location: "3:9"
                symbols:
                  - name: x
                    kind: Variable
                    location: "3:21"
          - location: "8:1"
            symbols:
              - name: main
                kind: Function
                location: "10:10"
            scopes:
              - location: "10:10"
                symbols:
                  - name: x
                    kind: Variable
                    location: "12:13"
        "#);
    }

    #[test]
    fn control_flow_introduces_new_scopes() {
        assert_resolution!(
            indoc!{"
                namespace test
                {
                    void f() {
                        int x = 0;
                        if (true) { int x = 1; }
                        if (false) int x = 2;
                        loop { int x = 3; }
                        loop int x = 4;
                        while (true) int x = 5;
                        while (true) { int x = 5; }
                        for (;;) int x = 6;
                        for (;;) { int x = 7; }
                        for (int x = 8;;) int x = 9;
                        for (int x = 10;;) { int x = 11; }
                    }
                }
            "},
        @r#"
        symbols:
          - name: test
            kind: Namespace
            location: "1:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: f
                kind: Function
                location: "3:10"
            scopes:
              - location: "3:10"
                symbols:
                  - name: x
                    kind: Variable
                    location: "4:13"
                scopes:
                  - location: "5:9"
                    scopes:
                      - location: "5:19"
                        symbols:
                          - name: x
                            kind: Variable
                            location: "5:25"
                  - location: "6:9"
                    symbols:
                      - name: x
                        kind: Variable
                        location: "6:24"
                  - location: "7:9"
                    scopes:
                      - location: "7:14"
                        symbols:
                          - name: x
                            kind: Variable
                            location: "7:20"
                  - location: "8:9"
                    symbols:
                      - name: x
                        kind: Variable
                        location: "8:18"
                  - location: "9:9"
                    symbols:
                      - name: x
                        kind: Variable
                        location: "9:26"
                  - location: "10:9"
                    scopes:
                      - location: "10:22"
                        symbols:
                          - name: x
                            kind: Variable
                            location: "10:28"
                  - location: "11:9"
                    scopes:
                      - location: "11:18"
                        symbols:
                          - name: x
                            kind: Variable
                            location: "11:22"
                  - location: "12:9"
                    scopes:
                      - location: "12:18"
                        symbols:
                          - name: x
                            kind: Variable
                            location: "12:24"
                  - location: "13:9"
                    symbols:
                      - name: x
                        kind: Variable
                        location: "13:18"
                    scopes:
                      - location: "13:27"
                        symbols:
                          - name: x
                            kind: Variable
                            location: "13:31"
                  - location: "14:9"
                    symbols:
                      - name: x
                        kind: Variable
                        location: "14:18"
                    scopes:
                      - location: "14:28"
                        symbols:
                          - name: x
                            kind: Variable
                            location: "14:34"
        "#);
    }

    #[test]
    fn qualified_namespace_member_undeclared() {
        assert_resolution!(
            indoc! {"
                namespace Utils
                {
                    int square (int x) { return x * x; }
                }
                processor P
                {
                    void main()
                    {
                        int x = Utils::bogus (4);
                    }
                }
            "},
        @r#"
        symbols:
          - name: Utils
            kind: Namespace
            location: "1:11"
          - name: P
            kind: Processor
            location: "5:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: square
                kind: Function
                location: "3:9"
            scopes:
              - location: "3:9"
                symbols:
                  - name: x
                    kind: Variable
                    location: "3:21"
          - location: "5:1"
            symbols:
              - name: main
                kind: Function
                location: "7:10"
            scopes:
              - location: "7:10"
                symbols:
                  - name: x
                    kind: Variable
                    location: "9:13"
        diagnostics:
          - "9:24: undeclared identifier 'bogus'"
        "#);
    }

    #[test]
    fn qualified_processor_member_resolves() {
        assert_resolution!(
            indoc! {"
                processor Utils
                {
                    int square (int x) { return x * x; }
                }
                processor P
                {
                    void main() { int x = Utils::square (4); }
                }
            "},
        @r#"
        symbols:
          - name: Utils
            kind: Processor
            location: "1:11"
          - name: P
            kind: Processor
            location: "5:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: square
                kind: Function
                location: "3:9"
            scopes:
              - location: "3:9"
                symbols:
                  - name: x
                    kind: Variable
                    location: "3:21"
          - location: "5:1"
            symbols:
              - name: main
                kind: Function
                location: "7:10"
            scopes:
              - location: "7:10"
                symbols:
                  - name: x
                    kind: Variable
                    location: "7:23"
        "#);
    }

    #[test]
    fn qualified_processor_member_undeclared() {
        assert_resolution!(
            indoc! {"
                processor Utils
                {
                    int square (int x) { return x * x; }
                }
                processor P
                {
                    void main() { int x = Utils::bogus (4); }
                }
            "},
        @r#"
        symbols:
          - name: Utils
            kind: Processor
            location: "1:11"
          - name: P
            kind: Processor
            location: "5:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: square
                kind: Function
                location: "3:9"
            scopes:
              - location: "3:9"
                symbols:
                  - name: x
                    kind: Variable
                    location: "3:21"
          - location: "5:1"
            symbols:
              - name: main
                kind: Function
                location: "7:10"
            scopes:
              - location: "7:10"
                symbols:
                  - name: x
                    kind: Variable
                    location: "7:23"
        diagnostics:
          - "7:34: undeclared identifier 'bogus'"
        "#);
    }

    #[test]
    fn qualified_struct_member_resolves() {
        assert_resolution!(
            indoc! {"
                struct Point { int x, y; }
                processor P { void main() { int a = Point::x; } }
            "},
        @r#"
        symbols:
          - name: Point
            kind: Struct
            location: "1:8"
          - name: P
            kind: Processor
            location: "2:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: x
                kind: Variable
                location: "1:20"
              - name: y
                kind: Variable
                location: "1:23"
          - location: "2:1"
            symbols:
              - name: main
                kind: Function
                location: "2:20"
            scopes:
              - location: "2:20"
                symbols:
                  - name: a
                    kind: Variable
                    location: "2:33"
        "#);
    }

    #[test]
    fn qualified_struct_member_undeclared() {
        assert_resolution!(
            indoc! {"
                struct Point { int x, y; }
                processor P { void main() { int a = Point::z; } }
            "},
        @r#"
        symbols:
          - name: Point
            kind: Struct
            location: "1:8"
          - name: P
            kind: Processor
            location: "2:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: x
                kind: Variable
                location: "1:20"
              - name: y
                kind: Variable
                location: "1:23"
          - location: "2:1"
            symbols:
              - name: main
                kind: Function
                location: "2:20"
            scopes:
              - location: "2:20"
                symbols:
                  - name: a
                    kind: Variable
                    location: "2:33"
        diagnostics:
          - "2:44: undeclared identifier 'z'"
        "#);
    }

    #[test]
    fn qualified_graph_member_resolves() {
        assert_resolution!(
            indoc! {"
                processor Foo { output stream int out; }
                graph G { node a = Foo; }
                processor P { void main() { let x = G::a; } }
            "},
        @r#"
        symbols:
          - name: Foo
            kind: Processor
            location: "1:11"
          - name: G
            kind: Graph
            location: "2:7"
          - name: P
            kind: Processor
            location: "3:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: out
                kind: Endpoint
                location: "1:35"
          - location: "2:1"
            symbols:
              - name: a
                kind: Node
                location: "2:16"
          - location: "3:1"
            symbols:
              - name: main
                kind: Function
                location: "3:20"
            scopes:
              - location: "3:20"
                symbols:
                  - name: x
                    kind: Variable
                    location: "3:33"
        "#);
    }

    #[test]
    fn qualified_graph_member_undeclared() {
        assert_resolution!(
            indoc! {"
                processor Foo { output stream int out; }
                graph G { node a = Foo; }
                processor P { void main() { let x = G::bogus; } }
            "},
        @r#"
        symbols:
          - name: Foo
            kind: Processor
            location: "1:11"
          - name: G
            kind: Graph
            location: "2:7"
          - name: P
            kind: Processor
            location: "3:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: out
                kind: Endpoint
                location: "1:35"
          - location: "2:1"
            symbols:
              - name: a
                kind: Node
                location: "2:16"
          - location: "3:1"
            symbols:
              - name: main
                kind: Function
                location: "3:20"
            scopes:
              - location: "3:20"
                symbols:
                  - name: x
                    kind: Variable
                    location: "3:33"
        diagnostics:
          - "3:40: undeclared identifier 'bogus'"
        "#);
    }

    #[test]
    fn qualified_enum_value_undeclared() {
        assert_resolution!(
            indoc! {"
                enum Mode { Play, Stop }
                processor P { void main() { let m = Mode::Bogus; } }
            "},
        @r#"
        symbols:
          - name: Mode
            kind: Enum
            location: "1:6"
          - name: P
            kind: Processor
            location: "2:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: Play
                kind: EnumValue
                location: "1:13"
              - name: Stop
                kind: EnumValue
                location: "1:19"
          - location: "2:1"
            symbols:
              - name: main
                kind: Function
                location: "2:20"
            scopes:
              - location: "2:20"
                symbols:
                  - name: m
                    kind: Variable
                    location: "2:33"
        diagnostics:
          - "2:43: undeclared identifier 'Bogus'"
        "#);
    }

    #[test]
    fn nested_qualified_chain_resolves() {
        assert_resolution!(
            indoc! {"
                namespace A { namespace B { int square (int x) { return x * x; } } }
                processor P { void main() { int x = A::B::square (4); } }
            "},
        @r#"
        symbols:
          - name: A
            kind: Namespace
            location: "1:11"
          - name: P
            kind: Processor
            location: "2:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: B
                kind: Namespace
                location: "1:25"
            scopes:
              - location: "1:15"
                symbols:
                  - name: square
                    kind: Function
                    location: "1:33"
                scopes:
                  - location: "1:33"
                    symbols:
                      - name: x
                        kind: Variable
                        location: "1:45"
          - location: "2:1"
            symbols:
              - name: main
                kind: Function
                location: "2:20"
            scopes:
              - location: "2:20"
                symbols:
                  - name: x
                    kind: Variable
                    location: "2:33"
        "#);
    }

    #[test]
    fn nested_qualified_chain_undeclared() {
        assert_resolution!(
            indoc! {"
                namespace A { namespace B { int square (int x) { return x * x; } } }
                processor P { void main() { int x = A::B::bogus (4); } }
            "},
        @r#"
        symbols:
          - name: A
            kind: Namespace
            location: "1:11"
          - name: P
            kind: Processor
            location: "2:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: B
                kind: Namespace
                location: "1:25"
            scopes:
              - location: "1:15"
                symbols:
                  - name: square
                    kind: Function
                    location: "1:33"
                scopes:
                  - location: "1:33"
                    symbols:
                      - name: x
                        kind: Variable
                        location: "1:45"
          - location: "2:1"
            symbols:
              - name: main
                kind: Function
                location: "2:20"
            scopes:
              - location: "2:20"
                symbols:
                  - name: x
                    kind: Variable
                    location: "2:33"
        diagnostics:
          - "2:43: undeclared identifier 'bogus'"
        "#);
    }

    #[test]
    fn qualified_enum() {
        assert_resolution!(
            indoc!{"
                enum Mode { Play, Stop }
                processor P { void main() { let m = Mode::Play; } }
            "},
        @r#"
        symbols:
          - name: Mode
            kind: Enum
            location: "1:6"
          - name: P
            kind: Processor
            location: "2:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: Play
                kind: EnumValue
                location: "1:13"
              - name: Stop
                kind: EnumValue
                location: "1:19"
          - location: "2:1"
            symbols:
              - name: main
                kind: Function
                location: "2:20"
            scopes:
              - location: "2:20"
                symbols:
                  - name: m
                    kind: Variable
                    location: "2:33"
        "#);
    }

    #[test]
    fn scope_path_on_non_container_symbol_results_in_diagnostic() {
        assert_resolution!(
            indoc! {"
                processor P { void main() { int a = 1; int b = a::x; } }
            "},
            @r#"
        symbols:
          - name: P
            kind: Processor
            location: "1:11"
        scopes:
          - location: "1:1"
            symbols:
              - name: main
                kind: Function
                location: "1:20"
            scopes:
              - location: "1:20"
                symbols:
                  - name: a
                    kind: Variable
                    location: "1:33"
                  - name: b
                    kind: Variable
                    location: "1:44"
        diagnostics:
          - "1:48: 'a' is not a namespace, processor, graph, struct, or enum"
        "#
        );
    }

    mod multi_unit {
        use super::*;

        #[test]
        fn resolving_zero_units_does_not_panic() {
            let resolution = resolve_all(&[]);
            assert!(resolution.diagnostics.is_empty());
            assert!(
                resolution
                    .symbols
                    .symbols_in(resolution.symbols.global_scope())
                    .count()
                    > 0
            );
        }

        #[test]
        fn resolves_namespaced_function_across_units() {
            let a_src = indoc! {"
                namespace std::math
                {
                    int square (int x) { return x * x; }
                }
            "};
            let b_src = indoc! {"
                processor P { void main() { int x = std::math::square (4); } }
            "};

            let a_tokens = lexer::tokenize(a_src);
            let b_tokens = lexer::tokenize(b_src);

            let (a_ast, _) = parser::parse(a_src, &a_tokens);
            let (b_ast, _) = parser::parse(b_src, &b_tokens);
            assert!(!a_ast.has_errors());
            assert!(!b_ast.has_errors());

            let resolution = resolve_all(&[
                Unit {
                    name: "a.cmajor",
                    source: a_src,
                    tokens: &a_tokens,
                    ast: &a_ast,
                },
                Unit {
                    name: "b.cmajor",
                    source: b_src,
                    tokens: &b_tokens,
                    ast: &b_ast,
                },
            ]);

            assert!(
                resolution.diagnostics.is_empty(),
                "{:?}",
                resolution.diagnostics
            );
        }

        #[test]
        fn namespace_reopened_across_units_shares_a_scope() {
            let a_src = "namespace n { processor A { output stream int out; } }\n";
            let b_src = "namespace n { processor B { output stream int out; } }\n";

            let a_tokens = lexer::tokenize(a_src);
            let b_tokens = lexer::tokenize(b_src);

            let (a_ast, _) = parser::parse(a_src, &a_tokens);
            let (b_ast, _) = parser::parse(b_src, &b_tokens);

            let resolution = resolve_all(&[
                Unit {
                    name: "a.cmajor",
                    source: a_src,
                    tokens: &a_tokens,
                    ast: &a_ast,
                },
                Unit {
                    name: "b.cmajor",
                    source: b_src,
                    tokens: &b_tokens,
                    ast: &b_ast,
                },
            ]);
            assert!(
                resolution.diagnostics.is_empty(),
                "{:?}",
                resolution.diagnostics
            );

            let global = resolution.symbols.global_scope();
            let namespace_scopes: Vec<_> = resolution.symbols.child_scopes(global).collect();
            assert_eq!(
                namespace_scopes.len(),
                1,
                "namespace n should be reopened into a single shared scope across both units"
            );

            let kinds: Vec<_> = resolution
                .symbols
                .symbols_in(namespace_scopes[0])
                .map(|symbol| symbol.kind)
                .collect();
            assert_eq!(kinds, vec![SymbolKind::Processor, SymbolKind::Processor]);
        }

        #[test]
        fn earlier_unit_can_forward_reference_a_later_units_symbol() {
            let a_src = "namespace n { int f() { return g(); } }\n";
            let b_src = "namespace n { int g() { return 1; } }\n";

            let a_tokens = lexer::tokenize(a_src);
            let b_tokens = lexer::tokenize(b_src);

            let (a_ast, _) = parser::parse(a_src, &a_tokens);
            let (b_ast, _) = parser::parse(b_src, &b_tokens);

            let resolution = resolve_all(&[
                Unit {
                    name: "a.cmajor",
                    source: a_src,
                    tokens: &a_tokens,
                    ast: &a_ast,
                },
                Unit {
                    name: "b.cmajor",
                    source: b_src,
                    tokens: &b_tokens,
                    ast: &b_ast,
                },
            ]);

            assert!(
                resolution.diagnostics.is_empty(),
                "{:?}",
                resolution.diagnostics
            );
        }

        #[test]
        fn redefinition_across_units_is_reported_against_the_second_unit() {
            let a_src = "processor P { output stream int out; }\n";
            let b_src = "processor P { output stream int out; }\n";

            let a_tokens = lexer::tokenize(a_src);
            let b_tokens = lexer::tokenize(b_src);

            let (a_ast, _) = parser::parse(a_src, &a_tokens);
            let (b_ast, _) = parser::parse(b_src, &b_tokens);

            let resolution = resolve_all(&[
                Unit {
                    name: "a.cmajor",
                    source: a_src,
                    tokens: &a_tokens,
                    ast: &a_ast,
                },
                Unit {
                    name: "b.cmajor",
                    source: b_src,
                    tokens: &b_tokens,
                    ast: &b_ast,
                },
            ]);

            assert_eq!(resolution.diagnostics.len(), 1);
            let diagnostic = &resolution.diagnostics[0];
            assert!(
                diagnostic.message.contains("redefinition of 'P'"),
                "{}",
                diagnostic.message
            );

            let processor_units = source_units_in_declaration_order(&resolution);
            assert_eq!(processor_units.len(), 2);
            assert_ne!(processor_units[0], processor_units[1]);
            assert_eq!(diagnostic.unit, processor_units[1]);
        }

        #[test]
        fn diagnostic_reports_the_unit_it_occurred_in() {
            let a_src = "namespace n {}\n";
            let b_src = "processor P { void main() { int x = qty; } }\n";

            let a_tokens = lexer::tokenize(a_src);
            let b_tokens = lexer::tokenize(b_src);

            let (a_ast, _) = parser::parse(a_src, &a_tokens);
            let (b_ast, _) = parser::parse(b_src, &b_tokens);

            let resolution = resolve_all(&[
                Unit {
                    name: "a.cmajor",
                    source: a_src,
                    tokens: &a_tokens,
                    ast: &a_ast,
                },
                Unit {
                    name: "b.cmajor",
                    source: b_src,
                    tokens: &b_tokens,
                    ast: &b_ast,
                },
            ]);

            assert_eq!(resolution.diagnostics.len(), 1);

            let units = source_units_in_declaration_order(&resolution);
            assert_eq!(units.len(), 2);
            assert_eq!(resolution.diagnostics[0].unit, units[1]);
        }

        fn source_units_in_declaration_order(resolution: &Resolution) -> Vec<UnitId> {
            let global = resolution.symbols.global_scope();
            resolution
                .symbols
                .symbols_in(global)
                .filter_map(|symbol| match symbol.origin {
                    SymbolOrigin::Source { unit, .. } => Some(unit),
                    SymbolOrigin::Builtin { .. } => None,
                })
                .collect()
        }

        #[test]
        fn reparsing_only_the_edited_unit_still_resolves_correctly() {
            let a_src = "namespace n { int helper() { return 1; } }\n";
            let a_tokens = lexer::tokenize(a_src);
            let (a_ast, _) = parser::parse(a_src, &a_tokens);

            let b_src_v1 = "processor P { void main() { int x = 1; } }\n";
            let b_tokens_v1 = lexer::tokenize(b_src_v1);
            let (b_ast_v1, _) = parser::parse(b_src_v1, &b_tokens_v1);
            let resolution_v1 = resolve_all(&[
                Unit {
                    name: "a.cmajor",
                    source: a_src,
                    tokens: &a_tokens,
                    ast: &a_ast,
                },
                Unit {
                    name: "b.cmajor",
                    source: b_src_v1,
                    tokens: &b_tokens_v1,
                    ast: &b_ast_v1,
                },
            ]);
            assert!(
                resolution_v1.diagnostics.is_empty(),
                "{:?}",
                resolution_v1.diagnostics
            );

            let b_src_v2 = "processor P { void main() { int x = n::helper(); } }\n";
            let b_tokens_v2 = lexer::tokenize(b_src_v2);
            let (b_ast_v2, _) = parser::parse(b_src_v2, &b_tokens_v2);
            let resolution_v2 = resolve_all(&[
                Unit {
                    name: "a.cmajor",
                    source: a_src,
                    tokens: &a_tokens,
                    ast: &a_ast,
                },
                Unit {
                    name: "b.cmajor",
                    source: b_src_v2,
                    tokens: &b_tokens_v2,
                    ast: &b_ast_v2,
                },
            ]);
            assert!(
                resolution_v2.diagnostics.is_empty(),
                "{:?}",
                resolution_v2.diagnostics
            );
        }
    }
}
