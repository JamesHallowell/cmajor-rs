mod scope;
mod symbol;
#[cfg(test)]
mod view;

pub use {
    scope::{Scope, ScopeId},
    symbol::{Symbol, SymbolId, SymbolKind, SymbolTable},
};

use crate::{
    Diagnostic,
    ast::{
        Alias, Ast, Block, EndpointDeclaration, EnumDecl, Expr, ForStmt, FunctionDecl, GraphDecl,
        HoistedEndpointDeclaration, IfStmt, LoopStmt, ModuleAlias, NamespaceDecl, Node, NodeDecl,
        NodeId, Param, ProcessorDecl, ScopeAccess, SpecialisationValue, Stmt, StructDecl,
        TypedDecl, Var, WhileStmt,
        visit::{Visitor, Walk},
    },
    lexer::{TokenId, TokenKind, TokenStream},
    parser::Parse,
    utils::{
        arena::SparseSecondaryArena,
        source::{Source, SourceLocation},
        span::Span,
    },
};

pub struct Resolution {
    pub symbols: SymbolTable,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn resolve(source: &str, parse: &Parse) -> Resolution {
    let mut resolver = Resolver::new(source, &parse.tokens, &parse.ast);

    parse.ast.visit(&mut resolver);

    Resolution {
        symbols: resolver.symbols,
        diagnostics: resolver.diagnostics,
    }
}

#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("undeclared identifier '{name}'")]
    UndeclaredIdentifier { name: String },

    #[error("redefinition of '{name}' (previously declared at {location})")]
    RedefinedIdentifier {
        name: String,
        location: SourceLocation,
    },

    #[error("unexpected node")]
    UnexpectedNode,

    #[error("'{name}' is not a namespace, processor, graph, struct, or enum")]
    NotAScope { name: String },
}

type Result<T> = std::result::Result<T, (Span<SourceLocation>, Error)>;

struct Resolver<'a> {
    ast: &'a Ast,
    tokens: &'a TokenStream,
    source: Source<'a>,
    symbols: SymbolTable,
    diagnostics: Vec<Diagnostic>,
    current_scope: ScopeId,
    symbols_scope: SparseSecondaryArena<SymbolId, ScopeId>,
}

impl<'a> Resolver<'a> {
    pub fn new(source: &'a str, tokens: &'a TokenStream, ast: &'a Ast) -> Self {
        let symbols = SymbolTable::new();
        let global_scope = symbols.global_scope();

        Resolver {
            ast,
            tokens,
            source: source.into(),
            symbols,
            diagnostics: Vec::new(),
            current_scope: global_scope,
            symbols_scope: SparseSecondaryArena::default(),
        }
    }

    fn error(&mut self, location: Span<SourceLocation>, err: Error) {
        self.diagnostics.push(Diagnostic {
            location,
            message: err.to_string(),
        });
    }

    fn text(&self, token: TokenId) -> &str {
        &self.source[self.tokens.span(token)]
    }

    fn declare(
        &mut self,
        scope: ScopeId,
        name_token: TokenId,
        kind: SymbolKind,
        node: NodeId,
    ) -> SymbolId {
        let name = self.text(name_token).to_owned();

        if !kind.allows_duplicates()
            && let Some(existing) = self.symbols.lookup_local(scope, &name)
        {
            let existing = self.symbols.symbol(existing).name_token;
            let location = self
                .tokens
                .span(existing)
                .to_source_location_span(&self.source)
                .start;

            self.error(
                self.tokens
                    .span(name_token)
                    .to_source_location_span(&self.source),
                Error::RedefinedIdentifier {
                    name: name.clone(),
                    location,
                },
            );
        }

        self.symbols.declare(scope, kind, node, name, name_token)
    }

    fn declare_container(
        &mut self,
        keyword: TokenId,
        name: TokenId,
        kind: SymbolKind,
        id: NodeId,
        items: &[NodeId],
    ) {
        let symbol = self.declare(self.current_scope, name, kind, id);

        self.with_new_scope_at(keyword, |this, inner| {
            this.symbols_scope.insert(symbol, inner);
            let ast = this.ast;
            for &member in items {
                this.current_scope = inner;
                this.visit(ast, member);
            }
        });
    }

