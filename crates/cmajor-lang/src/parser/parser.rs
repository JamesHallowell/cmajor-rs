use {
    crate::{
        Diagnostic,
        ast::{
            self, Alias, Assign, Ast, Attribute, AttributeList, Binary, Block, Bracketed,
            BreakStmt, Call, ChildPool, Connection, ConnectionDecl, ConnectionIf, ContinueStmt,
            Decl, DeclStmt, Declarator, EndpointDecl, EnumDecl, Expr, ExprStmt, Field, ForStmt,
            ForwardBranchStmt, FunctionDecl, Graph, GraphDecl, HoistTarget, HoistedPath, Ident,
            IfStmt, Import, InterpolationKind, Item, LoopStmt, ModuleAlias, NamespaceDecl, Node,
            NodeDecl, NodeId, Parentheses, PostfixUnary, ProcessorDecl, ProcessorProperty,
            ReturnStmt, ScopeAccess, Slice, Stmt, StructDecl, Ternary, TypeModifier, Unary, Var,
            VarRole, VectorSizeSuffix, WhileStmt,
        },
        lexer::{
            Literal, NonTrivialTokenStreamIterator, Token, TokenId, TokenKind, TokenStream,
            tokenize,
        },
        parser::precedence::{BindingPower, InfixBindingPower, PrecedenceLevel},
        skip_to_matching, token,
        utils::{
            self,
            arena::{Arena, SecondaryArena},
        },
    },
    std::range::Range,
};

pub struct Parse {
    pub ast: Ast,
    pub tokens: TokenStream,
    pub diagnostics: Vec<Diagnostic>,
}

