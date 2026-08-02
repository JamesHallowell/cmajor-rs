mod symbol;
#[cfg(test)]
mod view;

pub use symbol::{ScopeId, Symbol, SymbolId, SymbolKind, SymbolTable};

use crate::{
    Diagnostic,
    ast::{
        Alias, Assign, Ast, Binary, Block, Bracketed, Call, Connection, ConnectionDecl,
        ConnectionIf, Decl, DeclStmt, EndpointDecl, EnumDecl, Expr, Field, FunctionDecl, Graph,
        GraphDecl, Ident, Import, Item, ModuleAlias, NamespaceDecl, Node, NodeDecl, NodeId,
        Parentheses, PostfixUnary, ProcessorDecl, ProcessorProperty, ScopeAccess, Stmt, StructDecl,
        Ternary, TypeModifier, Unary, Var, VectorSizeSuffix,
    },
    lexer::{TokenId, TokenStream},
    parser::Parse,
    utils,
};

pub struct Resolution {
    pub table: SymbolTable,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn resolve(source: &str, parse: &Parse) -> Resolution {
    let mut resolver = Resolver {
        ast: &parse.ast,
        tokens: &parse.tokens,
        source,
        table: SymbolTable::new(),
        diagnostics: Vec::new(),
    };

    let global = resolver.table.global_scope();
    for &root in parse.ast.roots() {
        resolver.declare_item(global, root);
    }

    Resolution {
        table: resolver.table,
        diagnostics: resolver.diagnostics,
    }
}

struct Resolver<'a> {
    ast: &'a Ast,
    tokens: &'a TokenStream,
    source: &'a str,
    table: SymbolTable,
    diagnostics: Vec<Diagnostic>,
}

impl<'a> Resolver<'a> {
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
            && let Some(existing) = self.table.lookup_local(scope, &name)
        {
            let existing = self.table.symbol(existing).name_token;
            let (line, column) = self.location(existing);
            self.error(
                name_token,
                format!("redefinition of '{name}' (previously declared at {line}:{column})"),
            );
        }

