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
        Alias, Ast, Block, EndpointDecl, EnumDecl, ForStmt, FunctionDecl, GraphDecl, IfStmt,
        LoopStmt, ModuleAlias, NamespaceDecl, Node, NodeDecl, NodeId, ProcessorDecl, Stmt,
        StructDecl, Var, WhileStmt,
        visit::{Visitor, Walk},
    },
    lexer::{TokenId, TokenKind, TokenStream},
    parser::Parse,
    utils::{self, Column, Line, arena::SparseSecondaryArena},
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

struct Resolver<'a> {
    ast: &'a Ast,
    tokens: &'a TokenStream,
    source: &'a str,
    symbols: SymbolTable,
    diagnostics: Vec<Diagnostic>,
    current_scope: ScopeId,
    namespace_scopes: SparseSecondaryArena<SymbolId, ScopeId>,
}

impl<'a> Resolver<'a> {
    pub fn new(source: &'a str, tokens: &'a TokenStream, ast: &'a Ast) -> Self {
        let symbols = SymbolTable::new();
        let global_scope = symbols.global_scope();

        Resolver {
            ast,
            tokens,
            source,
            symbols,
            diagnostics: Vec::new(),
            current_scope: global_scope,
            namespace_scopes: SparseSecondaryArena::default(),
        }
    }

    fn scope(&self) -> ScopeId {
        self.current_scope
    }

    fn error(&mut self, token: TokenId, message: impl Into<String>) {
        self.diagnostics.push(Diagnostic::at_token(
            self.source,
            self.tokens,
            token,
            message,
        ))
    }

    fn name(&self, token: TokenId) -> &str {
        self.tokens.text(self.source, token)
    }

    fn declare(
        &mut self,
        scope: ScopeId,
        name_token: TokenId,
        kind: SymbolKind,
        node: NodeId,
    ) -> SymbolId {
        let name = self.name(name_token).to_owned();

        if !kind.allows_duplicates()
            && let Some(existing) = self.symbols.lookup_local(scope, &name)
        {
            let existing = self.symbols.symbol(existing).name_token;
            let (line, column) = self.location(existing);
            self.error(
                name_token,
                format!("redefinition of '{name}' (previously declared at {line}:{column})"),
            );
        }

        self.symbols.declare(scope, kind, node, name, name_token)
    }

    fn location(&self, token: TokenId) -> (Line, Column) {
        utils::line_col(self.source, self.tokens.span(token).start)
    }

    fn declare_container(
        &mut self,
        keyword: TokenId,
        name: TokenId,
        kind: SymbolKind,
        id: NodeId,
        items: &[NodeId],
    ) {
        let _symbol = self.declare(self.scope(), name, kind, id);

        self.with_new_scope_at(keyword, |this, inner| {
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
        let name = self.name(segment).to_owned();

        if let Some(existing_id) = self.symbols.lookup_local(scope, &name) {
            let existing = self.symbols.symbol(existing_id);
            if existing.kind == SymbolKind::Namespace {
                return *self
                    .namespace_scopes
                    .get(existing_id)
                    .expect("namespace symbol always has an inner scope");
            }
            self.error(
                segment,
                format!("'{name}' is already declared and is not a namespace"),
            );
        }

        let symbol = self
            .symbols
            .declare(scope, SymbolKind::Namespace, node, name, segment);
        let inner = self.symbols.new_scope(scope, keyword);
        self.namespace_scopes.insert(symbol, inner);
        inner
    }

    fn with_new_scope_at<R>(
        &mut self,
        anchor: TokenId,
        f: impl FnOnce(&mut Self, ScopeId) -> R,
    ) -> R {
        let parent = self.scope();
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

        let name = self.tokens.text(self.source, ident);

        if self.symbols.lookup_visible(scope, name).is_none() {
            self.error(ident, format!("undeclared identifier '{name}'"));
        }
    }
}

impl<'a> Visitor for Resolver<'a> {
    fn visit_root(&mut self, ast: &Ast, id: NodeId) {
        self.current_scope = self.symbols.global_scope();
        self.visit(ast, id);
    }

    fn visit_namespace_decl(&mut self, ast: &Ast, id: NodeId, namespace_decl: &NamespaceDecl) {
        let scope = self.scope();
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
        let scope = self.scope();
        let _symbol = self.declare(scope, enum_decl.name, SymbolKind::Enum, id);
        let inner = self.symbols.new_scope(scope, enum_decl.keyword);
        for &value in &enum_decl.values {
            self.declare(inner, value, SymbolKind::EnumValue, id);
        }
    }

    fn visit_function_decl(&mut self, ast: &Ast, id: NodeId, function_decl: &FunctionDecl) {
        self.declare(self.scope(), function_decl.name, SymbolKind::Function, id);

        self.with_new_scope_at(function_decl.name, |this, _| {
            if let Some(ty) = function_decl.ty {
                this.visit(ast, ty);
            }
            for &param in &function_decl.params {
                this.visit(ast, param);
            }
            if let Some(attributes) = function_decl.attributes {
                this.visit(ast, attributes);
            }

            let Node::Stmt(Stmt::Block(body)) = ast.get(function_decl.body) else {
                unreachable!("function body is always a block")
            };
            body.walk(ast, this);
        });
    }

    fn visit_module_alias(&mut self, _ast: &Ast, id: NodeId, module_alias: &ModuleAlias) {
        self.declare(self.scope(), module_alias.name, SymbolKind::Alias, id);
    }

    fn visit_var(&mut self, ast: &Ast, id: NodeId, var: &Var) {
        for declarator in &var.declarators {
            self.declare(self.scope(), declarator.name, SymbolKind::Variable, id);
        }

        var.walk(ast, self);
    }

    fn visit_alias(&mut self, _ast: &Ast, id: NodeId, alias: &Alias) {
        self.declare(self.scope(), alias.name, SymbolKind::Alias, id);
    }

    fn visit_endpoint_decl(&mut self, _ast: &Ast, id: NodeId, endpoint_decl: &EndpointDecl) {
        if let Some(name) = endpoint_decl.name {
            self.declare(self.scope(), name, SymbolKind::Endpoint, id);
        }
    }

    fn visit_node_decl(&mut self, _ast: &Ast, id: NodeId, node_decl: &NodeDecl) {
        self.declare(self.scope(), node_decl.name, SymbolKind::Node, id);
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
        self.check_ident_defined(self.scope(), token);
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
}