pub fn parse(source: &str) -> Parse {
    let tokens = tokenize(source);
    let mut parser = Parser::new(&tokens, source);
    parser.parse();
    Parse {
        ast: Ast::new(parser.nodes, parser.roots, parser.spans, parser.child_pool),
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

#[derive(Clone)]
struct Parser<'a> {
    tokens: &'a TokenStream,
    iter: NonTrivialTokenStreamIterator<'a>,
    source: &'a str,
    nodes: Arena<NodeId, Node>,
    roots: Vec<NodeId>,
    spans: SecondaryArena<NodeId, Range<TokenId>>,
    child_pool: ChildPool,
    diagnostics: Vec<Diagnostic>,
}

macro_rules! expect_matches {
    ($parser:expr, $expected:pat $(if $guard:expr)? $(,)?) => {
        if matches!($parser.peek(), $expected $(if $guard)?) {
            $parser.advance()
        } else {
            let (id, token) = $parser.peek_verbose();
            $parser.error(
                id,
                format!("expected {}, found {:?}", stringify!($expected), token.kind),
            );
            id
        }
    };
}

macro_rules! expect_identifier {
    ($parser:expr, $expected:pat $(if $guard:expr)? $(,)?) => {
        {
            let (id, token) = $parser.peek_verbose();
            match token.kind {
                TokenKind::Identifier => {
                    let text = $parser.tokens.text($parser.source, id);

                    if matches!(text, $expected $(if $guard)?) {
                        $parser.advance()
                    } else {
                        $parser.error(
                            id,
                            format!("expected {}, found {text:?}", stringify!($expected)),
                        );
                        id
                    }
                }
                actual_kind => {
                    $parser.error(
                        id,
                        format!("expected {}, found {actual_kind:?}", stringify!($expected)),
                    );
                    id
                }
            }
        }
    };
}

macro_rules! until {
    ($parser:ident, $token:pat, $body:block) => {
        while !matches!($parser.peek(), $token | TokenKind::EndOfFile) $body
    };
}

macro_rules! while_consuming {
    ($parser:ident, $token:expr, $body:block) => {
        while $parser.at($token) {
            $parser.advance();
            $body
        }
    };
}

macro_rules! list {
    ($parser:ident, (, $body:expr, )) => {
        list!(@impl $parser, token!('('), token!(,), token!(')'), $body)
    };
    ($parser:ident, (), $body:expr) => {
        list!(@impl2 $parser, token!('('), token!(')'), $body)
    };
    ($parser:ident, [, $body:expr, ]) => {
        list!(@impl $parser, token!('['), token!(,), token!(']'), $body)
    };
    ($parser:ident, {, $body:expr, }) => {
        list!(@impl $parser, token!('{'), token!(,), token!('}'), $body)
    };
    ($parser:ident, <, $body:expr, >) => {
        list!(@impl $parser, token!(<), token!(,), token!(>), $body)
    };
    ($parser:ident, [[, $body:expr, ]]) => {
        {
            $parser.expect(token!('['));
            $parser.expect(token!('['));

            let mut items = vec![];

            while !matches!(
                $parser.peek_2(),
                (token!(']'), token!(']')) | (TokenKind::EndOfFile, _)
            ) {
                items.push($body);
                if $parser.advance_if(token!(,)).is_none() {
                    break;
                }
            }

            $parser.expect(token!(']'));
            $parser.expect(token!(']'));
            items
        }
    };
    (@impl $parser:ident, $start:expr, $separator:expr, $stop:pat, $body:expr) => {
        {
            $parser.expect($start);

            let mut items = vec![];
            until!($parser, $stop, {
                items.push($body);
                if $parser.advance_if($separator).is_none() {
                    break;
                }
            });

            expect_matches!($parser, $stop);
            items
        }
    };
    (@impl2 $parser:ident, $start:expr, $stop:pat, $body:expr) => {
        {
            $parser.expect($start);
            until!($parser, $stop, {
                $body;
                if $parser.advance_if(token!(,)).is_none() {
                    break;
                }
            });
            expect_matches!($parser, $stop);
        }
    };
}

impl<'a> Parser<'a> {
    fn new(tokens: &'a TokenStream, source: &'a str) -> Parser<'a> {
        Parser {
            tokens,
            iter: tokens.into_iter().ignore_trivia(),
            source,
            nodes: Arena::default(),
            roots: vec![],
            spans: SecondaryArena::default(),
            child_pool: ChildPool::default(),
            diagnostics: Vec::new(),
        }
    }

    fn peek(&self) -> TokenKind {
        self.peek_nth(0)
    }

    fn peek_verbose(&self) -> (TokenId, Token) {
        self.iter.clone().next().unwrap_or_else(|| {
            let eof = self.tokens.end_of_file();
            (eof, self.tokens.get(eof))
        })
    }

    fn peek_nth(&self, n: usize) -> TokenKind {
        self.iter
            .clone()
            .nth(n)
            .map(|(_, token)| token.kind)
            .unwrap_or(TokenKind::EndOfFile)
    }

    fn peek_2(&self) -> (TokenKind, TokenKind) {
        (self.peek(), self.peek_nth(1))
    }

    fn peek_3(&self) -> (TokenKind, TokenKind, TokenKind) {
        (self.peek(), self.peek_nth(1), self.peek_nth(2))
    }

    fn text(&self, token: TokenId) -> &str {
        self.tokens.text(self.source, token)
    }

    fn at(&self, kind: impl Into<TokenKind>) -> bool {
        self.peek() == kind.into()
    }

    fn not_at(&self, kind: impl Into<TokenKind>) -> bool {
        self.peek() != kind.into()
    }

    fn at_attribute_list(&self) -> bool {
        self.peek_2() == (token!('['), token!('['))
    }

    fn advance(&mut self) -> TokenId {
        self.iter
            .next()
            .map(|(id, _)| id)
            .unwrap_or_else(|| self.tokens.end_of_file())
    }

    fn advance_if(&mut self, token: impl Into<TokenKind>) -> Option<TokenId> {
        let token = token.into();
        if self.at(token) {
            Some(self.advance())
        } else {
            None
        }
    }

    fn error(&mut self, token: TokenId, message: impl Into<String>) {
        let position = self.tokens.span(token).start;

        let (line, column) = utils::line_col(self.source, position);
        self.diagnostics.push(Diagnostic {
            token,
            line,
            column,
            message: message.into(),
        });
    }

    fn expect(&mut self, kind: impl Into<TokenKind>) -> TokenId {
        let kind = kind.into();
        self.advance_if(kind).unwrap_or_else(|| {
            let (id, token) = self.peek_verbose();
            self.error(id, format!("expected {kind:?}, found {:?}", token.kind));
            id
        })
    }

    fn add_node(&mut self, start: TokenId, node: impl Into<Node>) -> NodeId {
        let node = self.nodes.push(node.into());

        let prev = self
            .iter
            .prev_peek()
            .expect(" parser should always have a previous token when adding a node");

        self.spans.insert(node, (start..prev).into());

        node
    }

    pub fn parse(&mut self) {
        while !self.peek().is_end_of_file() {
            let root = self.parse_statement();
            self.roots.push(root);
        }
    }

    fn parse_expr(&mut self) -> NodeId {
        self.parse_expr_with_min_binding_power(PrecedenceLevel::lowest())
    }

    fn parse_expr_with_min_binding_power(&mut self, min_binding_power: BindingPower) -> NodeId {
        let mut lhs = match self.peek() {
            token!(! | ~ | - | ++ | --) => {
                let op = self.advance();
                let operand = self.parse_expr_with_min_binding_power(PrecedenceLevel::Unary.base());
                self.add_node(op, Expr::Unary(Unary { op, operand }))
            }
            TokenKind::Literal(literal) => self.parse_literal(literal),
            token!(true | false) => {
                let token = self.advance();
                self.add_node(token, Expr::Literal(ast::Literal::Bool { token }))
            }
            token!(processor) if self.peek_nth(1) == token!(.) => {
                let keyword = self.expect(token!(processor));
                self.expect(token!(.));
                let name = self.expect(TokenKind::Identifier);
                self.add_node(keyword, Expr::ProcessorProperty(ProcessorProperty { name }))
            }
            TokenKind::Identifier | token!(processor) => {
                let token = self.advance();
                self.add_node(token, Expr::Ident(Ident { token }))
            }
            token!('(') => {
                let (paren, _) = self.peek_verbose();
                let checkpoint = self.child_pool.checkpoint();
                list!(self, (), {
                    let expr = self.parse_expr();
                    self.child_pool.stage(expr);
                });
                let children = self.child_pool.commit(checkpoint);
                self.add_node(paren, Expr::Parentheses(Parentheses { inner: children }))
            }
            token if token.is_type_like() => {
                let token = self.advance();
                self.add_node(token, Expr::Ident(Ident { token }))
            }
            peeked => {
                let message = format!("expected expression, found {peeked:?}");
                let token = self.advance();
                self.error(token, message);
                self.add_node(token, Node::Error { token })
            }
        };

        loop {
            lhs = match self.peek() {
                token!('(') => self.parse_call(lhs),
                token!('[') => self.parse_bracketed_suffix(lhs),
                token!(.) => self.parse_field(lhs),
                token!(::) => self.parse_scope_access(lhs),
                token!(<) => match self.try_parse_vector_size_suffix(lhs) {
                    Some(suffixed) => suffixed,
                    None => break,
                },
                token!(++ | --) => {
                    let op = self.advance();
                    let start = self.spans[lhs].start;
                    self.add_node(start, Expr::PostfixUnary(PostfixUnary { op, operand: lhs }))
                }
                _ => break,
            };
        }

        loop {
            let token = self.peek();
            if token.is_end_of_file() {
                break;
            }

            let Some(infix) = Infix::from_token(token) else {
                break;
            };
            let binding_power = infix.binding_power();
            if binding_power.binds_at < min_binding_power {
                break;
            }

            lhs = match infix {
                Infix::Ternary => {
                    let question = self.expect(token!(?));
                    let then_branch = self.parse_expr();
                    self.expect(token!(:));
                    let else_branch =
                        self.parse_expr_with_min_binding_power(binding_power.min_for_rhs);

                    let start = self.spans[lhs].start;
                    self.add_node(
                        start,
                        Expr::Ternary(Ternary {
                            cond: lhs,
                            question,
                            then_branch,
                            else_branch,
                        }),
                    )
                }
                Infix::Binary(bin_op) => {
                    let op = self.advance();
                    let rhs = self.parse_expr_with_min_binding_power(binding_power.min_for_rhs);
                    let start = self.spans[lhs].start;
                    self.add_node(
                        start,
                        if bin_op.is_assignment() {
                            Expr::Assign(Assign {
                                op,
                                target: lhs,
                                value: rhs,
                            })
                        } else {
                            Expr::Binary(Binary { op, lhs, rhs })
                        },
                    )
                }
            };
        }

        lhs
    }

    fn parse_scope_access(&mut self, base: NodeId) -> NodeId {
        self.expect(token!(::));
        let name = self.expect(TokenKind::Identifier);
        let start = self.spans[base].start;
        self.add_node(start, Expr::ScopeAccess(ScopeAccess { name, base }))
    }

    fn parse_call(&mut self, callee: NodeId) -> NodeId {
        let (paren, _) = self.peek_verbose();
        let args = list!(self, (, self.parse_expr(), ));
        let start = self.spans[callee].start;
        self.add_node(
            start,
            Expr::Call(Call {
                paren,
                callee,
                args,
            }),
        )
    }

    fn parse_bracketed_suffix(&mut self, base: NodeId) -> NodeId {
        let (bracket, _) = self.peek_verbose();

        let checkpoint = self.child_pool.checkpoint();
        list!(self, [, {
            let term = self.parse_bracket_term();
            self.child_pool.stage(term);
        }, ]);
        let terms = self.child_pool.commit(checkpoint);

        let start = self.spans[base].start;
        self.add_node(
            start,
            Expr::Bracketed(Bracketed {
                bracket,
                base,
                terms,
            }),
        )
    }

    fn parse_bracket_term(&mut self) -> NodeId {
        if let Some(colon) = self.advance_if(token!(:)) {
            let end = self.not_at(token!(']')).then(|| self.parse_expr());
            return self.add_node(colon, Expr::Slice(Slice { start: None, end }));
        }

        let start = self.parse_expr();
        if self.advance_if(token!(:)).is_none() {
            return start;
        }

        let end = self.not_at(token!(']')).then(|| self.parse_expr());
        let start_tok = self.spans[start].start;
        self.add_node(
            start_tok,
            Expr::Slice(Slice {
                start: Some(start),
                end,
            }),
        )
    }

    fn parse_field(&mut self, base: NodeId) -> NodeId {
        self.expect(token!(.));
        let name = self.expect(TokenKind::Identifier);

        let start = self.spans[base].start;
        self.add_node(start, Expr::Field(Field { name, base }))
    }

    fn parse_literal(&mut self, literal: Literal) -> NodeId {
        let token = self.advance();
        let node = match literal {
            Literal::Int32 => ast::Literal::Int32 { token },
            Literal::Int64 => ast::Literal::Int64 { token },
            Literal::Float32 => ast::Literal::Float32 { token },
            Literal::Float64 => ast::Literal::Float64 { token },
            Literal::Imaginary32 => ast::Literal::Imaginary32 { token },
            Literal::Imaginary64 => ast::Literal::Imaginary64 { token },
            Literal::String => ast::Literal::String { token },
        };
        self.add_node(token, Expr::Literal(node))
    }

    fn try_parse_label(&mut self) -> Option<TokenId> {
        let is_label = matches!(
            self.peek_3(),
            (
                TokenKind::Identifier,
                token!(:),
                token!('{' | loop | for | while)
            )
        );

        if is_label {
            let label = self.expect(TokenKind::Identifier);
            self.expect(token!(:));
            Some(label)
        } else {
            None
        }
    }

    fn parse_statement(&mut self) -> NodeId {
        let label = self.try_parse_label();

        match self.peek() {
            token!('{') => self.parse_block(label),
            token!(let) => self.parse_let(),
            token!(var) => self.parse_var(),
            token!(using) => self.parse_using_stmt(),
            token!(if) => self.parse_if(),
            token!(while) => self.parse_while(label),
            token!(loop) => self.parse_loop(label),
            token!(for) => self.parse_for(label),
            token!(forward_branch) => self.parse_forward_branch(),
            token!(node) => {
                let mut nodes = self.parse_node_group();
                nodes.pop().unwrap_or_else(|| {
                    let (token, _) = self.peek_verbose();
                    self.add_node(token, Node::Error { token })
                })
            }
            token!(connection) => self.parse_connection_decl(),
            token!(return) => self.parse_return(),
            token!(break) => self.parse_break(),
            token!(continue) => self.parse_continue(),
            token!(namespace) => self.parse_namespace(),
            token!(import) => self.parse_import(),
            token!(enum) => self.parse_enum(),
            token!(external) => self.parse_external_decl(),
            token!(processor)
                if self.iter.clone().nth(1).map(|(_, token)| token.kind) == Some(token!(.)) =>
            {
                self.parse_expr_stmt()
            }
            token!(processor | graph | struct) => self.parse_container(),
            token!(input | output) => {
                let mut endpoints = self.parse_endpoint_group();
                endpoints.pop().unwrap_or_else(|| {
                    let (token, _) = self.peek_verbose();
                    self.add_node(token, Node::Error { token })
                })
            }
            token!(event) => self.parse_event_handler(),
            token!(const) => self.parse_typed_decl(),
            TokenKind::Keyword(keyword) if TokenKind::Keyword(keyword).is_type_like() => {
                self.parse_typed_decl()
            }
            TokenKind::Identifier if self.looks_like_typed_decl() => self.parse_typed_decl(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_container_items(&mut self) -> Vec<NodeId> {
        let mut items = Vec::new();
        until!(self, token!('}'), {
            if matches!(self.peek(), token!(input | output)) {
                items.extend(self.parse_endpoint_group());
            } else if self.at(token!(node)) {
                items.extend(self.parse_node_group());
            } else {
                items.push(self.parse_statement());
            }
        });
        items
    }

    fn parse_break(&mut self) -> NodeId {
        let keyword = self.expect(token!(break));
        let target = self.advance_if(TokenKind::Identifier);
        self.expect(token!(;));
        self.add_node(keyword, Stmt::BreakStmt(BreakStmt { keyword, target }))
    }

    fn parse_continue(&mut self) -> NodeId {
        let keyword = self.expect(token!(continue));
        let target = self.advance_if(TokenKind::Identifier);
        self.expect(token!(;));
        self.add_node(
            keyword,
            Stmt::ContinueStmt(ContinueStmt { keyword, target }),
        )
    }

    fn parse_forward_branch(&mut self) -> NodeId {
        let keyword = self.expect(token!(forward_branch));
        self.expect(token!('('));
        let cond = self.parse_expr();
        self.expect(token!(')'));
        self.expect(token!(->));
        let targets = list!(self, (, self.expect(TokenKind::Identifier), ));
        self.expect(token!(;));
        self.add_node(
            keyword,
            Stmt::ForwardBranchStmt(ForwardBranchStmt {
                keyword,
                cond,
                targets,
            }),
        )
    }

    fn parse_import(&mut self) -> NodeId {
        let keyword = self.expect(token!(import));
        let path = if self.at(Literal::String) {
            vec![self.advance()]
        } else {
            let mut path = vec![self.expect(TokenKind::Identifier)];
            while_consuming!(self, token!(.), {
                path.push(self.expect(TokenKind::Identifier));
            });
            path
        };
        self.expect(token!(;));
        self.add_node(keyword, Item::Import(Import { keyword, path }))
    }

    fn parse_enum(&mut self) -> NodeId {
        let keyword = self.expect(token!(enum));
        let name = self.expect(TokenKind::Identifier);
        let values = list!(self, {, self.expect(TokenKind::Identifier), });

        self.add_node(
            keyword,
            Item::EnumDecl(EnumDecl {
                keyword,
                name,
                values,
            }),
        )
    }

    fn parse_external_decl(&mut self) -> NodeId {
        self.expect(token!(external));
        self.parse_typed_decl_inner(true, true)
    }

    fn parse_using_stmt(&mut self) -> NodeId {
        let keyword = self.expect(token!(using));
        let name = self.expect(TokenKind::Identifier);
        self.expect(token!(=));
        let target = self.parse_expr();
        self.expect(token!(;));

        let decl = self.add_node(
            keyword,
            Decl::Alias(Alias {
                keyword,
                kind: ast::AliasKind::Using,
                name,
                target: Some(target),
            }),
        );
        let start = self.spans[decl].start;
        self.add_node(start, Stmt::DeclStmt(DeclStmt { decl }))
    }

    fn parse_typed_decl(&mut self) -> NodeId {
        self.parse_typed_decl_inner(true, false)
    }

    fn parse_typed_decl_inner(&mut self, consume_semicolon: bool, is_external: bool) -> NodeId {
        let ty = self.parse_type();
        let name = self.expect(TokenKind::Identifier);

        if matches!(self.peek(), token!(< | '(')) {
            let generics = self.parse_optional_generics();
            return self.parse_function_decl(ty, name, generics);
        }

        let mut declarators = vec![Declarator {
            name,
            init: self
                .advance_if(token!(=))
                .is_some()
                .then(|| self.parse_expr()),
        }];
        while_consuming!(self, token!(,), {
            let name = self.expect(TokenKind::Identifier);
            let init = self
                .advance_if(token!(=))
                .is_some()
                .then(|| self.parse_expr());
            declarators.push(Declarator { name, init });
        });
        let attributes = self
            .at_attribute_list()
            .then(|| self.parse_attribute_list());
        if consume_semicolon {
            self.expect(token!(;));
        }
        let start = self.spans[ty].start;
        let decl = self.add_node(
            start,
            Decl::Var(Var {
                role: VarRole::Typed,
                ty: Some(ty),
                is_external,
                declarators,
                attributes,
            }),
        );
        let start = self.spans[decl].start;
        self.add_node(start, Stmt::DeclStmt(DeclStmt { decl }))
    }

    fn parse_optional_generics(&mut self) -> Vec<TokenId> {
        if self.at(token!(<)) {
            list!(self, <, self.expect(TokenKind::Identifier), >)
        } else {
            Vec::new()
        }
    }

    fn parse_param(&mut self) -> NodeId {
        let ty = self.parse_type();
        let name = self.expect(TokenKind::Identifier);

        let start = self.spans[ty].start;
        self.add_node(
            start,
            Decl::Var(Var {
                role: VarRole::Parameter,
                ty: Some(ty),
                is_external: false,
                declarators: vec![Declarator { name, init: None }],
                attributes: None,
            }),
        )
    }

    fn parse_params(&mut self) -> Vec<NodeId> {
        list!(self, (, self.parse_param(), ))
    }

    fn parse_function_decl(&mut self, ty: NodeId, name: TokenId, generics: Vec<TokenId>) -> NodeId {
        let params = self.parse_params();
        let is_const = self.advance_if(token!(const)).is_some();
        let attributes = self
            .at_attribute_list()
            .then(|| self.parse_attribute_list());
        let body = self.parse_block(None);

        let start = self.spans[ty].start;
        self.add_node(
            start,
            Item::FunctionDecl(FunctionDecl {
                ty: Some(ty),
                name,
                generics,
                params,
                is_const,
                is_event_handler: false,
                attributes,
                body,
            }),
        )
    }

    fn parse_event_handler(&mut self) -> NodeId {
        let keyword = self.expect(token!(event));
        let name = self.expect(TokenKind::Identifier);
        let params = self.parse_params();
        let body = self.parse_block(None);

        self.add_node(
            keyword,
            Item::FunctionDecl(FunctionDecl {
                ty: None,
                name,
                generics: Vec::new(),
                params,
                is_const: false,
                is_event_handler: true,
                attributes: None,
                body,
            }),
        )
    }

    fn parse_namespace(&mut self) -> NodeId {
        let keyword = self.expect(token!(namespace));
        let mut segments = vec![self.expect(TokenKind::Identifier)];
        while_consuming!(self, token!(::), {
            segments.push(self.expect(TokenKind::Identifier));
        });
        let params = self.parse_optional_specialisation_params();

        if self.advance_if(token!(=)).is_some() {
            let target = self.parse_expr();
            self.expect(token!(;));
            let name = *segments.last().expect("namespace has at least one segment");
            return self.add_node(
                keyword,
                Item::ModuleAlias(ModuleAlias {
                    keyword,
                    kind: ast::AliasKind::Namespace,
                    name,
                    target,
                }),
            );
        }

        let attributes = self
            .at_attribute_list()
            .then(|| self.parse_attribute_list());
        self.expect(token!('{'));
        let items = self.parse_container_items();
        self.expect(token!('}'));

        self.add_node(
            keyword,
            Item::NamespaceDecl(NamespaceDecl {
                keyword,
                segments,
                params,
                attributes,
                items,
            }),
        )
    }

    fn parse_optional_specialisation_params(&mut self) -> Vec<NodeId> {
        if self.at(token!('(')) {
            list!(self, (, self.parse_specialisation_param(), ))
        } else {
            vec![]
        }
    }

    fn parse_specialisation_param(&mut self) -> NodeId {
        match self.peek() {
            token!(using) => {
                let keyword = self.advance();
                let name = self.expect(TokenKind::Identifier);
                let target = self
                    .advance_if(token!(=))
                    .is_some()
                    .then(|| self.parse_type());
                self.add_node(
                    keyword,
                    Decl::Alias(Alias {
                        keyword,
                        kind: ast::AliasKind::Using,
                        name,
                        target,
                    }),
                )
            }
            token!(processor) => {
                let keyword = self.advance();
                let name = self.expect(TokenKind::Identifier);
                let target = self
                    .advance_if(token!(=))
                    .is_some()
                    .then(|| self.parse_expr());
                self.add_node(
                    keyword,
                    Decl::Alias(Alias {
                        keyword,
                        kind: ast::AliasKind::Processor,
                        name,
                        target,
                    }),
                )
            }
            token!(namespace) => {
                let keyword = self.advance();
                let name = self.expect(TokenKind::Identifier);
                let target = self
                    .advance_if(token!(=))
                    .is_some()
                    .then(|| self.parse_expr());
                self.add_node(
                    keyword,
                    Decl::Alias(Alias {
                        keyword,
                        kind: ast::AliasKind::Namespace,
                        name,
                        target,
                    }),
                )
            }
            _ => {
                let ty = self.parse_type();
                let name = self.expect(TokenKind::Identifier);
                let init = self
                    .advance_if(token!(=))
                    .is_some()
                    .then(|| self.parse_expr());
                let start = self.spans[ty].start;
                self.add_node(
                    start,
                    Decl::Var(Var {
                        role: VarRole::SpecialisationValue,
                        ty: Some(ty),
                        is_external: false,
                        declarators: vec![Declarator { name, init }],
                        attributes: None,
                    }),
                )
            }
        }
    }

    fn parse_container(&mut self) -> NodeId {
        let keyword = expect_matches!(self, token!(processor | graph | struct));
        let keyword_kind = self.tokens.get(keyword).kind;
        let name = self.expect(TokenKind::Identifier);
        let params = self.parse_optional_specialisation_params();

        if matches!(keyword_kind, token!(processor | graph)) && self.at(token!(=)) {
            self.advance();
            let target = self.parse_expr();
            self.expect(token!(;));
            return self.add_node(
                keyword,
                Item::ModuleAlias(ModuleAlias {
                    keyword,
                    kind: ast::AliasKind::Processor,
                    name,
                    target,
                }),
            );
        }

        let attributes = self
            .at_attribute_list()
            .then(|| self.parse_attribute_list());

        self.expect(token!('{'));
        let items = self.parse_container_items();
        self.expect(token!('}'));

        match keyword_kind {
            token!(graph) => self.add_node(
                keyword,
                Item::GraphDecl(GraphDecl {
                    keyword,
                    name,
                    params,
                    attributes,
                    items,
                }),
            ),
            token!(struct) => self.add_node(
                keyword,
                Item::StructDecl(StructDecl {
                    keyword,
                    name,
                    attributes,
                    items,
                }),
            ),
            token!(processor) => self.add_node(
                keyword,
                Item::ProcessorDecl(ProcessorDecl {
                    keyword,
                    name,
                    params,
                    attributes,
                    items,
                }),
            ),
            _ => unreachable!(),
        }
    }

    fn parse_node_group(&mut self) -> Vec<NodeId> {
        let keyword = self.expect(token!(node));
        if self.advance_if(token!('{')).is_some() {
            let mut nodes = Vec::new();

            until!(self, token!('}'), {
                nodes.push(self.parse_node_decl_entry(keyword));
                self.expect(token!(;));
            });

            self.expect(token!('}'));
            nodes
        } else {
            let mut nodes = vec![self.parse_node_decl_entry(keyword)];
            while_consuming!(self, token!(,), {
                nodes.push(self.parse_node_decl_entry(keyword));
            });
            self.expect(token!(;));
            nodes
        }
    }

    fn parse_node_decl_entry(&mut self, keyword: TokenId) -> NodeId {
        let name = self.expect(TokenKind::Identifier);
        let array_size = self.advance_if(token!('[')).and_then(|_| {
            let size = self.at(token!(']')).then(|| self.parse_expr());
            self.expect(token!(']'));
            size
        });
        self.expect(token!(=));
        let processor = self.parse_expr();

        self.add_node(
            keyword,
            Graph::NodeDecl(NodeDecl {
                keyword,
                name,
                processor,
                array_size,
            }),
        )
    }

    fn parse_connection_decl(&mut self) -> NodeId {
        let keyword = self.expect(token!(connection));
        let connections = self.parse_connection_list();
        self.add_node(
            keyword,
            Graph::ConnectionDecl(ConnectionDecl {
                keyword,
                connections,
            }),
        )
    }

    fn parse_connection_list(&mut self) -> Vec<NodeId> {
        let braced = self.advance_if(token!('{')).is_some();

        let mut connections = Vec::new();
        loop {
            if braced && self.advance_if(token!('}')).is_some() {
                break;
            }

            if self.at(token!(if)) {
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
        let keyword = self.expect(token!(if));
        self.expect(token!('('));
        let cond = self.parse_expr();
        self.expect(token!(')'));
        let then_branch = self.parse_connection_list();
        let else_branch = self
            .advance_if(token!(else))
            .is_some()
            .then(|| self.parse_connection_list());

        self.add_node(
            keyword,
            Graph::ConnectionIf(ConnectionIf {
                keyword,
                cond,
                then_branch,
                else_branch,
            }),
        )
    }

    fn parse_connection_chain(&mut self) -> Vec<NodeId> {
        let interpolation = self.parse_interpolation_if_present();
        let mut connections = Vec::new();
        let mut sources = self.parse_connection_endpoints();
        loop {
            let arrow = self.expect(token!(->));
            let delay = self.at(token!('[')).then(|| {
                self.advance();
                let delay = self.parse_expr();
                self.expect(token!(']'));
                self.expect(token!(->));
                delay
            });
            let destinations = self.parse_connection_endpoints();

            if sources.len() > 1 && destinations.len() > 1 {
                self.error(arrow, "many-to-many connections are not supported");
            }

            let chain_continues = self.at(token!(->));
            if chain_continues {
                if destinations.len() > 1 {
                    self.error(
                        arrow,
                        "cannot chain a connection with multiple destinations",
                    );
                } else if let Some(&dest) = destinations.first()
                    && matches!(self.nodes[dest], Node::Expr(Expr::Field { .. }))
                {
                    self.error(
                        arrow,
                        "cannot name an endpoint in the middle of a connection chain",
                    );
                }
            }

            let start = self.spans[*sources
                .first()
                .expect("a connection chain always has at least one source")]
            .start;
            connections.push(self.add_node(
                start,
                Graph::Connection(Connection {
                    interpolation,
                    sources,
                    arrow,
                    delay,
                    destinations: destinations.clone(),
                }),
            ));

            if !chain_continues {
                break;
            }
            sources = destinations;
        }
        connections
    }

    fn parse_connection_endpoints(&mut self) -> Vec<NodeId> {
        let mut endpoints = vec![self.parse_expr()];
        while_consuming!(self, token!(,), {
            endpoints.push(self.parse_expr());
        });
        endpoints
    }

    fn parse_interpolation_if_present(&mut self) -> Option<InterpolationKind> {
        match self.peek_3() {
            (token!('['), TokenKind::Identifier, token!(']')) => {
                let kind = match self.iter.clone().nth(1).map(|(id, _)| self.text(id)) {
                    Some("none") => Some(InterpolationKind::None),
                    Some("latch") => Some(InterpolationKind::Latch),
                    Some("linear") => Some(InterpolationKind::Linear),
                    Some("sinc") => Some(InterpolationKind::Sinc),
                    Some("fast") => Some(InterpolationKind::Fast),
                    Some("best") => Some(InterpolationKind::Best),
                    _ => None,
                };

                if let Some(kind) = kind {
                    self.expect(token!('['));
                    self.expect(TokenKind::Identifier);
                    self.expect(token!(']'));
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

        let init = self.not_at(token!(;)).then(|| self.parse_for_init());

        let is_bounded_range_for = self.at(token!(')'))
            && matches!(
                init.map(|id| &self.nodes[id]),
                Some(Node::Stmt(Stmt::DeclStmt(DeclStmt { .. })))
            );

        if is_bounded_range_for {
            self.advance();
            let body = self.parse_statement();
            return self.add_node(
                label.unwrap_or(keyword),
                Stmt::LoopStmt(LoopStmt {
                    keyword,
                    label,
                    count: init,
                    body,
                }),
            );
        }

        self.expect(token!(;));
        let cond = self.not_at(token!(;)).then(|| self.parse_expr());
        self.expect(token!(;));
        let update = self.not_at(token!(')')).then(|| self.parse_expr());
        self.expect(token!(')'));
        let body = self.parse_statement();

        self.add_node(
            label.unwrap_or(keyword),
            Stmt::ForStmt(ForStmt {
                keyword,
                label,
                init,
                cond,
                update,
                body,
            }),
        )
    }

    fn parse_for_init(&mut self) -> NodeId {
        match self.peek() {
            token!(const) => self.parse_typed_decl_inner(false, false),
            token!(let) => self.parse_let_or_var_inner(token!(let), VarRole::Let, false),
            token!(var) => self.parse_let_or_var_inner(token!(var), VarRole::Var, false),
            TokenKind::Keyword(keyword) if keyword.is_type() => {
                self.parse_typed_decl_inner(false, false)
            }
            TokenKind::Identifier if self.looks_like_typed_decl() => {
                self.parse_typed_decl_inner(false, false)
            }
            _ => {
                let expr = self.parse_expr();
                let start = self.spans[expr].start;
                self.add_node(start, Stmt::ExprStmt(ExprStmt { expr }))
            }
        }
    }

    fn looks_like_typed_decl(&self) -> bool {
        let mut tokens = self.iter.clone().peekable();

        let kind = |(_, token): &(TokenId, Token)| token.kind;

        if tokens.peek().map(kind) != Some(TokenKind::Identifier) {
            return false;
        }
        tokens.next();

        loop {
            if tokens.peek().map(kind) == Some(token!('(')) {
                tokens = if let Ok(skipped) = skip_to_matching!(tokens, ()) {
                    skipped
                } else {
                    return false;
                }
            }

            if matches!(tokens.peek().map(kind), Some(token!(:: | .))) {
                tokens.next();
                if !matches!(tokens.next(), Some((_, token)) if token.kind == TokenKind::Identifier)
                {
                    return false;
                }
            } else {
                break;
            }
        }

        loop {
            let skipped = match tokens.peek().map(kind) {
                Some(token!(<)) => skip_to_matching!(tokens, <>),
                Some(token!('[')) => skip_to_matching!(tokens, []),
                _ => break,
            };

            match skipped {
                Ok(skipped) => {
                    tokens = skipped;
                }
                Err(_) => return false,
            }
        }

        matches!(tokens.next(), Some((_, token)) if token.kind == TokenKind::Identifier)
    }

    fn parse_attribute(&mut self) -> Attribute {
        let key = match self.peek() {
            TokenKind::Identifier | TokenKind::Keyword(_) => self.advance(),
            _ => self.expect(TokenKind::Identifier),
        };
        let value = self
            .advance_if(token!(:))
            .is_some()
            .then(|| self.parse_expr());

        Attribute { key, value }
    }

    fn parse_attribute_list(&mut self) -> NodeId {
        let (start, _) = self.peek_verbose();
        let attributes = list!(self, [[, self.parse_attribute(), ]]);
        self.add_node(start, AttributeList { attributes })
    }

    fn looks_like_hoisted_endpoint(&self) -> bool {
        self.peek_2() == (TokenKind::Identifier, token!(.))
    }

    fn parse_hoisted_endpoint(&mut self, direction: TokenId) -> NodeId {
        let mut segments = vec![self.expect(TokenKind::Identifier)];
        let index = self.at(token!('[')).then(|| {
            self.advance();
            let index = self.parse_expr();
            self.expect(token!(']'));
            index
        });
        self.expect(token!(.));

        while self.looks_like_hoisted_endpoint() {
            segments.push(self.expect(TokenKind::Identifier));
            self.expect(token!(.));
        }

        let target = if self.advance_if(token!(*)).is_some() {
            HoistTarget::Wildcard { prefix: None }
        } else {
            let name = self.expect(TokenKind::Identifier);
            if self.advance_if(token!(*)).is_some() {
                HoistTarget::Wildcard { prefix: Some(name) }
            } else {
                HoistTarget::Name(name)
            }
        };

        let rename = self.advance_if(TokenKind::Identifier);

        let attributes = self
            .at_attribute_list()
            .then(|| self.parse_attribute_list());
        self.expect(token!(;));

        let name = match target {
            HoistTarget::Name(name) => Some(rename.unwrap_or(name)),
            HoistTarget::Wildcard { .. } => None,
        };

        self.add_node(
            direction,
            Graph::EndpointDecl(EndpointDecl {
                direction,
                kind: None,
                types: Vec::new(),
                name,
                size: None,
                hoisted: Some(HoistedPath {
                    segments,
                    index,
                    target,
                }),
                attributes,
            }),
        )
    }

    fn parse_endpoint_member(&mut self, direction: TokenId, kind: TokenId) -> Vec<NodeId> {
        let types = if self.at(token!('(')) {
            list!(self, (, self.parse_type(), ))
        } else {
            vec![self.parse_type()]
        };

        let mut names = vec![self.parse_endpoint_name()];
        while_consuming!(self, token!(,), {
            names.push(self.parse_endpoint_name());
        });

        let attributes = self
            .at_attribute_list()
            .then(|| self.parse_attribute_list());
        self.expect(token!(;));

        names
            .into_iter()
            .map(|(name, size)| {
                self.add_node(
                    direction,
                    Graph::EndpointDecl(EndpointDecl {
                        direction,
                        kind: Some(kind),
                        types: types.clone(),
                        name: Some(name),
                        size,
                        hoisted: None,
                        attributes,
                    }),
                )
            })
            .collect()
    }

    fn parse_endpoint_name(&mut self) -> (TokenId, Option<NodeId>) {
        let name = self.expect(TokenKind::Identifier);
        let size = (self.at(token!('[')) && !self.at_attribute_list())
            .then(|| self.advance())
            .and_then(|_| {
                let size = self.not_at(token!(']')).then(|| self.parse_expr());
                self.expect(token!(']'));
                size
            });
        (name, size)
    }

    fn parse_endpoint_group(&mut self) -> Vec<NodeId> {
        let direction = expect_matches!(self, token!(input | output));
        if self.at(token!('{')) {
            self.advance();
            let mut endpoints = Vec::new();
            until!(self, token!('}'), {
                endpoints.extend(self.parse_endpoint_entry(direction));
            });
            self.expect(token!('}'));
            return endpoints;
        }
        self.parse_endpoint_entry(direction)
    }

    fn parse_endpoint_entry(&mut self, direction: TokenId) -> Vec<NodeId> {
        if self.looks_like_hoisted_endpoint() {
            return vec![self.parse_hoisted_endpoint(direction)];
        }

        let kind = if self.at(token!(event)) {
            self.advance()
        } else {
            expect_identifier!(self, "value" | "stream")
        };

        if self.at(token!('{')) {
            self.advance();
            let mut endpoints = Vec::new();
            until! {self, token!('}'), {
                endpoints.extend(self.parse_endpoint_member(direction, kind));
            }}
            self.expect(token!('}'));
            endpoints
        } else {
            self.parse_endpoint_member(direction, kind)
        }
    }

    fn parse_block(&mut self, label: Option<TokenId>) -> NodeId {
        let brace = self.expect(token!('{'));
        let mut stmts = Vec::new();
        until!(self, token!('}'), {
            stmts.push(self.parse_statement());
        });
        self.expect(token!('}'));

        self.add_node(
            label.unwrap_or(brace),
            Stmt::Block(Block {
                brace,
                label,
                stmts,
            }),
        )
    }

    fn parse_let(&mut self) -> NodeId {
        self.parse_let_or_var_inner(token!(let), VarRole::Let, true)
    }

    fn parse_var(&mut self) -> NodeId {
        self.parse_let_or_var_inner(token!(var), VarRole::Var, true)
    }

    fn parse_let_or_var_inner(
        &mut self,
        keyword: impl Into<TokenKind>,
        role: VarRole,
        consume_semicolon: bool,
    ) -> NodeId {
        let keyword = self.expect(keyword);

        let mut declarators = vec![self.parse_let_declarator()];
        while_consuming!(self, token!(,), {
            declarators.push(self.parse_let_declarator());
        });

        if consume_semicolon {
            self.expect(token!(;));
        }

        let decl = self.add_node(
            keyword,
            Decl::Var(Var {
                role,
                ty: None,
                is_external: false,
                declarators,
                attributes: None,
            }),
        );
        let start = self.spans[decl].start;
        self.add_node(start, Stmt::DeclStmt(DeclStmt { decl }))
    }

    fn parse_let_declarator(&mut self) -> Declarator {
        let name = self.expect(TokenKind::Identifier);
        self.expect(token!(=));
        let init = self.parse_expr();
        Declarator {
            name,
            init: Some(init),
        }
    }

    fn parse_if(&mut self) -> NodeId {
        let keyword = self.expect(token!(if));
        let is_const = self.advance_if(token!(const));
        self.expect(token!('('));
        let cond = self.parse_expr();
        self.expect(token!(')'));
        let then_branch = self.parse_statement();
        let else_branch = self
            .advance_if(token!(else))
            .is_some()
            .then(|| self.parse_statement());

        self.add_node(
            keyword,
            Stmt::IfStmt(IfStmt {
                keyword,
                is_const: is_const.is_some(),
                cond,
                then_branch,
                else_branch,
            }),
        )
    }

    fn parse_while(&mut self, label: Option<TokenId>) -> NodeId {
        let keyword = self.expect(token!(while));
        self.expect(token!('('));
        let cond = self.parse_expr();
        self.expect(token!(')'));
        let body = self.parse_statement();

        self.add_node(
            label.unwrap_or(keyword),
            Stmt::WhileStmt(WhileStmt {
                keyword,
                label,
                cond,
                body,
            }),
        )
    }

    fn parse_loop(&mut self, label: Option<TokenId>) -> NodeId {
        let keyword = self.expect(token!(loop));
        let count = self.advance_if(token!('(')).is_some().then(|| {
            let count = self.parse_expr();
            self.expect(token!(')'));
            count
        });
        let body = self.parse_statement();

        self.add_node(
            label.unwrap_or(keyword),
            Stmt::LoopStmt(LoopStmt {
                keyword,
                label,
                count,
                body,
            }),
        )
    }

    fn parse_return(&mut self) -> NodeId {
        let keyword = self.expect(token!(return));
        let value = self.not_at(token!(;)).then(|| self.parse_expr());
        self.expect(token!(;));

        self.add_node(keyword, Stmt::ReturnStmt(ReturnStmt { keyword, value }))
    }

    fn parse_expr_stmt(&mut self) -> NodeId {
        let expr = self.parse_expr();
        self.expect(token!(;));
        let start = self.spans[expr].start;
        self.add_node(start, Stmt::ExprStmt(ExprStmt { expr }))
    }

    fn parse_type(&mut self) -> NodeId {
        let is_const = self.advance_if(token!(const));
        let mut ty = self.parse_type_base();
        let is_ref = self.advance_if(token!(&)).is_some();

        if is_const.is_some() || is_ref {
            let start = is_const.unwrap_or_else(|| self.spans[ty].start);
            ty = self.add_node(
                start,
                Expr::TypeModifier(TypeModifier {
                    source: ty,
                    is_const: is_const.is_some(),
                    is_ref,
                }),
            );
        }

        ty
    }

    fn parse_type_base(&mut self) -> NodeId {
        let mut ty = match self.peek() {
            kind if kind.is_type_like() => self.parse_qualified_name(),
            peeked => {
                let message = format!("expected type, found {peeked:?}");
                let token = self.advance();
                self.error(token, message);
                self.add_node(token, Node::Error { token })
            }
        };

        loop {
            ty = match self.peek() {
                token!('[') => self.parse_bracketed_suffix(ty),
                token!(<) => self.parse_vector_size_suffix(ty),
                token!(.) => self.parse_field(ty),
                token!('(') => self.parse_call(ty),
                _ => break,
            };
        }

        ty
    }

    fn parse_qualified_name(&mut self) -> NodeId {
        let token = self.advance();
        let mut name = self.add_node(token, Expr::Ident(Ident { token }));

        loop {
            if self.at(token!('(')) {
                name = self.parse_call(name);
            }
            if self.at(token!(::)) {
                name = self.parse_scope_access(name);
            } else {
                break;
            }
        }

        name
    }

    fn parse_vector_size_suffix(&mut self, element: NodeId) -> NodeId {
        let angle = self.expect(token!(<));
        let term = self.parse_expr_with_min_binding_power(PrecedenceLevel::Shift.base());
        self.expect(token!(>));

        let start = self.spans[element].start;
        self.add_node(
            start,
            Expr::VectorSizeSuffix(VectorSizeSuffix {
                angle,
                element,
                terms: vec![term],
            }),
        )
    }

    fn try_parse_vector_size_suffix(&mut self, element: NodeId) -> Option<NodeId> {
        let mut checkpoint = self.clone();

        let angle = checkpoint.advance();
        let mut terms =
            vec![checkpoint.parse_expr_with_min_binding_power(PrecedenceLevel::Shift.base())];
        while_consuming!(checkpoint, token!(,), {
            terms.push(checkpoint.parse_expr_with_min_binding_power(PrecedenceLevel::Shift.base()));
        });

        let succeeded =
            self.diagnostics.len() == checkpoint.diagnostics.len() && checkpoint.at(token!(>));

        if !succeeded {
            return None;
        }
        *self = checkpoint;

        self.advance();
        let start = self.spans[element].start;
        Some(self.add_node(
            start,
            Expr::VectorSizeSuffix(VectorSizeSuffix {
                angle,
                element,
                terms,
            }),
        ))
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

        let ast = Ast::new(parser.nodes, vec![root], parser.spans, parser.child_pool);
        ast::dump(&ast, &tokens, source, root)
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
        insta::assert_snapshot!(parse_type("float"), @"float 0..5");
    }

    #[test]
    fn named_type() {
        insta::assert_snapshot!(parse_type("MyStruct"), @"MyStruct 0..8");
    }

    #[test]
    fn qualified_type_name() {
        insta::assert_snapshot!(parse_type("std::midi::Message"), @r#"
        ScopeAccess "Message" 0..18
          ScopeAccess "midi" 0..9
            std 0..3
        "#);
    }

    #[test]
    fn array_type() {
        insta::assert_snapshot!(parse_type("int[3]"), @"
        Bracketed 0..6
          int 0..3
          3 4..5
        ");
    }

    #[test]
    fn slice_type_has_no_size() {
        insta::assert_snapshot!(parse_type("int[]"), @"
        Bracketed 0..5
          int 0..3
        ");
    }

    #[test]
    fn vector_type() {
        insta::assert_snapshot!(parse_type("int<4>"), @"
        VectorSizeSuffix 0..6
          int 0..3
          4 4..5
        ");
    }

    #[test]
    fn clamp() {
        insta::assert_snapshot!(parse_type("clamp<10>"), @"
        VectorSizeSuffix 0..9
          clamp 0..5
          10 6..8
        ");
    }

    #[test]
    fn wrap() {
        insta::assert_snapshot!(parse_type("wrap<4>"), @"
        VectorSizeSuffix 0..7
          wrap 0..4
          4 5..6
        ");
    }

    #[test]
    fn angle_bracket_close_is_not_a_comparison() {
        insta::assert_snapshot!(parse_type("wrap<1 + 2>"), @r#"
        VectorSizeSuffix 0..11
          wrap 0..4
          Binary "+" 5..10
            1 5..6
            2 9..10
        "#);
    }

    #[test]
    fn postfix_type_modifiers_apply_left_to_right() {
        insta::assert_snapshot!(parse_type("int<4>[2]"), @"
        Bracketed 0..9
          VectorSizeSuffix 0..6
            int 0..3
            4 4..5
          2 7..8
        ");
    }

    #[test]
    fn const_type() {
        insta::assert_snapshot!(parse_type("const int"), @"
        TypeModifier const 0..9
          int 6..9
        ");
    }

    #[test]
    fn const_array_type() {
        insta::assert_snapshot!(parse_type("const int[]"), @"
        TypeModifier const 0..11
          Bracketed 6..11
            int 6..9
        ");
    }

    #[test]
    fn type_cast() {
        insta::assert_snapshot!(parse_expr("float (2.5)"), @"
        Call 0..11
          float 0..5
          2.5 7..10
        ");
    }

    #[test]
    fn null_literal() {
        insta::assert_snapshot!(parse_expr("()"), @"Parentheses 0..2");
    }

    #[test]
    fn aggregate_literal() {
        insta::assert_snapshot!(parse_expr("((1, 2), (3, 4))"), @"
        Parentheses 0..16
          Parentheses 1..7
            1 2..3
            2 5..6
          Parentheses 9..15
            3 10..11
            4 13..14
        ");
    }

    #[test]
    fn precedence_of_arithmetic() {
        insta::assert_snapshot!(parse_expr("1 + 2 * 3"), @r#"
        Binary "+" 0..9
          1 0..1
          Binary "*" 4..9
            2 4..5
            3 8..9
        "#);
    }

    #[test]
    fn power_is_right_associative() {
        insta::assert_snapshot!(parse_expr("2 ** 3 ** 4"), @r#"
        Binary "**" 0..11
          2 0..1
          Binary "**" 5..11
            3 5..6
            4 10..11
        "#);
    }

    #[test]
    fn subtraction_is_left_associative() {
        insta::assert_snapshot!(parse_expr("1 - 2 - 3"), @r#"
        Binary "-" 0..9
          Binary "-" 0..5
            1 0..1
            2 4..5
          3 8..9
        "#);
    }

    #[test]
    fn assignment_is_right_associative() {
        insta::assert_snapshot!(parse_expr("a = b = c"), @r#"
        Assign "=" 0..9
          a 0..1
          Assign "=" 4..9
            b 4..5
            c 8..9
        "#);
    }

    #[test]
    fn output_write_operator() {
        insta::assert_snapshot!(parse_expr("out <- in * gain"), @r#"
        Assign "<-" 0..16
          out 0..3
          Binary "*" 7..16
            in 7..9
            gain 12..16
        "#);
    }

    #[test]
    fn ternary_nests_to_the_right() {
        insta::assert_snapshot!(parse_expr("a ? b : c ? d : e"), @"
        Ternary 0..17
          a 0..1
          b 4..5
          Ternary 8..17
            c 8..9
            d 12..13
            e 16..17
        ");
    }

    #[test]
    fn ternary_binds_looser_than_logical_or() {
        insta::assert_snapshot!(parse_expr("a || b ? c : d"), @r#"
        Ternary 0..14
          Binary "||" 0..6
            a 0..1
            b 5..6
          c 9..10
          d 13..14
        "#);
    }

    #[test]
    fn unary_and_parens() {
        insta::assert_snapshot!(parse_expr("-(1 + 2)"), @r#"
        Unary "-" 0..8
          Parentheses 1..8
            Binary "+" 2..7
              1 2..3
              2 6..7
        "#);
    }

    #[test]
    fn prefix_increment() {
        insta::assert_snapshot!(parse_expr("++x"), @r#"
        Unary "++" 0..3
          x 2..3
        "#);
    }

    #[test]
    fn postfix_increment() {
        insta::assert_snapshot!(parse_expr("x++"), @r#"
        PostfixUnary "++" 0..3
          x 0..1
        "#);
    }

    #[test]
    fn call_with_args() {
        insta::assert_snapshot!(parse_expr("foo(1, 2 + 3)"), @r#"
        Call 0..13
          foo 0..3
          1 4..5
          Binary "+" 7..12
            2 7..8
            3 11..12
        "#);
    }

    #[test]
    fn call_with_no_args() {
        insta::assert_snapshot!(parse_expr("advance()"), @"
        Call 0..9
          advance 0..7
        ");
    }

    #[test]
    fn empty_index() {
        insta::assert_snapshot!(parse_expr("x[]"), @"
        Bracketed 0..3
          x 0..1
        ");
    }

    #[test]
    fn index_and_field_postfix() {
        insta::assert_snapshot!(parse_expr("x.left[3]"), @r#"
        Bracketed 0..9
          Field "left" 0..6
            x 0..1
          3 7..8
        "#);
    }

    #[test]
    fn let_statement() {
        insta::assert_snapshot!(parse_stmt("let x = 1;"), @r#"
        VarDecl let "x" 0..10
          1 8..9
        "#);
    }

    #[test]
    fn var_with_init_statement() {
        insta::assert_snapshot!(parse_stmt("var y = 3;"), @r#"
        VarDecl var "y" 0..10
          3 8..9
        "#);
    }

    #[test]
    fn var_multiple_declarators() {
        insta::assert_snapshot!(parse_stmt("var a = 1, b = 2;"), @r#"
        VarDecl var "a, b" 0..17
          1 8..9
          2 15..16
        "#);
    }

    #[test]
    fn typed_var_decl_statements() {
        insta::assert_snapshot!(parse_stmt("wrap<5> w; clamp<5> c; int n = 1;"), @r#"
        VarDecl typed "w" 0..10
          VectorSizeSuffix 0..7
            wrap 0..4
            5 5..6
        "#);
    }

    #[test]
    fn const_var_decl_statement() {
        insta::assert_snapshot!(parse_stmt("const int x = 1;"), @r#"
        VarDecl typed "x" 0..16
          TypeModifier const 0..9
            int 6..9
          1 14..15
        "#);
    }

    #[test]
    fn if_else_statement() {
        insta::assert_snapshot!(parse_stmt("if (a) { b; } else { c; }"), @"
        IfStmt 0..25
          a 4..5
          Block 7..13
            ExprStmt 9..11
              b 9..10
          Block 19..25
            ExprStmt 21..23
              c 21..22
        ");
    }

    #[test]
    fn if_without_else() {
        insta::assert_snapshot!(parse_stmt("if (a) { b; }"), @"
        IfStmt 0..13
          a 4..5
          Block 7..13
            ExprStmt 9..11
              b 9..10
        ");
    }

    #[test]
    fn if_const_statement() {
        insta::assert_snapshot!(parse_stmt("if const (a) { b; }"), @"
        IfStmt const 0..19
          a 10..11
          Block 13..19
            ExprStmt 15..17
              b 15..16
        ");
    }

    #[test]
    fn while_and_bounded_loop() {
        insta::assert_snapshot!(parse_stmt(
            "while (n > 0) { n = n - 1; } loop (4) { advance(); }"
        ), @r#"
        WhileStmt 0..28
          Binary ">" 7..12
            n 7..8
            0 11..12
          Block 14..28
            ExprStmt 16..26
              Assign "=" 16..25
                n 16..17
                Binary "-" 20..25
                  n 20..21
                  1 24..25
        "#);
    }

    #[test]
    fn unbounded_loop_has_no_count() {
        insta::assert_snapshot!(parse_stmt("loop { advance(); }"), @"
        LoopStmt 0..19
          Block 5..19
            ExprStmt 7..17
              Call 7..16
                advance 7..14
        ");
    }

    #[test]
    fn return_with_and_without_value() {
        insta::assert_snapshot!(parse_stmt("return; return x + 1;"), @"ReturnStmt 0..7");
    }

    #[test]
    fn break_and_continue() {
        insta::assert_snapshot!(parse_stmt("loop { break; continue; }"), @"
        LoopStmt 0..25
          Block 5..25
            BreakStmt 7..13
            ContinueStmt 14..23
        ");
    }

    #[test]
    fn function() {
        insta::assert_snapshot!(parse_stmt("int add(int a, int b) { return a + b; }"), @r#"
        FunctionDecl "add" 0..39
          int 0..3
          VarDecl param "a" 8..13
            int 8..11
          VarDecl param "b" 15..20
            int 15..18
          Block 22..39
            ReturnStmt 24..37
              Binary "+" 31..36
                a 31..32
                b 35..36
        "#);
    }

    #[test]
    fn function_with_const_params() {
        insta::assert_snapshot!(parse_stmt("void f(const int& a, const float32[10]& b) { }"), @r#"
        FunctionDecl "f" 0..46
          void 0..4
          VarDecl param "a" 7..19
            TypeModifier const ref 7..17
              int 13..16
          VarDecl param "b" 21..41
            TypeModifier const ref 21..39
              Bracketed 27..38
                float32 27..34
                10 35..37
          Block 43..46
        "#);
    }

    #[test]
    fn const_member_function() {
        insta::assert_snapshot!(parse_stmt("void f() const { }"), @r#"
        FunctionDecl "f" const 0..18
          void 0..4
          Block 15..18
        "#);
    }

    #[test]
    fn loop_with_unbraced_body() {
        insta::assert_snapshot!(parse_stmt("void main() { loop advance(); }"), @r#"
        FunctionDecl "main" 0..31
          void 0..4
          Block 12..31
            LoopStmt 14..29
              ExprStmt 19..29
                Call 19..28
                  advance 19..26
        "#);
    }

    #[test]
    fn processor_with_typed_specialisation_param() {
        insta::assert_snapshot!(parse_stmt(
            "processor SquareWave (int length) { output stream int out; }"
        ), @r#"
        ProcessorDecl "SquareWave" 0..60
          VarDecl specialisation "length" 22..32
            int 22..25
          EndpointDecl output stream "out" 36..58
            int 50..53
        "#);
    }

    #[test]
    fn processor_with_typed_specialisation_param_default_value() {
        insta::assert_snapshot!(parse_stmt(
            "processor Gain (int channelCount = 2) { output stream int out; }"
        ), @r#"
        ProcessorDecl "Gain" 0..64
          VarDecl specialisation "channelCount" 16..36
            int 16..19
            2 35..36
          EndpointDecl output stream "out" 40..62
            int 54..57
        "#);
    }

    #[test]
    fn processor_with_using_specialisation_param() {
        insta::assert_snapshot!(parse_stmt(
            "processor Source (using DataType) { output stream int out; }"
        ), @r#"
        ProcessorDecl "Source" 0..60
          Alias using "DataType" 18..32
          EndpointDecl output stream "out" 36..58
            int 50..53
        "#);
    }

    #[test]
    fn processor_with_using_specialisation_param_default_type() {
        insta::assert_snapshot!(parse_stmt(
            "processor P (using T = float32) { output stream int out; }"
        ), @r#"
        ProcessorDecl "P" 0..58
          Alias using "T" 13..30
            float32 23..30
          EndpointDecl output stream "out" 34..56
            int 48..51
        "#);
    }

    #[test]
    fn graph_with_processor_specialisation_param() {
        insta::assert_snapshot!(parse_stmt(
            "graph Wrapper (processor Parameterised, int x) { output stream int out; }"
        ), @r#"
        GraphDecl "Wrapper" 0..73
          Alias processor "Parameterised" 15..38
          VarDecl specialisation "x" 40..45
            int 40..43
          EndpointDecl output stream "out" 49..71
            int 63..66
        "#);
    }

    #[test]
    fn namespace_with_specialisation_params() {
        insta::assert_snapshot!(parse_stmt("namespace n (processor p, namespace ns) {}"), @r#"
        NamespaceDecl "n" 0..42
          Alias processor "p" 13..24
          Alias namespace "ns" 26..38
        "#);
    }

    #[test]
    fn multiple_specialisation_params_of_different_kinds() {
        insta::assert_snapshot!(parse_stmt(
            "processor P (using T, int length = 4) { output stream int out; }"
        ), @r#"
        ProcessorDecl "P" 0..64
          Alias using "T" 13..20
          VarDecl specialisation "length" 22..36
            int 22..25
            4 35..36
          EndpointDecl output stream "out" 40..62
            int 54..57
        "#);
    }

    #[test]
    fn processor_latency_assignment_is_not_a_container_decl() {
        insta::assert_snapshot!(parse_stmt("processor.latency = length;"), @r#"
        ExprStmt 0..27
          Assign "=" 0..26
            ProcessorProperty "latency" 0..17
            length 20..26
        "#);
    }

    #[test]
    fn chevron_suffix_wins_over_comparison_when_it_parses_cleanly() {
        insta::assert_snapshot!(parse_expr("a<b>"), @"
        VectorSizeSuffix 0..4
          a 0..1
          b 2..3
        ");
    }

    #[test]
    fn bare_less_than_comparison_without_a_following_greater_than() {
        insta::assert_snapshot!(parse_expr("a < b"), @r#"
        Binary "<" 0..5
          a 0..1
          b 4..5
        "#);
    }

    #[test]
    fn comparison_chain_requires_parens() {
        insta::assert_snapshot!(parse_expr("(a < b) > c"), @r#"
        Binary ">" 0..11
          Parentheses 0..7
            Binary "<" 1..6
              a 1..2
              b 5..6
          c 10..11
        "#);
    }

    #[test]
    fn dangling_identifier_after_greedy_chevron_suffix_is_a_parse_error_in_expression_position() {
        assert!(has_diagnostics("return a < b > c;", |parser| parser.parse_statement()));
    }

    #[test]
    fn parse_error_at_end_of_input_reports_a_diagnostic_instead_of_panicking() {
        let source = "let x = 1";
        let tokens = tokenize(source);
        let mut parser = Parser::new(&tokens, source);
        parser.parse_statement();

        assert_eq!(parser.diagnostics.len(), 1);
        let diagnostic = &parser.diagnostics[0];
        assert_eq!(
            diagnostic.to_string(),
            "1:10: expected Semicolon, found EndOfFile"
        );
    }

    #[test]
    fn ambiguous_chevron_at_statement_start_is_read_as_a_type_decl() {
        insta::assert_snapshot!(parse_stmt("a < b > c;"), @r#"
        VarDecl typed "c" 0..10
          VectorSizeSuffix 0..7
            a 0..1
            b 4..5
        "#);
    }

    #[test]
    fn short_circuit_comparison_is_unaffected_by_chevron_backtracking() {
        insta::assert_snapshot!(parse_expr("a < b && b < c"), @r#"
        Binary "&&" 0..14
          Binary "<" 0..5
            a 0..1
            b 4..5
          Binary "<" 9..14
            b 9..10
            c 13..14
        "#);
    }

    #[test]
    fn type_sized_call_argument() {
        insta::assert_snapshot!(parse_expr("Sine(float64<2>, 100.0f)"), @"
        Call 0..24
          Sine 0..4
          VectorSizeSuffix 5..15
            float64 5..12
            2 13..14
          100.0f 17..23
        ");
    }

    #[test]
    fn connection() {
        insta::assert_snapshot!(parse_stmt("connection node1.out -> node2.in;"), @r#"
        ConnectionDecl 0..33
          Connection 11..32
            Sources
              Field "out" 11..20
                node1 11..16
            Destinations
              Field "in" 24..32
                node2 24..29
        "#);
    }

    #[test]
    fn connection_to_single_input() {
        insta::assert_snapshot!(parse_stmt("connection node1.out -> node2;"), @r#"
        ConnectionDecl 0..30
          Connection 11..29
            Sources
              Field "out" 11..20
                node1 11..16
            Destinations
              node2 24..29
        "#);
    }

    #[test]
    fn connections_in_a_chain() {
        insta::assert_snapshot!(parse_stmt("connection node1.out -> node2 -> node3;"), @r#"
        ConnectionDecl 0..39
          Connection 11..29
            Sources
              Field "out" 11..20
                node1 11..16
            Destinations
              node2 24..29
          Connection 24..38
            Sources
              node2 24..29
            Destinations
              node3 33..38
        "#);
    }

    #[test]
    fn connection_to_multiple_destinations() {
        insta::assert_snapshot!(parse_stmt("connection node1.out -> node2, node3;"), @r#"
        ConnectionDecl 0..37
          Connection 11..36
            Sources
              Field "out" 11..20
                node1 11..16
            Destinations
              node2 24..29
              node3 31..36
        "#);
    }

    #[test]
    fn connection_to_multiple_sources() {
        insta::assert_snapshot!(parse_stmt("connection node1.out, node2.out -> node3;"), @r#"
        ConnectionDecl 0..41
          Connection 11..40
            Sources
              Field "out" 11..20
                node1 11..16
              Field "out" 22..31
                node2 22..27
            Destinations
              node3 35..40
        "#);
    }

    #[test]
    fn connection_with_delay() {
        insta::assert_snapshot!(parse_stmt("connection node1.out -> [100] -> node2;"), @r#"
        ConnectionDecl 0..39
          Connection 11..38
            Sources
              Field "out" 11..20
                node1 11..16
            Delay
              100 25..28
            Destinations
              node2 33..38
        "#);
    }

    #[test]
    fn connection_with_interpolation() {
        insta::assert_snapshot!(parse_stmt("connection [linear] node1.out -> node2;"), @r#"
        ConnectionDecl 0..39
          Connection [linear] 20..38
            Sources
              Field "out" 20..29
                node1 20..25
            Destinations
              node2 33..38
        "#);
    }

    #[test]
    fn connection_block() {
        insta::assert_snapshot!(parse_stmt("connection  { node1.out -> node2, node3; node2.out -> node4; }"), @r#"
        ConnectionDecl 0..62
          Connection 14..39
            Sources
              Field "out" 14..23
                node1 14..19
            Destinations
              node2 27..32
              node3 34..39
          Connection 41..59
            Sources
              Field "out" 41..50
                node2 41..46
            Destinations
              node4 54..59
        "#);
    }

    #[test]
    fn empty_connection_block() {
        insta::assert_snapshot!(parse_stmt("connection {}"), @"ConnectionDecl 0..13");
    }

    #[test]
    fn conditional_connection() {
        insta::assert_snapshot!(
            parse_stmt("connection { if (useDistortionFirst) in -> distortion -> out; else in -> out; }"),
            @"
        ConnectionDecl 0..79
          ConnectionIf 13..77
            useDistortionFirst 17..35
            Then
              Connection 37..53
                Sources
                  in 37..39
                Destinations
                  distortion 43..53
              Connection 43..60
                Sources
                  distortion 43..53
                Destinations
                  out 57..60
            Else
              Connection 67..76
                Sources
                  in 67..69
                Destinations
                  out 73..76
        "
        );
    }

    #[test]
    fn infinite_for_loop() {
        insta::assert_snapshot!(parse_stmt("for (;;) { advance(); }"), @"
        ForStmt 0..23
          Block 9..23
            ExprStmt 11..21
              Call 11..20
                advance 11..18
        ");
    }

    #[test]
    fn classic_for_loop() {
        insta::assert_snapshot!(parse_stmt("for (int i = 0; i < 10; ++i) { advance(); }"), @r#"
        ForStmt 0..43
          VarDecl typed "i" 5..14
            int 5..8
            0 13..14
          Binary "<" 16..22
            i 16..17
            10 20..22
          Unary "++" 24..27
            i 26..27
          Block 29..43
            ExprStmt 31..41
              Call 31..40
                advance 31..38
        "#);
    }

    #[test]
    fn bounded_range_for_loop() {
        insta::assert_snapshot!(parse_stmt("for (wrap<4> i) { advance(); }"), @r#"
        LoopStmt 0..30
          VarDecl typed "i" 5..14
            VectorSizeSuffix 5..12
              wrap 5..9
              4 10..11
          Block 16..30
            ExprStmt 18..28
              Call 18..27
                advance 18..25
        "#);
    }

    #[test]
    fn labelled_bounded_range_for_loop() {
        insta::assert_snapshot!(parse_stmt("outer: for (wrap<4> i) { advance(); }"), @r#"
        LoopStmt "outer" 0..37
          VarDecl typed "i" 12..21
            VectorSizeSuffix 12..19
              wrap 12..16
              4 17..18
          Block 23..37
            ExprStmt 25..35
              Call 25..34
                advance 25..32
        "#);
    }

    #[test]
    fn enum_decl() {
        insta::assert_snapshot!(parse_stmt("enum Mode { A, B, C }"), @r#"EnumDecl "Mode" {A, B, C} 0..21"#);
    }

    #[test]
    fn import_dotted_path() {
        insta::assert_snapshot!(parse_stmt("import std.audio;"), @r#"Import "std.audio" 0..17"#);
    }

    #[test]
    fn import_string_path() {
        insta::assert_snapshot!(parse_stmt(r#"import "foo.cmajor";"#), @r#"Import "\"foo.cmajor\"" 0..20"#);
    }

    #[test]
    fn external_var_decl() {
        insta::assert_snapshot!(parse_stmt("external float64 one;"), @r#"
        VarDecl external typed "one" 9..21
          float64 9..16
        "#);
    }

    #[test]
    fn using_type_alias_statement() {
        insta::assert_snapshot!(parse_stmt("using T = int;"), @r#"
        Alias using "T" 0..14
          int 10..13
        "#);
    }

    #[test]
    fn static_assert_with_message() {
        insta::assert_snapshot!(
            parse_stmt(r#"static_assert(x > 0, "must be positive");"#),
            @r#"
        ExprStmt 0..41
          Call 0..40
            static_assert 0..13
            Binary ">" 14..19
              x 14..15
              0 18..19
            "must be positive" 21..39
        "#
        );
    }

    #[test]
    fn forward_branch_stmt() {
        insta::assert_snapshot!(
            parse_stmt("forward_branch (cond) -> (a, b);"),
            @"
        ForwardBranchStmt 0..32
          cond 16..20
          a
          b
        "
        );
    }

    #[test]
    fn labelled_loop_and_break_target() {
        insta::assert_snapshot!(
            parse_stmt("outer: loop { break outer; }"),
            @r#"
        LoopStmt "outer" 0..28
          Block 12..28
            BreakStmt "outer" 14..26
        "#
        );
    }

    #[test]
    fn multi_dimensional_array_type() {
        insta::assert_snapshot!(parse_type("float32[1, 2]"), @"
        Bracketed 0..13
          float32 0..7
          1 8..9
          2 11..12
        ");
    }

    #[test]
    fn slicing_expression() {
        insta::assert_snapshot!(parse_expr("arr[1:3]"), @"
        Bracketed 0..8
          arr 0..3
          Slice 4..7
            1 4..5
            3 6..7
        ");
    }

    #[test]
    fn processor_alias_decl() {
        insta::assert_snapshot!(
            parse_stmt("processor Foo = Bar(4);"),
            @r#"
        ModuleAlias processor "Foo" 0..23
          Call 16..22
            Bar 16..19
            4 20..21
        "#
        );
    }

    #[test]
    fn hoisted_endpoint_named() {
        insta::assert_snapshot!(parse_stmt("output child.out;"), @"EndpointDecl output child.out 0..17");
    }

    #[test]
    fn hoisted_endpoint_wildcard() {
        insta::assert_snapshot!(parse_stmt("output child.*;"), @"EndpointDecl output child.* 0..15");
    }

    #[test]
    fn hoisted_endpoint_prefixed_wildcard() {
        insta::assert_snapshot!(parse_stmt("output g2.test*;"), @"EndpointDecl output g2.test* 0..16");
    }

    #[test]
    fn hoisted_endpoint_chained_through_nested_node() {
        insta::assert_snapshot!(
            parse_stmt("input q.unused.in;"),
            @"EndpointDecl input q.unused.in 0..18"
        );
    }

    #[test]
    fn sized_array_endpoint() {
        insta::assert_snapshot!(parse_stmt("input stream float in[10];"), @r#"
        EndpointDecl input stream "in" 0..26
          float 13..18
          10 22..24
        "#);
    }

    #[test]
    fn endpoint_with_attribute_list() {
        insta::assert_snapshot!(
            parse_stmt(r#"input event bool hpEnable [[ name: "HP Enable", init: true, boolean ]];"#),
            @r#"
        EndpointDecl input event "hpEnable" 0..71
          bool 12..16
          AttributeList 26..70
            "name"
              "HP Enable" 35..46
            "init"
              true 54..58
            "boolean"
        "#
        );
    }

    #[test]
    fn sized_array_endpoint_with_attribute_list() {
        insta::assert_snapshot!(parse_stmt("input stream float in[10] [[ min: 0.0 ]];"), @r#"
        EndpointDecl input stream "in" 0..41
          float 13..18
          10 22..24
          AttributeList 26..40
            "min"
              0.0 34..37
        "#);
    }

    #[test]
    fn multi_type_event_endpoint() {
        insta::assert_snapshot!(parse_stmt("input event (int, float) e;"), @r#"
        EndpointDecl input event "e" 0..27
          int 13..16
          float 18..23
        "#);
    }

    #[test]
    fn hoisted_endpoint_with_attributes() {
        insta::assert_snapshot!(parse_stmt("input filter.frequency [[ mid: 1000 ]];"), @r#"
        EndpointDecl input filter.frequency 0..39
          AttributeList 23..38
            "mid"
              1000 31..35
        "#);
    }

    #[test]
    fn hoisted_endpoint_with_rename_and_attributes() {
        insta::assert_snapshot!(
            parse_stmt("input modulator.frequencyIn modulationFrequency [[ min: 1.0 ]];"),
            @r#"
        EndpointDecl input modulator.frequencyIn 0..63
          AttributeList 48..62
            "min"
              1.0 56..59
        "#
        );
    }

    #[test]
    fn qualified_type_name_with_call_segment() {
        insta::assert_snapshot!(parse_type("Initialized(InitCode)::ADSR"), @r#"
        ScopeAccess "ADSR" 0..27
          Call 0..21
            Initialized 0..11
            InitCode 12..20
        "#);
    }

    #[test]
    fn outer_braced_endpoint_group_with_mixed_kinds() {
        insta::assert_snapshot!(
            dump("input { event int e; value float v; }", |parser| {
                let items = parser.parse_endpoint_group();
                let brace = parser.peek_verbose().0;
                let start = parser.spans[*items.first().expect("test has at least one item")].start;
                parser.add_node(
                    start,
                    Stmt::Block(Block {
                        brace,
                        label: None,
                        stmts: items,
                    }),
                )
            }),
            @r#"
        Block 0..37
          EndpointDecl input event "e" 0..20
            int 14..17
          EndpointDecl input value "v" 0..35
            float 27..32
        "#
        );
    }

    #[test]
    fn nested_subscript_splits_combined_close_bracket_token() {
        insta::assert_snapshot!(parse_expr("a[b[c]]"), @"
        Bracketed 0..7
          a 0..1
          Bracketed 2..6
            b 2..3
            c 4..5
        ");
    }

    #[test]
    fn three_clause_for_loop_with_var_init() {
        insta::assert_snapshot!(parse_stmt("for (var i = 0; i < 10; ++i) {}"), @r#"
        ForStmt 0..31
          VarDecl var "i" 5..14
            0 13..14
          Binary "<" 16..22
            i 16..17
            10 20..22
          Unary "++" 24..27
            i 26..27
          Block 29..31
        "#);
    }

    #[test]
    fn three_clause_for_loop_with_let_init() {
        insta::assert_snapshot!(parse_stmt("for (let i = 0; i < 10; ++i) {}"), @r#"
        ForStmt 0..31
          VarDecl let "i" 5..14
            0 13..14
          Binary "<" 16..22
            i 16..17
            10 20..22
          Unary "++" 24..27
            i 26..27
          Block 29..31
        "#);
    }

    #[test]
    fn dotted_member_access_in_type_position() {
        insta::assert_snapshot!(parse_type("ArrayType.elementType"), @r#"
        Field "elementType" 0..21
          ArrayType 0..9
        "#);
    }

    #[test]
    fn using_target_is_a_full_expression() {
        insta::assert_snapshot!(
            parse_stmt("using ComplexType = FloatArray.elementType.isFloat32 ? complex32 : complex64;"),
            @r#"
        Alias using "ComplexType" 0..77
          Ternary 20..76
            Field "isFloat32" 20..52
              Field "elementType" 20..42
                FloatArray 20..30
            complex32 55..64
            complex64 67..76
        "#
        );
    }

    #[test]
    fn braced_endpoint_group_desugars_to_flat_members() {
        insta::assert_snapshot!(
            dump("output stream { float32 a; int b; }", |parser| {
                let items = parser.parse_container_items();
                let brace = parser.peek_verbose().0;
                let start = parser.spans[*items.first().expect("test has at least one item")].start;
                parser.add_node(
                    start,
                    Stmt::Block(Block {
                        brace,
                        label: None,
                        stmts: items,
                    }),
                )
            }),
            @r#"
        Block 0..35
          EndpointDecl output stream "a" 0..26
            float32 16..23
          EndpointDecl output stream "b" 0..33
            int 27..30
        "#
        );
    }

    #[test]
    fn comma_separated_node_decls_without_braces() {
        insta::assert_snapshot!(
            dump("node b = B, c = C;", |parser| {
                let items = parser.parse_node_group();
                let brace = parser.peek_verbose().0;
                let start = parser.spans[*items.first().expect("test has at least one item")].start;
                parser.add_node(
                    start,
                    Stmt::Block(Block {
                        brace,
                        label: None,
                        stmts: items,
                    }),
                )
            }),
            @r#"
        Block 0..18
          NodeDecl "b" 0..10
            B 9..10
          NodeDecl "c" 0..17
            C 16..17
        "#
        );
    }
}