    fn declare_or_reuse_namespace(
        &mut self,
        scope: ScopeId,
        keyword: TokenId,
        segment: TokenId,
        node: NodeId,
    ) -> ScopeId {
        let name = self.text(segment).to_owned();

        if let Some(existing_id) = self.symbols.lookup_local(scope, &name) {
            let existing = self.symbols.symbol(existing_id);
            if existing.kind == SymbolKind::Namespace {
                return *self
                    .symbols_scope
                    .get(existing_id)
                    .expect("namespace symbol always has an inner scope");
            }

            let error_location = self
                .tokens
                .span(segment)
                .to_source_location_span(&self.source);

            self.error(
                error_location,
                Error::RedefinedIdentifier {
                    name: name.clone(),
                    location: self
                        .tokens
                        .span(existing.name_token)
                        .to_source_location_span(&self.source)
                        .start,
                },
            );
        }

        let symbol = self
            .symbols
            .declare(scope, SymbolKind::Namespace, node, name, segment);
        let inner = self.symbols.new_scope(scope, keyword);
        self.symbols_scope.insert(symbol, inner);
        inner
    }

    fn with_new_scope_at<R>(
        &mut self,
        anchor: TokenId,
        f: impl FnOnce(&mut Self, ScopeId) -> R,
    ) -> R {
        let parent = self.current_scope;
        let inner = self.symbols.new_scope(parent, anchor);
        self.current_scope = inner;
        let result = f(self, inner);
        self.current_scope = parent;
        result
    }

    fn check_ident_defined(&mut self, scope: ScopeId, ident: TokenId) {
        if let TokenKind::Keyword(keyword) = self.tokens.get(ident).kind
            && keyword.is_type()
        {
            return;
        }

        let name = self.text(ident);
        if self.symbols.lookup_visible(scope, name).is_none() {
            self.error(
                self.tokens
                    .span(ident)
                    .to_source_location_span(&self.source),
                Error::UndeclaredIdentifier {
                    name: name.to_owned(),
                },
            );
        }
    }

    fn resolve_scope_path(&self, node: NodeId) -> Result<SymbolId> {
        match self.ast.get(node) {
            Node::Expr(Expr::Ident(ident)) => {
                let symbol = self
                    .symbols
                    .lookup_visible(self.current_scope, self.text(ident.token))
                    .ok_or((
                        self.tokens
                            .span(ident.token)
                            .to_source_location_span(&self.source),
                        Error::UndeclaredIdentifier {
                            name: self.text(ident.token).to_owned(),
                        },
                    ))?;

                Ok(symbol)
            }
            Node::Expr(Expr::ScopeAccess(scope_access)) => {
                let base = self.resolve_scope_path(scope_access.base)?;

                let base_location = self
                    .ast
                    .span(scope_access.base)
                    .to_source_location_span(self.tokens, &self.source);

                let base_scope = *self.symbols_scope.get(base).ok_or_else(|| {
                    (
                        base_location,
                        Error::NotAScope {
                            name: self.source[base_location].to_owned(),
                        },
                    )
                })?;

                let Node::Expr(Expr::Ident(name)) = self.ast.get(scope_access.name) else {
                    return Err((
                        self.ast
                            .span(scope_access.name)
                            .to_source_location_span(self.tokens, &self.source),
                        Error::UnexpectedNode,
                    ));
                };

                let member = self
                    .symbols
                    .lookup_local(base_scope, self.text(name.token))
                    .ok_or((
                        self.tokens
                            .span(name.token)
                            .to_source_location_span(&self.source),
                        Error::UndeclaredIdentifier {
                            name: self.text(name.token).to_owned(),
                        },
                    ))?;

                Ok(member)
            }
            _ => {
                let location = self
                    .ast
                    .span(node)
                    .to_source_location_span(self.tokens, &self.source);

                Err((location, Error::UnexpectedNode))
            }
        }
    }
}

impl<'a> Visitor for Resolver<'a> {
    fn visit_root(&mut self, ast: &Ast, id: NodeId) {
        self.current_scope = self.symbols.global_scope();
        self.visit(ast, id);
    }

    fn visit_namespace_decl(&mut self, ast: &Ast, id: NodeId, namespace_decl: &NamespaceDecl) {
        let scope = self.current_scope;
        let inner = namespace_decl
            .segments
            .iter()
            .fold(scope, |scope, &segment| {
                self.declare_or_reuse_namespace(scope, namespace_decl.keyword, segment, id)
            });
        for &member in &namespace_decl.items {
            self.current_scope = inner;
            self.visit(ast, member);
        }
    }

    fn visit_processor_decl(&mut self, _ast: &Ast, id: NodeId, processor_decl: &ProcessorDecl) {
        self.declare_container(
            processor_decl.keyword,
            processor_decl.name,
            SymbolKind::Processor,
            id,
            &processor_decl.items,
        );
    }

