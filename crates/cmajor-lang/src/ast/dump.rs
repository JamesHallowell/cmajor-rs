use {
    crate::{
        ast::{
            AliasKind, Annotation, Annotations, Ast, Decl, EventHandlerDecl, Expr, Graph,
            HoistTarget, InterpolationKind, Item, NodeId, Stmt, VarRole,
            child::ChildList,
            decl::{Alias, External, Var},
            expr::{
                Assign, Binary, Bracketed, Call, Field, Ident, Parentheses, PostfixUnary,
                ProcessorProperty, ScopeAccess, Slice, Ternary, TypeModifier, Unary,
                VectorSizeSuffix,
            },
            graph::{
                Connection, ConnectionDecl, ConnectionIf, EndpointDeclaration,
                HoistedEndpointDeclaration, NodeDecl,
            },
            item::{
                EnumDecl, FunctionDecl, GraphDecl, Import, ModuleAlias, NamespaceDecl,
                ProcessorDecl, StructDecl,
            },
            stmt::{
                Block, BreakStmt, ContinueStmt, DeclStmt, ExprStmt, ForStmt, ForwardBranchStmt,
                IfConstStmt, IfStmt, LoopStmt, ReturnStmt, WhileStmt,
            },
            visit::ExhaustiveVisitor,
        },
        lexer::{TokenId, TokenStream},
        utils::source::Source,
    },
    std::fmt::Write as _,
};

pub fn dump(ast: &Ast, tokens: &TokenStream, source: &str, root: NodeId) -> String {
    let mut dumper = Dumper {
        ast,
        tokens,
        source: source.into(),
    };
    let mut root_node = dumper.visit(ast, root);
    append_span(&mut root_node, ast, tokens, root);

    let mut out = String::new();
    render(&root_node, 0, &mut out);
    out
}

fn append_span(node: &mut DumpNode, ast: &Ast, tokens: &TokenStream, id: NodeId) {
    let span = tokens.to_positions(ast.span(id));
    let _ = write!(node.label, " {}..{}", span.start, span.end);
}

struct DumpNode {
    label: String,
    children: Vec<DumpNode>,
}

fn render(node: &DumpNode, depth: usize, out: &mut String) {
    let indent = "  ".repeat(depth);
    let _ = writeln!(out, "{indent}{}", node.label);
    for child in &node.children {
        render(child, depth + 1, out);
    }
}

fn leaf(label: String) -> DumpNode {
    DumpNode {
        label,
        children: vec![],
    }
}

fn node(label: String, children: Vec<DumpNode>) -> DumpNode {
    DumpNode { label, children }
}

struct Dumper<'a> {
    ast: &'a Ast,
    tokens: &'a TokenStream,
    source: Source<'a>,
}

impl<'a> Dumper<'a> {
    fn text(&'a self, token: &TokenId) -> &'a str {
        let span = self.tokens.span(*token);
        &self.source[span]
    }

    fn child(&mut self, id: NodeId) -> DumpNode {
        let mut node = self.visit(self.ast, id);
        append_span(&mut node, self.ast, self.tokens, id);
        node
    }

    fn children(&mut self, child_list: ChildList) -> Vec<DumpNode> {
        self.ast
            .children(child_list)
            .iter()
            .map(|&child| self.child(child))
            .collect()
    }

    fn annotations(&mut self, annotations: &Annotations) -> Option<DumpNode> {
        annotations
            .get()
            .map(|annotations| node("Annotations".to_owned(), self.children(annotations)))
    }
}

fn role_label(role: &VarRole) -> &'static str {
    match role {
        VarRole::Let => "let",
        VarRole::Var => "var",
        VarRole::Typed => "typed",
        VarRole::Parameter => "param",
        VarRole::SpecialisationValue => "specialisation",
    }
}

fn alias_kind_label(kind: &AliasKind) -> &'static str {
    match kind {
        AliasKind::Using => "using",
        AliasKind::Processor => "processor",
        AliasKind::Namespace => "namespace",
    }
}

fn interpolation_label(kind: &InterpolationKind) -> &'static str {
    match kind {
        InterpolationKind::None => "none",
        InterpolationKind::Latch => "latch",
        InterpolationKind::Linear => "linear",
        InterpolationKind::Sinc => "sinc",
        InterpolationKind::Fast => "fast",
        InterpolationKind::Best => "best",
    }
}

impl<'a> ExhaustiveVisitor for Dumper<'a> {
    type Output = DumpNode;

