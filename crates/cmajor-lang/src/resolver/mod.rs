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
    CompileError, Diagnostic, UnitLocation,
    ast::{
        Alias, Ast, Block, EndpointDeclaration, EnumDecl, EnumValue, EventHandlerDecl, Expr,
        ForStmt, FunctionDecl, GenericParam, GraphDecl, HoistedEndpointDeclaration, Ident, IfStmt,
        LoopStmt, ModuleAlias, NamespaceDecl, Node, NodeDecl, NodeId, Param, ProcessorDecl,
        ScopeAccess, SpecialisationValue, Stmt, StructDecl, TypedDecl, Var, WhileStmt,
        visit::{Visitor, Walk},
    },
    compile::Program,
    lexer::TokenId,
    resolver::unit::Anchor,
    utils::{
        arena::{SecondaryArena, SparseSecondaryArena},
        source::SourceLocation,
        span::Span,
    },
};

pub struct Resolution {
    pub symbols: SymbolTable,
    pub diagnostics: Vec<(UnitId, Diagnostic)>,
}

pub fn resolve(program: &Program) -> Resolution {
    let mut state = State::new(program);

    declare_units(&mut state, program);
    compute_intrinsics_scope(&mut state, program);
    resolve_units(&mut state, program);

    Resolution {
        symbols: state.symbols,
        diagnostics: state.diagnostics,
    }
}

pub struct StandardLibrary {
    program: Program,
    state: State,
    diagnostics: Vec<(UnitId, Diagnostic)>,
}

pub fn declare_stdlib(program: &Program) -> StandardLibrary {
    let mut state = State::new(program);

    declare_units(&mut state, program);
    compute_intrinsics_scope(&mut state, program);
    let diagnostics = std::mem::take(&mut state.diagnostics);

    StandardLibrary {
        program: program.clone(),
        state,
        diagnostics,
    }
}

impl StandardLibrary {
    pub fn diagnostics(&self) -> &[(UnitId, Diagnostic)] {
        &self.diagnostics
    }
}

pub struct ResolvedProgram {
    pub program: Program,
    pub resolution: Resolution,
}

pub fn resolve_with_standard_library(
    program: Program,
    baseline: &StandardLibrary,
) -> ResolvedProgram {
    let mut combined = baseline.program.clone();
    let mut state = baseline.state.clone();

    let new_units: Vec<UnitId> = program
        .into_iter()
        .map(|unit| combined.add_parsed_unit(unit))
        .collect();

    declare_units_for(&mut state, &combined, new_units.iter().copied());
    resolve_units_for(&mut state, &combined, new_units.iter().copied());

    ResolvedProgram {
        program: combined,
        resolution: Resolution {
            symbols: state.symbols,
            diagnostics: state.diagnostics,
        },
    }
}

fn declare_units(state: &mut State, program: &Program) {
    declare_units_for(state, program, program.units());
}

fn declare_units_for(
    state: &mut State,
    program: &Program,
    units: impl IntoIterator<Item = UnitId>,
) {
    let global_scope = state.symbols.global_scope();

    for unit in units {
        state.ensure_unit(unit);
        state.current_scope = global_scope;
        let ast = &program[unit].ast;
        ast.visit(&mut Declare {
            state,
            program,
            unit,
        });
    }
}

fn resolve_units(state: &mut State, program: &Program) {
    resolve_units_for(state, program, program.units());
}

fn resolve_units_for(
    state: &mut State,
    program: &Program,
    units: impl IntoIterator<Item = UnitId>,
) {
    let global_scope = state.symbols.global_scope();

    for unit in units {
        state.current_scope = global_scope;
        let ast = &program[unit].ast;
        ast.visit(&mut Resolve {
            state,
            program,
            unit,
        });
    }
}

fn compute_intrinsics_scope(state: &mut State, program: &Program) {
    let global_scope = state.symbols.global_scope();

    let find_child_namespace = |state: &State, program: &Program, scope: ScopeId, name: &str| {
        let symbol = state.symbols.find_local(scope, |symbol| {
            symbol.kind == SymbolKind::Namespace
                && match symbol.origin {
                    SymbolOrigin::Builtin { name: symbol_name } => name == symbol_name,
                    SymbolOrigin::Source {
                        unit, name: token, ..
                    } => name == program[unit].text(token),
                }
        })?;
        state.symbols.inner_scope(symbol)
    };

    let scope = find_child_namespace(state, program, global_scope, "std")
        .and_then(|std_scope| find_child_namespace(state, program, std_scope, "intrinsics"));

    if let Some(scope) = scope {
        state.intrinsics_scope.replace(scope);
    }
}

