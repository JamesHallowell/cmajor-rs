use crate::{
    ast::{Ast, Node, NodeId},
    lexer::{Keyword, Token, TokenId, TokenKind, TokenStream},
};

pub struct Parse {
    pub ast: Ast,
    pub root: NodeId,
    pub tokens: TokenStream,
}

pub fn parse(source: &str) -> Parse {
    let tokens = TokenStream::tokenize(source);
    let mut parser = Parser {
        tokens: &tokens,
        ast: Ast::new(),
        pos: 0,
    };

    let mut stmts = Vec::new();
    while parser.peek().kind != TokenKind::EndOfFile {
        stmts.push(parser.parse_statement());
    }
    let root = parser.ast.push(Node::Block {
        brace: TokenId(0),
        stmts,
    });

    Parse {
        ast: parser.ast,
        root,
        tokens,
    }
}

pub fn parse_type(source: &str) -> Parse {
    let tokens = TokenStream::tokenize(source);
    let mut parser = Parser {
        tokens: &tokens,
        ast: Ast::new(),
        pos: 0,
    };
    let root = parser.parse_type();

    Parse {
        ast: parser.ast,
        root,
        tokens,
    }
}

fn is_type_name_start(kind: TokenKind) -> bool {
    if kind == TokenKind::Ident {
        return true;
    }
    let TokenKind::Keyword(kw) = kind else {
        return false;
    };
    matches!(
        kw,
        Keyword::Bool
            | Keyword::Int
            | Keyword::Int32
            | Keyword::Int64
            | Keyword::Float
            | Keyword::Float32
            | Keyword::Float64
            | Keyword::Double
            | Keyword::Complex
            | Keyword::Complex32
            | Keyword::Complex64
            | Keyword::String
            | Keyword::Void
    )
}

fn infix_binding_power(kind: TokenKind) -> Option<(u8, u8)> {
    use TokenKind::*;
    Some(match kind {
        Equal | ArrowLeft => (2, 1),
        PipePipe => (4, 5),
        AmpersandAmpersand => (6, 7),
        Pipe => (8, 9),
        Caret => (10, 11),
        Ampersand => (12, 13),
        EqualEqual | BangEqual => (14, 15),
        LessThan | LessThanOrEqual | GreaterThan | GreaterThanOrEqual => (16, 17),
        ShiftLeft | ShiftRight | ShiftRightShiftRight => (18, 19),
        Plus | Minus => (20, 21),
        Star | Slash | Percent => (22, 23),
        StarStar => (25, 24),
        _ => return None,
    })
}

const TERNARY_BP: u8 = 3;
const TERNARY_RIGHT_BP: u8 = 3;
const UNARY_BP: u8 = 26;

const TYPE_SIZE_BP: u8 = 18;

fn is_prefix_op(kind: TokenKind) -> bool {
    use TokenKind::*;
    matches!(kind, Bang | Tilde | Minus | PlusPlus | MinusMinus)
}

struct Parser<'a> {
    tokens: &'a TokenStream,
    ast: Ast,
    pos: u32,
}

