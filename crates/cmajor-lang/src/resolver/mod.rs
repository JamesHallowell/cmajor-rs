mod symbol;

pub use symbol::{ScopeId, Symbol, SymbolId, SymbolKind, SymbolTable};

use crate::{
    Diagnostic,
    ast::{Ast, Decl, Graph, Item, Node, NodeId, Stmt},
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
    for &root in &parse.roots {
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
            Item::NamespaceDecl {
                segments, items, ..
            } => {
                let inner = segments.iter().fold(scope, |scope, &segment| {
                    self.declare_or_reuse_namespace(scope, segment, id)
                });
                for &member in items {
                    self.declare_member(inner, member);
                }
            }
            Item::ProcessorDecl { name, items, .. } => {
                self.declare_container(scope, *name, SymbolKind::Processor, id, items);
            }
            Item::GraphDecl { name, items, .. } => {
                self.declare_container(scope, *name, SymbolKind::Graph, id, items);
            }
            Item::StructDecl { name, items, .. } => {
                self.declare_container(scope, *name, SymbolKind::Struct, id, items);
            }
            Item::EnumDecl { name, values, .. } => {
                let symbol = self.declare(scope, *name, SymbolKind::Enum, id);
                let inner = self.table.child_scope(symbol, scope);
                for &value in values {
                    self.declare(inner, value, SymbolKind::EnumValue, id);
                }
            }
            Item::FunctionDecl { name, body, .. } => {
                let children = if let Node::Stmt(Stmt::Block { stmts, .. }) = self.ast.get(*body) {
                    stmts
                } else {
                    &vec![]
                };

                self.declare_container(scope, *name, SymbolKind::Function, id, children);
            }
            Item::ModuleAlias { name, .. } => {
                self.declare(scope, *name, SymbolKind::Alias, id);
            }
            Item::Import { .. } => {}
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
        let inner = self.table.child_scope(symbol, scope);
        inner
    }

    fn declare_member(&mut self, scope: ScopeId, id: NodeId) {
        match self.ast.get(id).clone() {
            Node::Item(_) => self.declare_item(scope, id),
            Node::Decl(decl) => self.declare_decl(scope, id, &decl),
            Node::Graph(graph) => self.declare_graph(scope, id, &graph),
            Node::Stmt(Stmt::DeclStmt { decl }) => self.declare_member(scope, decl),
            Node::Stmt(_) | Node::Expr(_) | Node::Error { .. } => {}
        }
    }

    fn declare_decl(&mut self, scope: ScopeId, id: NodeId, decl: &Decl) {
        match decl {
            Decl::Var { declarators, .. } => {
                for declarator in declarators {
                    self.declare(scope, declarator.name, SymbolKind::Variable, id);
                }
            }
            Decl::Alias { name, .. } => {
                self.declare(scope, *name, SymbolKind::Alias, id);
            }
        }
    }

    fn declare_graph(&mut self, scope: ScopeId, id: NodeId, member: &Graph) {
        match member {
            &Graph::NodeDecl { name, .. } => {
                self.declare(scope, name, SymbolKind::Node, id);
            }
            &Graph::EndpointDecl {
                name: Some(name), ..
            } => {
                self.declare(scope, name, SymbolKind::Endpoint, id);
            }
            Graph::EndpointDecl { name: None, .. }
            | Graph::ConnectionDecl { .. }
            | Graph::Connection { .. }
            | Graph::ConnectionIf { .. } => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::parser, indoc::indoc};

    fn expect_diagnostic(program: &str) -> Diagnostic {
        let (_, mut diagnostics) = resolve_source(program);
        assert_eq!(diagnostics.len(), 1);
        diagnostics.remove(0)
    }

    macro_rules! assert_error {
        ($program:expr, @$error:literal) => {
            insta::assert_snapshot!(expect_diagnostic($program), @$error);
        };
    }

    fn resolve_source(source: &str) -> (SymbolTable, Vec<Diagnostic>) {
        let parse = parser::parse(source);
        assert!(!parse.ast.has_errors(), "source failed to parse: {source}");
        let resolution = resolve(source, &parse);
        (resolution.table, resolution.diagnostics)
    }

    #[test]
    fn declares_top_level_processor() {
        let (table, diagnostics) = resolve_source(indoc! {"
            processor P
            {
                output stream int out;
                void main() {}
            }
        "});
        assert!(diagnostics.is_empty());

        let global = table.global_scope();
        let symbols: Vec<_> = table.symbols_in(global).collect();
        assert_eq!(symbols.len(), 1);
        assert_eq!(symbols[0].name, "P");
        assert_eq!(symbols[0].kind, SymbolKind::Processor);
    }

    #[test]
    fn resolve_symbols_in_nested_scopes() {
        let (table, diagnostics) = resolve_source(indoc! {"
            processor P
            {
                int helper() { return 42; }
                void main() { int x = helper(); }
            }
        "});
        assert!(diagnostics.is_empty());

        let processor_scope = table
            .lookup_local(table.global_scope(), "P")
            .map(|id| table.symbol(id))
            .and_then(|s| s.inner_scope)
            .expect("processor P should be declared");

        let symbols = table.symbols_in(processor_scope).collect::<Vec<_>>();
        assert_eq!(symbols.len(), 2);

        let helper = symbols[0];
        assert_eq!(helper.name, "helper");
        assert_eq!(helper.kind, SymbolKind::Function);

        let main = symbols[1];
        assert_eq!(main.name, "main");
        assert_eq!(main.kind, SymbolKind::Function);
        assert!(main.inner_scope.is_some());

        let symbols = table
            .symbols_in(main.inner_scope.unwrap())
            .collect::<Vec<_>>();
        assert_eq!(symbols.len(), 1);

        let x = symbols[0];
        assert_eq!(x.name, "x");
        assert_eq!(x.kind, SymbolKind::Variable);
    }

    #[test]
    fn duplicate_processor_names_are_reported() {
        assert_error!(
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
            @"6:11: redefinition of 'P' (previously declared at 1:11)"
        );
    }

    #[test]
    fn function_overloads_do_not_conflict() {
        let (table, diagnostics) = resolve_source(indoc! {"
            processor P
            {
                void f(int x) {}
                void f(float x) {}
                output stream int out;
            }
        "});
        assert!(diagnostics.is_empty());

        let global = table.global_scope();
        let processor = table.symbols_in(global).next().unwrap();
        let inner = processor.inner_scope.unwrap();
        let functions: Vec<_> = table
            .symbols_in(inner)
            .filter(|s| s.kind == SymbolKind::Function)
            .collect();
        assert_eq!(functions.len(), 2);
    }

    #[test]
    fn reopened_namespaces_share_a_scope() {
        let (table, diagnostics) = resolve_source(indoc! {"
            namespace n
            {
                processor A { output stream int out; }
            }

            namespace n
            {
                processor B { output stream int out; }
            }
        "});
        assert!(diagnostics.is_empty());

        let global = table.global_scope();
        let namespaces: Vec<_> = table.symbols_in(global).collect();
        assert_eq!(namespaces.len(), 1);

        let inner = namespaces[0].inner_scope.unwrap();
        let members: Vec<_> = table.symbols_in(inner).map(|s| s.name.as_str()).collect();
        assert_eq!(members, vec!["A", "B"]);
    }

    #[test]
    fn duplicate_state_variables_are_reported() {
        assert_error!(
            indoc! {"
                processor P
                {
                    int x;
                    int x;
                    output stream int out;
                    void main() {}
                }
            "},
            @"4:9: redefinition of 'x' (previously declared at 3:9)"
        );
    }

    #[test]
    fn duplicate_node_names_in_a_graph_are_reported() {
        assert_error!(
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
            @"9:10: redefinition of 'a' (previously declared at 8:10)"
        );
    }
}