type Result<T> = std::result::Result<T, (Span<SourceLocation>, CompileError)>;

#[derive(Clone)]
struct State {
    symbols: SymbolTable,
    diagnostics: Vec<(UnitId, Diagnostic)>,
    current_scope: ScopeId,
    node_scope: SecondaryArena<UnitId, SparseSecondaryArena<NodeId, ScopeId>>,
    declared: SecondaryArena<UnitId, SparseSecondaryArena<NodeId, SymbolId>>,
    intrinsics_scope: Option<ScopeId>,
}

impl State {
    pub fn new(program: &Program) -> Self {
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

        let node_scope: SecondaryArena<UnitId, _> = program
            .units()
            .map(|id| (id, SparseSecondaryArena::default()))
            .collect();

        let declared = program
            .units()
            .map(|id| (id, SparseSecondaryArena::default()))
            .collect();

        State {
            symbols,
            diagnostics: Vec::new(),
            current_scope: global_scope,
            node_scope,
            declared,
            intrinsics_scope: None,
        }
    }

    fn ensure_unit(&mut self, unit: UnitId) {
        if self.node_scope.get(unit).is_none() {
            self.node_scope
                .insert(unit, SparseSecondaryArena::default());
        }
        if self.declared.get(unit).is_none() {
            self.declared.insert(unit, SparseSecondaryArena::default());
        }
    }

    fn error(&mut self, unit: UnitId, location: Span<SourceLocation>, error: CompileError) {
        self.diagnostics
            .push((unit, Diagnostic { location, error }));
    }

    fn declaration_location(&self, program: &Program, symbol: SymbolId) -> Option<UnitLocation> {
        match self.symbols.symbol(symbol).origin {
            SymbolOrigin::Builtin { .. } => None,
            SymbolOrigin::Source { unit, name, .. } => Some(UnitLocation {
                unit,
                location: program[unit].location(name).start,
            }),
        }
    }

    fn has_matching_name(
        &self,
        program: &Program,
        unit: UnitId,
        name: TokenId,
    ) -> impl FnMut(&Symbol) -> bool {
        let query = program[unit].text(name).to_string();
        move |symbol: &Symbol| symbol.name(program) == query
    }

    fn declare(
        &mut self,
        program: &Program,
        unit: UnitId,
        scope: ScopeId,
        name: TokenId,
        kind: SymbolKind,
        node: NodeId,
    ) -> SymbolId {
        if let Some(&symbol) = self.declared[unit].get(node) {
            return symbol;
        }

        if !kind.allows_duplicates()
            && let Some(existing) = self
                .symbols
                .find_local(scope, self.has_matching_name(program, unit, name))
        {
            let existing_location = self.declaration_location(program, existing);

            self.error(
                unit,
                program[unit].location(name),
                CompileError::NameInUse {
                    name: program[unit].text(name).to_string(),
                    previous_declaration: existing_location,
                },
            );
        }

        let symbol = self
            .symbols
            .declare(SymbolOrigin::Source { unit, name, node }, kind, scope);
        self.declared[unit].insert(node, symbol);
        symbol
    }

    fn declare_local(
        &mut self,
        unit: UnitId,
        name: TokenId,
        kind: SymbolKind,
        node: NodeId,
    ) -> SymbolId {
        if let Some(&symbol) = self.declared[unit].get(node) {
            return symbol;
        }

        let symbol = self.symbols.declare(
            SymbolOrigin::Source { unit, name, node },
            kind,
            self.current_scope,
        );
        self.declared[unit].insert(node, symbol);
        symbol
    }
}

struct Declare<'s, 'p> {
    state: &'s mut State,
    program: &'p Program,
    unit: UnitId,
}

impl<'s, 'p> Declare<'s, 'p> {
    fn with_scope<R>(&mut self, inner: ScopeId, f: impl FnOnce(&mut Self) -> R) -> R {
        let parent = self.state.current_scope;
        self.state.current_scope = inner;
        let result = f(self);
        self.state.current_scope = parent;
        result
    }

