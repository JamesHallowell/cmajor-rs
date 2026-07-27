use crate::{
    ast::{Ast, Node, NodeId, SpecialisationParamKind},
    lexer::{
        tokenize, Keyword, Literal, NonTrivialTokenStreamIterator, TokenId, TokenKind, TokenStream,
    },
    parser::precedence::{BindingPower, InfixBindingPower, PrecedenceLevel},
    utils, Diagnostic,
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

fn is_type_name_start(kind: TokenKind) -> bool {
    kind == TokenKind::Identifier || kind.is_type()
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
            TokenKind::Equal => Self::Assign,
            TokenKind::ArrowLeft => Self::Write,
            TokenKind::PlusEqual => Self::AddAssign,
            TokenKind::MinusEqual => Self::SubtractAssign,
            TokenKind::StarEqual => Self::MultiplyAssign,
            TokenKind::SlashEqual => Self::DivideAssign,
            TokenKind::PercentEqual => Self::RemainderAssign,
            TokenKind::AmpersandEqual => Self::BitwiseAndAssign,
            TokenKind::PipeEqual => Self::BitwiseOrAssign,
            TokenKind::CaretEqual => Self::BitwiseXorAssign,
            TokenKind::ShiftLeftEqual => Self::ShiftLeftAssign,
            TokenKind::ShiftRightEqual => Self::ShiftRightAssign,
            TokenKind::ShiftRightShiftRightEqual => Self::UnsignedShiftRightAssign,
            TokenKind::AmpersandAmpersandEqual => Self::LogicalAndAssign,
            TokenKind::PipePipeEqual => Self::LogicalOrAssign,
            TokenKind::PipePipe => Self::Or,
            TokenKind::AmpersandAmpersand => Self::And,
            TokenKind::Pipe => Self::BitwiseOr,
            TokenKind::Caret => Self::BitwiseXor,
            TokenKind::Ampersand => Self::BitwiseAnd,
            TokenKind::EqualEqual => Self::Equal,
            TokenKind::BangEqual => Self::NotEqual,
            TokenKind::AngleBracketLeft => Self::LessThan,
            TokenKind::AngleBracketLeftEqual => Self::LessThanOrEqual,
            TokenKind::AngleBracketRight => Self::GreaterThan,
            TokenKind::AngleBracketRightEqual => Self::GreaterThanOrEqual,
            TokenKind::ShiftLeft => Self::ShiftLeft,
            TokenKind::ShiftRight => Self::ShiftRight,
            TokenKind::ShiftRightShiftRight => Self::UnsignedShiftRight,
            TokenKind::Plus => Self::Add,
            TokenKind::Minus => Self::Subtract,
            TokenKind::Star => Self::Multiply,
            TokenKind::Slash => Self::Divide,
            TokenKind::Percent => Self::Remainder,
            TokenKind::StarStar => Self::Power,
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
        if kind == TokenKind::Question {
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
            Some(
                TokenKind::Bang
                | TokenKind::Tilde
                | TokenKind::Minus
                | TokenKind::PlusPlus
                | TokenKind::MinusMinus,
            ) => {
                let op = self.bump();
                let operand = self.parse_expr_with_min_binding_power(PrecedenceLevel::Unary.base());
                self.ast.push(Node::Unary { op, operand })
            }
            Some(TokenKind::Literal(literal)) => self.parse_literal(literal),
            Some(TokenKind::Keyword(Keyword::True | Keyword::False)) => {
                let token = self.bump();
                self.ast.push(Node::BoolLiteral { token })
            }
            Some(TokenKind::Identifier | TokenKind::Keyword(Keyword::Processor)) => {
                let token = self.bump();
                self.ast.push(Node::Ident { token })
            }
            Some(TokenKind::ParenthesisLeft) => {
                let paren = self.expect(TokenKind::ParenthesisLeft);
                let mut inner = vec![];
                if self.tokens.peek_kind() != Some(TokenKind::ParenthesisRight) {
                    loop {
                        inner.push(self.parse_expr());
                        if self.tokens.peek_kind() == Some(TokenKind::Comma) {
                            self.bump();
                        } else {
                            break;
                        }
                    }
                }
                self.expect(TokenKind::ParenthesisRight);
                self.ast.push(Node::Parentheses { paren, inner })
            }
            Some(token) if token.is_type() => {
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
                Some(TokenKind::ParenthesisLeft) => self.parse_call(lhs),
                Some(TokenKind::BracketLeft) => self.parse_index(lhs),
                Some(TokenKind::Dot) => self.parse_field(lhs),
                Some(TokenKind::ColonColon) => self.parse_scope_access(lhs),
                Some(TokenKind::AngleBracketLeft) if self.at_vector_construct_chevron() => {
                    self.parse_chevroned_suffix(lhs)
                }
                Some(TokenKind::PlusPlus | TokenKind::MinusMinus) => {
                    let op = self.bump();
                    self.ast.push(Node::PostfixUnary { op, operand: lhs })
                }
                _ => break,
            };
        }

        while let Some(token) = self.tokens.peek() {
            let Some(infix) = Infix::from_token(token.kind) else {
                break;
            };
            let binding_power = infix.binding_power();
            if binding_power.binds_at < min_binding_power {
                break;
            }

            lhs = match infix {
                Infix::Ternary => {
                    let question = self.bump();
                    let then_branch = self.parse_expr();
                    self.expect(TokenKind::Colon);
                    let else_branch =
                        self.parse_expr_with_min_binding_power(binding_power.min_for_rhs);
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

    fn at_vector_construct_chevron(&self) -> bool {
        self.skip_to_matching_angle_bracket(0)
            .is_some_and(|after| self.at(after, TokenKind::ParenthesisLeft))
    }

    fn parse_scope_access(&mut self, base: NodeId) -> NodeId {
        self.expect(TokenKind::ColonColon);
        let name = self.expect(TokenKind::Identifier);
        self.ast.push(Node::ScopeAccess { name, base })
    }

    fn parse_call(&mut self, callee: NodeId) -> NodeId {
        let paren = self.expect(TokenKind::ParenthesisLeft);
        let mut args = Vec::new();
        if self.tokens.peek_kind() != Some(TokenKind::ParenthesisRight) {
            loop {
                args.push(self.parse_expr());
                if self.tokens.peek_kind() == Some(TokenKind::Comma) {
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
        let bracket = self.expect(TokenKind::BracketLeft);
        let index = if self.tokens.peek_kind() != Some(TokenKind::BracketRight) {
            Some(self.parse_expr())
        } else {
            None
        };
        self.expect(TokenKind::BracketRight);
        self.ast.push(Node::Index {
            bracket,
            base,
            index,
        })
    }

    fn parse_field(&mut self, base: NodeId) -> NodeId {
        self.expect(TokenKind::Dot);
        let name = self.expect(TokenKind::Identifier);
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
            Some(TokenKind::BraceLeft) => self.parse_block(),
            Some(TokenKind::Keyword(Keyword::Let)) => self.parse_let(),
            Some(TokenKind::Keyword(Keyword::Var)) => self.parse_var(),
            Some(TokenKind::Keyword(Keyword::If)) => self.parse_if(),
            Some(TokenKind::Keyword(Keyword::While)) => self.parse_while(),
            Some(TokenKind::Keyword(Keyword::Loop)) => self.parse_loop(),
            Some(TokenKind::Keyword(Keyword::For)) => self.parse_for(),
            Some(TokenKind::Keyword(Keyword::Node)) => self.parse_node_decl(),
            Some(TokenKind::Keyword(Keyword::Connection)) => self.parse_connection_decl(),
            Some(TokenKind::Keyword(Keyword::Return)) => self.parse_return(),
            Some(TokenKind::Keyword(Keyword::Break)) => self.parse_break(),
            Some(TokenKind::Keyword(Keyword::Continue)) => self.parse_continue(),
            Some(TokenKind::Keyword(Keyword::Namespace)) => self.parse_namespace(),
            Some(TokenKind::Keyword(Keyword::Processor))
                if self.tokens.peek_nth(1).map(|token| token.kind) == Some(TokenKind::Dot) =>
            {
                self.parse_expr_stmt()
            }
            Some(TokenKind::Keyword(Keyword::Processor | Keyword::Graph | Keyword::Struct)) => {
                self.parse_container()
            }
            Some(TokenKind::Keyword(Keyword::Input | Keyword::Output)) => {
                self.parse_endpoint_group()
            }
            Some(TokenKind::Keyword(Keyword::Event)) => self.parse_event_handler(),
            Some(TokenKind::Keyword(Keyword::Const)) => self.parse_typed_decl(),
            Some(TokenKind::Keyword(keyword))
                if is_type_name_start(TokenKind::Keyword(keyword)) =>
            {
                self.parse_typed_decl()
            }
            Some(TokenKind::Identifier) if self.looks_like_typed_decl() => self.parse_typed_decl(),
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_break(&mut self) -> NodeId {
        let keyword = self.expect(Keyword::Break);
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::BreakStmt { keyword })
    }

    fn parse_continue(&mut self) -> NodeId {
        let keyword = self.expect(Keyword::Continue);
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::ContinueStmt { keyword })
    }

    fn parse_typed_decl(&mut self) -> NodeId {
        self.parse_typed_decl_inner(true)
    }

    fn parse_typed_decl_inner(&mut self, consume_semicolon: bool) -> NodeId {
        let ty = self.parse_type();
        let name = self.expect(TokenKind::Identifier);
        if self.tokens.peek_kind() == Some(TokenKind::AngleBracketLeft)
            || self.tokens.peek_kind() == Some(TokenKind::ParenthesisLeft)
        {
            let generics = self.parse_optional_generics();
            self.parse_function_decl(ty, name, generics)
        } else {
            let mut declarators = vec![self.parse_declarator(name)];
            while self.tokens.peek_kind() == Some(TokenKind::Comma) {
                self.bump();
                let name = self.expect(TokenKind::Identifier);
                declarators.push(self.parse_declarator(name));
            }
            if consume_semicolon {
                self.expect(TokenKind::Semicolon);
            }
            self.ast.push(Node::VarDeclStmt { ty, declarators })
        }
    }

    fn parse_declarator(&mut self, name: TokenId) -> (TokenId, Option<NodeId>) {
        let init = if self.tokens.peek_kind() == Some(TokenKind::Equal) {
            self.bump();
            Some(self.parse_expr())
        } else {
            None
        };
        (name, init)
    }

    fn parse_optional_generics(&mut self) -> Vec<TokenId> {
        if self.tokens.peek_kind() != Some(TokenKind::AngleBracketLeft) {
            return Vec::new();
        }
        self.bump();
        let mut generics = vec![self.expect(TokenKind::Identifier)];
        while self.tokens.peek_kind() == Some(TokenKind::Comma) {
            self.bump();
            generics.push(self.expect(TokenKind::Identifier));
        }
        self.expect(TokenKind::AngleBracketRight);
        generics
    }

    fn parse_params(&mut self) -> Vec<NodeId> {
        self.expect(TokenKind::ParenthesisLeft);
        let mut params = Vec::new();
        if self.tokens.peek_kind() != Some(TokenKind::ParenthesisRight) {
            loop {
                let ty = self.parse_type();
                let by_ref = if self.tokens.peek_kind() == Some(TokenKind::Ampersand) {
                    self.bump();
                    true
                } else {
                    false
                };
                let name = self.expect(TokenKind::Identifier);
                params.push(self.ast.push(Node::Param { ty, name, by_ref }));
                if self.tokens.peek_kind() == Some(TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::ParenthesisRight);
        params
    }

    fn parse_function_decl(&mut self, ty: NodeId, name: TokenId, generics: Vec<TokenId>) -> NodeId {
        let params = self.parse_params();
        let body = self.parse_block();
        self.ast.push(Node::FunctionDecl {
            ty,
            name,
            generics,
            params,
            body,
        })
    }

    fn parse_event_handler(&mut self) -> NodeId {
        let keyword = self.expect(Keyword::Event);
        let name = self.expect(TokenKind::Identifier);
        let params = self.parse_params();
        let body = self.parse_block();
        self.ast.push(Node::EventHandlerDecl {
            keyword,
            name,
            params,
            body,
        })
    }

    fn parse_namespace(&mut self) -> NodeId {
        let keyword = self.expect(Keyword::Namespace);
        let mut segments = vec![self.expect(TokenKind::Identifier)];
        while self.tokens.peek_kind() == Some(TokenKind::ColonColon) {
            self.bump();
            segments.push(self.expect(TokenKind::Identifier));
        }
        let params = self.parse_optional_specialisation_params();
        self.expect(TokenKind::BraceLeft);
        let mut items = Vec::new();
        while !matches!(self.tokens.peek_kind(), Some(TokenKind::BraceRight) | None) {
            items.push(self.parse_statement());
        }
        self.expect(TokenKind::BraceRight);
        self.ast.push(Node::NamespaceDecl {
            keyword,
            segments,
            params,
            items,
        })
    }

    fn parse_optional_specialisation_params(&mut self) -> Vec<NodeId> {
        if self.tokens.peek_kind() != Some(TokenKind::ParenthesisLeft) {
            return Vec::new();
        }
        self.bump();
        let mut params = Vec::new();
        if self.tokens.peek_kind() != Some(TokenKind::ParenthesisRight) {
            loop {
                params.push(self.parse_specialisation_param());
                if self.tokens.peek_kind() == Some(TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::ParenthesisRight);
        params
    }

    fn parse_specialisation_param(&mut self) -> NodeId {
        let kind = match self.tokens.peek_kind() {
            Some(TokenKind::Keyword(Keyword::Using)) => {
                self.bump();
                SpecialisationParamKind::Using
            }
            Some(TokenKind::Keyword(Keyword::Processor)) => {
                self.bump();
                SpecialisationParamKind::Processor
            }
            Some(TokenKind::Keyword(Keyword::Namespace)) => {
                self.bump();
                SpecialisationParamKind::Namespace
            }
            _ => {
                let ty = self.parse_type();
                SpecialisationParamKind::Value { ty }
            }
        };
        let name = self.expect(TokenKind::Identifier);
        let default = if self.tokens.peek_kind() == Some(TokenKind::Equal) {
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
        let keyword_kind = self.tokens.peek_kind();
        let keyword = self.bump();
        let name = self.expect(TokenKind::Identifier);
        let params = self.parse_optional_specialisation_params();
        let attributes = if self.tokens.peek_kind() == Some(TokenKind::DoubleBracketLeft) {
            Some(self.parse_attribute_list())
        } else {
            None
        };
        self.expect(TokenKind::BraceLeft);
        let mut items = Vec::new();
        while !matches!(self.tokens.peek_kind(), Some(TokenKind::BraceRight) | None) {
            items.push(self.parse_statement());
        }
        self.expect(TokenKind::BraceRight);
        match keyword_kind {
            Some(TokenKind::Keyword(Keyword::Graph)) => self.ast.push(Node::GraphDecl {
                keyword,
                name,
                params,
                attributes,
                items,
            }),
            Some(TokenKind::Keyword(Keyword::Struct)) => self.ast.push(Node::StructDecl {
                keyword,
                name,
                attributes,
                items,
            }),
            _ => self.ast.push(Node::ProcessorDecl {
                keyword,
                name,
                params,
                attributes,
                items,
            }),
        }
    }

    fn parse_node_decl(&mut self) -> NodeId {
        let keyword = self.expect(Keyword::Node);
        let name = self.expect(TokenKind::Identifier);
        self.expect(TokenKind::Equal);
        let value = self.parse_expr();
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::NodeDecl {
            keyword,
            name,
            value,
        })
    }

    fn parse_connection_decl(&mut self) -> NodeId {
        let keyword = self.expect(Keyword::Connection);
        let mut links = Vec::new();
        loop {
            let mut chain = vec![self.parse_expr()];
            while self.tokens.peek_kind() == Some(TokenKind::ArrowRight) {
                self.bump();
                chain.push(self.parse_expr());
            }
            links.push(chain);
            if self.tokens.peek_kind() == Some(TokenKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::ConnectionDecl { keyword, links })
    }

    fn parse_for(&mut self) -> NodeId {
        let keyword = self.expect(Keyword::For);
        self.expect(TokenKind::ParenthesisLeft);

        let init = if self.tokens.peek_kind() == Some(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_for_init())
        };

        if self.tokens.peek_kind() == Some(TokenKind::ParenthesisRight) {
            self.bump();
            let body = self.parse_statement();
            return self.ast.push(Node::ForStmt {
                keyword,
                init,
                cond: None,
                update: None,
                body,
            });
        }

        self.expect(TokenKind::Semicolon);
        let cond = if self.tokens.peek_kind() == Some(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expr())
        };
        self.expect(TokenKind::Semicolon);
        let update = if self.tokens.peek_kind() == Some(TokenKind::ParenthesisRight) {
            None
        } else {
            Some(self.parse_expr())
        };
        self.expect(TokenKind::ParenthesisRight);
        let body = self.parse_statement();
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
            Some(TokenKind::Keyword(Keyword::Const)) => self.parse_typed_decl_inner(false),
            Some(TokenKind::Keyword(keyword))
                if is_type_name_start(TokenKind::Keyword(keyword)) =>
            {
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
            .peek_nth(offset)
            .is_some_and(|token| token.kind == kind)
    }

    fn looks_like_typed_decl(&self) -> bool {
        let mut offset = 0;
        if !self.at(offset, TokenKind::Identifier) {
            return false;
        }
        offset += 1;
        while self.at(offset, TokenKind::ColonColon) {
            offset += 1;
            if !self.at(offset, TokenKind::Identifier) {
                return false;
            }
            offset += 1;
        }
        loop {
            let next = if self.at(offset, TokenKind::AngleBracketLeft) {
                self.skip_to_matching_angle_bracket(offset)
            } else if self.at(offset, TokenKind::BracketLeft) {
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
            match self.tokens.peek_nth(offset).map(|t| t.kind) {
                Some(TokenKind::AngleBracketRight) => return Some(offset + 1),
                Some(TokenKind::Semicolon | TokenKind::BraceLeft | TokenKind::BraceRight)
                | None => {
                    return None;
                }
                Some(_) => offset += 1,
            }
        }
    }

    fn skip_to_matching_bracket(&self, mut offset: usize) -> Option<usize> {
        let mut depth = 0;
        loop {
            match self.tokens.peek_nth(offset).map(|t| t.kind) {
                Some(TokenKind::BracketLeft) => {
                    depth += 1;
                    offset += 1;
                }
                Some(TokenKind::BracketRight) => {
                    depth -= 1;
                    offset += 1;
                    if depth == 0 {
                        return Some(offset);
                    }
                }
                Some(TokenKind::Semicolon | TokenKind::BraceLeft | TokenKind::BraceRight)
                | None => {
                    return None;
                }
                Some(_) => offset += 1,
            }
        }
    }

    fn parse_attribute_list(&mut self) -> NodeId {
        self.bump();
        let mut attrs = Vec::new();
        if self.tokens.peek_kind() != Some(TokenKind::DoubleBracketRight) {
            loop {
                let key = self.expect(TokenKind::Identifier);
                let value = if self.tokens.peek_kind() == Some(TokenKind::Colon) {
                    self.bump();
                    Some(self.parse_expr())
                } else {
                    None
                };
                attrs.push((key, value));
                if self.tokens.peek_kind() == Some(TokenKind::Comma) {
                    self.bump();
                } else {
                    break;
                }
            }
        }
        self.expect(TokenKind::DoubleBracketRight);
        self.ast.push(Node::AttributeList { attrs })
    }

    fn parse_type_list(&mut self) -> NodeId {
        let paren = self.bump();
        let mut types = vec![self.parse_type()];
        while self.tokens.peek_kind() == Some(TokenKind::Comma) {
            self.bump();
            types.push(self.parse_type());
        }
        self.expect(TokenKind::ParenthesisRight);
        self.ast.push(Node::TypeList { paren, types })
    }

    fn parse_endpoint_member(&mut self) -> NodeId {
        let ty = if self.tokens.peek_kind() == Some(TokenKind::ParenthesisLeft) {
            self.parse_type_list()
        } else {
            self.parse_type()
        };
        let name = self.expect(TokenKind::Identifier);
        let attributes = if self.tokens.peek_kind() == Some(TokenKind::DoubleBracketLeft) {
            Some(self.parse_attribute_list())
        } else {
            None
        };
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::EndpointDecl {
            ty,
            name,
            attributes,
        })
    }

    fn parse_endpoint_group(&mut self) -> NodeId {
        let direction = self.bump();
        if self.tokens.peek_kind() == Some(TokenKind::Identifier) && self.at(1, TokenKind::Dot) {
            let name = self.bump();
            self.bump();
            self.expect(TokenKind::Star);
            self.expect(TokenKind::Semicolon);
            return self.ast.push(Node::EndpointWildcard { direction, name });
        }
        let kind = self.bump();
        let endpoints = if self.tokens.peek_kind() == Some(TokenKind::BraceLeft) {
            self.bump();
            let mut endpoints = Vec::new();
            while !matches!(self.tokens.peek_kind(), Some(TokenKind::BraceRight) | None) {
                endpoints.push(self.parse_endpoint_member());
            }
            self.expect(TokenKind::BraceRight);
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
        let brace = self.bump();

        let mut stmts = Vec::new();
        while !matches!(self.tokens.peek_kind(), Some(TokenKind::BraceRight) | None) {
            stmts.push(self.parse_statement());
        }
        self.expect(TokenKind::BraceRight);
        self.ast.push(Node::Block { brace, stmts })
    }

    fn parse_let(&mut self) -> NodeId {
        self.bump();
        let mut declarators = vec![self.parse_let_declarator()];
        while self.tokens.peek_kind() == Some(TokenKind::Comma) {
            self.bump();
            declarators.push(self.parse_let_declarator());
        }
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::LetStmt { declarators })
    }

    fn parse_let_declarator(&mut self) -> (TokenId, NodeId) {
        let name = self.expect(TokenKind::Identifier);
        self.expect(TokenKind::Equal);
        let init = self.parse_expr();
        (name, init)
    }

    fn parse_var(&mut self) -> NodeId {
        self.bump();
        let name = self.expect(TokenKind::Identifier);
        let init = if self.tokens.peek_kind() == Some(TokenKind::Equal) {
            self.bump();
            Some(self.parse_expr())
        } else {
            None
        };
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::VarStmt { name, init })
    }

    fn parse_if(&mut self) -> NodeId {
        let keyword = self.bump();
        let is_const = if self.tokens.peek_kind() == Some(TokenKind::Keyword(Keyword::Const)) {
            self.bump();
            true
        } else {
            false
        };
        self.expect(TokenKind::ParenthesisLeft);
        let cond = self.parse_expr();
        self.expect(TokenKind::ParenthesisRight);
        let then_branch = self.parse_statement();
        let else_branch = if self.tokens.peek_kind() == Some(TokenKind::Keyword(Keyword::Else)) {
            self.bump();
            Some(self.parse_statement())
        } else {
            None
        };
        self.ast.push(Node::IfStmt {
            keyword,
            is_const,
            cond,
            then_branch,
            else_branch,
        })
    }

    fn parse_while(&mut self) -> NodeId {
        let keyword = self.expect(Keyword::While);
        self.expect(TokenKind::ParenthesisLeft);
        let cond = self.parse_expr();
        self.expect(TokenKind::ParenthesisRight);
        let body = self.parse_statement();
        self.ast.push(Node::WhileStmt {
            keyword,
            cond,
            body,
        })
    }

    fn parse_loop(&mut self) -> NodeId {
        let keyword = self.bump();
        let count = if self.tokens.peek_kind() == Some(TokenKind::ParenthesisLeft) {
            self.bump();
            let count = self.parse_expr();
            self.expect(TokenKind::ParenthesisRight);
            Some(count)
        } else {
            None
        };
        let body = self.parse_statement();
        self.ast.push(Node::LoopStmt {
            keyword,
            count,
            body,
        })
    }

    fn parse_return(&mut self) -> NodeId {
        let keyword = self.bump();
        let value = if self.tokens.peek_kind() == Some(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expr())
        };
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::ReturnStmt { keyword, value })
    }

    fn parse_expr_stmt(&mut self) -> NodeId {
        let expr = self.parse_expr();
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::ExprStmt { expr })
    }

    fn parse_type(&mut self) -> NodeId {
        if let Some(keyword) = self.bump_if(Keyword::Const) {
            let inner = self.parse_type();
            return self.ast.push(Node::ConstType { keyword, inner });
        }

        let mut ty = match self.tokens.peek_kind() {
            Some(kind) if is_type_name_start(kind) => self.parse_type_name(),
            peeked => {
                let message = format!("expected type, found {peeked:?}");
                let token = self.bump();
                self.error(token, message);
                self.ast.push(Node::Error { token })
            }
        };

        loop {
            ty = match self.tokens.peek_kind() {
                Some(TokenKind::BracketLeft) => self.parse_array(ty),
                Some(TokenKind::AngleBracketLeft) => self.parse_chevroned_suffix(ty),
                _ => break,
            };
        }

        ty
    }

    fn parse_type_name(&mut self) -> NodeId {
        let mut segments = vec![self.bump()];
        while self.tokens.peek_kind() == Some(TokenKind::ColonColon) {
            self.bump();
            segments.push(self.expect(TokenKind::Identifier));
        }
        self.ast.push(Node::TypeName { segments })
    }

    fn parse_array(&mut self, element: NodeId) -> NodeId {
        let bracket = self.bump();
        let size = if self.tokens.peek_kind() == Some(TokenKind::BracketRight) {
            None
        } else {
            Some(self.parse_expr())
        };
        self.expect(TokenKind::BracketRight);
        self.ast.push(Node::Array {
            bracket,
            element,
            size,
        })
    }

    fn parse_chevroned_suffix(&mut self, element: NodeId) -> NodeId {
        let angle = self.bump();
        let term = self.parse_expr_with_min_binding_power(PrecedenceLevel::Shift.base());
        self.expect(TokenKind::AngleBracketRight);
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
          Paren
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
        insta::assert_snapshot!(parse_expr("a < b > c"), @r#"
        Binary ">"
          Binary "<"
            a
            b
          c
        "#);
    }
}
