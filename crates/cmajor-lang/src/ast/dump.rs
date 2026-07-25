use {
    crate::{
        ast::node::{Ast, Node, NodeId},
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

fn write_node(ast: &Ast, tokens: &TokenStream, source: &str, id: NodeId) -> DumpNode {
    let text = |token: &TokenId| tokens.text(source, *token).expect("token is valid");
    let child = |id: &NodeId| write_node(ast, tokens, source, *id);
    let leaf = |label: String| DumpNode {
        label,
        children: vec![],
    };
    let node = |label: String, children: Vec<DumpNode>| DumpNode { label, children };

    match ast.get(id) {
        Node::IntLiteral { token }
        | Node::FloatLiteral { token }
        | Node::StringLiteral { token }
        | Node::BoolLiteral { token }
        | Node::ImaginaryLiteral { token }
        | Node::Ident { token } => leaf(text(token).to_string()),
        Node::Error { token } => leaf(format!("Error at {:?}", text(token))),
        Node::Paren { inner, .. } => node("Paren".to_string(), vec![child(inner)]),
        Node::Unary { op, operand } => node(format!("Unary {:?}", text(op)), vec![child(operand)]),
        Node::PostfixUnary { op, operand } => {
            node(format!("PostfixUnary {:?}", text(op)), vec![child(operand)])
        }
        Node::Binary { op, lhs, rhs } => node(
            format!("Binary {:?}", text(op)),
            vec![child(lhs), child(rhs)],
        ),
        Node::Assign { op, target, value } => node(
            format!("Assign {:?}", text(op)),
            vec![child(target), child(value)],
        ),
        Node::Ternary {
            cond,
            then_branch,
            else_branch,
            ..
        } => node(
            "Ternary".to_string(),
            vec![child(cond), child(then_branch), child(else_branch)],
        ),
        Node::Call { callee, args, .. } => {
            let mut children = vec![child(callee)];
            children.extend(args.iter().map(child));
            node("Call".to_string(), children)
        }
        Node::Index { base, index, .. } => {
            node("Index".to_string(), vec![child(base), child(index)])
        }
        Node::Field { name, base } => node(format!("Field {:?}", text(name)), vec![child(base)]),
        Node::Block { stmts, .. } => node("Block".to_string(), stmts.iter().map(child).collect()),
        Node::ExprStmt { expr } => node("ExprStmt".to_string(), vec![child(expr)]),
        Node::LetStmt { name, init } => {
            node(format!("LetStmt {:?}", text(name)), vec![child(init)])
        }
        Node::VarStmt { name, init } => {
            let children = init.iter().map(child).collect();
            node(format!("VarStmt {:?}", text(name)), children)
        }
        Node::VarDeclStmt {
            ty,
            name,
            init,
            is_const,
        } => {
            let const_prefix = if *is_const { "const " } else { "" };
            let mut children = vec![child(ty)];
            children.extend(init.iter().map(child));
            node(
                format!("VarDeclStmt {const_prefix}{:?}", text(name)),
                children,
            )
        }
        Node::IfStmt {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            let mut children = vec![child(cond), child(then_branch)];
            children.extend(else_branch.iter().map(child));
            node("IfStmt".to_string(), children)
        }
        Node::WhileStmt { cond, body, .. } => {
            node("WhileStmt".to_string(), vec![child(cond), child(body)])
        }
        Node::LoopStmt { count, body, .. } => {
            let mut children: Vec<_> = count.iter().map(child).collect();
            children.push(child(body));
            node("LoopStmt".to_string(), children)
        }
        Node::ReturnStmt { value, .. } => {
            node("ReturnStmt".to_string(), value.iter().map(child).collect())
        }
        Node::BreakStmt { .. } => leaf("BreakStmt".to_string()),
        Node::ContinueStmt { .. } => leaf("ContinueStmt".to_string()),
        Node::TypeName { segments } => {
            let path = segments.iter().map(text).collect::<Vec<_>>().join("::");
            leaf(format!("TypeName {path:?}"))
        }
        Node::Array { element, size, .. } => {
            let mut children = vec![child(element)];
            children.extend(size.iter().map(child));
            node("Array".to_string(), children)
        }
        Node::ChevronSuffix { element, term, .. } => node(
            "ChevronSuffix".to_string(),
            vec![child(element), child(term)],
        ),
        Node::NamespaceDecl {
            segments, items, ..
        } => {
            let path = segments.iter().map(text).collect::<Vec<_>>().join("::");
            node(
                format!("NamespaceDecl {path:?}"),
                items.iter().map(child).collect(),
            )
        }
        Node::ProcessorDecl { name, items, .. } => node(
            format!("ProcessorDecl {:?}", text(name)),
            items.iter().map(child).collect(),
        ),
        Node::GraphDecl { name, items, .. } => node(
            format!("GraphDecl {:?}", text(name)),
            items.iter().map(child).collect(),
        ),
        Node::StructDecl { name, items, .. } => node(
            format!("StructDecl {:?}", text(name)),
            items.iter().map(child).collect(),
        ),
        Node::EndpointGroup {
            direction,
            kind,
            endpoints,
        } => node(
            format!("EndpointGroup {} {}", text(direction), text(kind)),
            endpoints.iter().map(child).collect(),
        ),
        Node::EndpointDecl {
            ty,
            name,
            attributes,
        } => {
            let mut children = vec![child(ty)];
            children.extend(attributes.iter().map(child));
            node(format!("EndpointDecl {:?}", text(name)), children)
        }
        Node::AttributeList { attrs } => {
            let children = attrs
                .iter()
                .map(|(key, value)| node(format!("{:?}", text(key)), vec![child(value)]))
                .collect();
            node("AttributeList".to_string(), children)
        }
        Node::FunctionDecl {
            ty,
            name,
            params,
            body,
        } => {
            let mut children = vec![child(ty)];
            children.extend(params.iter().map(child));
            children.push(child(body));
            node(format!("FunctionDecl {:?}", text(name)), children)
        }
        Node::Param { ty, name } => node(format!("Param {:?}", text(name)), vec![child(ty)]),
        Node::EventHandlerDecl {
            name, params, body, ..
        } => {
            let mut children: Vec<_> = params.iter().map(child).collect();
            children.push(child(body));
            node(format!("EventHandlerDecl {:?}", text(name)), children)
        }
        Node::ScopeAccess { name, base } => {
            node(format!("ScopeAccess {:?}", text(name)), vec![child(base)])
        }
    }
}