impl Parser<'_> {
    fn peek(&self) -> Token {
        self.tokens.token(TokenId(self.pos))
    }

    fn bump(&mut self) -> TokenId {
        let id = TokenId(self.pos);
        if (self.pos as usize) + 1 < self.tokens.len() {
            self.pos += 1;
        }
        id
    }

    fn expect(&mut self, kind: TokenKind) -> TokenId {
        if self.peek().kind == kind {
            self.bump()
        } else {
            TokenId(self.pos)
        }
    }

    fn at_keyword(&self, keyword: Keyword) -> bool {
        self.peek().kind == TokenKind::Keyword(keyword)
    }

    fn parse_expr(&mut self, min_bp: u8) -> NodeId {
        let mut lhs = self.parse_prefix();

        loop {
            let kind = self.peek().kind;

            if kind == TokenKind::Question && TERNARY_BP >= min_bp {
                let question = self.bump();
                let then_branch = self.parse_expr(0);
                self.expect(TokenKind::Colon);
                let else_branch = self.parse_expr(TERNARY_RIGHT_BP);
                lhs = self.ast.push(Node::Ternary {
                    question,
                    cond: lhs,
                    then_branch,
                    else_branch,
                });
                continue;
            }

            let Some((l_bp, r_bp)) = infix_binding_power(kind) else {
                break;
            };
            if l_bp < min_bp {
                break;
            }

            let op = self.bump();
            let rhs = self.parse_expr(r_bp);
            lhs = self.ast.push(match kind {
                TokenKind::Equal | TokenKind::ArrowLeft => Node::Assign {
                    op,
                    target: lhs,
                    value: rhs,
                },
                _ => Node::Binary { op, lhs, rhs },
            });
        }

        lhs
    }

    fn parse_prefix(&mut self) -> NodeId {
        if is_prefix_op(self.peek().kind) {
            let op = self.bump();
            let operand = self.parse_expr(UNARY_BP);
            self.ast.push(Node::Unary { op, operand })
        } else {
            self.parse_postfix()
        }
    }

    fn parse_postfix(&mut self) -> NodeId {
        let mut expr = self.parse_primary();

        loop {
            expr = match self.peek().kind {
                TokenKind::ParenthesisLeft => self.parse_call(expr),
                TokenKind::BracketLeft => self.parse_index(expr),
                TokenKind::Dot => self.parse_field(expr),
                _ => break,
            };
        }

        expr
    }

    fn parse_call(&mut self, callee: NodeId) -> NodeId {
        let paren = self.bump();
        let mut args = Vec::new();
        if self.peek().kind != TokenKind::ParenthesisRight {
            loop {
                args.push(self.parse_expr(0));
                if self.peek().kind == TokenKind::Comma {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::ParenthesisRight);
        self.ast.push(Node::Call {
            paren,
            callee,
            args,
        })
    }

    fn parse_index(&mut self, base: NodeId) -> NodeId {
        let bracket = self.bump();
        let index = self.parse_expr(0);
        self.expect(TokenKind::BracketRight);
        self.ast.push(Node::Index {
            bracket,
            base,
            index,
        })
    }

    fn parse_field(&mut self, base: NodeId) -> NodeId {
        self.bump();
        let name = self.expect(TokenKind::Ident);
        self.ast.push(Node::Field { name, base })
    }

    fn parse_primary(&mut self) -> NodeId {
        match self.peek().kind {
            TokenKind::IntLiteral => {
                let token = self.bump();
                self.ast.push(Node::IntLiteral { token })
            }
            TokenKind::FloatLiteral => {
                let token = self.bump();
                self.ast.push(Node::FloatLiteral { token })
            }
            TokenKind::StringLiteral => {
                let token = self.bump();
                self.ast.push(Node::StringLiteral { token })
            }
            TokenKind::Keyword(Keyword::True) | TokenKind::Keyword(Keyword::False) => {
                let token = self.bump();
                self.ast.push(Node::BoolLiteral { token })
            }
            TokenKind::Ident => {
                let token = self.bump();
                self.ast.push(Node::Ident { token })
            }
            TokenKind::ParenthesisLeft => {
                let paren = self.bump();
                let inner = self.parse_expr(0);
                self.expect(TokenKind::ParenthesisRight);
                self.ast.push(Node::Paren { paren, inner })
            }
            _ => {
                let token = self.bump();
                self.ast.push(Node::Error { token })
            }
        }
    }

    fn parse_statement(&mut self) -> NodeId {
        match self.peek().kind {
            TokenKind::BraceLeft => self.parse_block(),
            TokenKind::Keyword(Keyword::Let) => self.parse_let(),
            TokenKind::Keyword(Keyword::Var) => self.parse_var(),
            TokenKind::Keyword(Keyword::If) => self.parse_if(),
            TokenKind::Keyword(Keyword::While) => self.parse_while(),
            TokenKind::Keyword(Keyword::Loop) => self.parse_loop(),
            TokenKind::Keyword(Keyword::Return) => self.parse_return(),
            TokenKind::Keyword(Keyword::Break) => {
                let kw = self.bump();
                self.expect(TokenKind::Semicolon);
                self.ast.push(Node::BreakStmt { kw })
            }
            TokenKind::Keyword(Keyword::Continue) => {
                let kw = self.bump();
                self.expect(TokenKind::Semicolon);
                self.ast.push(Node::ContinueStmt { kw })
            }
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_block(&mut self) -> NodeId {
        let brace = self.bump();

        let mut stmts = Vec::new();
        while !matches!(
            self.peek().kind,
            TokenKind::BraceRight | TokenKind::EndOfFile
        ) {
            stmts.push(self.parse_statement());
        }
        self.expect(TokenKind::BraceRight);
        self.ast.push(Node::Block { brace, stmts })
    }

    fn parse_let(&mut self) -> NodeId {
        self.bump();
        let name = self.expect(TokenKind::Ident);
        self.expect(TokenKind::Equal);
        let init = self.parse_expr(0);
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::LetStmt { name, init })
    }

    fn parse_var(&mut self) -> NodeId {
        self.bump();
        let name = self.expect(TokenKind::Ident);
        let init = if self.peek().kind == TokenKind::Equal {
            self.bump();
            Some(self.parse_expr(0))
        } else {
            None
        };
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::VarStmt { name, init })
    }

    fn parse_if(&mut self) -> NodeId {
        let kw = self.bump();
        self.expect(TokenKind::ParenthesisLeft);
        let cond = self.parse_expr(0);
        self.expect(TokenKind::ParenthesisRight);
        let then_branch = self.parse_statement();
        let else_branch = if self.at_keyword(Keyword::Else) {
            self.bump();
            Some(self.parse_statement())
        } else {
            None
        };
        self.ast.push(Node::IfStmt {
            kw,
            cond,
            then_branch,
            else_branch,
        })
    }

    fn parse_while(&mut self) -> NodeId {
        let kw = self.bump();
        self.expect(TokenKind::ParenthesisLeft);
        let cond = self.parse_expr(0);
        self.expect(TokenKind::ParenthesisRight);
        let body = self.parse_statement();
        self.ast.push(Node::WhileStmt { kw, cond, body })
    }

    fn parse_loop(&mut self) -> NodeId {
        let kw = self.bump();
        let count = if self.peek().kind == TokenKind::ParenthesisLeft {
            self.bump();
            let count = self.parse_expr(0);
            self.expect(TokenKind::ParenthesisRight);
            Some(count)
        } else {
            None
        };
        let body = self.parse_block();
        self.ast.push(Node::LoopStmt { kw, count, body })
    }

    fn parse_return(&mut self) -> NodeId {
        let kw = self.bump();
        let value = if self.peek().kind == TokenKind::Semicolon {
            None
        } else {
            Some(self.parse_expr(0))
        };
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::ReturnStmt { kw, value })
    }

    fn parse_expr_stmt(&mut self) -> NodeId {
        let expr = self.parse_expr(0);
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::ExprStmt { expr })
    }

    fn parse_type(&mut self) -> NodeId {
        let mut ty = match self.peek().kind {
            TokenKind::Keyword(Keyword::Wrap) => self.parse_wrap_or_clamp(true),
            TokenKind::Keyword(Keyword::Clamp) => self.parse_wrap_or_clamp(false),
            kind if is_type_name_start(kind) => self.parse_type_name(),
            _ => {
                let token = self.bump();
                self.ast.push(Node::Error { token })
            }
        };

        loop {
            ty = match self.peek().kind {
                TokenKind::BracketLeft => self.parse_type_array(ty),
                TokenKind::LessThan => self.parse_type_vector(ty),
                _ => break,
            };
        }

        ty
    }

    fn parse_wrap_or_clamp(&mut self, is_wrap: bool) -> NodeId {
        let kw = self.bump();
        self.expect(TokenKind::LessThan);
        let size = self.parse_expr(TYPE_SIZE_BP);
        self.expect(TokenKind::GreaterThan);
        self.ast.push(if is_wrap {
            Node::TypeWrap { kw, size }
        } else {
            Node::TypeClamp { kw, size }
        })
    }

    fn parse_type_name(&mut self) -> NodeId {
        let mut segments = vec![self.bump()];
        while self.peek().kind == TokenKind::ColonColon {
            self.bump();
            segments.push(self.expect(TokenKind::Ident));
        }
        self.ast.push(Node::TypeName { segments })
    }

    fn parse_type_array(&mut self, element: NodeId) -> NodeId {
        let bracket = self.bump();
        let size = if self.peek().kind == TokenKind::BracketRight {
            None
        } else {
            Some(self.parse_expr(0))
        };
        self.expect(TokenKind::BracketRight);
        self.ast.push(Node::TypeArray {
            bracket,
            element,
            size,
        })
    }

    fn parse_type_vector(&mut self, element: NodeId) -> NodeId {
        let angle = self.bump();
        let size = self.parse_expr(TYPE_SIZE_BP);
        self.expect(TokenKind::GreaterThan);
        self.ast.push(Node::TypeVector {
            angle,
            element,
            size,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, PartialEq)]
    enum Tag {
        IntLiteral,
        FloatLiteral,
        StringLiteral,
        BoolLiteral,
        Ident,
        Paren,
        Unary,
        Binary,
        Assign,
        Ternary,
        Call,
        Index,
        Field,
        Block,
        ExprStmt,
        LetStmt,
        VarStmt,
        IfStmt,
        WhileStmt,
        LoopStmt,
        ReturnStmt,
        BreakStmt,
        ContinueStmt,
        TypeName,
        TypeWrap,
        TypeClamp,
        TypeArray,
        TypeVector,
        Error,
    }

    #[derive(Debug, PartialEq)]
    struct Tree {
        tag: Tag,
        text: String,
        children: Vec<Tree>,
    }

    fn node(tag: Tag, text: &str, children: Vec<Tree>) -> Tree {
        Tree {
            tag,
            text: text.to_string(),
            children,
        }
    }

    fn leaf(tag: Tag, text: &str) -> Tree {
        node(tag, text, vec![])
    }

    fn walk(ast: &Ast, tokens: &TokenStream, source: &str, id: NodeId) -> Tree {
        let text_of = |token: TokenId| tokens.text(source, token).to_string();
        let child = |id: NodeId| walk(ast, tokens, source, id);

        match ast.get(id) {
            Node::IntLiteral { token } => leaf(Tag::IntLiteral, &text_of(*token)),
            Node::FloatLiteral { token } => leaf(Tag::FloatLiteral, &text_of(*token)),
            Node::StringLiteral { token } => leaf(Tag::StringLiteral, &text_of(*token)),
            Node::BoolLiteral { token } => leaf(Tag::BoolLiteral, &text_of(*token)),
            Node::Ident { token } => leaf(Tag::Ident, &text_of(*token)),
            Node::Error { token } => leaf(Tag::Error, &text_of(*token)),

            Node::Paren { paren, inner } => node(Tag::Paren, &text_of(*paren), vec![child(*inner)]),
            Node::Unary { op, operand } => node(Tag::Unary, &text_of(*op), vec![child(*operand)]),
            Node::Binary { op, lhs, rhs } => {
                node(Tag::Binary, &text_of(*op), vec![child(*lhs), child(*rhs)])
            }
            Node::Assign { op, target, value } => node(
                Tag::Assign,
                &text_of(*op),
                vec![child(*target), child(*value)],
            ),
            Node::Ternary {
                question,
                cond,
                then_branch,
                else_branch,
            } => node(
                Tag::Ternary,
                &text_of(*question),
                vec![child(*cond), child(*then_branch), child(*else_branch)],
            ),
            Node::Call {
                paren,
                callee,
                args,
            } => {
                let mut children = vec![child(*callee)];
                children.extend(args.iter().map(|&arg| child(arg)));
                node(Tag::Call, &text_of(*paren), children)
            }
            Node::Index {
                bracket,
                base,
                index,
            } => node(
                Tag::Index,
                &text_of(*bracket),
                vec![child(*base), child(*index)],
            ),
            Node::Field { name, base } => node(Tag::Field, &text_of(*name), vec![child(*base)]),

            Node::Block { brace, stmts } => node(
                Tag::Block,
                &text_of(*brace),
                stmts.iter().map(|&s| child(s)).collect(),
            ),
            Node::ExprStmt { expr } => node(Tag::ExprStmt, "", vec![child(*expr)]),
            Node::LetStmt { name, init } => node(Tag::LetStmt, &text_of(*name), vec![child(*init)]),
            Node::VarStmt { name, init } => node(
                Tag::VarStmt,
                &text_of(*name),
                init.iter().map(|&i| child(i)).collect(),
            ),
            Node::IfStmt {
                kw,
                cond,
                then_branch,
                else_branch,
            } => {
                let mut children = vec![child(*cond), child(*then_branch)];
                children.extend(else_branch.iter().map(|&e| child(e)));
                node(Tag::IfStmt, &text_of(*kw), children)
            }
            Node::WhileStmt { kw, cond, body } => node(
                Tag::WhileStmt,
                &text_of(*kw),
                vec![child(*cond), child(*body)],
            ),
            Node::LoopStmt { kw, count, body } => {
                let mut children: Vec<_> = count.iter().map(|&c| child(c)).collect();
                children.push(child(*body));
                node(Tag::LoopStmt, &text_of(*kw), children)
            }
            Node::ReturnStmt { kw, value } => node(
                Tag::ReturnStmt,
                &text_of(*kw),
                value.iter().map(|&v| child(v)).collect(),
            ),
            Node::BreakStmt { kw } => leaf(Tag::BreakStmt, &text_of(*kw)),
            Node::ContinueStmt { kw } => leaf(Tag::ContinueStmt, &text_of(*kw)),

            Node::TypeName { segments } => {
                let path = segments
                    .iter()
                    .map(|&t| text_of(t))
                    .collect::<Vec<_>>()
                    .join("::");
                leaf(Tag::TypeName, &path)
            }
            Node::TypeWrap { kw, size } => node(Tag::TypeWrap, &text_of(*kw), vec![child(*size)]),
            Node::TypeClamp { kw, size } => node(Tag::TypeClamp, &text_of(*kw), vec![child(*size)]),
            Node::TypeArray {
                bracket,
                element,
                size,
            } => {
                let mut children = vec![child(*element)];
                children.extend(size.iter().map(|&s| child(s)));
                node(Tag::TypeArray, &text_of(*bracket), children)
            }
            Node::TypeVector {
                angle,
                element,
                size,
            } => node(
                Tag::TypeVector,
                &text_of(*angle),
                vec![child(*element), child(*size)],
            ),
        }
    }

    fn parse_expr(source: &str) -> Tree {
        let padded = format!("{source};");
        let Parse { ast, root, tokens } = parse(&padded);
        let Node::Block { stmts, .. } = ast.get(root) else {
            unreachable!()
        };
        let Node::ExprStmt { expr } = ast.get(stmts[0]) else {
            unreachable!()
        };
        walk(&ast, &tokens, &padded, *expr)
    }

    fn parse_source(source: &str) -> Tree {
        let Parse { ast, root, tokens } = parse(source);
        walk(&ast, &tokens, source, root)
    }

    fn parse_type_source(source: &str) -> Tree {
        let Parse { ast, root, tokens } = parse_type(source);
        walk(&ast, &tokens, source, root)
    }

    #[test]
    fn primitive_type() {
        use Tag::*;
        assert_eq!(parse_type_source("float"), leaf(TypeName, "float"));
    }

    #[test]
    fn named_type() {
        use Tag::*;
        assert_eq!(parse_type_source("MyStruct"), leaf(TypeName, "MyStruct"));
    }

    #[test]
    fn qualified_type_name() {
        use Tag::*;
        assert_eq!(
            parse_type_source("std::complex64"),
            leaf(TypeName, "std::complex64")
        );
    }

    #[test]
    fn array_type() {
        use Tag::*;
        assert_eq!(
            parse_type_source("int[3]"),
            node(
                TypeArray,
                "[",
                vec![leaf(TypeName, "int"), leaf(IntLiteral, "3")]
            )
        );
    }

    #[test]
    fn slice_type_has_no_size() {
        use Tag::*;
        assert_eq!(
            parse_type_source("int[]"),
            node(TypeArray, "[", vec![leaf(TypeName, "int")])
        );
    }

    #[test]
    fn vector_type() {
        use Tag::*;
        assert_eq!(
            parse_type_source("int<4>"),
            node(
                TypeVector,
                "<",
                vec![leaf(TypeName, "int"), leaf(IntLiteral, "4")]
            )
        );
    }

    #[test]
    fn wrap_and_clamp_types() {
        use Tag::*;
        assert_eq!(
            parse_type_source("wrap<4>"),
            node(TypeWrap, "wrap", vec![leaf(IntLiteral, "4")])
        );
        assert_eq!(
            parse_type_source("clamp<10>"),
            node(TypeClamp, "clamp", vec![leaf(IntLiteral, "10")])
        );
    }

    #[test]
    fn angle_bracket_close_is_not_a_comparison() {
        use Tag::*;
        assert_eq!(
            parse_type_source("wrap<1 + 2>"),
            node(
                TypeWrap,
                "wrap",
                vec![node(
                    Binary,
                    "+",
                    vec![leaf(IntLiteral, "1"), leaf(IntLiteral, "2")]
                )]
            )
        );
    }

    #[test]
    fn postfix_type_modifiers_apply_left_to_right() {
        use Tag::*;
        assert_eq!(
            parse_type_source("int<4>[2]"),
            node(
                TypeArray,
                "[",
                vec![
                    node(
                        TypeVector,
                        "<",
                        vec![leaf(TypeName, "int"), leaf(IntLiteral, "4")]
                    ),
                    leaf(IntLiteral, "2"),
                ]
            )
        );
    }

    #[test]
    fn precedence_of_arithmetic() {
        use Tag::*;
        assert_eq!(
            parse_expr("1 + 2 * 3"),
            node(
                Binary,
                "+",
                vec![
                    leaf(IntLiteral, "1"),
                    node(
                        Binary,
                        "*",
                        vec![leaf(IntLiteral, "2"), leaf(IntLiteral, "3")]
                    ),
                ]
            )
        );
    }

    #[test]
    fn power_is_right_associative() {
        use Tag::*;
        assert_eq!(
            parse_expr("2 ** 3 ** 4"),
            node(
                Binary,
                "**",
                vec![
                    leaf(IntLiteral, "2"),
                    node(
                        Binary,
                        "**",
                        vec![leaf(IntLiteral, "3"), leaf(IntLiteral, "4")]
                    ),
                ]
            )
        );
    }

    #[test]
    fn subtraction_is_left_associative() {
        use Tag::*;
        assert_eq!(
            parse_expr("1 - 2 - 3"),
            node(
                Binary,
                "-",
                vec![
                    node(
                        Binary,
                        "-",
                        vec![leaf(IntLiteral, "1"), leaf(IntLiteral, "2")]
                    ),
                    leaf(IntLiteral, "3"),
                ]
            )
        );
    }

    #[test]
    fn assignment_is_right_associative() {
        use Tag::*;
        assert_eq!(
            parse_expr("a = b = c"),
            node(
                Assign,
                "=",
                vec![
                    leaf(Ident, "a"),
                    node(Assign, "=", vec![leaf(Ident, "b"), leaf(Ident, "c")]),
                ]
            )
        );
    }

    #[test]
    fn output_write_operator() {
        use Tag::*;
        assert_eq!(
            parse_expr("out <- in * gain"),
            node(
                Assign,
                "<-",
                vec![
                    leaf(Ident, "out"),
                    node(Binary, "*", vec![leaf(Ident, "in"), leaf(Ident, "gain")]),
                ]
            )
        );
    }

    #[test]
    fn ternary_nests_to_the_right() {
        use Tag::*;
        assert_eq!(
            parse_expr("a ? b : c ? d : e"),
            node(
                Ternary,
                "?",
                vec![
                    leaf(Ident, "a"),
                    leaf(Ident, "b"),
                    node(
                        Ternary,
                        "?",
                        vec![leaf(Ident, "c"), leaf(Ident, "d"), leaf(Ident, "e")]
                    ),
                ]
            )
        );
    }

    #[test]
    fn ternary_binds_looser_than_logical_or() {
        use Tag::*;
        assert_eq!(
            parse_expr("a || b ? c : d"),
            node(
                Ternary,
                "?",
                vec![
                    node(Binary, "||", vec![leaf(Ident, "a"), leaf(Ident, "b")]),
                    leaf(Ident, "c"),
                    leaf(Ident, "d"),
                ]
            )
        );
    }

    #[test]
    fn unary_and_parens() {
        use Tag::*;
        assert_eq!(
            parse_expr("-(1 + 2)"),
            node(
                Unary,
                "-",
                vec![node(
                    Paren,
                    "(",
                    vec![node(
                        Binary,
                        "+",
                        vec![leaf(IntLiteral, "1"), leaf(IntLiteral, "2")]
                    )]
                )]
            )
        );
    }

    #[test]
    fn call_with_args() {
        use Tag::*;
        assert_eq!(
            parse_expr("foo(1, 2 + 3)"),
            node(
                Call,
                "(",
                vec![
                    leaf(Ident, "foo"),
                    leaf(IntLiteral, "1"),
                    node(
                        Binary,
                        "+",
                        vec![leaf(IntLiteral, "2"), leaf(IntLiteral, "3")]
                    ),
                ]
            )
        );
    }

    #[test]
    fn call_with_no_args() {
        use Tag::*;
        assert_eq!(
            parse_expr("advance()"),
            node(Call, "(", vec![leaf(Ident, "advance")])
        );
    }

    #[test]
    fn index_and_field_postfix() {
        use Tag::*;
        assert_eq!(
            parse_expr("x.left[3]"),
            node(
                Index,
                "[",
                vec![
                    node(Field, "left", vec![leaf(Ident, "x")]),
                    leaf(IntLiteral, "3"),
                ]
            )
        );
    }

    #[test]
    fn let_and_var_statements() {
        use Tag::*;
        assert_eq!(
            parse_source("let x = 1; var y;"),
            node(
                Block,
                "let",
                vec![
                    node(LetStmt, "x", vec![leaf(IntLiteral, "1")]),
                    node(VarStmt, "y", vec![]),
                ]
            )
        );
    }

    #[test]
    fn if_else_statement() {
        use Tag::*;
        assert_eq!(
            parse_source("if (a) { b; } else { c; }"),
            node(
                Block,
                "if",
                vec![node(
                    IfStmt,
                    "if",
                    vec![
                        leaf(Ident, "a"),
                        node(Block, "{", vec![node(ExprStmt, "", vec![leaf(Ident, "b")])]),
                        node(Block, "{", vec![node(ExprStmt, "", vec![leaf(Ident, "c")])]),
                    ]
                )]
            )
        );
    }

    #[test]
    fn if_without_else() {
        use Tag::*;
        assert_eq!(
            parse_source("if (a) { b; }"),
            node(
                Block,
                "if",
                vec![node(
                    IfStmt,
                    "if",
                    vec![
                        leaf(Ident, "a"),
                        node(Block, "{", vec![node(ExprStmt, "", vec![leaf(Ident, "b")])]),
                    ]
                )]
            )
        );
    }

    #[test]
    fn while_and_bounded_loop() {
        use Tag::*;
        assert_eq!(
            parse_source("while (n > 0) { n = n - 1; } loop (4) { advance(); }"),
            node(
                Block,
                "while",
                vec![
                    node(
                        WhileStmt,
                        "while",
                        vec![
                            node(Binary, ">", vec![leaf(Ident, "n"), leaf(IntLiteral, "0")]),
                            node(
                                Block,
                                "{",
                                vec![node(
                                    ExprStmt,
                                    "",
                                    vec![node(
                                        Assign,
                                        "=",
                                        vec![
                                            leaf(Ident, "n"),
                                            node(
                                                Binary,
                                                "-",
                                                vec![leaf(Ident, "n"), leaf(IntLiteral, "1")]
                                            ),
                                        ]
                                    )]
                                )]
                            ),
                        ]
                    ),
                    node(
                        LoopStmt,
                        "loop",
                        vec![
                            leaf(IntLiteral, "4"),
                            node(
                                Block,
                                "{",
                                vec![node(
                                    ExprStmt,
                                    "",
                                    vec![node(Call, "(", vec![leaf(Ident, "advance")])]
                                )]
                            ),
                        ]
                    ),
                ]
            )
        );
    }

    #[test]
    fn unbounded_loop_has_no_count() {
        use Tag::*;
        assert_eq!(
            parse_source("loop { advance(); }"),
            node(
                Block,
                "loop",
                vec![node(
                    LoopStmt,
                    "loop",
                    vec![node(
                        Block,
                        "{",
                        vec![node(
                            ExprStmt,
                            "",
                            vec![node(Call, "(", vec![leaf(Ident, "advance")])]
                        )]
                    )]
                )]
            )
        );
    }

    #[test]
    fn return_with_and_without_value() {
        use Tag::*;
        assert_eq!(
            parse_source("return; return x + 1;"),
            node(
                Block,
                "return",
                vec![
                    leaf(ReturnStmt, "return"),
                    node(
                        ReturnStmt,
                        "return",
                        vec![node(
                            Binary,
                            "+",
                            vec![leaf(Ident, "x"), leaf(IntLiteral, "1")]
                        )]
                    ),
                ]
            )
        );
    }

    #[test]
    fn break_and_continue() {
        use Tag::*;
        assert_eq!(
            parse_source("loop { break; continue; }"),
            node(
                Block,
                "loop",
                vec![node(
                    LoopStmt,
                    "loop",
                    vec![node(
                        Block,
                        "{",
                        vec![leaf(BreakStmt, "break"), leaf(ContinueStmt, "continue")]
                    )]
                )]
            )
        );
    }
}