    fn visit_item(&mut self, _ast: &Ast, _id: NodeId, item: &Item) -> DumpNode {
        match item {
            Item::NamespaceDecl(NamespaceDecl {
                segments,
                params,
                items,
                annotations,
                ..
            }) => {
                let path = segments
                    .iter()
                    .map(|t| self.text(t))
                    .collect::<Vec<_>>()
                    .join("::");
                let mut children: Vec<_> = params.iter().map(|&id| self.child(id)).collect();
                if let Some(annotations) = self.annotations(annotations) {
                    children.push(annotations);
                }
                children.extend(items.iter().map(|&id| self.child(id)));
                node(format!("NamespaceDecl {path:?}"), children)
            }
            Item::ProcessorDecl(ProcessorDecl {
                name,
                params,
                annotations,
                items,
                ..
            }) => {
                let mut children: Vec<_> = params.iter().map(|&id| self.child(id)).collect();
                if let Some(annotations) = self.annotations(annotations) {
                    children.push(annotations);
                }
                children.extend(items.iter().map(|&id| self.child(id)));
                node(format!("ProcessorDecl {:?}", self.text(name)), children)
            }
            Item::GraphDecl(GraphDecl {
                name,
                params,
                annotations,
                items,
                ..
            }) => {
                let mut children: Vec<_> = params.iter().map(|&id| self.child(id)).collect();
                if let Some(annotations) = self.annotations(annotations) {
                    children.push(annotations);
                }
                children.extend(items.iter().map(|&id| self.child(id)));
                node(format!("GraphDecl {:?}", self.text(name)), children)
            }
            Item::StructDecl(StructDecl {
                name,
                annotations,
                items,
                ..
            }) => {
                let mut children = vec![];
                if let Some(annotations) = self.annotations(annotations) {
                    children.push(annotations);
                }
                children.extend(items.iter().map(|&id| self.child(id)));
                node(format!("StructDecl {:?}", self.text(name)), children)
            }
            Item::EnumDecl(EnumDecl { name, values, .. }) => {
                let values = values
                    .iter()
                    .map(|t| self.text(t))
                    .collect::<Vec<_>>()
                    .join(", ");
                leaf(format!("EnumDecl {:?} {{{values}}}", self.text(name)))
            }
            Item::FunctionDecl(FunctionDecl {
                returns,
                name,
                generics,
                params,
                is_const,
                annotations,
                body,
            }) => {
                let generics_suffix = if generics.is_empty() {
                    String::new()
                } else {
                    format!(
                        "<{}>",
                        generics
                            .iter()
                            .map(|t| self.text(t))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                let const_suffix = if *is_const { " const" } else { "" };
                let mut children = vec![self.child(*returns)];
                children.extend(params.iter().map(|&id| self.child(id)));
                if let Some(annotations) = self.annotations(annotations) {
                    children.push(annotations);
                }
                children.push(self.child(*body));
                node(
                    format!(
                        "FunctionDecl {:?}{generics_suffix}{const_suffix}",
                        self.text(name)
                    ),
                    children,
                )
            }
            Item::EventHandlerDecl(EventHandlerDecl {
                name,
                generics,
                params,
                is_const,
                annotations,
                body,
            }) => {
                let generics_suffix = if generics.is_empty() {
                    String::new()
                } else {
                    format!(
                        "<{}>",
                        generics
                            .iter()
                            .map(|t| self.text(t))
                            .collect::<Vec<_>>()
                            .join(", ")
                    )
                };
                let const_suffix = if *is_const { " const" } else { "" };
                let mut children = vec![];
                children.extend(params.iter().map(|&id| self.child(id)));
                if let Some(annotations) = self.annotations(annotations) {
                    children.push(annotations);
                }
                children.push(self.child(*body));
                node(
                    format!(
                        "EventHandlerDecl {:?}{generics_suffix}{const_suffix}",
                        self.text(name)
                    ),
                    children,
                )
            }
            Item::Import(Import { path, .. }) => {
                let path = path
                    .iter()
                    .map(|t| self.text(t))
                    .collect::<Vec<_>>()
                    .join(".");
                leaf(format!("Import {path:?}"))
            }
            Item::ModuleAlias(ModuleAlias {
                kind, name, target, ..
            }) => {
                let label = format!(
                    "ModuleAlias {} {:?}",
                    alias_kind_label(kind),
                    self.text(name)
                );
                let target = self.child(*target);
                node(label, vec![target])
            }
        }
    }

    fn visit_decl(&mut self, ast: &Ast, _id: NodeId, decl: &Decl) -> DumpNode {
        match decl {
            Decl::Var(Var {
                role,
                ty,
                declarators,
                annotations,
            }) => {
                let names = declarators
                    .iter()
                    .map(|d| self.text(&d.name))
                    .collect::<Vec<_>>()
                    .join(", ");
                let mut children: Vec<_> = ty.map(|id| self.child(id)).into_iter().collect();
                if let Some(annotations) = self.annotations(annotations) {
                    children.push(annotations);
                }
                children.extend(
                    declarators
                        .iter()
                        .filter_map(|d| d.init.map(|id| self.child(id))),
                );
                node(format!("VarDecl {} {names:?}", role_label(role)), children)
            }
            Decl::Alias(Alias {
                kind, name, target, ..
            }) => {
                let label = format!("Alias {} {:?}", alias_kind_label(kind), self.text(name));
                let children = target.map(|id| self.child(id)).into_iter().collect();
                node(label, children)
            }
            Decl::External(External {
                ty,
                names,
                annotations,
            }) => {
                let label = "External".to_string();
                let mut children = vec![self.child(*ty)];
                children.extend(ast.children(*names).iter().map(|&node| self.child(node)));
                if let Some(annotations) = self.annotations(annotations) {
                    children.push(annotations);
                }
                node(label, children)
            }
        }
    }

    fn visit_graph(&mut self, _ast: &Ast, _id: NodeId, member: &Graph) -> DumpNode {
        match member {
            Graph::EndpointDeclaration(EndpointDeclaration {
                direction,
                kind,
                types,
                name,
                size,
                annotations,
            }) => {
                let dir = self.text(direction);
                let kind_text = self.text(kind);
                let name_text = self.text(name);
                let label = format!("EndpointDecl {dir} {kind_text} {name_text:?}");
                let mut children: Vec<_> = types.iter().map(|&id| self.child(id)).collect();
                children.extend(size.map(|id| self.child(id)));
                if let Some(annotations) = self.annotations(annotations) {
                    children.push(annotations);
                }
                node(label, children)
            }
            Graph::HoistedEndpointDeclaration(HoistedEndpointDeclaration {
                direction,
                segments,
                index,
                target,
                annotations,
                ..
            }) => {
                let dir = self.text(direction);
                let path = segments
                    .iter()
                    .map(|t| self.text(t))
                    .collect::<Vec<_>>()
                    .join(".");
                let target = match target {
                    HoistTarget::Name(name) => self.text(name).to_string(),
                    HoistTarget::Wildcard {
                        prefix: Some(prefix),
                    } => {
                        format!("{}*", self.text(prefix))
                    }
                    HoistTarget::Wildcard { prefix: None } => "*".to_string(),
                };
                let label = format!("EndpointDecl {dir} {path}.{target}");
                let mut children: Vec<_> = index.map(|id| self.child(id)).into_iter().collect();
                if let Some(annotations) = self.annotations(annotations) {
                    children.push(annotations);
                }
                node(label, children)
            }
            Graph::NodeDecl(NodeDecl {
                name,
                processor,
                array_size,
                ..
            }) => {
                let mut children: Vec<_> =
                    array_size.map(|id| self.child(id)).into_iter().collect();
                children.push(self.child(*processor));
                node(format!("NodeDecl {:?}", self.text(name)), children)
            }
            Graph::ConnectionDecl(ConnectionDecl { connections, .. }) => {
                let children = connections.iter().map(|&id| self.child(id)).collect();
                node("ConnectionDecl".to_string(), children)
            }
            Graph::Connection(Connection {
                interpolation,
                sources,
                delay,
                destinations,
                ..
            }) => {
                let label = match interpolation {
                    Some(kind) => format!("Connection [{}]", interpolation_label(kind)),
                    None => "Connection".to_string(),
                };
                let sources = self
                    .ast
                    .children(*sources)
                    .iter()
                    .map(|&id| self.child(id))
                    .collect();
                let mut children = vec![node("Sources".to_string(), sources)];
                if let Some(delay) = delay {
                    let delay = self.child(*delay);
                    children.push(node("Delay".to_string(), vec![delay]));
                }
                let destinations = self
                    .ast
                    .children(*destinations)
                    .iter()
                    .map(|&id| self.child(id))
                    .collect();
                children.push(node("Destinations".to_string(), destinations));
                node(label, children)
            }
            Graph::ConnectionIf(ConnectionIf {
                cond,
                then_branch,
                else_branch,
                ..
            }) => {
                let cond = self.child(*cond);
                let then_branch = then_branch.iter().map(|&id| self.child(id)).collect();
                let mut children = vec![cond, node("Then".to_string(), then_branch)];
                if let Some(else_branch) = else_branch {
                    let else_branch = else_branch.iter().map(|&id| self.child(id)).collect();
                    children.push(node("Else".to_string(), else_branch));
                }
                node("ConnectionIf".to_string(), children)
            }
        }
    }

    fn visit_stmt(&mut self, _ast: &Ast, id: NodeId, stmt: &Stmt) -> DumpNode {
        match stmt {
            Stmt::Block(Block { stmts }) => {
                let prefix = self
                    .ast
                    .label(id)
                    .map(|l| format!(" {:?}", self.text(&l)))
                    .unwrap_or_default();
                let children = stmts
                    .map(|stmts| self.ast.children(stmts))
                    .into_iter()
                    .flatten()
                    .map(|&child| self.child(child))
                    .collect();
                node(format!("Block{prefix}"), children)
            }
            Stmt::ExprStmt(ExprStmt { expr }) => {
                let expr = self.child(*expr);
                node("ExprStmt".to_string(), vec![expr])
            }
            Stmt::DeclStmt(DeclStmt { decl }) => self.visit(self.ast, *decl),
            Stmt::ForStmt(ForStmt {
                init,
                cond,
                update,
                body,
                ..
            }) => {
                let prefix = self
                    .ast
                    .label(id)
                    .map(|l| format!(" {:?}", self.text(&l)))
                    .unwrap_or_default();
                let mut children: Vec<_> = init.map(|id| self.child(id)).into_iter().collect();
                children.extend(cond.map(|id| self.child(id)));
                children.extend(update.map(|id| self.child(id)));
                children.push(self.child(*body));
                node(format!("ForStmt{prefix}"), children)
            }
            Stmt::IfConstStmt(IfConstStmt {
                cond,
                then_branch,
                else_branch,
                ..
            }) => {
                let mut children = vec![self.child(*cond), self.child(*then_branch)];
                children.extend(else_branch.map(|id| self.child(id)));
                node("IfConstStmt".to_string(), children)
            }
            Stmt::IfStmt(IfStmt {
                cond,
                then_branch,
                else_branch,
                ..
            }) => {
                let mut children = vec![self.child(*cond), self.child(*then_branch)];
                children.extend(else_branch.map(|id| self.child(id)));
                node("IfStmt".to_string(), children)
            }
            Stmt::WhileStmt(WhileStmt { cond, body, .. }) => {
                let prefix = self
                    .ast
                    .label(id)
                    .map(|l| format!(" {:?}", self.text(&l)))
                    .unwrap_or_default();
                let cond = self.child(*cond);
                let body = self.child(*body);
                node(format!("WhileStmt{prefix}"), vec![cond, body])
            }
            Stmt::LoopStmt(LoopStmt { count, body, .. }) => {
                let prefix = self
                    .ast
                    .label(id)
                    .map(|l| format!(" {:?}", self.text(&l)))
                    .unwrap_or_default();
                let mut children: Vec<_> = count.map(|id| self.child(id)).into_iter().collect();
                children.push(self.child(*body));
                node(format!("LoopStmt{prefix}"), children)
            }
            Stmt::ReturnStmt(ReturnStmt { value, .. }) => {
                let children = value.map(|id| self.child(id)).into_iter().collect();
                node("ReturnStmt".to_string(), children)
            }
            Stmt::BreakStmt(BreakStmt { target, .. }) => {
                let suffix = target
                    .map(|t| format!(" {:?}", self.text(&t)))
                    .unwrap_or_default();
                leaf(format!("BreakStmt{suffix}"))
            }
            Stmt::ContinueStmt(ContinueStmt { target, .. }) => {
                let suffix = target
                    .map(|t| format!(" {:?}", self.text(&t)))
                    .unwrap_or_default();
                leaf(format!("ContinueStmt{suffix}"))
            }
            Stmt::ForwardBranchStmt(ForwardBranchStmt { cond, targets, .. }) => {
                let mut children = vec![self.child(*cond)];
                children.extend(
                    targets
                        .map(|targets| self.ast.children(targets))
                        .into_iter()
                        .flatten()
                        .map(|&id| self.child(id)),
                );
                node("ForwardBranchStmt".to_string(), children)
            }
        }
    }

    fn visit_expr(&mut self, _ast: &Ast, _id: NodeId, expr: &Expr) -> DumpNode {
        match expr {
            Expr::Literal(literal) => leaf(self.text(literal.token()).to_string()),
            &Expr::Ident(Ident { token }) => leaf(self.text(&token).to_string()),
            Expr::Parentheses(Parentheses { inner, .. }) => {
                let children = inner
                    .map(|inner| self.ast.children(inner))
                    .into_iter()
                    .flatten()
                    .map(|&id| self.child(id))
                    .collect();
                node("Parentheses".to_string(), children)
            }
            Expr::Unary(Unary { op, operand }) => {
                let label = format!("Unary {:?}", self.text(op));
                let operand = self.child(*operand);
                node(label, vec![operand])
            }
            Expr::PostfixUnary(PostfixUnary { op, operand }) => {
                let label = format!("PostfixUnary {:?}", self.text(op));
                let operand = self.child(*operand);
                node(label, vec![operand])
            }
            Expr::Binary(Binary { op, lhs, rhs }) => {
                let label = format!("Binary {:?}", self.text(op));
                let lhs = self.child(*lhs);
                let rhs = self.child(*rhs);
                node(label, vec![lhs, rhs])
            }
            Expr::Assign(Assign { op, target, value }) => {
                let label = format!("Assign {:?}", self.text(op));
                let target = self.child(*target);
                let value = self.child(*value);
                node(label, vec![target, value])
            }
            Expr::Ternary(Ternary {
                cond,
                then_branch,
                else_branch,
                ..
            }) => {
                let cond = self.child(*cond);
                let then_branch = self.child(*then_branch);
                let else_branch = self.child(*else_branch);
                node("Ternary".to_string(), vec![cond, then_branch, else_branch])
            }
            Expr::Call(Call { callee, args, .. }) => {
                let mut children = vec![self.child(*callee)];
                children.extend(
                    args.map(|children| self.ast.children(children))
                        .into_iter()
                        .flatten()
                        .map(|&id| self.child(id)),
                );
                node("Call".to_string(), children)
            }
            Expr::Bracketed(Bracketed { base, terms, .. }) => {
                let mut children = vec![self.child(*base)];
                children.extend(
                    terms
                        .map(|terms| self.ast.children(terms))
                        .into_iter()
                        .flatten()
                        .map(|&id| self.child(id)),
                );
                node("Bracketed".to_string(), children)
            }
            Expr::Slice(Slice { start, end }) => {
                let mut children = Vec::new();
                children.extend(start.map(|id| self.child(id)));
                children.extend(end.map(|id| self.child(id)));
                node("Slice".to_string(), children)
            }
            Expr::Field(Field { name, base }) => {
                let label = format!("Field {:?}", self.text(name));
                let base = self.child(*base);
                node(label, vec![base])
            }
            Expr::ScopeAccess(ScopeAccess { name, base }) => {
                let label = format!("ScopeAccess {:?}", self.text(name));
                let base = self.child(*base);
                node(label, vec![base])
            }
            Expr::TypeModifier(TypeModifier {
                source,
                is_const,
                is_ref,
            }) => {
                let mut label = "TypeModifier".to_string();
                if *is_const {
                    label.push_str(" const");
                }
                if *is_ref {
                    label.push_str(" ref");
                }
                let source = self.child(*source);
                node(label, vec![source])
            }
            Expr::VectorSizeSuffix(VectorSizeSuffix { element, terms, .. }) => {
                let mut children = vec![self.child(*element)];
                children.extend(self.ast.children(*terms).iter().map(|&id| self.child(id)));
                node("VectorSizeSuffix".to_string(), children)
            }
            Expr::ProcessorProperty(ProcessorProperty { name }) => {
                leaf(format!("ProcessorProperty {:?}", self.text(name)))
            }
        }
    }

    fn visit_annotation(&mut self, _ast: &Ast, _id: NodeId, annotation: &Annotation) -> DumpNode {
        let key_text = self.text(&annotation.key);
        match annotation.value {
            Some(id) => node(format!("{key_text:?}"), vec![self.child(id)]),
            None => leaf(format!("{key_text:?}")),
        }
    }

    fn visit_error(&mut self, _ast: &Ast, _id: NodeId, token: TokenId) -> DumpNode {
        leaf(format!("Error at {token:?}"))
    }
}
