use {
    crate::{
        ast::node::{Ast, Node, NodeId},
        lexer::{TokenId, TokenStream},
    },
    std::fmt::Write as _,
};

pub fn dump(ast: &Ast, tokens: &TokenStream, source: &str, root: NodeId) -> String {
    let mut out = String::new();
    write_node(ast, tokens, source, root, 0, &mut out);
    out
}

fn write_node(
    ast: &Ast,
    tokens: &TokenStream,
    source: &str,
    id: NodeId,
    depth: usize,
    out: &mut String,
) {
    let indent = "  ".repeat(depth);
    let mut write = |text: &str| {
        let _ = writeln!(out, "{indent}{text}");
    };

    let text = |token: &TokenId| tokens.text(source, *token).expect("token is valid");
    let child =
        |id: &NodeId, out: &mut String| write_node(ast, tokens, source, *id, depth + 1, out);

    match ast.get(id) {
        Node::IntLiteral { token }
        | Node::FloatLiteral { token }
        | Node::StringLiteral { token }
        | Node::BoolLiteral { token }
        | Node::Ident { token } => {
            write(text(token));
        }

        Node::Error { token } => {
            write(&format!("Error at {:?}", text(token)));
        }

        Node::Paren { inner, .. } => {
            write("Paren");
            child(inner, out);
        }
        Node::Unary { op, operand } => {
            write(&format!("Unary {:?}", text(op)));
            child(operand, out);
        }
        Node::PostfixUnary { op, operand } => {
            write(&format!("PostfixUnary {:?}", text(op)));
            child(operand, out);
        }

        Node::Binary { op, lhs, rhs } => {
            write(&format!("Binary {:?}", text(op)));
            child(lhs, out);
            child(rhs, out);
        }
        Node::Assign { op, target, value } => {
            write(&format!("Assign {:?}", text(op)));
            child(target, out);
            child(value, out);
        }

        Node::Ternary {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            write("Ternary");
            child(cond, out);
            child(then_branch, out);
            child(else_branch, out);
        }

        Node::Call { callee, args, .. } => {
            write("Call");
            child(callee, out);
            for arg in args {
                child(arg, out);
            }
        }

        Node::Index { base, index, .. } => {
            write("Index");
            child(base, out);
            child(index, out);
        }
        Node::Field { name, base } => {
            write(&format!("Field {:?}", text(name)));
            child(base, out);
        }

        Node::Block { stmts, .. } => {
            write("Block");
            for stmt in stmts {
                child(stmt, out);
            }
        }
        Node::ExprStmt { expr } => {
            write("ExprStmt");
            child(expr, out);
        }

        Node::LetStmt { name, init } => {
            write(&format!("LetStmt {:?}", text(name)));
            child(init, out);
        }
        Node::VarStmt { name, init } => {
            write(&format!("VarStmt {:?}", text(name)));
            if let Some(init) = init {
                child(init, out);
            }
        }
        Node::VarDeclStmt {
            ty,
            name,
            init,
            is_const,
        } => {
            let const_prefix = if *is_const { "const " } else { "" };
            write(&format!("VarDeclStmt {const_prefix}{:?}", text(name)));
            child(ty, out);
            if let Some(init) = init {
                child(init, out);
            }
        }

        Node::IfStmt {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            write("IfStmt");
            child(cond, out);
            child(then_branch, out);
            if let Some(else_branch) = else_branch {
                child(else_branch, out);
            }
        }
        Node::WhileStmt { cond, body, .. } => {
            write("WhileStmt");
            child(cond, out);
            child(body, out);
        }
        Node::LoopStmt { count, body, .. } => {
            write("LoopStmt");
            if let Some(count) = count {
                child(count, out);
            }
            child(body, out);
        }
        Node::ReturnStmt { value, .. } => {
            write("ReturnStmt");
            if let Some(value) = value {
                child(value, out);
            }
        }
        Node::BreakStmt { .. } => {
            write("BreakStmt");
        }
        Node::ContinueStmt { .. } => {
            write("ContinueStmt");
        }

        Node::TypeName { segments } => {
            let path = segments.iter().map(text).collect::<Vec<_>>().join("::");
            write(&format!("TypeName {path:?}"));
        }
        Node::Array { element, size, .. } => {
            write("Array");
            child(element, out);
            if let Some(size) = size {
                child(size, out);
            }
        }
        Node::ChevronSuffix { element, term, .. } => {
            write("ChevronSuffix");
            child(element, out);
            child(term, out);
        }
        Node::NamespaceDecl {
            segments, items, ..
        } => {
            let path = segments.iter().map(text).collect::<Vec<_>>().join("::");
            write(&format!("NamespaceDecl {path:?}"));
            for item in items {
                child(item, out);
            }
        }
        Node::ContainerDecl {
            keyword,
            name,
            items,
        } => {
            write(&format!(
                "ContainerDecl {:?} ({})",
                text(name),
                text(keyword)
            ));
            for item in items {
                child(item, out);
            }
        }
        Node::EndpointGroup {
            direction,
            kind,
            endpoints,
        } => {
            write(&format!("EndpointGroup {} {}", text(direction), text(kind)));
            for endpoint in endpoints {
                child(endpoint, out);
            }
        }
        Node::EndpointDecl {
            ty,
            name,
            attributes,
        } => {
            write(&format!("EndpointDecl {:?}", text(name)));
            child(ty, out);
            if let Some(attributes) = attributes {
                child(attributes, out);
            }
        }
        Node::AttributeList { attrs } => {
            write("AttributeList");
            for (key, value) in attrs {
                let key_indent = "  ".repeat(depth + 1);
                let _ = writeln!(out, "{key_indent}{:?}", text(key));
                write_node(ast, tokens, source, *value, depth + 2, out);
            }
        }
        Node::FunctionDecl {
            ty,
            name,
            params,
            body,
        } => {
            write(&format!("FunctionDecl {:?}", text(name)));
            child(ty, out);
            for param in params {
                child(param, out);
            }
            child(body, out);
        }
        Node::Param { ty, name } => {
            write(&format!("Param {:?}", text(name)));
            child(ty, out);
        }
        Node::EventHandlerDecl {
            name, params, body, ..
        } => {
            write(&format!("EventHandlerDecl {:?}", text(name)));
            for param in params {
                child(param, out);
            }
            child(body, out);
        }
        Node::ScopeAccess { name, base } => {
            write(&format!("ScopeAccess {:?}", text(name)));
            child(base, out);
        }
    }
}