    fn visit_graph_decl(&mut self, _ast: &Ast, id: NodeId, graph_decl: &GraphDecl) {
        self.declare_container(
            graph_decl.keyword,
            graph_decl.name,
            SymbolKind::Graph,
            id,
            &graph_decl.items,
        );
    }

    fn visit_struct_decl(&mut self, _ast: &Ast, id: NodeId, struct_decl: &StructDecl) {
        self.declare_container(
            struct_decl.keyword,
            struct_decl.name,
            SymbolKind::Struct,
            id,
            &struct_decl.items,
        );
    }

    fn visit_enum_decl(&mut self, _ast: &Ast, id: NodeId, enum_decl: &EnumDecl) {
        let scope = self.current_scope;
        let symbol = self.declare(scope, enum_decl.name, SymbolKind::Enum, id);
        let inner = self.symbols.new_scope(scope, enum_decl.keyword);
        self.symbols_scope.insert(symbol, inner);
        for &value in &enum_decl.values {
            self.declare(inner, value, SymbolKind::EnumValue, id);
        }
    }

    fn visit_function_decl(&mut self, ast: &Ast, id: NodeId, function_decl: &FunctionDecl) {
        self.declare(
            self.current_scope,
            function_decl.name,
            SymbolKind::Function,
            id,
        );

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

    fn visit_module_alias(&mut self, _ast: &Ast, id: NodeId, module_alias: &ModuleAlias) {
        self.declare(self.current_scope, module_alias.name, SymbolKind::Alias, id);
    }

    fn visit_var(&mut self, ast: &Ast, id: NodeId, var: &Var) {
        let _ = id;

        for (id, declarator) in var.declarators(ast) {
            self.declare(
                self.current_scope,
                declarator.name,
                SymbolKind::Variable,
                id,
            );
        }
        var.walk(ast, self);
    }

    fn visit_typed_decl(&mut self, ast: &Ast, id: NodeId, typed_decl: &TypedDecl) {
        let _ = id;
        for (id, declarator) in typed_decl.declarators(ast) {
            self.declare(
                self.current_scope,
                declarator.name,
                SymbolKind::Variable,
                id,
            );
        }
        typed_decl.walk(ast, self);
    }

    fn visit_param(&mut self, ast: &Ast, id: NodeId, param: &Param) {
        self.declare(self.current_scope, param.name, SymbolKind::Variable, id);

        param.walk(ast, self);
    }

    fn visit_specialisation_value(
        &mut self,
        ast: &Ast,
        id: NodeId,
        specialisation_value: &SpecialisationValue,
    ) {
        self.declare(
            self.current_scope,
            specialisation_value.name,
            SymbolKind::Variable,
            id,
        );

        specialisation_value.walk(ast, self);
    }

    fn visit_alias(&mut self, _ast: &Ast, id: NodeId, alias: &Alias) {
        self.declare(self.current_scope, alias.name, SymbolKind::Alias, id);
    }

    fn visit_endpoint_declaration(
        &mut self,
        _ast: &Ast,
        id: NodeId,
        endpoint_declaration: &EndpointDeclaration,
    ) {
        self.declare(
            self.current_scope,
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
            self.declare(self.current_scope, name, SymbolKind::Endpoint, id);
        }
    }

    fn visit_node_decl(&mut self, _ast: &Ast, id: NodeId, node_decl: &NodeDecl) {
        self.declare(self.current_scope, node_decl.name, SymbolKind::Node, id);
    }

    fn visit_block(&mut self, ast: &Ast, id: NodeId, block: &Block) {
        let scope_start = self.ast.span(id).start;
        self.with_new_scope_at(scope_start, |this, _| block.walk(ast, this));
    }

    fn visit_for_stmt(&mut self, ast: &Ast, id: NodeId, for_stmt: &ForStmt) {
        let scope_start = self.ast.span(id).start;
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
        let scope_start = self.ast.span(id).start;
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

    fn visit_ident(&mut self, _ast: &Ast, _id: NodeId, token: TokenId) {
        self.check_ident_defined(self.current_scope, token);
    }

    fn visit_scope_access(&mut self, _ast: &Ast, id: NodeId, _scope_access: &ScopeAccess) {
        if let Err((location, err)) = self.resolve_scope_path(id) {
            self.diagnostics.push(Diagnostic {
                location,
                message: err.to_string(),
            });
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::parser, indoc::indoc, view::ResolutionView};

    fn expect_resolution(source: &'_ str) -> ResolutionView<'_> {
        let parse = parser::parse(source);
        assert!(!parse.ast.has_errors(), "source failed to parse: {source}");
        let resolution = resolve(source, &parse);
        ResolutionView::new(resolution, parse.tokens, source)
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
}