        self.table.declare(scope, name, kind, node, name_token)
    }

    fn location(&self, token: TokenId) -> (utils::Line, utils::Column) {
        utils::line_col(self.source, self.tokens.position(token))
    }

    fn declare_item(&mut self, scope: ScopeId, id: NodeId) {
        let Node::Item(item) = self.ast.get(id) else {
            return;
        };

        match item {
            Item::NamespaceDecl(NamespaceDecl {
                segments, items, ..
            }) => {
                let inner = segments.iter().fold(scope, |scope, &segment| {
                    self.declare_or_reuse_namespace(scope, segment, id)
                });
                for &member in items {
                    self.declare_member(inner, member);
                }
            }
            Item::ProcessorDecl(ProcessorDecl { name, items, .. }) => {
                self.declare_container(scope, *name, SymbolKind::Processor, id, items);
            }
            Item::GraphDecl(GraphDecl { name, items, .. }) => {
                self.declare_container(scope, *name, SymbolKind::Graph, id, items);
            }
            Item::StructDecl(StructDecl { name, items, .. }) => {
                self.declare_container(scope, *name, SymbolKind::Struct, id, items);
            }
            Item::EnumDecl(EnumDecl { name, values, .. }) => {
                let symbol = self.declare(scope, *name, SymbolKind::Enum, id);
                let inner = self.table.child_scope(symbol, scope);
                for &value in values {
                    self.declare(inner, value, SymbolKind::EnumValue, id);
                }
            }
            Item::FunctionDecl(FunctionDecl { name, body, .. }) => {
                let children =
                    if let Node::Stmt(Stmt::Block(Block { stmts, .. })) = self.ast.get(*body) {
                        stmts
                    } else {
                        &vec![]
                    };

                self.declare_container(scope, *name, SymbolKind::Function, id, children);
            }
            Item::ModuleAlias(ModuleAlias { name, .. }) => {
                self.declare(scope, *name, SymbolKind::Alias, id);
            }
            Item::Import(Import { .. }) => {}
        }
    }

    fn declare_container(
        &mut self,
        scope: ScopeId,
        name: TokenId,
        kind: SymbolKind,
        id: NodeId,
        items: &Vec<NodeId>,
    ) {
        let symbol = self.declare(scope, name, kind, id);
        let inner = self.table.child_scope(symbol, scope);
        for &member in items {
            self.declare_member(inner, member);
        }
    }

    fn declare_or_reuse_namespace(
        &mut self,
        scope: ScopeId,
        segment: TokenId,
        node: NodeId,
    ) -> ScopeId {
        let name = self.name(segment);

        if let Some(existing) = self.table.lookup_local(scope, &name) {
            let existing = self.table.symbol(existing);
            if existing.kind == SymbolKind::Namespace {
                return existing
                    .inner_scope
                    .expect("namespace symbol always has an inner scope");
            }
            self.error(
                segment,
                format!("'{name}' is already declared and is not a namespace"),
            );
        }

        let symbol = self
            .table
            .declare(scope, name, SymbolKind::Namespace, node, segment);
        self.table.child_scope(symbol, scope)
    }

    fn declare_member(&mut self, scope: ScopeId, id: NodeId) {
        match self.ast.get(id).clone() {
            Node::Item(_) => self.declare_item(scope, id),
            Node::Decl(decl) => self.declare_decl(scope, id, &decl),
            Node::Graph(graph) => self.declare_graph(scope, id, &graph),
            Node::Stmt(Stmt::DeclStmt(DeclStmt { decl })) => self.declare_member(scope, decl),
            Node::Stmt(_) | Node::Expr(_) | Node::AttributeList(_) | Node::Error { .. } => {}
        }
    }

    fn declare_decl(&mut self, scope: ScopeId, id: NodeId, decl: &Decl) {
        match decl {
            Decl::Var(Var { declarators, .. }) => {
                for declarator in declarators {
                    self.declare(scope, declarator.name, SymbolKind::Variable, id);

                    if let Some(init) = declarator.init
                        && let Node::Expr(expr) = self.ast.get(init)
                    {
                        self.visit_expr(scope, expr);
                    }
                }
            }
            Decl::Alias(Alias { name, .. }) => {
                self.declare(scope, *name, SymbolKind::Alias, id);
            }
        }
    }

    fn declare_graph(&mut self, scope: ScopeId, id: NodeId, member: &Graph) {
        match member {
            &Graph::NodeDecl(NodeDecl { name, .. }) => {
                self.declare(scope, name, SymbolKind::Node, id);
            }
            &Graph::EndpointDecl(EndpointDecl {
                name: Some(name), ..
            }) => {
                self.declare(scope, name, SymbolKind::Endpoint, id);
            }
            Graph::EndpointDecl(EndpointDecl { name: None, .. })
            | Graph::ConnectionDecl(ConnectionDecl { .. })
            | Graph::Connection(Connection { .. })
            | Graph::ConnectionIf(ConnectionIf { .. }) => {}
        }
    }

    fn visit_expr(&mut self, scope: ScopeId, expr: &Expr) {
        match expr {
            Expr::Literal(_) => {}
            Expr::Ident(Ident { token }) => self.check_ident_defined(scope, *token),
            Expr::Parentheses(Parentheses { .. }) => {}
            Expr::Unary(Unary { .. }) => {}
            Expr::PostfixUnary(PostfixUnary { .. }) => {}
            Expr::Binary(Binary { .. }) => {}
            Expr::Assign(Assign { .. }) => {}
            Expr::Ternary(Ternary { .. }) => {}
            Expr::Call(Call { .. }) => {}
            Expr::Bracketed(Bracketed { .. }) => {}
            Expr::Field(Field { .. }) => {}
            Expr::ScopeAccess(ScopeAccess { .. }) => {}
            Expr::TypeModifier(TypeModifier { .. }) => {}
            Expr::VectorSizeSuffix(VectorSizeSuffix { .. }) => {}
            Expr::ProcessorProperty(ProcessorProperty { .. }) => {}
        }
    }

    fn check_ident_defined(&mut self, scope: ScopeId, ident: TokenId) {
        let name = self
            .tokens
            .text(self.source, ident)
            .expect("token should be in source");

        if self.table.lookup_visible(scope, name).is_none() {
            self.error(ident, format!("undeclared identifier '{name}'"));
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
            members:
              - name: out
                kind: Endpoint
                location: "3:23"
              - name: main
                kind: Function
                location: "4:10"
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
            members:
              - name: helper
                kind: Function
                location: "3:9"
              - name: main
                kind: Function
                location: "4:10"
                members:
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
            members:
              - name: out
                kind: Endpoint
                location: "3:23"
          - name: P
            kind: Processor
            location: "6:11"
            members:
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
            members:
              - name: f
                kind: Function
                location: "3:10"
              - name: f
                kind: Function
                location: "4:10"
              - name: out
                kind: Endpoint
                location: "5:23"
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
            members:
              - name: A
                kind: Processor
                location: "3:15"
                members:
                  - name: out
                    kind: Endpoint
                    location: "3:37"
              - name: B
                kind: Processor
                location: "8:15"
                members:
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
            members:
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
            members:
              - name: out
                kind: Endpoint
                location: "3:23"
          - name: G
            kind: Graph
            location: "6:7"
            members:
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
            members:
              - name: main
                kind: Function
                location: "3:10"
                members:
                  - name: x
                    kind: Variable
                    location: "5:13"
        diagnostics:
          - "5:17: undeclared identifier 'qty'"
        "#);
    }
}
