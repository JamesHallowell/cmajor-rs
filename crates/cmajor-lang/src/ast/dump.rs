use {
    crate::{
        ast::{
            AliasKind, Ast, BracketTerm, Decl, Expr, Graph, HoistTarget, InterpolationKind, Item,
            Node, NodeId, Stmt, VarRole,
        },
        lexer::{TokenId, TokenStream},
    },
    std::fmt::Write as _,
};

pub fn dump(ast: &Ast, tokens: &TokenStream, source: &str, root: NodeId) -> String {
    let node = write_node(ast, tokens, source, root);
    let mut out = String::new();
    render(&node, 0, &mut out);
    out
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

fn attribute_list_child(
    ast: &Ast,
    tokens: &TokenStream,
    source: &str,
    attrs: &Option<Vec<(TokenId, Option<NodeId>)>>,
) -> Option<DumpNode> {
    let text = |token: &TokenId| tokens.text(source, *token).expect("token is valid");
    attrs.as_ref().map(|attrs| {
        let children = attrs
            .iter()
            .map(|(key, value)| {
                let child = value
                    .as_ref()
                    .map(|id| write_node(ast, tokens, source, *id));
                node(format!("{:?}", text(key)), child.into_iter().collect())
            })
            .collect();
        node("AttributeList".to_string(), children)
    })
}

fn write_node(ast: &Ast, tokens: &TokenStream, source: &str, id: NodeId) -> DumpNode {
    match ast.get(id) {
        Node::Expr(expr) => write_expr(ast, tokens, source, expr),
        Node::Stmt(stmt) => write_stmt(ast, tokens, source, stmt),
        Node::Decl(decl) => write_decl(ast, tokens, source, decl),
        Node::Item(item) => write_item(ast, tokens, source, item),
        Node::Graph(member) => write_graph_member(ast, tokens, source, member),
        Node::Error { token } => leaf(format!("Error at {token:?}")),
    }
}

fn write_bracket_term(
    ast: &Ast,
    tokens: &TokenStream,
    source: &str,
    term: &BracketTerm,
) -> DumpNode {
    let child = |id: &NodeId| write_node(ast, tokens, source, *id);
    if term.is_range {
        let mut children = Vec::new();
        children.extend(term.start.iter().map(child));
        children.extend(term.end.iter().map(child));
        node("Slice".to_string(), children)
    } else if let Some(start) = &term.start {
        child(start)
    } else {
        leaf("_".to_string())
    }
}

fn write_expr(ast: &Ast, tokens: &TokenStream, source: &str, expr: &Expr) -> DumpNode {
    let text = |token: &TokenId| tokens.text(source, *token).expect("token is valid");
    let child = |id: &NodeId| write_node(ast, tokens, source, *id);

    match expr {
        Expr::Literal(literal) => leaf(text(literal.token()).to_string()),
        Expr::Ident { token } => leaf(text(token).to_string()),
        Expr::Parentheses { inner, .. } => {
            node("Parentheses".to_string(), inner.iter().map(child).collect())
        }
        Expr::Unary { op, operand } => node(format!("Unary {:?}", text(op)), vec![child(operand)]),
        Expr::PostfixUnary { op, operand } => {
            node(format!("PostfixUnary {:?}", text(op)), vec![child(operand)])
        }
        Expr::Binary { op, lhs, rhs } => node(
            format!("Binary {:?}", text(op)),
            vec![child(lhs), child(rhs)],
        ),
        Expr::Assign { op, target, value } => node(
            format!("Assign {:?}", text(op)),
            vec![child(target), child(value)],
        ),
        Expr::Ternary {
            cond,
            then_branch,
            else_branch,
            ..
        } => node(
            "Ternary".to_string(),
            vec![child(cond), child(then_branch), child(else_branch)],
        ),
        Expr::Call { callee, args, .. } => {
            let mut children = vec![child(callee)];
            children.extend(args.iter().map(child));
            node("Call".to_string(), children)
        }
        Expr::Bracketed { base, terms, .. } => {
            let mut children = vec![child(base)];
            children.extend(
                terms
                    .iter()
                    .map(|term| write_bracket_term(ast, tokens, source, term)),
            );
            node("Bracketed".to_string(), children)
        }
        Expr::Field { name, base } => node(format!("Field {:?}", text(name)), vec![child(base)]),
        Expr::ScopeAccess { name, base } => {
            node(format!("ScopeAccess {:?}", text(name)), vec![child(base)])
        }
        Expr::TypeModifier {
            source: inner,
            is_const,
            is_ref,
        } => {
            let mut label = "TypeModifier".to_string();
            if *is_const {
                label.push_str(" const");
            }
            if *is_ref {
                label.push_str(" ref");
            }
            node(label, vec![child(inner)])
        }
        Expr::VectorSizeSuffix { element, terms, .. } => {
            let mut children = vec![child(element)];
            children.extend(terms.iter().map(child));
            node("VectorSizeSuffix".to_string(), children)
        }
        Expr::ProcessorProperty { name } => leaf(format!("ProcessorProperty {:?}", text(name))),
    }
}

fn write_stmt(ast: &Ast, tokens: &TokenStream, source: &str, stmt: &Stmt) -> DumpNode {
    let text = |token: &TokenId| tokens.text(source, *token).expect("token is valid");
    let child = |id: &NodeId| write_node(ast, tokens, source, *id);

    match stmt {
        Stmt::Block { stmts, label, .. } => {
            let prefix = label
                .map(|l| format!(" {:?}", text(&l)))
                .unwrap_or_default();
            node(format!("Block{prefix}"), stmts.iter().map(child).collect())
        }
        Stmt::ExprStmt { expr } => node("ExprStmt".to_string(), vec![child(expr)]),
        // Transparent: the wrapped `Decl` already renders its own label.
        Stmt::DeclStmt { decl } => child(decl),
        Stmt::ForStmt {
            init,
            cond,
            update,
            body,
            label,
            ..
        } => {
            let prefix = label
                .map(|l| format!(" {:?}", text(&l)))
                .unwrap_or_default();
            let mut children: Vec<_> = init.iter().map(child).collect();
            children.extend(cond.iter().map(child));
            children.extend(update.iter().map(child));
            children.push(child(body));
            node(format!("ForStmt{prefix}"), children)
        }
        Stmt::IfStmt {
            is_const,
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            let label = if *is_const { "IfStmt const" } else { "IfStmt" };
            let mut children = vec![child(cond), child(then_branch)];
            children.extend(else_branch.iter().map(child));
            node(label.to_string(), children)
        }
        Stmt::WhileStmt {
            cond, body, label, ..
        } => {
            let prefix = label
                .map(|l| format!(" {:?}", text(&l)))
                .unwrap_or_default();
            node(format!("WhileStmt{prefix}"), vec![child(cond), child(body)])
        }
        Stmt::LoopStmt {
            count, body, label, ..
        } => {
            let prefix = label
                .map(|l| format!(" {:?}", text(&l)))
                .unwrap_or_default();
            let mut children: Vec<_> = count.iter().map(child).collect();
            children.push(child(body));
            node(format!("LoopStmt{prefix}"), children)
        }
        Stmt::ReturnStmt { value, .. } => {
            node("ReturnStmt".to_string(), value.iter().map(child).collect())
        }
        Stmt::BreakStmt { target, .. } => {
            let suffix = target
                .map(|t| format!(" {:?}", text(&t)))
                .unwrap_or_default();
            leaf(format!("BreakStmt{suffix}"))
        }
        Stmt::ContinueStmt { target, .. } => {
            let suffix = target
                .map(|t| format!(" {:?}", text(&t)))
                .unwrap_or_default();
            leaf(format!("ContinueStmt{suffix}"))
        }
        Stmt::ForwardBranchStmt { cond, targets, .. } => {
            let mut children = vec![child(cond)];
            children.extend(targets.iter().map(|t| leaf(text(t).to_string())));
            node("ForwardBranchStmt".to_string(), children)
        }
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

fn write_decl(ast: &Ast, tokens: &TokenStream, source: &str, decl: &Decl) -> DumpNode {
    let text = |token: &TokenId| tokens.text(source, *token).expect("token is valid");
    let child = |id: &NodeId| write_node(ast, tokens, source, *id);

    match decl {
        Decl::Var {
            role,
            ty,
            is_external,
            declarators,
            attributes,
        } => {
            let names = declarators
                .iter()
                .map(|d| text(&d.name))
                .collect::<Vec<_>>()
                .join(", ");
            let external_prefix = if *is_external { "external " } else { "" };
            let mut children: Vec<_> = ty.iter().map(child).collect();
            children.extend(attribute_list_child(ast, tokens, source, attributes));
            children.extend(
                declarators
                    .iter()
                    .filter_map(|d| d.init.as_ref().map(child)),
            );
            node(
                format!("VarDecl {external_prefix}{} {names:?}", role_label(role)),
                children,
            )
        }
        Decl::Alias {
            kind, name, target, ..
        } => node(
            format!("Alias {} {:?}", alias_kind_label(kind), text(name)),
            target.iter().map(child).collect(),
        ),
    }
}

fn write_item(ast: &Ast, tokens: &TokenStream, source: &str, item: &Item) -> DumpNode {
    let text = |token: &TokenId| tokens.text(source, *token).expect("token is valid");
    let child = |id: &NodeId| write_node(ast, tokens, source, *id);

    match item {
        Item::NamespaceDecl {
            segments,
            params,
            items,
            attributes,
            ..
        } => {
            let path = segments.iter().map(text).collect::<Vec<_>>().join("::");
            let mut children: Vec<_> = params.iter().map(child).collect();
            children.extend(attribute_list_child(ast, tokens, source, attributes));
            children.extend(items.iter().map(child));
            node(format!("NamespaceDecl {path:?}"), children)
        }
        Item::ProcessorDecl {
            name,
            params,
            attributes,
            items,
            ..
        } => {
            let mut children: Vec<_> = params.iter().map(child).collect();
            children.extend(attribute_list_child(ast, tokens, source, attributes));
            children.extend(items.iter().map(child));
            node(format!("ProcessorDecl {:?}", text(name)), children)
        }
        Item::GraphDecl {
            name,
            params,
            attributes,
            items,
            ..
        } => {
            let mut children: Vec<_> = params.iter().map(child).collect();
            children.extend(attribute_list_child(ast, tokens, source, attributes));
            children.extend(items.iter().map(child));
            node(format!("GraphDecl {:?}", text(name)), children)
        }
        Item::StructDecl {
            name,
            attributes,
            items,
            ..
        } => {
            let mut children: Vec<_> = attribute_list_child(ast, tokens, source, attributes)
                .into_iter()
                .collect();
            children.extend(items.iter().map(child));
            node(format!("StructDecl {:?}", text(name)), children)
        }
        Item::EnumDecl { name, values, .. } => {
            let values = values.iter().map(text).collect::<Vec<_>>().join(", ");
            leaf(format!("EnumDecl {:?} {{{values}}}", text(name)))
        }
        Item::FunctionDecl {
            ty,
            name,
            generics,
            params,
            is_const,
            is_event_handler,
            attributes,
            body,
        } => {
            let generics_suffix = if generics.is_empty() {
                String::new()
            } else {
                format!(
                    "<{}>",
                    generics.iter().map(text).collect::<Vec<_>>().join(", ")
                )
            };
            let const_suffix = if *is_const { " const" } else { "" };
            let kind = if *is_event_handler {
                "EventHandlerDecl"
            } else {
                "FunctionDecl"
            };
            let mut children: Vec<_> = ty.iter().map(child).collect();
            children.extend(params.iter().map(child));
            children.extend(attribute_list_child(ast, tokens, source, attributes));
            children.push(child(body));
            node(
                format!("{kind} {:?}{generics_suffix}{const_suffix}", text(name)),
                children,
            )
        }
        Item::Import { path, .. } => {
            let path = path.iter().map(text).collect::<Vec<_>>().join(".");
            leaf(format!("Import {path:?}"))
        }
        Item::ModuleAlias {
            kind, name, target, ..
        } => node(
            format!("ModuleAlias {} {:?}", alias_kind_label(kind), text(name)),
            vec![child(target)],
        ),
    }
}

fn write_graph_member(ast: &Ast, tokens: &TokenStream, source: &str, member: &Graph) -> DumpNode {
    let text = |token: &TokenId| tokens.text(source, *token).expect("token is valid");
    let child = |id: &NodeId| write_node(ast, tokens, source, *id);

    match member {
        Graph::EndpointDecl {
            direction,
            kind,
            types,
            name,
            size,
            hoisted,
            attributes,
        } => {
            let dir = text(direction);
            let label = if let Some(hoisted) = hoisted {
                let path = hoisted
                    .segments
                    .iter()
                    .map(text)
                    .collect::<Vec<_>>()
                    .join(".");
                let target = match &hoisted.target {
                    HoistTarget::Name(name) => text(name).to_string(),
                    HoistTarget::Wildcard {
                        prefix: Some(prefix),
                    } => format!("{}*", text(prefix)),
                    HoistTarget::Wildcard { prefix: None } => "*".to_string(),
                };
                format!("EndpointDecl {dir} {path}.{target}")
            } else {
                let kind_text = kind.as_ref().map(text).unwrap_or_default();
                let name_text = name.as_ref().map(text).unwrap_or_default();
                format!("EndpointDecl {dir} {kind_text} {name_text:?}")
            };
            let mut children: Vec<_> = types.iter().map(child).collect();
            children.extend(size.iter().map(child));
            children.extend(attribute_list_child(ast, tokens, source, attributes));
            node(label, children)
        }
        Graph::NodeDecl {
            name,
            processor,
            array_size,
            ..
        } => {
            let mut children: Vec<_> = array_size.iter().map(child).collect();
            children.push(child(processor));
            node(format!("NodeDecl {:?}", text(name)), children)
        }
        Graph::ConnectionDecl { connections, .. } => node(
            "ConnectionDecl".to_string(),
            connections.iter().map(child).collect(),
        ),
        Graph::Connection {
            interpolation,
            sources,
            delay,
            destinations,
            ..
        } => {
            let label = match interpolation {
                Some(kind) => format!("Connection [{}]", interpolation_label(kind)),
                None => "Connection".to_string(),
            };
            let mut children = vec![node(
                "Sources".to_string(),
                sources.iter().map(child).collect(),
            )];
            if let Some(delay) = delay {
                children.push(node("Delay".to_string(), vec![child(delay)]));
            }
            children.push(node(
                "Destinations".to_string(),
                destinations.iter().map(child).collect(),
            ));
            node(label, children)
        }
        Graph::ConnectionIf {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            let mut children = vec![
                child(cond),
                node("Then".to_string(), then_branch.iter().map(child).collect()),
            ];
            if let Some(else_branch) = else_branch {
                children.push(node(
                    "Else".to_string(),
                    else_branch.iter().map(child).collect(),
                ));
            }
            node("ConnectionIf".to_string(), children)
        }
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
