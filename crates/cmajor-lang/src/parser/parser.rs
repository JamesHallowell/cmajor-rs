use crate::{
    ast::{Ast, Node, NodeId, SpecialisationParamKind},
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
                self.ast.push(Node::Unary { op, operand })
            }
            Some(TokenKind::Literal(literal)) => self.parse_literal(literal),
            Some(token!(true | false)) => {
                let token = self.bump();
                self.ast.push(Node::BoolLiteral { token })
            }
            Some(TokenKind::Identifier | token!(processor)) => {
                let token = self.bump();
                self.ast.push(Node::Ident { token })
            }
            Some(token!('(')) => {
                let (paren, inner, _) = self.parse_parenthetical_list(|parser| parser.parse_expr());
                self.ast.push(Node::Parentheses { paren, inner })
            }
            Some(token) if token.is_type_like() => {
                let token = self.bump();
                self.ast.push(Node::Ident { token })
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
                Some(token!('[')) => self.parse_index(lhs),
                Some(token!(.)) => self.parse_field(lhs),
                Some(token!(::)) => self.parse_scope_access(lhs),
                Some(token!(<)) => {
                    let is_part_of_type_constructor = self
                        .tokens
                        .clone()
                        .take_while(|(_, token)| !matches!(token.kind, token!(; | '{' | '}')))
                        .find_map(|(id, token)| (token.kind == token!(>)).then_some(id))
                        .and_then(|token| self.tokens.peek_after(token))
                        .map(|token| token.kind == token!('('))
                        .unwrap_or(false);

                    if is_part_of_type_constructor {
                        self.parse_chevroned_suffix(lhs)
                    } else {
                        break;
                    }
                }
                Some(token!(++ | --)) => {
                    let op = self.bump();
                    self.ast.push(Node::PostfixUnary { op, operand: lhs })
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

                    self.ast.push(Node::Ternary {
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
                        Node::Assign {
                            op,
                            target: lhs,
                            value: rhs,
                        }
                    } else {
                        Node::Binary { op, lhs, rhs }
                    })
                }
            };
        }

        lhs
    }

    fn parse_scope_access(&mut self, base: NodeId) -> NodeId {
        let (_, name) = expect!(self, token!(::), TokenKind::Identifier);
        self.ast.push(Node::ScopeAccess { name, base })
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
        self.ast.push(Node::Call {
            paren,
            callee,
            args,
        })
    }

    fn parse_index(&mut self, base: NodeId) -> NodeId {
        let (bracket, index, _) = expect!(
            self,
            token!('['),
            if !peek token!(']') {
                self.parse_expr()
            },
            token!(']'),
        );

        self.ast.push(Node::Index {
            bracket,
            base,
            index,
        })
    }

    fn parse_field(&mut self, base: NodeId) -> NodeId {
        let (_, name) = expect!(self, token!(.), TokenKind::Identifier);
        self.ast.push(Node::Field { name, base })
    }

    fn parse_literal(&mut self, literal: Literal) -> NodeId {
        let token = self.bump();
        let node = match literal {
            Literal::Int32 | Literal::Int64 => Node::IntLiteral { token },
            Literal::Float32 | Literal::Float64 => Node::FloatLiteral { token },
            Literal::Imaginary32 | Literal::Imaginary64 => Node::ImaginaryLiteral { token },
            Literal::String => Node::StringLiteral { token },
        };
        self.ast.push(node)
    }

    fn parse_statement(&mut self) -> NodeId {
        match self.tokens.peek_kind() {
            Some(token!('{')) => self.parse_block(),
            Some(token!(let)) => self.parse_let(),
            Some(token!(var)) => self.parse_var(),
            Some(token!(if)) => self.parse_if(),
            Some(token!(while)) => self.parse_while(),
            Some(token!(loop)) => self.parse_loop(),
            Some(token!(for)) => self.parse_for(),
            Some(token!(node)) => self.parse_node_decl(),
            Some(token!(connection)) => self.parse_connection_decl(),
            Some(token!(return)) => self.parse_return(),
            Some(token!(break)) => self.parse_break(),
            Some(token!(continue)) => self.parse_continue(),
            Some(token!(namespace)) => self.parse_namespace(),
            Some(token!(processor))
                if self.tokens.clone().nth(1).map(|(_, token)| token.kind) == Some(token!(.)) =>
            {
                self.parse_expr_stmt()
            }
            Some(token!(processor | graph | struct)) => self.parse_container(),
            Some(token!(input | output)) => self.parse_endpoint_group(),
            Some(token!(event)) => self.parse_event_handler(),
            Some(token!(const)) => self.parse_typed_decl(),
            Some(TokenKind::Keyword(keyword)) if TokenKind::Keyword(keyword).is_type_like() => {
                self.parse_typed_decl()
            }
            Some(TokenKind::Identifier) if self.looks_like_typed_decl() => self.parse_typed_decl(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_break(&mut self) -> NodeId {
        let (keyword, _) = expect!(self, token!(break), token!(;));
        self.ast.push(Node::BreakStmt { keyword })
    }

    fn parse_continue(&mut self) -> NodeId {
        let (keyword, _) = expect!(self, token!(continue), token!(;));
        self.ast.push(Node::ContinueStmt { keyword })
    }

    fn parse_typed_decl(&mut self) -> NodeId {
        self.parse_typed_decl_inner(true)
    }

    fn parse_typed_decl_inner(&mut self, consume_semicolon: bool) -> NodeId {
        let (ty, name) = expect!(self, self.parse_type(), TokenKind::Identifier);

        if self.tokens.peek_kind() == Some(token!(<))
            || self.tokens.peek_kind() == Some(token!('('))
        {
            let generics = self.parse_optional_generics();
            self.parse_function_decl(ty, name, generics)
        } else {
            let mut declarators = vec![(
                name,
                expect!(
                    self,
                    if token!(=) {
                        self.parse_expr()
                    }
                )
                .map(|(_, init)| init),
            )];
            while self.tokens.peek_kind() == Some(token!(,)) {
                self.bump();

                let (name, init) = expect!(
                    self,
                    TokenKind::Identifier,
                    if token!(=) {
                        self.parse_expr()
                    }
                );
                declarators.push((name, init.map(|(_, init)| init)));
            }
            if consume_semicolon {
                self.expect(token!(;));
            }
            self.ast.push(Node::VarDeclStmt { ty, declarators })
        }
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
        let (ty, by_ref, name) = expect!(
            self,
            self.parse_type(),
            if token!(&),
            TokenKind::Identifier
        );

        self.ast.push(Node::Param {
            ty,
            name,
            by_ref: by_ref.is_some(),
        })
    }

    fn parse_params(&mut self) -> Vec<NodeId> {
        let (_, params, _) = self.parse_parenthetical_list(|parser| parser.parse_param());
        params
    }

    fn parse_function_decl(&mut self, ty: NodeId, name: TokenId, generics: Vec<TokenId>) -> NodeId {
        let (params, is_const, body) = expect!(
            self,
            self.parse_params(),
            if token!(const),
            self.parse_block()
        );

        self.ast.push(Node::FunctionDecl {
            ty,
            name,
            generics,
            params,
            is_const: is_const.is_some(),
            body,
        })
    }

    fn parse_event_handler(&mut self) -> NodeId {
        let (keyword, name, params, body) = expect!(
            self,
            token!(event),
            TokenKind::Identifier,
            self.parse_params(),
            self.parse_block()
        );

        self.ast.push(Node::EventHandlerDecl {
            keyword,
            name,
            params,
            body,
        })
    }

    fn parse_namespace(&mut self) -> NodeId {
        let (keyword, segments, params, _, items, _) = expect!(
            self,
            token!(namespace),
            {
                let mut segments = vec![self.expect(TokenKind::Identifier)];
                while self.bump_if(token!(::)).is_some() {
                    segments.push(self.expect(TokenKind::Identifier));
                }
                segments
            },
            self.parse_optional_specialisation_params(),
            token!('{'),
            {
                let mut items = Vec::new();
                while !matches!(self.tokens.peek_kind(), Some(token!('}')) | None) {
                    items.push(self.parse_statement());
                }
                items
            },
            token!('}')
        );

        self.ast.push(Node::NamespaceDecl {
            keyword,
            segments,
            params,
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
        let kind = match self.tokens.peek_kind() {
            Some(token!(using)) => {
                self.bump();
                SpecialisationParamKind::Using
            }
            Some(token!(processor)) => {
                self.bump();
                SpecialisationParamKind::Processor
            }
            Some(token!(namespace)) => {
                self.bump();
                SpecialisationParamKind::Namespace
            }
            _ => {
                let ty = self.parse_type();
                SpecialisationParamKind::Value { ty }
            }
        };
        let name = self.expect(TokenKind::Identifier);
        let default = if self.tokens.peek_kind() == Some(token!(=)) {
            self.bump();
            Some(match kind {
                SpecialisationParamKind::Value { .. } => self.parse_expr(),
                _ => self.parse_type(),
            })
        } else {
            None
        };
        self.ast.push(Node::SpecialisationParam {
            kind,
            name,
            default,
        })
    }

    fn parse_container(&mut self) -> NodeId {
        let keyword = expect_matches!(self, token!(processor | graph | struct));
        let name = self.expect(TokenKind::Identifier);
        let params = self.parse_optional_specialisation_params();
        let attributes =
            (self.tokens.peek_kind() == Some(token!("[["))).then(|| self.parse_attribute_list());
        self.expect(token!('{'));

        let mut items = Vec::new();
        while !matches!(self.tokens.peek_kind(), Some(token!('}')) | None) {
            items.push(self.parse_statement());
        }
        self.expect(token!('}'));

        match self.tokens.stream().get(keyword).map(|token| token.kind) {
            Some(token!(graph)) => self.ast.push(Node::GraphDecl {
                keyword,
                name,
                params,
                attributes,
                items,
            }),
            Some(token!(struct)) => self.ast.push(Node::StructDecl {
                keyword,
                name,
                attributes,
                items,
            }),
            Some(token!(processor)) => self.ast.push(Node::ProcessorDecl {
                keyword,
                name,
                params,
                attributes,
                items,
            }),
            _ => unreachable!(),
        }
    }

    fn parse_node_decl(&mut self) -> NodeId {
        let (keyword, name, _, value, _) = expect!(
            self,
            token!(node),
            TokenKind::Identifier,
            token!(=),
            self.parse_expr(),
            token!(;)
        );

        self.ast.push(Node::NodeDecl {
            keyword,
            name,
            value,
        })
    }

    fn parse_connection_decl(&mut self) -> NodeId {
        let keyword = self.expect(token!(connection));
        let connections = self.parse_connection_list();
        self.ast.push(Node::ConnectionDecl {
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

        self.ast.push(Node::ConnectionIf {
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
                    if matches!(self.ast.get(dest), Node::Field { .. }) {
                        self.error(
                            arrow,
                            "cannot name an endpoint in the middle of a connection chain",
                        );
                    }
                }
            }

            connections.push(self.ast.push(Node::Connection {
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

    fn parse_interpolation_if_present(&mut self) -> Option<TokenId> {
        let mut peek = self.tokens.clone().map(|(id, token)| (id, token.kind));

        match (peek.next(), peek.next(), peek.next()) {
            (
                Some((_, token!('['))),
                Some((delay, TokenKind::Identifier)),
                Some((_, token!(']'))),
            ) => {
                if matches!(
                    self.tokens.stream().text(self.source, delay),
                    Some("none" | "latch" | "linear" | "sinc" | "fast" | "best")
                ) {
                    let (_, interpolation, _) =
                        expect!(self, token!('['), TokenKind::Identifier, token!(']'));
                    return Some(interpolation);
                }

                None
            }
            _ => None,
        }
    }

    fn parse_for(&mut self) -> NodeId {
        let (keyword, _, init, _, cond, _, update, _, body) = expect!(
            self,
            token!(for),
            token!('('),
            if !peek token!(;) {
                self.parse_for_init()
            },
            token!(;),
            if !peek token!(;) {
                self.parse_expr()
            },
            token!(;),
            if !peek token!(')') {
                self.parse_expr()
            },
            token!(')'),
            self.parse_statement()
        );

        self.ast.push(Node::ForStmt {
            keyword,
            init,
            cond,
            update,
            body,
        })
    }

    fn parse_for_init(&mut self) -> NodeId {
        match self.tokens.peek_kind() {
            Some(token!(const)) => self.parse_typed_decl_inner(false),
            Some(TokenKind::Keyword(keyword)) if keyword.is_type() => {
                self.parse_typed_decl_inner(false)
            }
            Some(TokenKind::Identifier) if self.looks_like_typed_decl() => {
                self.parse_typed_decl_inner(false)
            }
            _ => {
                let expr = self.parse_expr();
                self.ast.push(Node::ExprStmt { expr })
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
        let (key, value) = expect!(
            self,
            TokenKind::Identifier,
            if token!(:) {
                self.parse_expr()
            }
        );

        (key, value.map(|(_, value)| value))
    }

    fn parse_attribute_list(&mut self) -> NodeId {
        let (_, attrs, _) = self.parse_list(
            token!("[["),
            |parser| parser.parse_attribute(),
            token!(,),
            token!("]]"),
        );

        self.ast.push(Node::AttributeList { attrs })
    }

    fn parse_type_list(&mut self) -> NodeId {
        let (paren, types, _) = self.parse_parenthetical_list(|parser| parser.parse_type());
        self.ast.push(Node::TypeList { paren, types })
    }

    fn parse_endpoint_member(&mut self) -> NodeId {
        let (ty, name, attributes, _) = expect!(
            self,
            if peek token!('(') {
                self.parse_type_list()
            } else {
                self.parse_type()
            },
            TokenKind::Identifier,
            if peek token!("[[") {
                self.parse_attribute_list()
            },
            token!(;)
        );

        self.ast.push(Node::EndpointDecl {
            ty,
            name,
            attributes,
        })
    }

    fn parse_endpoint_wildcard(&mut self, direction: TokenId) -> NodeId {
        let (name, _, _, _) = expect!(self, TokenKind::Identifier, token!(.), token!(*), token!(;));
        self.ast.push(Node::EndpointWildcard { direction, name })
    }

    fn parse_endpoint_group(&mut self) -> NodeId {
        let direction = expect_matches!(self, token!(input | output));
        if self.tokens.peek_kind() == Some(TokenKind::Identifier) && self.at(1, token!(.)) {
            return self.parse_endpoint_wildcard(direction);
        }
        let kind = expect_identifier!(self, "value" | "stream" | "event");
        let endpoints = if self.tokens.peek_kind() == Some(token!('{')) {
            self.bump();
            let mut endpoints = Vec::new();
            while !matches!(self.tokens.peek_kind(), Some(token!('}')) | None) {
                endpoints.push(self.parse_endpoint_member());
            }
            self.expect(token!('}'));
            endpoints
        } else {
            vec![self.parse_endpoint_member()]
        };
        self.ast.push(Node::EndpointGroup {
            direction,
            kind,
            endpoints,
        })
    }

    fn parse_block(&mut self) -> NodeId {
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

        self.ast.push(Node::Block { brace, stmts })
    }

    fn parse_let(&mut self) -> NodeId {
        let (_, declarators, _) = self.parse_list(
            token!(let),
            |parser| parser.parse_let_declarator(),
            token!(,),
            token!(;),
        );

        self.ast.push(Node::LetStmt { declarators })
    }

    fn parse_let_declarator(&mut self) -> (TokenId, NodeId) {
        let (name, _, init) = expect!(self, TokenKind::Identifier, token!(=), self.parse_expr());
        (name, init)
    }

    fn parse_var(&mut self) -> NodeId {
        let (_, name, init, _) = expect!(
            self,
            token!(var),
            TokenKind::Identifier,
            { self.bump_if(token!(=)).is_some().then(|| self.parse_expr()) },
            token!(;)
        );

        self.ast.push(Node::VarStmt { name, init })
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

        self.ast.push(Node::IfStmt {
            keyword,
            is_const: is_const.is_some(),
            cond,
            then_branch,
            else_branch: else_branch.map(|(_, else_branch)| else_branch),
        })
    }

    fn parse_while(&mut self) -> NodeId {
        let (keyword, _, cond, _, body) = expect!(
            self,
            token!(while),
            token!('('),
            self.parse_expr(),
            token!(')'),
            self.parse_statement()
        );

        self.ast.push(Node::WhileStmt {
            keyword,
            cond,
            body,
        })
    }

    fn parse_loop(&mut self) -> NodeId {
        let (keyword, count, body) = expect!(
            self,
            token!(loop),
            if token!('(') {
                self.parse_expr(), token!(')')
            },
            self.parse_statement()
        );

        self.ast.push(Node::LoopStmt {
            keyword,
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

        self.ast.push(Node::ReturnStmt { keyword, value })
    }

    fn parse_expr_stmt(&mut self) -> NodeId {
        let (expr, _) = expect!(self, self.parse_expr(), token!(;));
        self.ast.push(Node::ExprStmt { expr })
    }

    fn parse_type(&mut self) -> NodeId {
        if let Some((keyword, inner)) = expect!(
            self,
            if token!(const) {
                self.parse_type()
            }
        ) {
            return self.ast.push(Node::ConstType { keyword, inner });
        }

        let mut ty = match self.tokens.peek_kind() {
            Some(kind) if kind.is_type_like() => self.parse_type_name(),
            peeked => {
                let message = format!("expected type, found {peeked:?}");
                let token = self.bump();
                self.error(token, message);
                self.ast.push(Node::Error { token })
            }
        };

        loop {
            ty = match self.tokens.peek_kind() {
                Some(token!('[')) => self.parse_array(ty),
                Some(token!(<)) => self.parse_chevroned_suffix(ty),
                _ => break,
            };
        }

        ty
    }

    fn parse_type_name(&mut self) -> NodeId {
        let mut segments = vec![self.bump()];

        loop {
            let Some((_, segment)) = expect!(
                self,
                if token!(::) {
                    TokenKind::Identifier
                }
            ) else {
                break;
            };

            segments.push(segment);
        }

        self.ast.push(Node::TypeName { segments })
    }

    fn parse_array(&mut self, element: NodeId) -> NodeId {
        let (bracket, size, _) = expect!(
            self,
            token!('['),
            if !peek token!(']') {
                self.parse_expr()
            },
            token!(']')
        );

        self.ast.push(Node::Array {
            bracket,
            element,
            size,
        })
    }

    fn parse_chevroned_suffix(&mut self, element: NodeId) -> NodeId {
        let (angle, term, _) = expect!(
            self,
            token!(<),
            self.parse_expr_with_min_binding_power(PrecedenceLevel::Shift.base()),
            token!(>)
        );

        self.ast.push(Node::ChevronSuffix {
            angle,
            element,
            term,
        })
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
        insta::assert_snapshot!(parse_type("float"), @r#"TypeName "float""#);
    }

    #[test]
    fn named_type() {
        insta::assert_snapshot!(parse_type("MyStruct"), @r#"TypeName "MyStruct""#);
    }

    #[test]
    fn qualified_type_name() {
        insta::assert_snapshot!(parse_type("std::midi::Message"), @r#"TypeName "std::midi::Message""#);
    }

    #[test]
    fn array_type() {
        insta::assert_snapshot!(parse_type("int[3]"), @r#"
        Array
          TypeName "int"
          3
        "#);
    }

    #[test]
    fn slice_type_has_no_size() {
        insta::assert_snapshot!(parse_type("int[]"), @r#"
        Array
          TypeName "int"
        "#);
    }

    #[test]
    fn vector_type() {
        insta::assert_snapshot!(parse_type("int<4>"), @r#"
        ChevronSuffix
          TypeName "int"
          4
        "#);
    }

    #[test]
    fn clamp() {
        insta::assert_snapshot!(parse_type("clamp<10>"), @r#"
        ChevronSuffix
          TypeName "clamp"
          10
        "#);
    }

    #[test]
    fn wrap() {
        insta::assert_snapshot!(parse_type("wrap<4>"), @r#"
        ChevronSuffix
          TypeName "wrap"
          4
        "#);
    }

    #[test]
    fn angle_bracket_close_is_not_a_comparison() {
        insta::assert_snapshot!(parse_type("wrap<1 + 2>"), @r#"
        ChevronSuffix
          TypeName "wrap"
          Binary "+"
            1
            2
        "#);
    }

    #[test]
    fn postfix_type_modifiers_apply_left_to_right() {
        insta::assert_snapshot!(parse_type("int<4>[2]"), @r#"
        Array
          ChevronSuffix
            TypeName "int"
            4
          2
        "#);
    }

    #[test]
    fn const_type() {
        insta::assert_snapshot!(parse_type("const int"), @r#"
        ConstType
          TypeName "int"
        "#);
    }

    #[test]
    fn const_array_type() {
        insta::assert_snapshot!(parse_type("const int[]"), @r#"
        ConstType
          Array
            TypeName "int"
        "#);
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
        insta::assert_snapshot!(parse_expr("x[]"), @r#"
        Index
          x
        "#);
    }

    #[test]
    fn index_and_field_postfix() {
        insta::assert_snapshot!(parse_expr("x.left[3]"), @r#"
        Index
          Field "left"
            x
          3
        "#);
    }

    #[test]
    fn let_statement() {
        insta::assert_snapshot!(parse_stmt("let x = 1;"), @r#"
        LetStmt "x"
          1
        "#);
    }

    #[test]
    fn var_statement() {
        insta::assert_snapshot!(parse_stmt("var y;"), @r#"VarStmt "y""#);
    }

    #[test]
    fn var_with_init_statement() {
        insta::assert_snapshot!(parse_stmt("var y = 3;"), @r#"
        VarStmt "y"
          3
        "#);
    }

    #[test]
    fn typed_var_decl_statements() {
        insta::assert_snapshot!(parse_stmt("wrap<5> w; clamp<5> c; int n = 1;"), @r#"
        VarDeclStmt "w"
          ChevronSuffix
            TypeName "wrap"
            5
        "#);
    }

    #[test]
    fn const_var_decl_statement() {
        insta::assert_snapshot!(parse_stmt("const int x = 1;"), @r#"
        VarDeclStmt "x"
          ConstType
            TypeName "int"
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
          TypeName "int"
          Param "a"
            TypeName "int"
          Param "b"
            TypeName "int"
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
          TypeName "void"
          Param &"a"
            ConstType
              TypeName "int"
          Param &"b"
            ConstType
              Array
                TypeName "float32"
                10
          Block
        "#);
    }

    #[test]
    fn const_member_function() {
        insta::assert_snapshot!(parse_stmt("void f() const { }"), @r#"
        FunctionDecl "f" const
          TypeName "void"
          Block
        "#);
    }

    #[test]
    fn loop_with_unbraced_body() {
        insta::assert_snapshot!(parse_stmt("void main() { loop advance(); }"), @r#"
        FunctionDecl "main"
          TypeName "void"
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
          SpecialisationParam value "length"
            TypeName "int"
          EndpointGroup output stream
            EndpointDecl "out"
              TypeName "int"
        "#);
    }

    #[test]
    fn processor_with_typed_specialisation_param_default_value() {
        insta::assert_snapshot!(parse_stmt(
            "processor Gain (int channelCount = 2) { output stream int out; }"
        ), @r#"
        ProcessorDecl "Gain"
          SpecialisationParam value "channelCount"
            TypeName "int"
            2
          EndpointGroup output stream
            EndpointDecl "out"
              TypeName "int"
        "#);
    }

    #[test]
    fn processor_with_using_specialisation_param() {
        insta::assert_snapshot!(parse_stmt(
            "processor Source (using DataType) { output stream int out; }"
        ), @r#"
        ProcessorDecl "Source"
          SpecialisationParam using "DataType"
          EndpointGroup output stream
            EndpointDecl "out"
              TypeName "int"
        "#);
    }

    #[test]
    fn processor_with_using_specialisation_param_default_type() {
        insta::assert_snapshot!(parse_stmt(
            "processor P (using T = float32) { output stream int out; }"
        ), @r#"
        ProcessorDecl "P"
          SpecialisationParam using "T"
            TypeName "float32"
          EndpointGroup output stream
            EndpointDecl "out"
              TypeName "int"
        "#);
    }

    #[test]
    fn graph_with_processor_specialisation_param() {
        insta::assert_snapshot!(parse_stmt(
            "graph Wrapper (processor Parameterised, int x) { output stream int out; }"
        ), @r#"
        GraphDecl "Wrapper"
          SpecialisationParam processor "Parameterised"
          SpecialisationParam value "x"
            TypeName "int"
          EndpointGroup output stream
            EndpointDecl "out"
              TypeName "int"
        "#);
    }

    #[test]
    fn namespace_with_specialisation_params() {
        insta::assert_snapshot!(parse_stmt("namespace n (processor p, namespace ns) {}"), @r#"
        NamespaceDecl "n"
          SpecialisationParam processor "p"
          SpecialisationParam namespace "ns"
        "#);
    }

    #[test]
    fn multiple_specialisation_params_of_different_kinds() {
        insta::assert_snapshot!(parse_stmt(
            "processor P (using T, int length = 4) { output stream int out; }"
        ), @r#"
        ProcessorDecl "P"
          SpecialisationParam using "T"
          SpecialisationParam value "length"
            TypeName "int"
            4
          EndpointGroup output stream
            EndpointDecl "out"
              TypeName "int"
        "#);
    }

    #[test]
    fn processor_latency_assignment_is_not_a_container_decl() {
        insta::assert_snapshot!(parse_stmt("processor.latency = length;"), @r#"
        ExprStmt
          Assign "="
            Field "latency"
              processor
            length
        "#);
    }

    #[test]
    fn ambiguous_angle_brackets_expression() {
        insta::assert_snapshot!(parse_expr("a<b>c"), @r#"
        Binary ">"
          Binary "<"
            a
            b
          c
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
          VarDeclStmt "i"
            TypeName "int"
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
}
