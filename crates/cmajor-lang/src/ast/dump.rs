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
    let text = |token: &TokenId| tokens.text(source, *token).expect("token is valid");
    let child =
        |id: &NodeId, out: &mut String| write_node(ast, tokens, source, *id, depth + 1, out);

    match ast.get(id) {
        Node::IntLiteral { token }
        | Node::FloatLiteral { token }
        | Node::StringLiteral { token }
        | Node::BoolLiteral { token }
        | Node::Ident { token } => {
            let _ = writeln!(out, "{indent}{:?}", text(token));
        }

        Node::Error { token } => {
            let _ = writeln!(out, "{indent}Error at {:?}", text(token));
        }

        Node::Paren { inner, .. } => {
            let _ = writeln!(out, "{indent}Paren");
            child(inner, out);
        }
        Node::Unary { op, operand } => {
            let _ = writeln!(out, "{indent}Unary {:?}", text(op));
            child(operand, out);
        }

        Node::Binary { op, lhs, rhs } => {
            let _ = writeln!(out, "{indent}Binary {:?}", text(op));
            child(lhs, out);
            child(rhs, out);
        }
        Node::Assign { op, target, value } => {
            let _ = writeln!(out, "{indent}Assign {:?}", text(op));
            child(target, out);
            child(value, out);
        }

        Node::Ternary {
            cond,
            then_branch,
            else_branch,
            ..
        } => {
            let _ = writeln!(out, "{indent}Ternary");
            child(cond, out);
            child(then_branch, out);
            child(else_branch, out);
        }

        Node::Call { callee, args, .. } => {
            let _ = writeln!(out, "{indent}Call");
            child(callee, out);
            for arg in args {
                child(arg, out);
            }
        }

        Node::Index { base, index, .. } => {
            let _ = writeln!(out, "{indent}Index");
            child(base, out);
            child(index, out);
        }
        Node::Field { name, base } => {
            let _ = writeln!(out, "{indent}Field {:?}", text(name));
            child(base, out);
        }

        Node::Block { stmts, .. } => {
            let _ = writeln!(out, "{indent}Block");
            for stmt in stmts {
                child(stmt, out);
            }
        }
        Node::ExprStmt { expr } => {
            let _ = writeln!(out, "{indent}ExprStmt");
            child(expr, out);
        }

        Node::LetStmt { name, init } => {
            let _ = writeln!(out, "{indent}LetStmt {:?}", text(name));
            child(init, out);
        }
        Node::VarStmt { name, init } => {
            let _ = writeln!(out, "{indent}VarStmt {:?}", text(name));
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
            let _ = writeln!(out, "{indent}IfStmt");
            child(cond, out);
            child(then_branch, out);
            if let Some(else_branch) = else_branch {
                child(else_branch, out);
            }
        }
        Node::WhileStmt { cond, body, .. } => {
            let _ = writeln!(out, "{indent}WhileStmt");
            child(cond, out);
            child(body, out);
        }
        Node::LoopStmt { count, body, .. } => {
            let _ = writeln!(out, "{indent}LoopStmt");
            if let Some(count) = count {
                child(count, out);
            }
            child(body, out);
        }
        Node::ReturnStmt { value, .. } => {
            let _ = writeln!(out, "{indent}ReturnStmt");
            if let Some(value) = value {
                child(value, out);
            }
        }
        Node::BreakStmt { .. } => {
            let _ = writeln!(out, "{indent}BreakStmt");
        }
        Node::ContinueStmt { .. } => {
            let _ = writeln!(out, "{indent}ContinueStmt");
        }

        Node::TypeName { segments } => {
            let path = segments.iter().map(text).collect::<Vec<_>>().join("::");
            let _ = writeln!(out, "{indent}TypeName {path:?}");
        }
        Node::Wrap { size, .. } => {
            let _ = writeln!(out, "{indent}Wrap");
            child(size, out);
        }
        Node::Clamp { size, .. } => {
            let _ = writeln!(out, "{indent}Clamp");
            child(size, out);
        }
        Node::Array { element, size, .. } => {
            let _ = writeln!(out, "{indent}Array");
            child(element, out);
            if let Some(size) = size {
                child(size, out);
            }
        }
        Node::Vector { element, size, .. } => {
            let _ = writeln!(out, "{indent}Vector");
            child(element, out);
            child(size, out);
        }
    }
}