    fn declare_container(
        &mut self,
        program: &Program,
        unit: UnitId,
        ast: &Ast,
        name: TokenId,
        kind: SymbolKind,
        node: NodeId,
        builtins: &[(&'static str, SymbolKind)],
        params: &[NodeId],
        items: &[NodeId],
    ) -> SymbolId {
        let symbol = self
            .state
            .declare(program, unit, self.state.current_scope, name, kind, node);

        let scope_start = Anchor {
            unit,
            token: program[unit].ast.span(node).start,
        };

        let container_scope = self.state.symbols.new_inner_scope(symbol, scope_start);

        self.state.node_scope[unit].insert(node, container_scope);

        self.with_scope(container_scope, |this| {
            for &(name, kind) in builtins {
                this.state.symbols.declare(
                    SymbolOrigin::Builtin { name },
                    kind,
                    this.state.current_scope,
                );
            }

            for &param in params {
                this.visit(ast, param);
            }

            for &member in items {
                this.visit(ast, member);
            }
        });

        symbol
    }

    fn declare_or_reuse_namespace(
        &mut self,
        program: &Program,
        unit: UnitId,
        scope: ScopeId,
        name: TokenId,
        node: NodeId,
    ) -> ScopeId {
        if let Some(existing_id) = self
            .state
            .symbols
            .find_local(scope, self.state.has_matching_name(program, unit, name))
        {
            let existing = self.state.symbols.symbol(existing_id);
            if existing.kind == SymbolKind::Namespace {
                return self
                    .state
                    .symbols
                    .inner_scope(existing_id)
                    .expect("namespace symbol always has an inner scope");
            }

            let error_location = program[unit].location(name);

            self.state.error(
                unit,
                error_location,
                CompileError::NameInUse {
                    name: program[unit].text(name).to_string(),
                    previous_declaration: self.state.declaration_location(program, existing_id),
                },
            );
        }

        let symbol = self.state.symbols.declare(
            SymbolOrigin::Source { unit, name, node },
            SymbolKind::Namespace,
            scope,
        );
        let scope_start = Anchor {
            unit,
            token: program[unit].ast.span(node).start,
        };

        self.state.symbols.new_inner_scope(symbol, scope_start)
    }
}

impl<'s, 'a> Visitor for Declare<'s, 'a> {
    fn visit_namespace_decl(&mut self, ast: &Ast, id: NodeId, namespace_decl: &NamespaceDecl) {
        let scope = self.state.current_scope;
        let inner = namespace_decl
            .segments
            .iter()
            .fold(scope, |scope, &segment| {
                self.declare_or_reuse_namespace(self.program, self.unit, scope, segment, id)
            });

        self.state.node_scope[self.unit].insert(id, inner);

        self.with_scope(inner, |this| {
            for &param in &namespace_decl.params {
                this.visit(ast, param);
            }

            for &member in &namespace_decl.items {
                this.visit(ast, member);
            }
        });
    }

    fn visit_processor_decl(&mut self, ast: &Ast, id: NodeId, processor_decl: &ProcessorDecl) {
        self.declare_container(
            self.program,
            self.unit,
            ast,
            processor_decl.name,
            SymbolKind::Processor,
            id,
            &[
                ("advance", SymbolKind::Function),
                ("processor", SymbolKind::Variable),
                ("console", SymbolKind::Endpoint),
            ],
            &processor_decl.params,
            &processor_decl.items,
        );
    }

    fn visit_graph_decl(&mut self, ast: &Ast, id: NodeId, graph_decl: &GraphDecl) {
        self.declare_container(
            self.program,
            self.unit,
            ast,
            graph_decl.name,
            SymbolKind::Graph,
            id,
            &[],
            &graph_decl.params,
            &graph_decl.items,
        );
    }

    fn visit_struct_decl(&mut self, ast: &Ast, id: NodeId, struct_decl: &StructDecl) {
        self.declare_container(
            self.program,
            self.unit,
            ast,
            struct_decl.name,
            SymbolKind::Struct,
            id,
            &[("this", SymbolKind::Variable)],
            &[],
            &struct_decl.items,
        );
    }

    fn visit_enum_decl(&mut self, ast: &Ast, id: NodeId, enum_decl: &EnumDecl) {
        self.declare_container(
            self.program,
            self.unit,
            ast,
            enum_decl.name,
            SymbolKind::Enum,
            id,
            &[],
            &[],
            ast.children(enum_decl.values),
        );
    }

    fn visit_enum_value(&mut self, _ast: &Ast, id: NodeId, enum_value: &EnumValue) {
        self.state.declare(
            self.program,
            self.unit,
            self.state.current_scope,
            enum_value.name,
            SymbolKind::EnumValue,
            id,
        );
    }

    fn visit_function_decl(&mut self, _ast: &Ast, id: NodeId, function_decl: &FunctionDecl) {
        self.state.declare(
            self.program,
            self.unit,
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
            self.program,
            self.unit,
            self.state.current_scope,
            module_alias.name,
            SymbolKind::Alias,
            id,
        );
    }

    fn visit_var(&mut self, ast: &Ast, _id: NodeId, var: &Var) {
        for (id, declarator) in var.declarators(ast) {
            self.state.declare(
                self.program,
                self.unit,
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
                self.program,
                self.unit,
                self.state.current_scope,
                declarator.name,
                SymbolKind::Variable,
                id,
            );
        }
    }

    fn visit_specialisation_value(
        &mut self,
        _ast: &Ast,
        id: NodeId,
        specialisation_value: &SpecialisationValue,
    ) {
        self.state.declare(
            self.program,
            self.unit,
            self.state.current_scope,
            specialisation_value.name,
            SymbolKind::Variable,
            id,
        );
    }

    fn visit_alias(&mut self, _ast: &Ast, id: NodeId, alias: &Alias) {
        self.state.declare(
            self.program,
            self.unit,
            self.state.current_scope,
            alias.name,
            SymbolKind::Alias,
            id,
        );
    }

    fn visit_endpoint_declaration(
        &mut self,
        _ast: &Ast,
        id: NodeId,
        endpoint_declaration: &EndpointDeclaration,
    ) {
        self.state.declare(
            self.program,
            self.unit,
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
            self.state.declare(
                self.program,
                self.unit,
                self.state.current_scope,
                name,
                SymbolKind::Endpoint,
                id,
            );
        }
    }

    fn visit_node_decl(&mut self, _ast: &Ast, id: NodeId, node_decl: &NodeDecl) {
        self.state.declare(
            self.program,
            self.unit,
            self.state.current_scope,
            node_decl.name,
            SymbolKind::Node,
            id,
        );
    }
}

struct Resolve<'s, 'p> {
    state: &'s mut State,
    program: &'p Program,
    unit: UnitId,
}

impl<'s, 'a> Resolve<'s, 'a> {
    fn with_new_scope_at<R>(
        &mut self,
        anchor: TokenId,
        f: impl FnOnce(&mut Self, ScopeId) -> R,
    ) -> R {
        let anchor = Anchor {
            unit: self.unit,
            token: anchor,
        };
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

    fn resolve_container(&mut self, ast: &Ast, id: NodeId, params: &[NodeId], items: &[NodeId]) {
        let inner = *self.state.node_scope[self.unit]
            .get(id)
            .expect("container scope is created during the declare pass");

        self.with_scope(inner, |this| {
            for &param in params {
                this.visit(ast, param);
            }

            for &member in items {
                this.visit(ast, member);
            }
        });
    }

    fn resolve_scope_path(&self, node: NodeId) -> Result<SymbolId> {
        let unit = &self.program[self.unit];
        let path_start = unit.location(unit.ast.span(node).start);
        let segments = self.scope_path_segments(node)?;
        let (&first, rest) = segments
            .split_first()
            .expect("a scope path has at least one segment");

        let mut symbol = self
            .state
            .symbols
            .find_visible(
                self.state.current_scope,
                self.state.has_matching_name(self.program, self.unit, first),
            )
            .ok_or_else(|| {
                (
                    path_start,
                    CompileError::UnresolvedSymbol {
                        name: self.written_path(&segments),
                    },
                )
            })?;

        for (index, &segment) in rest.iter().enumerate() {
            if matches!(
                self.state.symbols.symbol(symbol).kind,
                SymbolKind::Alias | SymbolKind::Generic
            ) {
                return Ok(symbol);
            }

            let scope = self.state.symbols.inner_scope(symbol).ok_or_else(|| {
                (
                    path_start,
                    CompileError::NotAScope {
                        name: self
                            .state
                            .symbols
                            .symbol(symbol)
                            .name(self.program)
                            .to_owned(),
                    },
                )
            })?;

            symbol = self
                .state
                .symbols
                .find_local(
                    scope,
                    self.state
                        .has_matching_name(self.program, self.unit, segment),
                )
                .ok_or_else(|| {
                    (
                        path_start,
                        CompileError::UnresolvedSymbol {
                            name: format!(
                                "{}::{}",
                                self.qualified_name(symbol),
                                self.written_path(&rest[index..])
                            ),
                        },
                    )
                })?;
        }

        Ok(symbol)
    }

    fn scope_path_segments(&self, node: NodeId) -> Result<Vec<TokenId>> {
        let unit = &self.program[self.unit];

        match unit.ast.get(node) {
            Node::Expr(Expr::Ident(ident)) => Ok(vec![ident.token]),
            Node::Expr(Expr::ScopeAccess(scope_access)) => {
                let Node::Expr(Expr::Ident(name)) = unit.ast.get(scope_access.name) else {
                    return Err((
                        unit.location(unit.ast.span(scope_access.name).start),
                        CompileError::UnexpectedNode,
                    ));
                };
                let mut segments = self.scope_path_segments(scope_access.base)?;
                segments.push(name.token);
                Ok(segments)
            }
            Node::Expr(Expr::Call(call)) => self.scope_path_segments(call.callee),
            _ => Err((
                unit.location(unit.ast.span(node).start),
                CompileError::UnexpectedNode,
            )),
        }
    }

    fn written_path(&self, segments: &[TokenId]) -> String {
        let unit = &self.program[self.unit];
        segments
            .iter()
            .map(|&segment| unit.text(segment))
            .collect::<Vec<_>>()
            .join("::")
    }

    fn qualified_name(&self, symbol: SymbolId) -> String {
        self.state
            .symbols
            .qualified_path(symbol)
            .into_iter()
            .map(|symbol| self.state.symbols.symbol(symbol).name(self.program))
            .collect::<Vec<_>>()
            .join("::")
    }
}

impl<'s, 'a> Visitor for Resolve<'s, 'a> {
    fn visit_namespace_decl(&mut self, ast: &Ast, id: NodeId, namespace_decl: &NamespaceDecl) {
        self.resolve_container(ast, id, &namespace_decl.params, &namespace_decl.items);
    }

    fn visit_processor_decl(&mut self, ast: &Ast, id: NodeId, processor_decl: &ProcessorDecl) {
        self.resolve_container(ast, id, &processor_decl.params, &processor_decl.items);
    }

    fn visit_graph_decl(&mut self, ast: &Ast, id: NodeId, graph_decl: &GraphDecl) {
        self.resolve_container(ast, id, &graph_decl.params, &graph_decl.items);
    }

    fn visit_struct_decl(&mut self, ast: &Ast, id: NodeId, struct_decl: &StructDecl) {
        self.resolve_container(ast, id, &[], &struct_decl.items);
    }

    fn visit_function_decl(&mut self, ast: &Ast, _id: NodeId, function_decl: &FunctionDecl) {
        self.with_new_scope_at(function_decl.name, |this, _| {
            for &generic in &function_decl.generics {
                this.visit(ast, generic);
            }
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
            for &generic in &event_handler.generics {
                this.visit(ast, generic);
            }
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
            self.state
                .declare_local(self.unit, declarator.name, SymbolKind::Variable, id);
        }
        var.walk(ast, self);
    }

    fn visit_typed_decl(&mut self, ast: &Ast, _id: NodeId, typed_decl: &TypedDecl) {
        for (id, declarator) in typed_decl.declarators(ast) {
            self.state
                .declare_local(self.unit, declarator.name, SymbolKind::Variable, id);
        }
        typed_decl.walk(ast, self);
    }

    fn visit_param(&mut self, ast: &Ast, id: NodeId, param: &Param) {
        self.state.declare(
            self.program,
            self.unit,
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
            self.program,
            self.unit,
            self.state.current_scope,
            specialisation_value.name,
            SymbolKind::Variable,
            id,
        );

        specialisation_value.walk(ast, self);
    }

    fn visit_alias(&mut self, ast: &Ast, id: NodeId, alias: &Alias) {
        self.state.declare(
            self.program,
            self.unit,
            self.state.current_scope,
            alias.name,
            SymbolKind::Alias,
            id,
        );

        alias.walk(ast, self);
    }

    fn visit_generic_param(&mut self, _ast: &Ast, id: NodeId, generic_param: &GenericParam) {
        self.state.declare(
            self.program,
            self.unit,
            self.state.current_scope,
            generic_param.name,
            SymbolKind::Generic,
            id,
        );
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
        let found = self
            .state
            .symbols
            .find_visible(
                self.state.current_scope,
                self.state
                    .has_matching_name(self.program, self.unit, ident.token),
            )
            .is_some()
            || self.state.intrinsics_scope.is_some_and(|scope| {
                self.state
                    .symbols
                    .find_local(
                        scope,
                        self.state
                            .has_matching_name(self.program, self.unit, ident.token),
                    )
                    .is_some()
            });

        if !found {
            self.state.error(
                self.unit,
                self.program[self.unit].location(ident.token),
                CompileError::UnresolvedSymbol {
                    name: self.program[self.unit].text(ident.token).to_owned(),
                },
            );
        }
    }

    fn visit_scope_access(&mut self, _ast: &Ast, id: NodeId, _scope_access: &ScopeAccess) {
        if let Err((location, err)) = self.resolve_scope_path(id) {
            self.state.error(self.unit, location, err);
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::SourceFile, indoc::indoc, view::ResolutionView};

    fn expect_resolution(source: &'_ str) -> ResolutionView {
        let mut program = Program::new();
        let unit = program.add_source_file(SourceFile::new(source));

        assert!(
            !program[unit].ast.has_errors(),
            "source failed to parse: {source}"
        );
        let resolution = resolve(&program);
        ResolutionView::new(program, resolution)
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
          - "6:11: error: The name 'P' is already in use //// 1:11: note: See previous declaration"
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
    fn local_shadowing_a_parameter_is_permitted() {
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
        "#
        );
    }

    #[test]
    fn local_shadowing_another_local_in_the_same_block_is_permitted() {
        assert_resolution!(
            indoc! {"
                processor P
                {
                    void main() { let a = 1; let a = 2; }
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
                  - name: a
                    kind: Variable
                    location: "3:23"
                  - name: a
                    kind: Variable
                    location: "3:34"
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
          - "4:9: error: The name 'x' is already in use //// 3:9: note: See previous declaration"
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
          - "9:10: error: The name 'a' is already in use //// 8:10: note: See previous declaration"
        "#
        );
    }

    #[test]
    fn unresolved_qualified_name_reports_the_full_path() {
        let program = Program::new().with_source_file(SourceFile::new(indoc! {"
            namespace blah {}
            processor P
            {
                void main() { let x = blah::cos (1.0f); }
            }
        "}));

        let errors: Vec<_> = resolve(&program)
            .diagnostics
            .into_iter()
            .map(|(_, diagnostic)| diagnostic.error)
            .collect();

        assert_eq!(
            errors,
            vec![CompileError::UnresolvedSymbol {
                name: "blah::cos".into()
            }]
        );
    }

    fn resolution_errors(source: &str) -> Vec<CompileError> {
        let program = Program::new().with_source_file(SourceFile::new(source));
        resolve(&program)
            .diagnostics
            .into_iter()
            .map(|(_, diagnostic)| diagnostic.error)
            .collect()
    }

    #[test]
    fn unresolved_member_is_named_by_its_qualified_path() {
        let errors = resolution_errors(indoc! {"
            namespace tests
            {
                struct S { int32 x; }
                bool f() { return S::y.isScalar; }
            }
        "});

        assert_eq!(
            errors,
            vec![CompileError::UnresolvedSymbol {
                name: "tests::S::y".into()
            }]
        );
    }

    #[test]
    fn unresolved_path_root_names_the_whole_path_as_written() {
        let errors = resolution_errors(indoc! {"
            processor P
            {
                void main() { let v = blah :: cos (1.0); }
            }
        "});

        assert_eq!(
            errors,
            vec![CompileError::UnresolvedSymbol {
                name: "blah::cos".into()
            }]
        );
    }

    #[test]
    fn not_a_scope_names_the_segment_that_has_no_members() {
        let errors = resolution_errors(indoc! {"
            namespace A { int B = 1; }
            processor P
            {
                void main() { let v = A::B::c; }
            }
        "});

        assert_eq!(errors, vec![CompileError::NotAScope { name: "B".into() }]);
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
          - "5:17: error: Cannot find symbol 'qty'"
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
          - "9:17: error: Cannot find symbol 'Utils::bogus'"
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
          - "7:27: error: Cannot find symbol 'Utils::bogus'"
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
          - "2:37: error: Cannot find symbol 'Point::z'"
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
          - "3:37: error: Cannot find symbol 'G::bogus'"
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
          - "2:37: error: Cannot find symbol 'Mode::Bogus'"
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
          - "2:37: error: Cannot find symbol 'A::B::bogus'"
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
          - "1:48: error: 'a' is not a namespace, processor, graph, struct, or enum"
        "#
        );
    }

    #[test]
    fn processor_using_specialisation_param_is_visible_to_members() {
        assert_resolution!(
            indoc! {"
                processor P (using T)
                {
                    T helper (T x) { return x; }
                    void main() { T y; }
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
              - name: T
                kind: Alias
                location: "1:20"
              - name: helper
                kind: Function
                location: "3:7"
              - name: main
                kind: Function
                location: "4:10"
            scopes:
              - location: "3:7"
                symbols:
                  - name: x
                    kind: Variable
                    location: "3:17"
              - location: "4:10"
                symbols:
                  - name: y
                    kind: Variable
                    location: "4:21"
        "#
        );
    }

    #[test]
    fn scope_access_on_a_generic_specialisation_param_is_deferred() {
        assert_resolution!(
            indoc! {"
                processor P (using SampleContent)
                {
                    output stream SampleContent::frames.elementType out;
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
              - name: SampleContent
                kind: Alias
                location: "1:20"
              - name: out
                kind: Endpoint
                location: "3:53"
        "#
        );
    }

    #[test]
    fn processor_value_specialisation_param_is_visible_to_members() {
        assert_resolution!(
            indoc! {"
                processor P (int N)
                {
                    void main() { int x = N; }
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
              - name: N
                kind: Variable
                location: "1:18"
              - name: main
                kind: Function
                location: "3:10"
            scopes:
              - location: "3:10"
                symbols:
                  - name: x
                    kind: Variable
                    location: "3:23"
        "#
        );
    }

    #[test]
    fn function_generic_param_is_visible_to_signature_and_body() {
        assert_resolution!(
            indoc! {"
                processor P
                {
                    T id<T> (T x) { return x; }
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
              - name: id
                kind: Function
                location: "3:7"
              - name: main
                kind: Function
                location: "4:10"
            scopes:
              - location: "3:7"
                symbols:
                  - name: T
                    kind: Generic
                    location: "3:10"
                  - name: x
                    kind: Variable
                    location: "3:16"
              - location: "4:10"
        "#
        );
    }

    #[test]
    fn local_using_alias_is_visible_after_its_declaration() {
        assert_resolution!(
            indoc! {"
                processor P
                {
                    void main()
                    {
                        using T = int;
                        T x = 1;
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
                  - name: T
                    kind: Alias
                    location: "5:15"
                  - name: x
                    kind: Variable
                    location: "6:11"
        "#
        );
    }

    mod multi_unit {
        use {super::*, crate::SourceFile};

        #[test]
        fn resolving_zero_units_does_not_panic() {
            let program = Program::new();
            let resolution = resolve(&program);
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
            let program = Program::new()
                .with_source_file(SourceFile::new(
                    "namespace std::math { int square (int x) { return x * x; } }",
                ))
                .with_source_file(SourceFile::new(
                    "processor P { void main() { int x = std::math::square (4); } }",
                ));

            for unit in program.units() {
                assert!(!program[unit].ast.has_errors());
            }

            let resolution = resolve(&program);

            assert!(
                resolution.diagnostics.is_empty(),
                "{:?}",
                resolution.diagnostics
            );
        }

        #[test]
        fn unqualified_call_falls_back_to_std_intrinsics() {
            let program = Program::new()
                .with_source_file(SourceFile::new(
                    "namespace std::intrinsics { T sum<T> (T value) { return value; } }",
                ))
                .with_source_file(SourceFile::new(
                    "processor P { void main() { let total = sum (1); } }",
                ));

            for unit in program.units() {
                assert!(!program[unit].ast.has_errors());
            }

            let resolution = resolve(&program);

            assert!(
                resolution.diagnostics.is_empty(),
                "{:?}",
                resolution.diagnostics
            );
        }

        #[test]
        fn namespace_reopened_across_units_shares_a_scope() {
            let program = Program::new()
                .with_source_file(SourceFile::new(
                    "namespace n { processor A { output stream int out; } }",
                ))
                .with_source_file(SourceFile::new(
                    "namespace n { processor B { output stream int out; } }",
                ));

            let resolution = resolve(&program);
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
            let program = Program::new()
                .with_source_file(SourceFile::new("namespace n { int f() { return g(); } }"))
                .with_source_file(SourceFile::new("namespace n { int g() { return 1; } }"));

            let resolution = resolve(&program);

            assert!(
                resolution.diagnostics.is_empty(),
                "{:?}",
                resolution.diagnostics
            );
        }

        #[test]
        fn redefinition_across_units_is_reported_against_the_second_unit() {
            let program = Program::new()
                .with_source_file(SourceFile::new("processor P { output stream int out; }"))
                .with_source_file(SourceFile::new("processor P { output stream int out; }"));

            let resolution = resolve(&program);

            assert_eq!(resolution.diagnostics.len(), 1);
            let (unit, diagnostic) = &resolution.diagnostics[0];
            assert!(
                matches!(&diagnostic.error, CompileError::NameInUse { name, .. } if name == "P"),
                "{diagnostic}"
            );

            let processor_units = source_units_in_declaration_order(&resolution);
            assert_eq!(processor_units.len(), 2);
            assert_ne!(processor_units[0], processor_units[1]);
            assert_eq!(*unit, processor_units[1]);
        }

        #[test]
        fn diagnostic_reports_the_unit_it_occurred_in() {
            let program = Program::new()
                .with_source_file(SourceFile::new("namespace n {}"))
                .with_source_file(SourceFile::new(
                    "processor P { void main() { int x = qty; } }",
                ));

            let resolution = resolve(&program);

            assert_eq!(resolution.diagnostics.len(), 1);

            let units = source_units_in_declaration_order(&resolution);
            assert_eq!(units.len(), 2);
            assert_eq!(resolution.diagnostics[0].0, units[1]);
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

        fn redeclare_stdlib_processor() -> ResolvedProgram {
            let stdlib = declare_stdlib(&Program::new().with_source_file(
                SourceFile::new("processor P { output stream int out; }").with_name("lib.cmajor"),
            ));
            let program = Program::new().with_source_file(
                SourceFile::new("processor P { output stream int out; }").with_name("main.cmajor"),
            );

            resolve_with_standard_library(program, &stdlib)
        }

        fn stdlib_declaring_p_twice() -> StandardLibrary {
            declare_stdlib(
                &Program::new()
                    .with_source_file(
                        SourceFile::new("processor P { output stream int out; }")
                            .with_name("a.cmajor"),
                    )
                    .with_source_file(
                        SourceFile::new("processor P { output stream int out; }")
                            .with_name("b.cmajor"),
                    ),
            )
        }

        #[test]
        fn stdlib_reports_its_own_declaration_diagnostics() {
            let stdlib = stdlib_declaring_p_twice();

            assert!(matches!(
                stdlib.diagnostics(),
                [(
                    _,
                    Diagnostic {
                        error: CompileError::NameInUse { .. },
                        ..
                    }
                )]
            ));
        }

        #[test]
        fn stdlib_declaration_diagnostics_are_not_reported_with_programs_resolved_against_it() {
            let stdlib = stdlib_declaring_p_twice();
            let program = Program::new()
                .with_source_file(SourceFile::new("processor Q { output stream int out; }"));

            let ResolvedProgram { resolution, .. } =
                resolve_with_standard_library(program, &stdlib);

            assert_eq!(resolution.diagnostics, vec![]);
        }

        #[test]
        fn stdlib_resolution_reports_the_callers_unit_in_the_returned_program() {
            let ResolvedProgram {
                program,
                resolution,
            } = redeclare_stdlib_processor();

            let [(unit, _)] = resolution.diagnostics.as_slice() else {
                panic!("expected one diagnostic: {:?}", resolution.diagnostics);
            };
            assert_eq!(program[*unit].source.name(), Some("main.cmajor"));
        }

        #[test]
        fn stdlib_resolution_note_points_at_the_stdlib_unit_in_the_returned_program() {
            let ResolvedProgram {
                program,
                resolution,
            } = redeclare_stdlib_processor();

            let [(_, diagnostic)] = resolution.diagnostics.as_slice() else {
                panic!("expected one diagnostic: {:?}", resolution.diagnostics);
            };
            let CompileError::NameInUse {
                previous_declaration: Some(previous),
                ..
            } = &diagnostic.error
            else {
                panic!("expected a name clash with a previous declaration: {diagnostic}");
            };
            assert_eq!(program[previous.unit].source.name(), Some("lib.cmajor"));
        }

        #[test]
        fn reparsing_only_the_edited_unit_still_resolves_correctly() {
            let a_src = "namespace n { int helper() { return 1; } }";

            let program_v1 = Program::new()
                .with_source_file(SourceFile::new(a_src))
                .with_source_file(SourceFile::new(
                    "processor P { void main() { int x = 1; } }",
                ));
            let resolution_v1 = resolve(&program_v1);
            assert!(
                resolution_v1.diagnostics.is_empty(),
                "{:?}",
                resolution_v1.diagnostics
            );

            let program_v2 = Program::new()
                .with_source_file(SourceFile::new(a_src))
                .with_source_file(SourceFile::new(
                    "processor P { void main() { int x = n::helper(); } }",
                ));
            let resolution_v2 = resolve(&program_v2);
            assert!(
                resolution_v2.diagnostics.is_empty(),
                "{:?}",
                resolution_v2.diagnostics
            );
        }
    }
}
