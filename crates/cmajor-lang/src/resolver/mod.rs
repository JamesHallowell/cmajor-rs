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
        Alias, Ast, Block, EndpointDecl, EnumDecl, FunctionDecl, GraphDecl, ModuleAlias,
        NamespaceDecl, NodeDecl, NodeId, ProcessorDecl, StructDecl, Var, visit::Visitor,
    },
    lexer::{TokenId, TokenStream},
    parser::Parse,
    utils::{self, Column, Line, arena::SparseSecondaryArena},
};

pub struct Resolution {
    pub symbols: SymbolTable,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn resolve(source: &str, parse: &Parse) -> Resolution {
    let mut resolver = Resolver::new(source, &parse.tokens, &parse.ast);

    for &root in parse.ast.roots() {
        resolver.current_scope = resolver.symbols.global_scope();
        resolver.visit(&parse.ast, root);
    }

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
    pending_owner: Option<SymbolId>,
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
            pending_owner: None,
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

    fn name(&self, token: TokenId) -> String {
        self.tokens
            .text(self.source, token)
            .expect("name token has source text")
            .to_string()
    }

    fn declare(
        &mut self,
        scope: ScopeId,
        name_token: TokenId,
        kind: SymbolKind,
        node: NodeId,
    ) -> SymbolId {
        let name = self.name(name_token);

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

        self.symbols.declare(scope, name, kind, node, name_token)
    }

    fn location(&self, token: TokenId) -> (Line, Column) {
        utils::line_col(self.source, self.tokens.position(token))
    }

    fn declare_container(
        &mut self,
        scope: ScopeId,
        keyword: TokenId,
        name: TokenId,
        kind: SymbolKind,
        id: NodeId,
        items: &[NodeId],
    ) {
        let _symbol = self.declare(scope, name, kind, id);
        let inner = self.symbols.new_scope(scope, keyword);

        let ast = self.ast;
        for &member in items {
            self.current_scope = inner;
            self.visit(ast, member);
        }
    }

    fn declare_or_reuse_namespace(
        &mut self,
        scope: ScopeId,
        keyword: TokenId,
        segment: TokenId,
        node: NodeId,
    ) -> ScopeId {
        let name = self.name(segment);

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
            .declare(scope, name, SymbolKind::Namespace, node, segment);
        let inner = self.symbols.new_scope(scope, keyword);
        self.namespace_scopes.insert(symbol, inner);
        inner
    }

    fn check_ident_defined(&mut self, scope: ScopeId, ident: TokenId) {
        let name = self
            .tokens
            .text(self.source, ident)
            .expect("token should be in source");

        if self.symbols.lookup_visible(scope, name).is_none() {
            self.error(ident, format!("undeclared identifier '{name}'"));
        }
    }
}

impl<'a> Visitor for Resolver<'a> {
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
            self.scope(),
            processor_decl.keyword,
            processor_decl.name,
            SymbolKind::Processor,
            id,
            &processor_decl.items,
        );
    }

    fn visit_graph_decl(&mut self, _ast: &Ast, id: NodeId, graph_decl: &GraphDecl) {
        self.declare_container(
            self.scope(),
            graph_decl.keyword,
            graph_decl.name,
            SymbolKind::Graph,
            id,
            &graph_decl.items,
        );
    }

    fn visit_struct_decl(&mut self, _ast: &Ast, id: NodeId, struct_decl: &StructDecl) {
        self.declare_container(
            self.scope(),
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
        let symbol = self.declare(self.scope(), function_decl.name, SymbolKind::Function, id);
        self.pending_owner = Some(symbol);

        self.current_scope = self.symbols.new_scope(self.scope(), function_decl.name);

        for &param in &function_decl.params {
            self.visit(ast, param);
        }

        self.visit(ast, function_decl.body);
    }

    fn visit_module_alias(&mut self, _ast: &Ast, id: NodeId, module_alias: &ModuleAlias) {
        self.declare(self.scope(), module_alias.name, SymbolKind::Alias, id);
    }

    fn visit_var(&mut self, ast: &Ast, id: NodeId, var: &Var) {
        for declarator in &var.declarators {
            self.declare(self.scope(), declarator.name, SymbolKind::Variable, id);

            if let Some(init) = declarator.init {
                self.visit(ast, init);
            }
        }
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

    fn visit_block(&mut self, ast: &Ast, _id: NodeId, block: &Block) {
        let parent = self.scope();
        let inner = match self.pending_owner.take() {
            Some(_) => self.current_scope,
            None => self.symbols.new_scope(parent, block.brace),
        };
        self.current_scope = inner;

        for &stmt in &block.stmts {
            self.visit(ast, stmt);
        }
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
}
