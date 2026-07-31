use crate::{
    ast::{
        self, Ast, BracketTerm, Decl, Declarator, Expr, Graph, HoistTarget, HoistedPath,
        InterpolationKind, Item, Node, NodeId, Stmt, VarRole,
    },
    lexer::{tokenize, Literal, NonTrivialTokenStreamIterator, TokenId, TokenKind, TokenStream},
    parser::precedence::{BindingPower, InfixBindingPower, PrecedenceLevel},
    token, utils, Diagnostic,
};

pub struct Parse {
    pub ast: Ast,
    pub roots: Vec<NodeId>,
    pub tokens: TokenStream,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse(source: &str) -> Parse {
    let tokens = tokenize(source);
    let mut parser = Parser::new(&tokens, source);
    let roots = parser.parse();
    Parse {
        ast: parser.ast,
        roots,
        diagnostics: parser.diagnostics,
        tokens,
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum BinaryOp {
    Assign,
    Write,
    AddAssign,
    SubtractAssign,
    MultiplyAssign,
    DivideAssign,
    RemainderAssign,
    BitwiseAndAssign,
    BitwiseOrAssign,
    BitwiseXorAssign,
    ShiftLeftAssign,
    ShiftRightAssign,
    UnsignedShiftRightAssign,
    LogicalAndAssign,
    LogicalOrAssign,
    Or,
    And,
    BitwiseOr,
    BitwiseXor,
    BitwiseAnd,
    Equal,
    NotEqual,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    ShiftLeft,
    ShiftRight,
    UnsignedShiftRight,
    Add,
    Subtract,
    Multiply,
    Divide,
    Remainder,
    Power,
}

impl BinaryOp {
    fn from_token(kind: TokenKind) -> Option<Self> {
        Some(match kind {
            token!(=) => Self::Assign,
            token!(<-) => Self::Write,
            token!(+=) => Self::AddAssign,
            token!(-=) => Self::SubtractAssign,
            token!(*=) => Self::MultiplyAssign,
            token!(/=) => Self::DivideAssign,
            token!(%=) => Self::RemainderAssign,
            token!(&=) => Self::BitwiseAndAssign,
            token!(|=) => Self::BitwiseOrAssign,
            token!(^=) => Self::BitwiseXorAssign,
            token!(<<=) => Self::ShiftLeftAssign,
            token!(>>=) => Self::ShiftRightAssign,
            token!(>>>=) => Self::UnsignedShiftRightAssign,
            token!(&&=) => Self::LogicalAndAssign,
            token!(||=) => Self::LogicalOrAssign,
            token!(||) => Self::Or,
            token!(&&) => Self::And,
            token!(|) => Self::BitwiseOr,
            token!(^) => Self::BitwiseXor,
            token!(&) => Self::BitwiseAnd,
            token!(==) => Self::Equal,
            token!(!=) => Self::NotEqual,
            token!(<) => Self::LessThan,
            token!(<=) => Self::LessThanOrEqual,
            token!(>) => Self::GreaterThan,
            token!(>=) => Self::GreaterThanOrEqual,
            token!(<<) => Self::ShiftLeft,
            token!(>>) => Self::ShiftRight,
            token!(>>>) => Self::UnsignedShiftRight,
            token!(+) => Self::Add,
            token!(-) => Self::Subtract,
            token!(*) => Self::Multiply,
            token!(/) => Self::Divide,
            token!(%) => Self::Remainder,
            token!(**) => Self::Power,
            _ => return None,
        })
    }

    fn binding_power(self) -> InfixBindingPower {
        match self {
            Self::Assign
            | Self::Write
            | Self::AddAssign
            | Self::SubtractAssign
            | Self::MultiplyAssign
            | Self::DivideAssign
            | Self::RemainderAssign
            | Self::BitwiseAndAssign
            | Self::BitwiseOrAssign
            | Self::BitwiseXorAssign
            | Self::ShiftLeftAssign
            | Self::ShiftRightAssign
            | Self::UnsignedShiftRightAssign
            | Self::LogicalAndAssign
            | Self::LogicalOrAssign => PrecedenceLevel::Assign.right_associative(),
            Self::Or => PrecedenceLevel::Or.left_associative(),
            Self::And => PrecedenceLevel::And.left_associative(),
            Self::BitwiseOr => PrecedenceLevel::BitwiseOr.left_associative(),
            Self::BitwiseXor => PrecedenceLevel::BitwiseXor.left_associative(),
            Self::BitwiseAnd => PrecedenceLevel::BitwiseAnd.left_associative(),
            Self::Equal | Self::NotEqual => PrecedenceLevel::Equality.left_associative(),
            Self::LessThan
            | Self::LessThanOrEqual
            | Self::GreaterThan
            | Self::GreaterThanOrEqual => PrecedenceLevel::Relational.left_associative(),
            Self::ShiftLeft | Self::ShiftRight | Self::UnsignedShiftRight => {
                PrecedenceLevel::Shift.left_associative()
            }
            Self::Add | Self::Subtract => PrecedenceLevel::Additive.left_associative(),
            Self::Multiply | Self::Divide | Self::Remainder => {
                PrecedenceLevel::Multiplicative.left_associative()
            }
            Self::Power => PrecedenceLevel::Power.right_associative(),
        }
    }

    fn is_assignment(self) -> bool {
        matches!(
            self,
            Self::Assign
                | Self::Write
                | Self::AddAssign
                | Self::SubtractAssign
                | Self::MultiplyAssign
                | Self::DivideAssign
                | Self::RemainderAssign
                | Self::BitwiseAndAssign
                | Self::BitwiseOrAssign
                | Self::BitwiseXorAssign
                | Self::ShiftLeftAssign
                | Self::ShiftRightAssign
                | Self::UnsignedShiftRightAssign
                | Self::LogicalAndAssign
                | Self::LogicalOrAssign
        )
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
enum Infix {
    Binary(BinaryOp),
    Ternary,
}

impl Infix {
    fn from_token(kind: TokenKind) -> Option<Self> {
        if kind == token!(?) {
            Some(Self::Ternary)
        } else {
            BinaryOp::from_token(kind).map(Self::Binary)
        }
    }

    fn binding_power(self) -> InfixBindingPower {
        match self {
            Self::Ternary => {
                let base = PrecedenceLevel::Ternary.base();
                InfixBindingPower {
                    binds_at: base,
                    min_for_rhs: base,
                }
            }
            Self::Binary(op) => op.binding_power(),
        }
    }
}

struct Parser<'a> {
    tokens: NonTrivialTokenStreamIterator<'a>,
    source: &'a str,
    ast: Ast,
    diagnostics: Vec<Diagnostic>,
}

macro_rules! expect_matches {
    ($parser:expr, $expected:pat) => {
        if matches!($parser.tokens.peek_kind(), Some($expected)) {
            $parser.bump()
        } else {
            let actual_kind = $parser.tokens.peek_kind();
            let pos = $parser.tokens.current();
            $parser.error(
                pos,
                format!("expected {}, found {actual_kind:?}", stringify!($expected)),
            );
            pos
        }
    };
}

macro_rules! expect_identifier {
    ($parser:expr, $expected:pat) => {
        match $parser.tokens.peek_kind() {
            Some(TokenKind::Identifier) => {
                let text = $parser
                    .tokens
                    .peek_id()
                    .and_then(|id| $parser.tokens.stream().text($parser.source, id))
                    .expect("failed to query text for token");

                if matches!(text, $expected) {
                    $parser.bump()
                } else {
                    let pos = $parser.tokens.current();
                    $parser.error(
                        pos,
                        format!("expected {}, found {text:?}", stringify!($expected)),
                    );
                    pos
                }
            }
            actual_kind => {
                let pos = $parser.tokens.current();
                $parser.error(
                    pos,
                    format!("expected {}, found {actual_kind:?}", stringify!($expected)),
                );
                pos
            }
        }
    };
}

macro_rules! expect {
    ($parser:expr, $($rest:tt)+) => {
        expect!(@munch $parser; (); $($rest)+)
    };
    (@munch $parser:expr; ($($acc:expr),*); { $($e:tt)* } $(, $($rest:tt)*)?) => {
        expect!(@munch $parser; ($($acc,)* { $($e)* }); $($($rest)*)?)
    };
    (@munch $parser:expr; ($($acc:expr),*); if ! peek token!($($cond:tt)+) $body:block else $else_body:block $(, $($rest:tt)*)?) => {
        expect!(@munch $parser; ($($acc,)* if $parser.tokens.peek_kind() != Some(token!($($cond)+)) { $body } else { $else_body }); $($($rest)*)?)
    };
    (@munch $parser:expr; ($($acc:expr),*); if ! peek token!($($cond:tt)+) $body:block $(, $($rest:tt)*)?) => {
        expect!(@munch $parser; ($($acc,)* if $parser.tokens.peek_kind() != Some(token!($($cond)+)) { Some($body) } else { None }); $($($rest)*)?)
    };
    (@munch $parser:expr; ($($acc:expr),*); if peek token!($($cond:tt)+) $body:block else $else_body:block $(, $($rest:tt)*)?) => {
        expect!(@munch $parser; ($($acc,)* if $parser.tokens.peek_kind() == Some(token!($($cond)+)) { $body } else { $else_body }); $($($rest)*)?)
    };
    (@munch $parser:expr; ($($acc:expr),*); if peek token!($($cond:tt)+) $body:block $(, $($rest:tt)*)?) => {
        expect!(@munch $parser; ($($acc,)* if $parser.tokens.peek_kind() == Some(token!($($cond)+)) { Some($body) } else { None }); $($($rest)*)?)
    };
    (@munch $parser:expr; ($($acc:expr),*); if token!($($cond:tt)+) { $($seq:tt)+ } else $else_body:block $(, $($rest:tt)*)?) => {
        expect!(@munch $parser; ($($acc,)* if $parser.tokens.peek_kind() == Some(token!($($cond)+)) {
            expect!($parser, token!($($cond)+), $($seq)+)
        } else { $else_body }); $($($rest)*)?)
    };
    (@munch $parser:expr; ($($acc:expr),*); if token!($($cond:tt)+) { $($seq:tt)+ } $(, $($rest:tt)*)?) => {
        expect!(@munch $parser; ($($acc,)* if $parser.tokens.peek_kind() == Some(token!($($cond)+)) {
            Some(expect!($parser, token!($($cond)+), $($seq)+))
        } else { None }); $($($rest)*)?)
    };
    (@munch $parser:expr; ($($acc:expr),*); if token!($($cond:tt)+) else $else_body:block $(, $($rest:tt)*)?) => {
        expect!(@munch $parser; ($($acc,)* if $parser.tokens.peek_kind() == Some(token!($($cond)+)) { $parser.bump() } else { $else_body }); $($($rest)*)?)
    };
    (@munch $parser:expr; ($($acc:expr),*); if token!($($cond:tt)+) $(, $($rest:tt)*)?) => {
        expect!(@munch $parser; ($($acc,)* if $parser.tokens.peek_kind() == Some(token!($($cond)+)) { Some($parser.bump()) } else { None }); $($($rest)*)?)
    };
    (@munch $parser:expr; ($($acc:expr),*); token!($($kind:tt)+) $(, $($rest:tt)*)?) => {
        expect!(@munch $parser; ($($acc,)* $parser.expect(token!($($kind)+))); $($($rest)*)?)
    };
    (@munch $parser:expr; ($($acc:expr),*); TokenKind::$kind:ident $(, $($rest:tt)*)?) => {
        expect!(@munch $parser; ($($acc,)* $parser.expect(TokenKind::$kind)); $($($rest)*)?)
    };
    (@munch $parser:expr; ($($acc:expr),*); $e:expr $(, $($rest:tt)*)?) => {
        expect!(@munch $parser; ($($acc,)* $e); $($($rest)*)?)
    };
    (@munch $parser:expr; ($($acc:expr),+); ) => {
        ($($acc),+)
    };
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a TokenStream, source: &'a str) -> Parser<'a> {
        Parser {
            tokens: tokens.into_iter().ignore_trivia(),
            source,
            ast: Ast::new(),
            diagnostics: Vec::new(),
        }
    }

    fn bump(&mut self) -> TokenId {
        self.tokens
            .next()
            .map(|(id, _)| id)
            .unwrap_or_else(|| self.tokens.current())
    }

    fn bump_if(&mut self, kind: impl Into<TokenKind>) -> Option<TokenId> {
        if let Some(id) = self.tokens.peek_kind() {
            if id == kind.into() {
                return Some(self.bump());
            }
        }
        None
    }

    fn error(&mut self, token: TokenId, message: impl Into<String>) {
        let offset = self
            .tokens
            .stream()
            .span(token)
            .map(|span| span.start)
            .unwrap_or(self.source.len() as u32);
        let (line, column) = utils::line_col(self.source, offset);
        self.diagnostics.push(Diagnostic {
            token,
            line,
            column,
            message: message.into(),
        });
    }

    fn expect(&mut self, kind: impl Into<TokenKind>) -> TokenId {
        let kind = kind.into();

        if let Some(token) = self.bump_if(kind) {
            token
        } else {
            let actual_kind = self.tokens.peek_kind();
            let pos = self.tokens.current();

            self.error(pos, format!("expected {kind:?}, found {actual_kind:?}"));
            pos
        }
    }

    fn peek_text_is(&self, expected: &str) -> bool {
        self.tokens
            .peek_id()
            .and_then(|id| self.tokens.stream().text(self.source, id))
            == Some(expected)
    }

    pub fn parse(&mut self) -> Vec<NodeId> {
        let mut stmts = Vec::new();
        while self.tokens.peek().is_some() {
            stmts.push(self.parse_statement());
        }
        stmts
    }

    fn parse_expr(&mut self) -> NodeId {
        self.parse_expr_with_min_binding_power(PrecedenceLevel::lowest())
    }

    fn parse_expr_with_min_binding_power(&mut self, min_binding_power: BindingPower) -> NodeId {
        let mut lhs = match self.tokens.peek_kind() {
            Some(token!(! | ~ | - | ++ | --)) => {
                let op = self.bump();
                let operand = self.parse_expr_with_min_binding_power(PrecedenceLevel::Unary.base());
                self.ast.push(Expr::Unary { op, operand })
            }
            Some(TokenKind::Literal(literal)) => self.parse_literal(literal),
            Some(token!(true | false)) => {
                let token = self.bump();
                self.ast.push(Expr::Literal(ast::Literal::Bool { token }))
            }
            Some(token!(processor)) if self.at(1, token!(.)) => {
                self.bump();
                self.bump();
                let name = self.expect(TokenKind::Identifier);
                self.ast.push(Expr::ProcessorProperty { name })
            }
            Some(TokenKind::Identifier | token!(processor)) => {
                let token = self.bump();
                self.ast.push(Expr::Ident { token })
            }
            Some(token!('(')) => {
                let (paren, inner, _) = self.parse_parenthetical_list(|parser| parser.parse_expr());
                self.ast.push(Expr::Parentheses { paren, inner })
            }
            Some(token) if token.is_type_like() => {
                let token = self.bump();
                self.ast.push(Expr::Ident { token })
            }
            peeked => {
                let message = format!("expected expression, found {peeked:?}");
                let token = self.bump();
                self.error(token, message);
                self.ast.push(Node::Error { token })
            }
        };

        loop {
            lhs = match self.tokens.peek_kind() {
                Some(token!('(')) => self.parse_call(lhs),
                Some(token!('[')) => self.parse_bracketed_suffix(lhs),
                Some(token!(.)) => self.parse_field(lhs),
                Some(token!(::)) => self.parse_scope_access(lhs),
                Some(token!(<)) => match self.try_parse_vector_size_suffix(lhs) {
                    Some(suffixed) => suffixed,
                    None => break,
                },
                Some(token!(++ | --)) => {
                    let op = self.bump();
                    self.ast.push(Expr::PostfixUnary { op, operand: lhs })
                }
                _ => break,
            };
        }

        while let Some(token) = self.tokens.peek_kind() {
            let Some(infix) = Infix::from_token(token) else {
                break;
            };
            let binding_power = infix.binding_power();
            if binding_power.binds_at < min_binding_power {
                break;
            }

            lhs = match infix {
                Infix::Ternary => {
                    let (question, then_branch, _, else_branch) = expect!(
                        self,
                        token!(?),
                        self.parse_expr(),
                        token!(:),
                        self.parse_expr_with_min_binding_power(binding_power.min_for_rhs)
                    );

                    self.ast.push(Expr::Ternary {
                        question,
                        cond: lhs,
                        then_branch,
                        else_branch,
                    })
                }
                Infix::Binary(bin_op) => {
                    let op = self.bump();
                    let rhs = self.parse_expr_with_min_binding_power(binding_power.min_for_rhs);
                    self.ast.push(if bin_op.is_assignment() {
                        Expr::Assign {
                            op,
                            target: lhs,
                            value: rhs,
                        }
                    } else {
                        Expr::Binary { op, lhs, rhs }
                    })
                }
            };
        }

        lhs
    }

    fn parse_scope_access(&mut self, base: NodeId) -> NodeId {
        let (_, name) = expect!(self, token!(::), TokenKind::Identifier);
        self.ast.push(Expr::ScopeAccess { name, base })
    }

    fn parse_list<T>(
        &mut self,
        start: impl Into<TokenKind>,
        parse: fn(&mut Self) -> T,
        separator: impl Into<TokenKind>,
        stop: impl Into<TokenKind>,
    ) -> (TokenId, Vec<T>, TokenId) {
        let (start, separator, stop) = (start.into(), separator.into(), stop.into());

        expect!(
            self,
            self.expect(start),
            {
                let mut items = Vec::new();
                while self.tokens.peek_kind() != Some(stop) {
                    items.push(parse(self));
                    if self.bump_if(separator).is_none() {
                        break;
                    }
                }
                items
            },
            self.expect(stop)
        )
    }

    fn parse_parenthetical_list(
        &mut self,
        parse: fn(&mut Self) -> NodeId,
    ) -> (TokenId, Vec<NodeId>, TokenId) {
        self.parse_list(token!('('), parse, token!(,), token!(')'))
    }

    fn parse_call(&mut self, callee: NodeId) -> NodeId {
        let (paren, args, _) = self.parse_parenthetical_list(|parser| parser.parse_expr());
        self.ast.push(Expr::Call {
            paren,
            callee,
            args,
        })
    }

    fn parse_bracketed_suffix(&mut self, base: NodeId) -> NodeId {
        let (bracket, terms, _) = self.parse_list(
            token!('['),
            |parser| parser.parse_bracket_term(),
            token!(,),
            token!(']'),
        );
        self.ast.push(Expr::Bracketed {
            bracket,
            base,
            terms,
        })
    }

    fn parse_bracket_term(&mut self) -> BracketTerm {
        let start = if self.tokens.peek_kind() != Some(token!(:)) {
            Some(self.parse_expr())
        } else {
            None
        };

        if self.bump_if(token!(:)).is_some() {
            let end = if !matches!(self.tokens.peek_kind(), Some(token!(']') | token!(,))) {
                Some(self.parse_expr())
            } else {
                None
            };
            BracketTerm {
                start,
                end,
                is_range: true,
            }
        } else {
            BracketTerm {
                start,
                end: None,
                is_range: false,
            }
        }
    }

    fn parse_field(&mut self, base: NodeId) -> NodeId {
        let (_, name) = expect!(self, token!(.), TokenKind::Identifier);
        self.ast.push(Expr::Field { name, base })
    }

    fn parse_literal(&mut self, literal: Literal) -> NodeId {
        let token = self.bump();
        let node = match literal {
            Literal::Int32 => ast::Literal::Int32 { token },
            Literal::Int64 => ast::Literal::Int64 { token },
            Literal::Float32 => ast::Literal::Float32 { token },
            Literal::Float64 => ast::Literal::Float64 { token },
            Literal::Imaginary32 => ast::Literal::Imaginary32 { token },
            Literal::Imaginary64 => ast::Literal::Imaginary64 { token },
            Literal::String => ast::Literal::String { token },
        };
        self.ast.push(Expr::Literal(node))
    }

    fn try_parse_label(&mut self) -> Option<TokenId> {
        let starts_labellable_stmt = matches!(
            self.tokens.clone().nth(2).map(|(_, token)| token.kind),
            Some(token!('{') | token!(loop) | token!(for) | token!(while))
        );
        if self.tokens.peek_kind() == Some(TokenKind::Identifier)
            && self.at(1, token!(:))
            && starts_labellable_stmt
        {
            let label = self.bump();
            self.bump();
            Some(label)
        } else {
            None
        }
    }

    fn parse_statement(&mut self) -> NodeId {
        let label = self.try_parse_label();

        match self.tokens.peek_kind() {
            Some(token!('{')) => self.parse_block(label),
            Some(token!(let)) => self.parse_let(),
            Some(token!(var)) => self.parse_var(),
            Some(token!(using)) => self.parse_using_stmt(),
            Some(token!(if)) => self.parse_if(),
            Some(token!(while)) => self.parse_while(label),
            Some(token!(loop)) => self.parse_loop(label),
            Some(token!(for)) => self.parse_for(label),
            Some(token!(forward_branch)) => self.parse_forward_branch(),
            Some(token!(node)) => {
                let mut nodes = self.parse_node_group();
                nodes.pop().unwrap_or_else(|| {
                    let token = self.tokens.current();
                    self.ast.push(Node::Error { token })
                })
            }
            Some(token!(connection)) => self.parse_connection_decl(),
            Some(token!(return)) => self.parse_return(),
            Some(token!(break)) => self.parse_break(),
            Some(token!(continue)) => self.parse_continue(),
            Some(token!(namespace)) => self.parse_namespace(),
            Some(token!(import)) => self.parse_import(),
            Some(token!(enum)) => self.parse_enum(),
            Some(token!(external)) => self.parse_external_decl(),
            Some(token!(processor))
                if self.tokens.clone().nth(1).map(|(_, token)| token.kind) == Some(token!(.)) =>
            {
                self.parse_expr_stmt()
            }
            Some(token!(processor | graph | struct)) => self.parse_container(),
            Some(token!(input | output)) => {
                let mut endpoints = self.parse_endpoint_group();
                endpoints.pop().unwrap_or_else(|| {
                    let token = self.tokens.current();
                    self.ast.push(Node::Error { token })
                })
            }
            Some(token!(event)) => self.parse_event_handler(),
            Some(token!(const)) => self.parse_typed_decl(),
            Some(TokenKind::Identifier) if self.peek_text_is("static_assert") => {
                self.parse_static_assert()
            }
            Some(TokenKind::Keyword(keyword)) if TokenKind::Keyword(keyword).is_type_like() => {
                self.parse_typed_decl()
            }
            Some(TokenKind::Identifier) if self.looks_like_typed_decl() => self.parse_typed_decl(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_container_items(&mut self) -> Vec<NodeId> {
        let mut items = Vec::new();
        while !matches!(self.tokens.peek_kind(), Some(token!('}')) | None) {
            if matches!(self.tokens.peek_kind(), Some(token!(input | output))) {
                items.extend(self.parse_endpoint_group());
            } else if matches!(self.tokens.peek_kind(), Some(token!(node))) {
                items.extend(self.parse_node_group());
            } else {
                items.push(self.parse_statement());
            }
        }
        items
    }

    fn parse_break(&mut self) -> NodeId {
        let keyword = self.expect(token!(break));
        let target = (self.tokens.peek_kind() == Some(TokenKind::Identifier)).then(|| self.bump());
        self.expect(token!(;));
        self.ast.push(Stmt::BreakStmt { keyword, target })
    }

    fn parse_continue(&mut self) -> NodeId {
        let keyword = self.expect(token!(continue));
        let target = (self.tokens.peek_kind() == Some(TokenKind::Identifier)).then(|| self.bump());
        self.expect(token!(;));
        self.ast.push(Stmt::ContinueStmt { keyword, target })
    }

    fn parse_forward_branch(&mut self) -> NodeId {
        let keyword = self.expect(token!(forward_branch));
        self.expect(token!('('));
        let cond = self.parse_expr();
        self.expect(token!(')'));
        self.expect(token!(->));
        let (_, targets, _) = self.parse_list(
            token!('('),
            |p| p.expect(TokenKind::Identifier),
            token!(,),
            token!(')'),
        );
        self.expect(token!(;));
        self.ast.push(Stmt::ForwardBranchStmt {
            keyword,
            cond,
            targets,
        })
    }

    fn parse_static_assert(&mut self) -> NodeId {
        let keyword = self.bump();
        self.expect(token!('('));
        let cond = self.parse_expr();
        let message = self.bump_if(token!(,)).map(|_| self.parse_expr());
        self.expect(token!(')'));
        self.expect(token!(;));
        self.ast.push(Stmt::StaticAssertStmt {
            keyword,
            cond,
            message,
        })
    }

    fn parse_import(&mut self) -> NodeId {
        let keyword = self.expect(token!(import));
        let path = if self.tokens.peek_kind() == Some(TokenKind::Literal(Literal::String)) {
            vec![self.bump()]
        } else {
            let mut path = vec![self.expect(TokenKind::Identifier)];
            while self.bump_if(token!(.)).is_some() {
                path.push(self.expect(TokenKind::Identifier));
            }
            path
        };
        self.expect(token!(;));
        self.ast.push(Item::Import { keyword, path })
    }

    fn parse_enum(&mut self) -> NodeId {
        let keyword = self.expect(token!(enum));
        let name = self.expect(TokenKind::Identifier);
        let (_, values, _) = self.parse_list(
            token!('{'),
            |p| p.expect(TokenKind::Identifier),
            token!(,),
            token!('}'),
        );
        self.ast.push(Item::EnumDecl {
            keyword,
            name,
            values,
        })
    }

    fn parse_external_decl(&mut self) -> NodeId {
        self.expect(token!(external));
        self.parse_typed_decl_inner(true, true)
    }

    fn parse_using_stmt(&mut self) -> NodeId {
        let (keyword, name, _, target, _) = expect!(
            self,
            token!(using),
            TokenKind::Identifier,
            token!(=),
            self.parse_type(),
            token!(;)
        );
        let decl = self.ast.push(Decl::Alias {
            keyword,
            kind: ast::AliasKind::Using,
            name,
            target: Some(target),
        });
        self.ast.push(Stmt::DeclStmt { decl })
    }

    fn parse_typed_decl(&mut self) -> NodeId {
        self.parse_typed_decl_inner(true, false)
    }

    fn parse_typed_decl_inner(&mut self, consume_semicolon: bool, is_external: bool) -> NodeId {
        let (ty, name) = expect!(self, self.parse_type(), TokenKind::Identifier);

        if self.tokens.peek_kind() == Some(token!(<))
            || self.tokens.peek_kind() == Some(token!('('))
        {
            let generics = self.parse_optional_generics();
            return self.parse_function_decl(Some(ty), name, generics);
        }

        let mut declarators = vec![Declarator {
            name,
            init: expect!(
                self,
                if token!(=) {
                    self.parse_expr()
                }
            )
            .map(|(_, init)| init),
        }];
        while self.tokens.peek_kind() == Some(token!(,)) {
            self.bump();

            let (name, init) = expect!(
                self,
                TokenKind::Identifier,
                if token!(=) {
                    self.parse_expr()
                }
            );
            declarators.push(Declarator {
                name,
                init: init.map(|(_, init)| init),
            });
        }
        let attributes =
            (self.tokens.peek_kind() == Some(token!("[["))).then(|| self.parse_attribute_list());
        if consume_semicolon {
            self.expect(token!(;));
        }
        let decl = self.ast.push(Decl::Var {
            role: VarRole::Typed,
            ty: Some(ty),
            is_external,
            declarators,
            attributes,
        });
        self.ast.push(Stmt::DeclStmt { decl })
    }

    fn parse_optional_generics(&mut self) -> Vec<TokenId> {
        expect!(
            self,
            if peek token!(<) {
                self.parse_list(
                    token!(<),
                    |parser| parser.expect(TokenKind::Identifier),
                    token!(,),
                    token!(>),
                )
            }
        )
        .map(|(_, generics, _)| generics)
        .unwrap_or_default()
    }

    fn parse_param(&mut self) -> NodeId {
        let (ty, name) = expect!(self, self.parse_type(), TokenKind::Identifier);

        self.ast.push(Decl::Var {
            role: VarRole::Parameter,
            ty: Some(ty),
            is_external: false,
            declarators: vec![Declarator { name, init: None }],
            attributes: None,
        })
    }

    fn parse_params(&mut self) -> Vec<NodeId> {
        let (_, params, _) = self.parse_parenthetical_list(|parser| parser.parse_param());
        params
    }

    fn parse_function_decl(
        &mut self,
        ty: Option<NodeId>,
        name: TokenId,
        generics: Vec<TokenId>,
    ) -> NodeId {
        let params = self.parse_params();
        let is_const = self.bump_if(token!(const)).is_some();
        let attributes =
            (self.tokens.peek_kind() == Some(token!("[["))).then(|| self.parse_attribute_list());
        let body = self.parse_block(None);

        self.ast.push(Item::FunctionDecl {
            ty,
            name,
            generics,
            params,
            is_const,
            is_event_handler: false,
            attributes,
            body,
        })
    }

    fn parse_event_handler(&mut self) -> NodeId {
        self.expect(token!(event));
        let name = self.expect(TokenKind::Identifier);
        let params = self.parse_params();
        let body = self.parse_block(None);

        self.ast.push(Item::FunctionDecl {
            ty: None,
            name,
            generics: Vec::new(),
            params,
            is_const: false,
            is_event_handler: true,
            attributes: None,
            body,
        })
    }

    fn parse_namespace(&mut self) -> NodeId {
        let keyword = self.expect(token!(namespace));
        let mut segments = vec![self.expect(TokenKind::Identifier)];
        while self.bump_if(token!(::)).is_some() {
            segments.push(self.expect(TokenKind::Identifier));
        }
        let params = self.parse_optional_specialisation_params();

        if self.bump_if(token!(=)).is_some() {
            let target = self.parse_expr();
            self.expect(token!(;));
            let name = *segments.last().expect("namespace has at least one segment");
            return self.ast.push(Item::ModuleAlias {
                keyword,
                kind: ast::AliasKind::Namespace,
                name,
                target,
            });
        }

        let attributes =
            (self.tokens.peek_kind() == Some(token!("[["))).then(|| self.parse_attribute_list());
        self.expect(token!('{'));
        let items = self.parse_container_items();
        self.expect(token!('}'));

        self.ast.push(Item::NamespaceDecl {
            keyword,
            segments,
            params,
            attributes,
            items,
        })
    }

    fn parse_optional_specialisation_params(&mut self) -> Vec<NodeId> {
        expect!(self, if peek token!('(') {
            self.parse_parenthetical_list(|parser| parser.parse_specialisation_param())
        })
        .map(|(_, params, _)| params)
        .unwrap_or_default()
    }

    fn parse_specialisation_param(&mut self) -> NodeId {
        match self.tokens.peek_kind() {
            Some(token!(using)) => {
                let keyword = self.bump();
                let name = self.expect(TokenKind::Identifier);
                let target = self.bump_if(token!(=)).map(|_| self.parse_type());
                self.ast.push(Decl::Alias {
                    keyword,
                    kind: ast::AliasKind::Using,
                    name,
                    target,
                })
            }
            Some(token!(processor)) => {
                let keyword = self.bump();
                let name = self.expect(TokenKind::Identifier);
                let target = self.bump_if(token!(=)).map(|_| self.parse_expr());
                self.ast.push(Decl::Alias {
                    keyword,
                    kind: ast::AliasKind::Processor,
                    name,
                    target,
                })
            }
            Some(token!(namespace)) => {
                let keyword = self.bump();
                let name = self.expect(TokenKind::Identifier);
                let target = self.bump_if(token!(=)).map(|_| self.parse_expr());
                self.ast.push(Decl::Alias {
                    keyword,
                    kind: ast::AliasKind::Namespace,
                    name,
                    target,
                })
            }
            _ => {
                let ty = self.parse_type();
                let name = self.expect(TokenKind::Identifier);
                let init = self.bump_if(token!(=)).map(|_| self.parse_expr());
                self.ast.push(Decl::Var {
                    role: VarRole::SpecialisationValue,
                    ty: Some(ty),
                    is_external: false,
                    declarators: vec![Declarator { name, init }],
                    attributes: None,
                })
            }
        }
    }

    fn parse_container(&mut self) -> NodeId {
        let keyword = expect_matches!(self, token!(processor | graph | struct));
        let keyword_kind = self.tokens.stream().get(keyword).map(|token| token.kind);
        let name = self.expect(TokenKind::Identifier);
        let params = self.parse_optional_specialisation_params();

        if matches!(keyword_kind, Some(token!(processor | graph)))
            && self.tokens.peek_kind() == Some(token!(=))
        {
            self.bump();
            let target = self.parse_expr();
            self.expect(token!(;));
            return self.ast.push(Item::ModuleAlias {
                keyword,
                kind: ast::AliasKind::Processor,
                name,
                target,
            });
        }

        let attributes =
            (self.tokens.peek_kind() == Some(token!("[["))).then(|| self.parse_attribute_list());
        self.expect(token!('{'));
        let items = self.parse_container_items();
        self.expect(token!('}'));

        match keyword_kind {
            Some(token!(graph)) => self.ast.push(Item::GraphDecl {
                keyword,
                name,
                params,
                attributes,
                items,
            }),
            Some(token!(struct)) => self.ast.push(Item::StructDecl {
                keyword,
                name,
                attributes,
                items,
            }),
            Some(token!(processor)) => self.ast.push(Item::ProcessorDecl {
                keyword,
                name,
                params,
                attributes,
                items,
            }),
            _ => unreachable!(),
        }
    }

    fn parse_node_group(&mut self) -> Vec<NodeId> {
        let keyword = self.expect(token!(node));
        if self.bump_if(token!('{')).is_some() {
            let mut nodes = Vec::new();
            while !matches!(self.tokens.peek_kind(), Some(token!('}')) | None) {
                nodes.push(self.parse_node_decl_entry(keyword));
            }
            self.expect(token!('}'));
            nodes
        } else {
            vec![self.parse_node_decl_entry(keyword)]
        }
    }

    fn parse_node_decl_entry(&mut self, keyword: TokenId) -> NodeId {
        let name = self.expect(TokenKind::Identifier);
        let array_size = self.bump_if(token!('[')).map(|_| {
            let size = if self.tokens.peek_kind() != Some(token!(']')) {
                Some(self.parse_expr())
            } else {
                None
            };
            self.expect(token!(']'));
            size
        });
        self.expect(token!(=));
        let processor = self.parse_expr();
        self.expect(token!(;));

        self.ast.push(Graph::NodeDecl {
            keyword,
            name,
            processor,
            array_size: array_size.flatten(),
        })
    }

    fn parse_connection_decl(&mut self) -> NodeId {
        let keyword = self.expect(token!(connection));
        let connections = self.parse_connection_list();
        self.ast.push(Graph::ConnectionDecl {
            keyword,
            connections,
        })
    }

    fn parse_connection_list(&mut self) -> Vec<NodeId> {
        let braced = self.bump_if(token!('{')).is_some();

        let mut connections = Vec::new();
        loop {
            if braced && self.bump_if(token!('}')).is_some() {
                break;
            }

            if self.tokens.peek_kind() == Some(token!(if)) {
                connections.push(self.parse_connection_if());
            } else {
                connections.extend(self.parse_connection_chain());
                self.expect(token!(;));
            }

            if !braced {
                break;
            }
        }
        connections
    }

    fn parse_connection_if(&mut self) -> NodeId {
        let (keyword, _, cond, _, then_branch, else_branch) = expect!(
            self,
            token!(if),
            token!('('),
            self.parse_expr(),
            token!(')'),
            self.parse_connection_list(),
            if token!(else) {
                self.parse_connection_list()
            }
        );
        let else_branch = else_branch.map(|(_, else_branch)| else_branch);

        self.ast.push(Graph::ConnectionIf {
            keyword,
            cond,
            then_branch,
            else_branch,
        })
    }

    fn parse_connection_chain(&mut self) -> Vec<NodeId> {
        let interpolation = self.parse_interpolation_if_present();
        let mut connections = Vec::new();
        let mut sources = self.parse_connection_endpoints();
        loop {
            let (arrow, delay, destinations) = expect!(
                self,
                token!(->),
                if token!('[') {
                    self.parse_expr(),
                    token!(']'),
                    token!(->)
                },
                self.parse_connection_endpoints()
            );

            if sources.len() > 1 && destinations.len() > 1 {
                self.error(arrow, "many-to-many connections are not supported");
            }

            let is_end_of_chain = self.tokens.peek_kind() != Some(token!(->));
            if !is_end_of_chain {
                if destinations.len() > 1 {
                    self.error(
                        arrow,
                        "cannot chain a connection with multiple destinations",
                    );
                } else if let Some(&dest) = destinations.first() {
                    if matches!(self.ast.get(dest), Node::Expr(Expr::Field { .. })) {
                        self.error(
                            arrow,
                            "cannot name an endpoint in the middle of a connection chain",
                        );
                    }
                }
            }

            connections.push(self.ast.push(Graph::Connection {
                interpolation,
                sources,
                arrow,
                delay: delay.map(|(_, delay, _, _)| delay),
                destinations: destinations.clone(),
            }));

            if is_end_of_chain {
                break;
            }
            sources = destinations;
        }
        connections
    }

    fn parse_connection_endpoints(&mut self) -> Vec<NodeId> {
        let mut endpoints = vec![self.parse_expr()];
        while self.bump_if(token!(,)).is_some() {
            endpoints.push(self.parse_expr());
        }
        endpoints
    }

    fn parse_interpolation_if_present(&mut self) -> Option<InterpolationKind> {
        let mut peek = self.tokens.clone().map(|(id, token)| (id, token.kind));

        match (peek.next(), peek.next(), peek.next()) {
            (
                Some((_, token!('['))),
                Some((delay, TokenKind::Identifier)),
                Some((_, token!(']'))),
            ) => {
                let kind = match self.tokens.stream().text(self.source, delay) {
                    Some("none") => Some(InterpolationKind::None),
                    Some("latch") => Some(InterpolationKind::Latch),
                    Some("linear") => Some(InterpolationKind::Linear),
                    Some("sinc") => Some(InterpolationKind::Sinc),
                    Some("fast") => Some(InterpolationKind::Fast),
                    Some("best") => Some(InterpolationKind::Best),
                    _ => None,
                };

                if let Some(kind) = kind {
                    let _ = expect!(self, token!('['), TokenKind::Identifier, token!(']'));
                    return Some(kind);
                }

                None
            }
            _ => None,
        }
    }

    fn parse_for(&mut self, label: Option<TokenId>) -> NodeId {
        let keyword = self.expect(token!(for));
        self.expect(token!('('));

        let init = (self.tokens.peek_kind() != Some(token!(;))).then(|| self.parse_for_init());

        let is_bounded_range_for = self.tokens.peek_kind() == Some(token!(')'))
            && matches!(
                init.map(|id| self.ast.get(id)),
                Some(Node::Stmt(Stmt::DeclStmt { .. }))
            );

        if is_bounded_range_for {
            self.bump();
            let body = self.parse_statement();
            return self.ast.push(Stmt::LoopStmt {
                keyword,
                label,
                count: init,
                body,
            });
        }

        self.expect(token!(;));
        let cond = (self.tokens.peek_kind() != Some(token!(;))).then(|| self.parse_expr());
        self.expect(token!(;));
        let update = (self.tokens.peek_kind() != Some(token!(')'))).then(|| self.parse_expr());
        self.expect(token!(')'));
        let body = self.parse_statement();

        self.ast.push(Stmt::ForStmt {
            keyword,
            label,
            init,
            cond,
            update,
            body,
        })
    }

    fn parse_for_init(&mut self) -> NodeId {
        match self.tokens.peek_kind() {
            Some(token!(const)) => self.parse_typed_decl_inner(false, false),
            Some(TokenKind::Keyword(keyword)) if keyword.is_type() => {
                self.parse_typed_decl_inner(false, false)
            }
            Some(TokenKind::Identifier) if self.looks_like_typed_decl() => {
                self.parse_typed_decl_inner(false, false)
            }
            _ => {
                let expr = self.parse_expr();
                self.ast.push(Stmt::ExprStmt { expr })
            }
        }
    }

    fn at(&self, offset: usize, kind: TokenKind) -> bool {
        self.tokens
            .clone()
            .nth(offset)
            .is_some_and(|(_, token)| token.kind == kind)
    }

    fn looks_like_typed_decl(&self) -> bool {
        let mut offset = 0;
        if !self.at(offset, TokenKind::Identifier) {
            return false;
        }
        offset += 1;
        while self.at(offset, token!(::)) {
            offset += 1;
            if !self.at(offset, TokenKind::Identifier) {
                return false;
            }
            offset += 1;
        }
        loop {
            let next = if self.at(offset, token!(<)) {
                self.skip_to_matching_angle_bracket(offset)
            } else if self.at(offset, token!('[')) {
                self.skip_to_matching_bracket(offset)
            } else {
                break;
            };
            match next {
                Some(next) => offset = next,
                None => return false,
            }
        }
        self.at(offset, TokenKind::Identifier)
    }

    fn skip_to_matching_angle_bracket(&self, mut offset: usize) -> Option<usize> {
        offset += 1;
        loop {
            match self.tokens.clone().nth(offset).map(|(_, t)| t.kind)? {
                token!(>) => return Some(offset + 1),
                token!(; | '{' | '}') => return None,
                _ => offset += 1,
            }
        }
    }

    fn skip_to_matching_bracket(&self, mut offset: usize) -> Option<usize> {
        let mut depth = 0;
        loop {
            match self.tokens.clone().nth(offset).map(|(_, t)| t.kind)? {
                token!('[') => {
                    depth += 1;
                    offset += 1;
                }
                token!(']') => {
                    depth -= 1;
                    offset += 1;
                    if depth == 0 {
                        return Some(offset);
                    }
                }
                token!(; | '{' | '}') => return None,
                _ => offset += 1,
            }
        }
    }

    fn parse_attribute(&mut self) -> (TokenId, Option<NodeId>) {
        let key = match self.tokens.peek_kind() {
            Some(TokenKind::Identifier | TokenKind::Keyword(_)) => self.bump(),
            _ => self.expect(TokenKind::Identifier),
        };
        let value = self.bump_if(token!(:)).map(|_| self.parse_expr());

        (key, value)
    }

    fn parse_attribute_list(&mut self) -> Vec<(TokenId, Option<NodeId>)> {
        let (_, attrs, _) = self.parse_list(
            token!("[["),
            |parser| parser.parse_attribute(),
            token!(,),
            token!("]]"),
        );
        attrs
    }

    fn looks_like_hoisted_endpoint(&self) -> bool {
        self.tokens.peek_kind() == Some(TokenKind::Identifier) && self.at(1, token!(.))
    }

    fn parse_hoisted_endpoint(&mut self, direction: TokenId) -> NodeId {
        let child = self.expect(TokenKind::Identifier);
        let index = self.bump_if(token!('[')).map(|_| {
            let index = self.parse_expr();
            self.expect(token!(']'));
            index
        });
        self.expect(token!(.));

        let target = if self.bump_if(token!(*)).is_some() {
            HoistTarget::Wildcard { prefix: None }
        } else {
            let name = self.expect(TokenKind::Identifier);
            if self.bump_if(token!(*)).is_some() {
                HoistTarget::Wildcard { prefix: Some(name) }
            } else {
                HoistTarget::Name(name)
            }
        };
        self.expect(token!(;));

        let name = match target {
            HoistTarget::Name(name) => Some(name),
            HoistTarget::Wildcard { .. } => None,
        };

        self.ast.push(Graph::EndpointDecl {
            direction,
            kind: None,
            types: Vec::new(),
            name,
            size: None,
            hoisted: Some(HoistedPath {
                segments: vec![child],
                index,
                target,
            }),
            attributes: None,
        })
    }

    fn parse_endpoint_member(&mut self, direction: TokenId, kind: TokenId) -> Vec<NodeId> {
        let types = if self.tokens.peek_kind() == Some(token!('(')) {
            let (_, types, _) = self.parse_parenthetical_list(|parser| parser.parse_type());
            types
        } else {
            vec![self.parse_type()]
        };

        let mut names = vec![self.parse_endpoint_name()];
        while self.bump_if(token!(,)).is_some() {
            names.push(self.parse_endpoint_name());
        }

        let attributes =
            (self.tokens.peek_kind() == Some(token!("[["))).then(|| self.parse_attribute_list());
        self.expect(token!(;));

        names
            .into_iter()
            .map(|(name, size)| {
                self.ast.push(Graph::EndpointDecl {
                    direction,
                    kind: Some(kind),
                    types: types.clone(),
                    name: Some(name),
                    size,
                    hoisted: None,
                    attributes: attributes.clone(),
                })
            })
            .collect()
    }

    fn parse_endpoint_name(&mut self) -> (TokenId, Option<NodeId>) {
        let name = self.expect(TokenKind::Identifier);
        let size = self.bump_if(token!('[')).map(|_| {
            let size = if self.tokens.peek_kind() != Some(token!(']')) {
                Some(self.parse_expr())
            } else {
                None
            };
            self.expect(token!(']'));
            size
        });
        (name, size.flatten())
    }

    fn parse_endpoint_group(&mut self) -> Vec<NodeId> {
        let direction = expect_matches!(self, token!(input | output));
        if self.looks_like_hoisted_endpoint() {
            return vec![self.parse_hoisted_endpoint(direction)];
        }
        let kind = if self.tokens.peek_kind() == Some(token!(event)) {
            self.bump()
        } else {
            expect_identifier!(self, "value" | "stream")
        };
        if self.tokens.peek_kind() == Some(token!('{')) {
            self.bump();
            let mut endpoints = Vec::new();
            while !matches!(self.tokens.peek_kind(), Some(token!('}')) | None) {
                endpoints.extend(self.parse_endpoint_member(direction, kind));
            }
            self.expect(token!('}'));
            endpoints
        } else {
            self.parse_endpoint_member(direction, kind)
        }
    }

    fn parse_block(&mut self, label: Option<TokenId>) -> NodeId {
        let (brace, stmts, _) = expect!(
            self,
            token!('{'),
            {
                let mut stmts = Vec::new();
                while !matches!(self.tokens.peek_kind(), Some(token!('}')) | None) {
                    stmts.push(self.parse_statement());
                }
                stmts
            },
            token!('}')
        );

        self.ast.push(Stmt::Block {
            brace,
            label,
            stmts,
        })
    }

    fn parse_let(&mut self) -> NodeId {
        self.parse_let_or_var(token!(let), VarRole::Let)
    }

    fn parse_var(&mut self) -> NodeId {
        self.parse_let_or_var(token!(var), VarRole::Var)
    }

    fn parse_let_or_var(&mut self, keyword: impl Into<TokenKind>, role: VarRole) -> NodeId {
        let (_, declarators, _) = self.parse_list(
            keyword,
            |parser| parser.parse_let_declarator(),
            token!(,),
            token!(;),
        );

        let decl = self.ast.push(Decl::Var {
            role,
            ty: None,
            is_external: false,
            declarators,
            attributes: None,
        });
        self.ast.push(Stmt::DeclStmt { decl })
    }

    fn parse_let_declarator(&mut self) -> Declarator {
        let (name, _, init) = expect!(self, TokenKind::Identifier, token!(=), self.parse_expr());
        Declarator {
            name,
            init: Some(init),
        }
    }

    fn parse_if(&mut self) -> NodeId {
        let (keyword, is_const, _, cond, _, then_branch, else_branch) = expect!(
            self,
            token!(if),
            if token!(const),
            token!('('),
            self.parse_expr(),
            token!(')'),
            self.parse_statement(),
            if token!(else) {
                self.parse_statement()
            }
        );

        self.ast.push(Stmt::IfStmt {
            keyword,
            is_const: is_const.is_some(),
            cond,
            then_branch,
            else_branch: else_branch.map(|(_, else_branch)| else_branch),
        })
    }

    fn parse_while(&mut self, label: Option<TokenId>) -> NodeId {
        let (keyword, _, cond, _, body) = expect!(
            self,
            token!(while),
            token!('('),
            self.parse_expr(),
            token!(')'),
            self.parse_statement()
        );

        self.ast.push(Stmt::WhileStmt {
            keyword,
            label,
            cond,
            body,
        })
    }

    fn parse_loop(&mut self, label: Option<TokenId>) -> NodeId {
        let (keyword, count, body) = expect!(
            self,
            token!(loop),
            if token!('(') {
                self.parse_expr(), token!(')')
            },
            self.parse_statement()
        );

        self.ast.push(Stmt::LoopStmt {
            keyword,
            label,
            count: count.map(|(_, count, _)| count),
            body,
        })
    }

    fn parse_return(&mut self) -> NodeId {
        let (keyword, value, _) = expect!(
            self,
            token!(return),
            if !peek token!(;) {
                self.parse_expr()
            },
            token!(;)
        );

        self.ast.push(Stmt::ReturnStmt { keyword, value })
    }

    fn parse_expr_stmt(&mut self) -> NodeId {
        let (expr, _) = expect!(self, self.parse_expr(), token!(;));
        self.ast.push(Stmt::ExprStmt { expr })
    }

    fn parse_type(&mut self) -> NodeId {
        let is_const = self.bump_if(token!(const)).is_some();
        let mut ty = self.parse_type_base();
        let is_ref = self.bump_if(token!(&)).is_some();

        if is_const || is_ref {
            ty = self.ast.push(Expr::TypeModifier {
                source: ty,
                is_const,
                is_ref,
            });
        }

        ty
    }

    fn parse_type_base(&mut self) -> NodeId {
        let mut ty = match self.tokens.peek_kind() {
            Some(kind) if kind.is_type_like() => self.parse_qualified_name(),
            peeked => {
                let message = format!("expected type, found {peeked:?}");
                let token = self.bump();
                self.error(token, message);
                self.ast.push(Node::Error { token })
            }
        };

        loop {
            ty = match self.tokens.peek_kind() {
                Some(token!('[')) => self.parse_bracketed_suffix(ty),
                Some(token!(<)) => self.parse_vector_size_suffix(ty),
                _ => break,
            };
        }

        ty
    }

    fn parse_qualified_name(&mut self) -> NodeId {
        let token = self.bump();
        let mut name = self.ast.push(Expr::Ident { token });

        while self.tokens.peek_kind() == Some(token!(::)) {
            name = self.parse_scope_access(name);
        }

        name
    }

    fn parse_vector_size_suffix(&mut self, element: NodeId) -> NodeId {
        let (angle, term, _) = expect!(
            self,
            token!(<),
            self.parse_expr_with_min_binding_power(PrecedenceLevel::Shift.base()),
            token!(>)
        );

        self.ast.push(Expr::VectorSizeSuffix {
            angle,
            element,
            terms: vec![term],
        })
    }

    fn try_parse_vector_size_suffix(&mut self, element: NodeId) -> Option<NodeId> {
        let tokens_checkpoint = self.tokens.clone();
        let ast_len = self.ast.len();
        let diagnostics_len = self.diagnostics.len();

        let angle = self.bump();
        let mut terms = vec![self.parse_expr_with_min_binding_power(PrecedenceLevel::Shift.base())];
        while self.bump_if(token!(,)).is_some() {
            terms.push(self.parse_expr_with_min_binding_power(PrecedenceLevel::Shift.base()));
        }

        let succeeded =
            self.diagnostics.len() == diagnostics_len && self.tokens.peek_kind() == Some(token!(>));

        if !succeeded {
            self.tokens = tokens_checkpoint;
            self.ast.truncate(ast_len);
            self.diagnostics.truncate(diagnostics_len);
            return None;
        }

        self.bump();
        Some(self.ast.push(Expr::VectorSizeSuffix {
            angle,
            element,
            terms,
        }))
    }
}

#[cfg(test)]
mod tests {
    use {super::*, crate::ast};

    fn dump(source: &str, parse_fn: fn(&mut Parser) -> NodeId) -> String {
        let tokens = tokenize(source);
        let mut parser = Parser::new(&tokens, source);
        let root = parse_fn(&mut parser);
        assert_eq!(parser.diagnostics, vec![]);

        ast::dump(&parser.ast, &tokens, source, root)
    }

    fn has_diagnostics(source: &str, parse_fn: fn(&mut Parser) -> NodeId) -> bool {
        let tokens = tokenize(source);
        let mut parser = Parser::new(&tokens, source);
        parse_fn(&mut parser);
        !parser.diagnostics.is_empty()
    }

    fn parse_type(source: &str) -> String {
        dump(source, |parser| parser.parse_type())
    }

    fn parse_expr(source: &str) -> String {
        dump(source, |parser| parser.parse_expr())
    }

    fn parse_stmt(source: &str) -> String {
        dump(source, |parser| parser.parse_statement())
    }

    #[test]
    fn primitive_type() {
        insta::assert_snapshot!(parse_type("float"), @"float");
    }

    #[test]
    fn named_type() {
        insta::assert_snapshot!(parse_type("MyStruct"), @"MyStruct");
    }

    #[test]
    fn qualified_type_name() {
        insta::assert_snapshot!(parse_type("std::midi::Message"), @r#"
        ScopeAccess "Message"
          ScopeAccess "midi"
            std
        "#);
    }

    #[test]
    fn array_type() {
        insta::assert_snapshot!(parse_type("int[3]"), @"
        Bracketed
          int
          3
        ");
    }

    #[test]
    fn slice_type_has_no_size() {
        insta::assert_snapshot!(parse_type("int[]"), @"
        Bracketed
          int
        ");
    }

    #[test]
    fn vector_type() {
        insta::assert_snapshot!(parse_type("int<4>"), @"
        VectorSizeSuffix
          int
          4
        ");
    }

    #[test]
    fn clamp() {
        insta::assert_snapshot!(parse_type("clamp<10>"), @"
        VectorSizeSuffix
          clamp
          10
        ");
    }

    #[test]
    fn wrap() {
        insta::assert_snapshot!(parse_type("wrap<4>"), @"
        VectorSizeSuffix
          wrap
          4
        ");
    }

    #[test]
    fn angle_bracket_close_is_not_a_comparison() {
        insta::assert_snapshot!(parse_type("wrap<1 + 2>"), @r#"
        VectorSizeSuffix
          wrap
          Binary "+"
            1
            2
        "#);
    }

    #[test]
    fn postfix_type_modifiers_apply_left_to_right() {
        insta::assert_snapshot!(parse_type("int<4>[2]"), @"
        Bracketed
          VectorSizeSuffix
            int
            4
          2
        ");
    }

    #[test]
    fn const_type() {
        insta::assert_snapshot!(parse_type("const int"), @"
        TypeModifier const
          int
        ");
    }

    #[test]
    fn const_array_type() {
        insta::assert_snapshot!(parse_type("const int[]"), @"
        TypeModifier const
          Bracketed
            int
        ");
    }

    #[test]
    fn type_cast() {
        insta::assert_snapshot!(parse_expr("float (2.5)"), @"
        Call
          float
          2.5
        ");
    }

    #[test]
    fn null_literal() {
        insta::assert_snapshot!(parse_expr("()"), @"Parentheses");
    }

    #[test]
    fn aggregate_literal() {
        insta::assert_snapshot!(parse_expr("((1, 2), (3, 4))"), @"
        Parentheses
          Parentheses
            1
            2
          Parentheses
            3
            4
        ");
    }

    #[test]
    fn precedence_of_arithmetic() {
        insta::assert_snapshot!(parse_expr("1 + 2 * 3"), @r#"
        Binary "+"
          1
          Binary "*"
            2
            3
        "#);
    }

    #[test]
    fn power_is_right_associative() {
        insta::assert_snapshot!(parse_expr("2 ** 3 ** 4"), @r#"
        Binary "**"
          2
          Binary "**"
            3
            4
        "#);
    }

    #[test]
    fn subtraction_is_left_associative() {
        insta::assert_snapshot!(parse_expr("1 - 2 - 3"), @r#"
        Binary "-"
          Binary "-"
            1
            2
          3
        "#);
    }

    #[test]
    fn assignment_is_right_associative() {
        insta::assert_snapshot!(parse_expr("a = b = c"), @r#"
        Assign "="
          a
          Assign "="
            b
            c
        "#);
    }

    #[test]
    fn output_write_operator() {
        insta::assert_snapshot!(parse_expr("out <- in * gain"), @r#"
        Assign "<-"
          out
          Binary "*"
            in
            gain
        "#);
    }

    #[test]
    fn ternary_nests_to_the_right() {
        insta::assert_snapshot!(parse_expr("a ? b : c ? d : e"), @"
        Ternary
          a
          b
          Ternary
            c
            d
            e
        ");
    }

    #[test]
    fn ternary_binds_looser_than_logical_or() {
        insta::assert_snapshot!(parse_expr("a || b ? c : d"), @r#"
        Ternary
          Binary "||"
            a
            b
          c
          d
        "#);
    }

    #[test]
    fn unary_and_parens() {
        insta::assert_snapshot!(parse_expr("-(1 + 2)"), @r#"
        Unary "-"
          Parentheses
            Binary "+"
              1
              2
        "#);
    }

    #[test]
    fn prefix_increment() {
        insta::assert_snapshot!(parse_expr("++x"), @r#"
        Unary "++"
          x
        "#);
    }

    #[test]
    fn postfix_increment() {
        insta::assert_snapshot!(parse_expr("x++"), @r#"
        PostfixUnary "++"
          x
        "#);
    }

    #[test]
    fn call_with_args() {
        insta::assert_snapshot!(parse_expr("foo(1, 2 + 3)"), @r#"
        Call
          foo
          1
          Binary "+"
            2
            3
        "#);
    }

    #[test]
    fn call_with_no_args() {
        insta::assert_snapshot!(parse_expr("advance()"), @"
        Call
          advance
        ");
    }

    #[test]
    fn empty_index() {
        insta::assert_snapshot!(parse_expr("x[]"), @"
        Bracketed
          x
        ");
    }

    #[test]
    fn index_and_field_postfix() {
        insta::assert_snapshot!(parse_expr("x.left[3]"), @r#"
        Bracketed
          Field "left"
            x
          3
        "#);
    }

    #[test]
    fn let_statement() {
        insta::assert_snapshot!(parse_stmt("let x = 1;"), @r#"
        VarDecl let "x"
          1
        "#);
    }

    #[test]
    fn var_with_init_statement() {
        insta::assert_snapshot!(parse_stmt("var y = 3;"), @r#"
        VarDecl var "y"
          3
        "#);
    }

    #[test]
    fn var_multiple_declarators() {
        insta::assert_snapshot!(parse_stmt("var a = 1, b = 2;"), @r#"
        VarDecl var "a, b"
          1
          2
        "#);
    }

    #[test]
    fn typed_var_decl_statements() {
        insta::assert_snapshot!(parse_stmt("wrap<5> w; clamp<5> c; int n = 1;"), @r#"
        VarDecl typed "w"
          VectorSizeSuffix
            wrap
            5
        "#);
    }

    #[test]
    fn const_var_decl_statement() {
        insta::assert_snapshot!(parse_stmt("const int x = 1;"), @r#"
        VarDecl typed "x"
          TypeModifier const
            int
          1
        "#);
    }

    #[test]
    fn if_else_statement() {
        insta::assert_snapshot!(parse_stmt("if (a) { b; } else { c; }"), @"
        IfStmt
          a
          Block
            ExprStmt
              b
          Block
            ExprStmt
              c
        ");
    }

    #[test]
    fn if_without_else() {
        insta::assert_snapshot!(parse_stmt("if (a) { b; }"), @"
        IfStmt
          a
          Block
            ExprStmt
              b
        ");
    }

    #[test]
    fn if_const_statement() {
        insta::assert_snapshot!(parse_stmt("if const (a) { b; }"), @"
        IfStmt const
          a
          Block
            ExprStmt
              b
        ");
    }

    #[test]
    fn while_and_bounded_loop() {
        insta::assert_snapshot!(parse_stmt(
            "while (n > 0) { n = n - 1; } loop (4) { advance(); }"
        ), @r#"
        WhileStmt
          Binary ">"
            n
            0
          Block
            ExprStmt
              Assign "="
                n
                Binary "-"
                  n
                  1
        "#);
    }

    #[test]
    fn unbounded_loop_has_no_count() {
        insta::assert_snapshot!(parse_stmt("loop { advance(); }"), @"
        LoopStmt
          Block
            ExprStmt
              Call
                advance
        ");
    }

    #[test]
    fn return_with_and_without_value() {
        insta::assert_snapshot!(parse_stmt("return; return x + 1;"), @"ReturnStmt");
    }

    #[test]
    fn break_and_continue() {
        insta::assert_snapshot!(parse_stmt("loop { break; continue; }"), @"
        LoopStmt
          Block
            BreakStmt
            ContinueStmt
        ");
    }

    #[test]
    fn function() {
        insta::assert_snapshot!(parse_stmt("int add(int a, int b) { return a + b; }"), @r#"
        FunctionDecl "add"
          int
          VarDecl param "a"
            int
          VarDecl param "b"
            int
          Block
            ReturnStmt
              Binary "+"
                a
                b
        "#);
    }

    #[test]
    fn function_with_const_params() {
        insta::assert_snapshot!(parse_stmt("void f(const int& a, const float32[10]& b) { }"), @r#"
        FunctionDecl "f"
          void
          VarDecl param "a"
            TypeModifier const ref
              int
          VarDecl param "b"
            TypeModifier const ref
              Bracketed
                float32
                10
          Block
        "#);
    }

    #[test]
    fn const_member_function() {
        insta::assert_snapshot!(parse_stmt("void f() const { }"), @r#"
        FunctionDecl "f" const
          void
          Block
        "#);
    }

    #[test]
    fn loop_with_unbraced_body() {
        insta::assert_snapshot!(parse_stmt("void main() { loop advance(); }"), @r#"
        FunctionDecl "main"
          void
          Block
            LoopStmt
              ExprStmt
                Call
                  advance
        "#);
    }

    #[test]
    fn processor_with_typed_specialisation_param() {
        insta::assert_snapshot!(parse_stmt(
            "processor SquareWave (int length) { output stream int out; }"
        ), @r#"
        ProcessorDecl "SquareWave"
          VarDecl specialisation "length"
            int
          EndpointDecl output stream "out"
            int
        "#);
    }

    #[test]
    fn processor_with_typed_specialisation_param_default_value() {
        insta::assert_snapshot!(parse_stmt(
            "processor Gain (int channelCount = 2) { output stream int out; }"
        ), @r#"
        ProcessorDecl "Gain"
          VarDecl specialisation "channelCount"
            int
            2
          EndpointDecl output stream "out"
            int
        "#);
    }

    #[test]
    fn processor_with_using_specialisation_param() {
        insta::assert_snapshot!(parse_stmt(
            "processor Source (using DataType) { output stream int out; }"
        ), @r#"
        ProcessorDecl "Source"
          Alias using "DataType"
          EndpointDecl output stream "out"
            int
        "#);
    }

    #[test]
    fn processor_with_using_specialisation_param_default_type() {
        insta::assert_snapshot!(parse_stmt(
            "processor P (using T = float32) { output stream int out; }"
        ), @r#"
        ProcessorDecl "P"
          Alias using "T"
            float32
          EndpointDecl output stream "out"
            int
        "#);
    }

    #[test]
    fn graph_with_processor_specialisation_param() {
        insta::assert_snapshot!(parse_stmt(
            "graph Wrapper (processor Parameterised, int x) { output stream int out; }"
        ), @r#"
        GraphDecl "Wrapper"
          Alias processor "Parameterised"
          VarDecl specialisation "x"
            int
          EndpointDecl output stream "out"
            int
        "#);
    }

    #[test]
    fn namespace_with_specialisation_params() {
        insta::assert_snapshot!(parse_stmt("namespace n (processor p, namespace ns) {}"), @r#"
        NamespaceDecl "n"
          Alias processor "p"
          Alias namespace "ns"
        "#);
    }

    #[test]
    fn multiple_specialisation_params_of_different_kinds() {
        insta::assert_snapshot!(parse_stmt(
            "processor P (using T, int length = 4) { output stream int out; }"
        ), @r#"
        ProcessorDecl "P"
          Alias using "T"
          VarDecl specialisation "length"
            int
            4
          EndpointDecl output stream "out"
            int
        "#);
    }

    #[test]
    fn processor_latency_assignment_is_not_a_container_decl() {
        insta::assert_snapshot!(parse_stmt("processor.latency = length;"), @r#"
        ExprStmt
          Assign "="
            ProcessorProperty "latency"
            length
        "#);
    }

    #[test]
    fn chevron_suffix_wins_over_comparison_when_it_parses_cleanly() {
        insta::assert_snapshot!(parse_expr("a<b>"), @r#"
        VectorSizeSuffix
          a
          b
        "#);
    }

    #[test]
    fn bare_less_than_comparison_without_a_following_greater_than() {
        insta::assert_snapshot!(parse_expr("a < b"), @r#"
        Binary "<"
          a
          b
        "#);
    }

    #[test]
    fn comparison_chain_requires_parens() {
        insta::assert_snapshot!(parse_expr("(a < b) > c"), @r#"
        Binary ">"
          Parentheses
            Binary "<"
              a
              b
          c
        "#);
    }

    #[test]
    fn dangling_identifier_after_greedy_chevron_suffix_is_a_parse_error_in_expression_position() {
        assert!(has_diagnostics("return a < b > c;", |parser| parser.parse_statement()));
    }

    #[test]
    fn ambiguous_chevron_at_statement_start_is_read_as_a_type_decl() {
        insta::assert_snapshot!(parse_stmt("a < b > c;"), @r#"
        VarDecl typed "c"
          VectorSizeSuffix
            a
            b
        "#);
    }

    #[test]
    fn short_circuit_comparison_is_unaffected_by_chevron_backtracking() {
        insta::assert_snapshot!(parse_expr("a < b && b < c"), @r#"
        Binary "&&"
          Binary "<"
            a
            b
          Binary "<"
            b
            c
        "#);
    }

    #[test]
    fn type_sized_call_argument() {
        insta::assert_snapshot!(parse_expr("Sine(float64<2>, 100.0f)"), @r#"
        Call
          Sine
          VectorSizeSuffix
            float64
            2
          100.0f
        "#);
    }

    #[test]
    fn connection() {
        insta::assert_snapshot!(parse_stmt("connection node1.out -> node2.in;"), @r#"
        ConnectionDecl
          Connection
            Sources
              Field "out"
                node1
            Destinations
              Field "in"
                node2
        "#);
    }

    #[test]
    fn connection_to_single_input() {
        insta::assert_snapshot!(parse_stmt("connection node1.out -> node2;"), @r#"
        ConnectionDecl
          Connection
            Sources
              Field "out"
                node1
            Destinations
              node2
        "#);
    }

    #[test]
    fn connections_in_a_chain() {
        insta::assert_snapshot!(parse_stmt("connection node1.out -> node2 -> node3;"), @r#"
        ConnectionDecl
          Connection
            Sources
              Field "out"
                node1
            Destinations
              node2
          Connection
            Sources
              node2
            Destinations
              node3
        "#);
    }

    #[test]
    fn connection_to_multiple_destinations() {
        insta::assert_snapshot!(parse_stmt("connection node1.out -> node2, node3;"), @r#"
        ConnectionDecl
          Connection
            Sources
              Field "out"
                node1
            Destinations
              node2
              node3
        "#);
    }

    #[test]
    fn connection_to_multiple_sources() {
        insta::assert_snapshot!(parse_stmt("connection node1.out, node2.out -> node3;"), @r#"
        ConnectionDecl
          Connection
            Sources
              Field "out"
                node1
              Field "out"
                node2
            Destinations
              node3
        "#);
    }

    #[test]
    fn connection_with_delay() {
        insta::assert_snapshot!(parse_stmt("connection node1.out -> [100] -> node2;"), @r#"
        ConnectionDecl
          Connection
            Sources
              Field "out"
                node1
            Delay
              100
            Destinations
              node2
        "#);
    }

    #[test]
    fn connection_with_interpolation() {
        insta::assert_snapshot!(parse_stmt("connection [linear] node1.out -> node2;"), @r#"
        ConnectionDecl
          Connection [linear]
            Sources
              Field "out"
                node1
            Destinations
              node2
        "#);
    }

    #[test]
    fn connection_block() {
        insta::assert_snapshot!(parse_stmt("connection  { node1.out -> node2, node3; node2.out -> node4; }"), @r#"
        ConnectionDecl
          Connection
            Sources
              Field "out"
                node1
            Destinations
              node2
              node3
          Connection
            Sources
              Field "out"
                node2
            Destinations
              node4
        "#);
    }

    #[test]
    fn empty_connection_block() {
        insta::assert_snapshot!(parse_stmt("connection {}"), @r#"
        ConnectionDecl
        "#);
    }

    #[test]
    fn conditional_connection() {
        insta::assert_snapshot!(
            parse_stmt("connection { if (useDistortionFirst) in -> distortion -> out; else in -> out; }"),
            @"
        ConnectionDecl
          ConnectionIf
            useDistortionFirst
            Then
              Connection
                Sources
                  in
                Destinations
                  distortion
              Connection
                Sources
                  distortion
                Destinations
                  out
            Else
              Connection
                Sources
                  in
                Destinations
                  out
        "
        );
    }

    #[test]
    fn infinite_for_loop() {
        insta::assert_snapshot!(parse_stmt("for (;;) { advance(); }"), @"
        ForStmt
          Block
            ExprStmt
              Call
                advance
        ");
    }

    #[test]
    fn classic_for_loop() {
        insta::assert_snapshot!(parse_stmt("for (int i = 0; i < 10; ++i) { advance(); }"), @r#"
        ForStmt
          VarDecl typed "i"
            int
            0
          Binary "<"
            i
            10
          Unary "++"
            i
          Block
            ExprStmt
              Call
                advance
        "#);
    }

    #[test]
    fn bounded_range_for_loop() {
        insta::assert_snapshot!(parse_stmt("for (wrap<4> i) { advance(); }"), @r#"
        LoopStmt
          VarDecl typed "i"
            VectorSizeSuffix
              wrap
              4
          Block
            ExprStmt
              Call
                advance
        "#);
    }

    #[test]
    fn labelled_bounded_range_for_loop() {
        insta::assert_snapshot!(parse_stmt("outer: for (wrap<4> i) { advance(); }"), @r#"
        LoopStmt "outer"
          VarDecl typed "i"
            VectorSizeSuffix
              wrap
              4
          Block
            ExprStmt
              Call
                advance
        "#);
    }

    #[test]
    fn enum_decl() {
        insta::assert_snapshot!(parse_stmt("enum Mode { A, B, C }"), @r#"EnumDecl "Mode" {A, B, C}"#);
    }

    #[test]
    fn import_dotted_path() {
        insta::assert_snapshot!(parse_stmt("import std.audio;"), @r#"Import "std.audio""#);
    }

    #[test]
    fn import_string_path() {
        insta::assert_snapshot!(parse_stmt(r#"import "foo.cmajor";"#), @r#"Import "\"foo.cmajor\"""#);
    }

    #[test]
    fn external_var_decl() {
        insta::assert_snapshot!(parse_stmt("external float64 one;"), @r#"
        VarDecl external typed "one"
          float64
        "#);
    }

    #[test]
    fn using_type_alias_statement() {
        insta::assert_snapshot!(parse_stmt("using T = int;"), @r#"
        Alias using "T"
          int
        "#);
    }

    #[test]
    fn static_assert_with_message() {
        insta::assert_snapshot!(
            parse_stmt(r#"static_assert(x > 0, "must be positive");"#),
            @r#"
        StaticAssertStmt
          Binary ">"
            x
            0
          "must be positive"
        "#
        );
    }

    #[test]
    fn forward_branch_stmt() {
        insta::assert_snapshot!(
            parse_stmt("forward_branch (cond) -> (a, b);"),
            @r#"
        ForwardBranchStmt
          cond
          a
          b
        "#
        );
    }

    #[test]
    fn labelled_loop_and_break_target() {
        insta::assert_snapshot!(
            parse_stmt("outer: loop { break outer; }"),
            @r#"
        LoopStmt "outer"
          Block
            BreakStmt "outer"
        "#
        );
    }

    #[test]
    fn multi_dimensional_array_type() {
        insta::assert_snapshot!(parse_type("float32[1, 2]"), @"
        Bracketed
          float32
          1
          2
        ");
    }

    #[test]
    fn slicing_expression() {
        insta::assert_snapshot!(parse_expr("arr[1:3]"), @r#"
        Bracketed
          arr
          Slice
            1
            3
        "#);
    }

    #[test]
    fn processor_alias_decl() {
        insta::assert_snapshot!(
            parse_stmt("processor Foo = Bar(4);"),
            @r#"
        ModuleAlias processor "Foo"
          Call
            Bar
            4
        "#
        );
    }

    #[test]
    fn hoisted_endpoint_named() {
        insta::assert_snapshot!(parse_stmt("output child.out;"), @r#"EndpointDecl output child.out"#);
    }

    #[test]
    fn hoisted_endpoint_wildcard() {
        insta::assert_snapshot!(parse_stmt("output child.*;"), @r#"EndpointDecl output child.*"#);
    }

    #[test]
    fn hoisted_endpoint_prefixed_wildcard() {
        insta::assert_snapshot!(parse_stmt("output g2.test*;"), @r#"EndpointDecl output g2.test*"#);
    }

    #[test]
    fn sized_array_endpoint() {
        insta::assert_snapshot!(parse_stmt("input stream float in[10];"), @r#"
        EndpointDecl input stream "in"
          float
          10
        "#);
    }

    #[test]
    fn multi_type_event_endpoint() {
        insta::assert_snapshot!(parse_stmt("input event (int, float) e;"), @r#"
        EndpointDecl input event "e"
          int
          float
        "#);
    }

    #[test]
    fn braced_endpoint_group_desugars_to_flat_members() {
        insta::assert_snapshot!(
            dump("output stream { float32 a; int b; }", |parser| {
                let items = parser.parse_container_items();
                parser.ast.push(Stmt::Block {
                    brace: parser.tokens.current(),
                    label: None,
                    stmts: items,
                })
            }),
            @r#"
        Block
          EndpointDecl output stream "a"
            float32
          EndpointDecl output stream "b"
            int
        "#
        );
    }
}
