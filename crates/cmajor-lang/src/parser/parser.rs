use {
    crate::{
        ast::{Ast, Node, NodeId, SpecialisationParamKind},
        lexer::{
            Keyword, Literal, Token, TokenId,
            TokenKind::{self},
            TokenStream,
        },
    },
    bp::{BindingPower, InfixBindingPower, PrecedenceLevel},
};

pub struct Parse {
    pub ast: Ast,
    pub root: NodeId,
    pub tokens: TokenStream,
}

pub fn parse(source: &str) -> Parse {
    let mut parser = Parser::new(TokenStream::tokenize(source));
    let root = parser.parse();
    Parse {
        ast: parser.ast,
        root,
        tokens: parser.tokens,
    }
}

fn is_type_name_start(kind: TokenKind) -> bool {
    kind == TokenKind::Identifier || kind.is_type()
}

mod bp {
    #[derive(Debug, Copy, Clone, Eq, PartialEq, Hash, Ord, PartialOrd)]
    pub struct BindingPower(u8);

    #[derive(Debug, Copy, Clone)]
    pub struct InfixBindingPower {
        pub binds_at: BindingPower,
        pub min_for_rhs: BindingPower,
    }

    #[derive(Debug, Copy, Clone, Eq, PartialEq)]
    pub enum PrecedenceLevel {
        Assign,
        Ternary,
        Or,
        And,
        BitwiseOr,
        BitwiseXor,
        BitwiseAnd,
        Equality,
        Relational,
        Shift,
        Additive,
        Multiplicative,
        Power,
        Unary,
    }

    impl PrecedenceLevel {
        pub const fn lowest() -> BindingPower {
            BindingPower(0)
        }

        pub const fn base(self) -> BindingPower {
            BindingPower(2 * (self as u8 + 1))
        }

        pub const fn left_associative(self) -> InfixBindingPower {
            let base = self.base();
            InfixBindingPower {
                binds_at: base,
                min_for_rhs: BindingPower(base.0 + 1),
            }
        }

        pub const fn right_associative(self) -> InfixBindingPower {
            let base = self.base();
            InfixBindingPower {
                binds_at: BindingPower(base.0 + 1),
                min_for_rhs: base,
            }
        }
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
            TokenKind::LessThan => Self::LessThan,
            TokenKind::LessThanOrEqual => Self::LessThanOrEqual,
            TokenKind::GreaterThan => Self::GreaterThan,
            TokenKind::GreaterThanOrEqual => Self::GreaterThanOrEqual,
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

fn is_prefix_op(kind: TokenKind) -> bool {
    use TokenKind::*;
    matches!(kind, Bang | Tilde | Minus | PlusPlus | MinusMinus)
}

struct Parser {
    tokens: TokenStream,
    ast: Ast,
    pos: u32,
}

impl Parser {
    fn new(tokens: TokenStream) -> Parser {
        Parser {
            tokens,
            ast: Ast::new(),
            pos: 0,
        }
    }

    fn peek(&self) -> Option<Token> {
        self.tokens.token(TokenId(self.pos))
    }

    fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|token| token.kind)
    }

    fn peek_kind_at(&self, offset: u32) -> Option<TokenKind> {
        self.tokens.token(TokenId(self.pos + offset)).map(|token| token.kind)
    }

    fn bump(&mut self) -> TokenId {
        let id = TokenId(self.pos);
        if (self.pos as usize) < self.tokens.len() {
            self.pos += 1;
        }
        id
    }

    fn expect(&mut self, kind: TokenKind) -> TokenId {
        if self.peek_kind() == Some(kind) {
            self.bump()
        } else {
            TokenId(self.pos)
        }
    }

    fn at_keyword(&self, keyword: Keyword) -> bool {
        self.peek_kind() == Some(TokenKind::Keyword(keyword))
    }

    pub fn parse(&mut self) -> NodeId {
        let mut stmts = Vec::new();
        while self.peek().is_some() {
            stmts.push(self.parse_statement());
        }
        let root = self.ast.push(Node::Block {
            brace: TokenId(0),
            stmts,
        });
        root
    }

    fn parse_expr(&mut self) -> NodeId {
        self.parse_expr_with_min_binding_power(PrecedenceLevel::lowest())
    }

    fn parse_expr_with_min_binding_power(&mut self, min_binding_power: BindingPower) -> NodeId {
        let mut lhs = self.parse_prefix();

        while let Some(token) = self.peek() {
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

    fn parse_prefix(&mut self) -> NodeId {
        if self.peek_kind().is_some_and(is_prefix_op) {
            let op = self.bump();
            let operand = self.parse_expr_with_min_binding_power(PrecedenceLevel::Unary.base());
            self.ast.push(Node::Unary { op, operand })
        } else {
            self.parse_postfix()
        }
    }

    fn parse_postfix(&mut self) -> NodeId {
        let mut expr = self.parse_primary();

        loop {
            expr = match self.peek_kind() {
                Some(TokenKind::ParenthesisLeft) => self.parse_call(expr),
                Some(TokenKind::BracketLeft) => self.parse_index(expr),
                Some(TokenKind::Dot) => self.parse_field(expr),
                Some(TokenKind::ColonColon) => self.parse_scope_access(expr),
                Some(TokenKind::LessThan) if self.at_vector_construct_chevron() => {
                    self.parse_chevroned_suffix(expr)
                }
                Some(TokenKind::PlusPlus | TokenKind::MinusMinus) => {
                    let op = self.bump();
                    self.ast.push(Node::PostfixUnary { op, operand: expr })
                }
                _ => break,
            };
        }

        expr
    }

    fn at_vector_construct_chevron(&self) -> bool {
        self.skip_to_matching_angle_bracket(0)
            .is_some_and(|after| self.at(after, TokenKind::ParenthesisLeft))
    }

    fn parse_scope_access(&mut self, base: NodeId) -> NodeId {
        self.bump();
        let name = self.expect(TokenKind::Identifier);
        self.ast.push(Node::ScopeAccess { name, base })
    }

    fn parse_call(&mut self, callee: NodeId) -> NodeId {
        let paren = self.bump();
        let mut args = Vec::new();
        if self.peek_kind() != Some(TokenKind::ParenthesisRight) {
            loop {
                args.push(self.parse_expr());
                if self.peek_kind() == Some(TokenKind::Comma) {
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
        let index = self.parse_expr();
        self.expect(TokenKind::BracketRight);
        self.ast.push(Node::Index {
            bracket,
            base,
            index,
        })
    }

    fn parse_field(&mut self, base: NodeId) -> NodeId {
        self.bump();
        let name = self.expect(TokenKind::Identifier);
        self.ast.push(Node::Field { name, base })
    }

    fn parse_literal(&mut self, literal: Literal) -> NodeId {
        match literal {
            Literal::Int32 | Literal::Int64 => {
                let token = self.bump();
                self.ast.push(Node::IntLiteral { token })
            }
            Literal::Float32 | Literal::Float64 => {
                let token = self.bump();
                self.ast.push(Node::FloatLiteral { token })
            }
            Literal::Imaginary32 | Literal::Imaginary64 => {
                let token = self.bump();
                self.ast.push(Node::ImaginaryLiteral { token })
            }
            Literal::String => {
                let token = self.bump();
                self.ast.push(Node::StringLiteral { token })
            }
        }
    }

    fn parse_primary(&mut self) -> NodeId {
        match self.peek_kind() {
            Some(TokenKind::Literal(literal)) => self.parse_literal(literal),
            Some(TokenKind::Keyword(Keyword::True | Keyword::False)) => {
                let token = self.bump();
                self.ast.push(Node::BoolLiteral { token })
            }
            Some(TokenKind::Identifier) => {
                let token = self.bump();
                self.ast.push(Node::Ident { token })
            }
            Some(kind) if kind == TokenKind::Keyword(Keyword::Processor) || kind.is_type() => {
                let token = self.bump();
                self.ast.push(Node::Ident { token })
            }
            Some(TokenKind::ParenthesisLeft) => {
                let paren = self.bump();
                let inner = self.parse_expr();
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
        match self.peek_kind() {
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
            Some(TokenKind::Keyword(Keyword::Break)) => {
                let keyword = self.bump();
                self.expect(TokenKind::Semicolon);
                self.ast.push(Node::BreakStmt { keyword })
            }
            Some(TokenKind::Keyword(Keyword::Continue)) => {
                let keyword = self.bump();
                self.expect(TokenKind::Semicolon);
                self.ast.push(Node::ContinueStmt { keyword })
            }
            Some(TokenKind::Keyword(Keyword::Namespace)) => self.parse_namespace(),
            Some(TokenKind::Keyword(Keyword::Processor))
                if self.peek_kind_at(1) == Some(TokenKind::Dot) =>
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
            Some(TokenKind::Keyword(Keyword::Const)) => {
                self.bump();
                self.parse_typed_decl(true)
            }
            Some(TokenKind::Keyword(keyword))
                if is_type_name_start(TokenKind::Keyword(keyword)) =>
            {
                self.parse_typed_decl(false)
            }
            Some(TokenKind::Identifier) if self.looks_like_typed_decl() => {
                self.parse_typed_decl(false)
            }
            _ => self.parse_expr_stmt(),
        }
    }

    fn parse_typed_decl(&mut self, is_const: bool) -> NodeId {
        self.parse_typed_decl_inner(is_const, true)
    }

    fn parse_typed_decl_inner(&mut self, is_const: bool, consume_semicolon: bool) -> NodeId {
        let ty = self.parse_type();
        let name = self.expect(TokenKind::Identifier);
        if self.peek_kind() == Some(TokenKind::LessThan)
            || self.peek_kind() == Some(TokenKind::ParenthesisLeft)
        {
            let generics = self.parse_optional_generics();
            self.parse_function_decl(ty, name, generics)
        } else {
            let mut declarators = vec![self.parse_declarator(name)];
            while self.peek_kind() == Some(TokenKind::Comma) {
                self.bump();
                let name = self.expect(TokenKind::Identifier);
                declarators.push(self.parse_declarator(name));
            }
            if consume_semicolon {
                self.expect(TokenKind::Semicolon);
            }
            self.ast.push(Node::VarDeclStmt {
                ty,
                declarators,
                is_const,
            })
        }
    }

    fn parse_declarator(&mut self, name: TokenId) -> (TokenId, Option<NodeId>) {
        let init = if self.peek_kind() == Some(TokenKind::Equal) {
            self.bump();
            Some(self.parse_expr())
        } else {
            None
        };
        (name, init)
    }

    fn parse_optional_generics(&mut self) -> Vec<TokenId> {
        if self.peek_kind() != Some(TokenKind::LessThan) {
            return Vec::new();
        }
        self.bump();
        let mut generics = vec![self.expect(TokenKind::Identifier)];
        while self.peek_kind() == Some(TokenKind::Comma) {
            self.bump();
            generics.push(self.expect(TokenKind::Identifier));
        }
        self.expect(TokenKind::GreaterThan);
        generics
    }

    fn parse_params(&mut self) -> Vec<NodeId> {
        self.expect(TokenKind::ParenthesisLeft);
        let mut params = Vec::new();
        if self.peek_kind() != Some(TokenKind::ParenthesisRight) {
            loop {
                let ty = self.parse_type();
                let by_ref = if self.peek_kind() == Some(TokenKind::Ampersand) {
                    self.bump();
                    true
                } else {
                    false
                };
                let name = self.expect(TokenKind::Identifier);
                params.push(self.ast.push(Node::Param { ty, name, by_ref }));
                if self.peek_kind() == Some(TokenKind::Comma) {
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
        let keyword = self.bump();
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
        let keyword = self.bump();
        let mut segments = vec![self.expect(TokenKind::Identifier)];
        while self.peek_kind() == Some(TokenKind::ColonColon) {
            self.bump();
            segments.push(self.expect(TokenKind::Identifier));
        }
        let params = self.parse_optional_specialisation_params();
        self.expect(TokenKind::BraceLeft);
        let mut items = Vec::new();
        while !matches!(self.peek_kind(), Some(TokenKind::BraceRight) | None) {
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
        if self.peek_kind() != Some(TokenKind::ParenthesisLeft) {
            return Vec::new();
        }
        self.bump();
        let mut params = Vec::new();
        if self.peek_kind() != Some(TokenKind::ParenthesisRight) {
            loop {
                params.push(self.parse_specialisation_param());
                if self.peek_kind() == Some(TokenKind::Comma) {
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
        let kind = match self.peek_kind() {
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
        let default = if self.peek_kind() == Some(TokenKind::Equal) {
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
        let keyword_kind = self.peek_kind();
        let keyword = self.bump();
        let name = self.expect(TokenKind::Identifier);
        let params = self.parse_optional_specialisation_params();
        let attributes = if self.peek_kind() == Some(TokenKind::DoubleBracketLeft) {
            Some(self.parse_attribute_list())
        } else {
            None
        };
        self.expect(TokenKind::BraceLeft);
        let mut items = Vec::new();
        while !matches!(self.peek_kind(), Some(TokenKind::BraceRight) | None) {
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
        let keyword = self.bump();
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
        let keyword = self.bump();
        let mut links = Vec::new();
        loop {
            let mut chain = vec![self.parse_expr()];
            while self.peek_kind() == Some(TokenKind::ArrowRight) {
                self.bump();
                chain.push(self.parse_expr());
            }
            links.push(chain);
            if self.peek_kind() == Some(TokenKind::Comma) {
                self.bump();
            } else {
                break;
            }
        }
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::ConnectionDecl { keyword, links })
    }

    fn parse_for(&mut self) -> NodeId {
        let keyword = self.bump();
        self.expect(TokenKind::ParenthesisLeft);

        let init = if self.peek_kind() == Some(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_for_init())
        };

        if self.peek_kind() == Some(TokenKind::ParenthesisRight) {
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
        let cond = if self.peek_kind() == Some(TokenKind::Semicolon) {
            None
        } else {
            Some(self.parse_expr())
        };
        self.expect(TokenKind::Semicolon);
        let update = if self.peek_kind() == Some(TokenKind::ParenthesisRight) {
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
        match self.peek_kind() {
            Some(TokenKind::Keyword(Keyword::Const)) => {
                self.bump();
                self.parse_typed_decl_inner(true, false)
            }
            Some(TokenKind::Keyword(keyword))
                if is_type_name_start(TokenKind::Keyword(keyword)) =>
            {
                self.parse_typed_decl_inner(false, false)
            }
            Some(TokenKind::Identifier) if self.looks_like_typed_decl() => {
                self.parse_typed_decl_inner(false, false)
            }
            _ => {
                let expr = self.parse_expr();
                self.ast.push(Node::ExprStmt { expr })
            }
        }
    }

    fn at(&self, offset: u32, kind: TokenKind) -> bool {
        self.tokens
            .token(TokenId(self.pos + offset))
            .is_some_and(|token| token.kind == kind)
    }

    fn looks_like_typed_decl(&self) -> bool {
        let mut offset = 0u32;
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
            let next = if self.at(offset, TokenKind::LessThan) {
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

    fn skip_to_matching_angle_bracket(&self, mut offset: u32) -> Option<u32> {
        offset += 1;
        loop {
            match self
                .tokens
                .token(TokenId(self.pos + offset))
                .map(|t| t.kind)
            {
                Some(TokenKind::GreaterThan) => return Some(offset + 1),
                Some(TokenKind::Semicolon | TokenKind::BraceLeft | TokenKind::BraceRight)
                | None => {
                    return None;
                }
                Some(_) => offset += 1,
            }
        }
    }

    fn skip_to_matching_bracket(&self, mut offset: u32) -> Option<u32> {
        let mut depth = 0i32;
        loop {
            match self
                .tokens
                .token(TokenId(self.pos + offset))
                .map(|t| t.kind)
            {
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
        if self.peek_kind() != Some(TokenKind::DoubleBracketRight) {
            loop {
                let key = self.expect(TokenKind::Identifier);
                let value = if self.peek_kind() == Some(TokenKind::Colon) {
                    self.bump();
                    Some(self.parse_expr())
                } else {
                    None
                };
                attrs.push((key, value));
                if self.peek_kind() == Some(TokenKind::Comma) {
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
        while self.peek_kind() == Some(TokenKind::Comma) {
            self.bump();
            types.push(self.parse_type());
        }
        self.expect(TokenKind::ParenthesisRight);
        self.ast.push(Node::TypeList { paren, types })
    }

    fn parse_endpoint_member(&mut self) -> NodeId {
        let ty = if self.peek_kind() == Some(TokenKind::ParenthesisLeft) {
            self.parse_type_list()
        } else {
            self.parse_type()
        };
        let name = self.expect(TokenKind::Identifier);
        let attributes = if self.peek_kind() == Some(TokenKind::DoubleBracketLeft) {
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
        if self.peek_kind() == Some(TokenKind::Identifier) && self.at(1, TokenKind::Dot) {
            let name = self.bump();
            self.bump();
            self.expect(TokenKind::Star);
            self.expect(TokenKind::Semicolon);
            return self.ast.push(Node::EndpointWildcard { direction, name });
        }
        let kind = self.bump();
        let endpoints = if self.peek_kind() == Some(TokenKind::BraceLeft) {
            self.bump();
            let mut endpoints = Vec::new();
            while !matches!(self.peek_kind(), Some(TokenKind::BraceRight) | None) {
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
        while !matches!(self.peek_kind(), Some(TokenKind::BraceRight) | None) {
            stmts.push(self.parse_statement());
        }
        self.expect(TokenKind::BraceRight);
        self.ast.push(Node::Block { brace, stmts })
    }

    fn parse_let(&mut self) -> NodeId {
        self.bump();
        let mut declarators = vec![self.parse_let_declarator()];
        while self.peek_kind() == Some(TokenKind::Comma) {
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
        let init = if self.peek_kind() == Some(TokenKind::Equal) {
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
        self.expect(TokenKind::ParenthesisLeft);
        let cond = self.parse_expr();
        self.expect(TokenKind::ParenthesisRight);
        let then_branch = self.parse_statement();
        let else_branch = if self.at_keyword(Keyword::Else) {
            self.bump();
            Some(self.parse_statement())
        } else {
            None
        };
        self.ast.push(Node::IfStmt {
            keyword,
            cond,
            then_branch,
            else_branch,
        })
    }

    fn parse_while(&mut self) -> NodeId {
        let keyword = self.bump();
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
        let count = if self.peek_kind() == Some(TokenKind::ParenthesisLeft) {
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
        let value = if self.peek_kind() == Some(TokenKind::Semicolon) {
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
        let mut ty = match self.peek_kind() {
            Some(kind) if is_type_name_start(kind) => self.parse_type_name(),
            _ => {
                let token = self.bump();
                self.ast.push(Node::Error { token })
            }
        };

        loop {
            ty = match self.peek_kind() {
                Some(TokenKind::BracketLeft) => self.parse_array(ty),
                Some(TokenKind::LessThan) => self.parse_chevroned_suffix(ty),
                _ => break,
            };
        }

        ty
    }

    fn parse_type_name(&mut self) -> NodeId {
        let mut segments = vec![self.bump()];
        while self.peek_kind() == Some(TokenKind::ColonColon) {
            self.bump();
            segments.push(self.expect(TokenKind::Identifier));
        }
        self.ast.push(Node::TypeName { segments })
    }

    fn parse_array(&mut self, element: NodeId) -> NodeId {
        let bracket = self.bump();
        let size = if self.peek_kind() == Some(TokenKind::BracketRight) {
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
        self.expect(TokenKind::GreaterThan);
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
        let mut parser = Parser::new(TokenStream::tokenize(source));
        let root = parse_fn(&mut parser);
        ast::dump(&parser.ast, &parser.tokens, source, root)
    }

    #[test]
    fn primitive_type() {
        insta::assert_snapshot!(dump("float", Parser::parse_type));
    }

    #[test]
    fn named_type() {
        insta::assert_snapshot!(dump("MyStruct", Parser::parse_type));
    }

    #[test]
    fn qualified_type_name() {
        insta::assert_snapshot!(dump("std::complex64", Parser::parse_type));
    }

    #[test]
    fn array_type() {
        insta::assert_snapshot!(dump("int[3]", Parser::parse_type));
    }

    #[test]
    fn slice_type_has_no_size() {
        insta::assert_snapshot!(dump("int[]", Parser::parse_type));
    }

    #[test]
    fn vector_type() {
        insta::assert_snapshot!(dump("int<4>", Parser::parse_type));
    }

    #[test]
    fn wrap_and_clamp_are_not_keywords_and_parse_as_generic_type_names() {
        insta::assert_snapshot!(dump("clamp<10>", Parser::parse_type));
        insta::assert_snapshot!(dump("wrap<4>", Parser::parse_type));
    }

    #[test]
    fn angle_bracket_close_is_not_a_comparison() {
        insta::assert_snapshot!(dump("wrap<1 + 2>", Parser::parse_type));
    }

    #[test]
    fn postfix_type_modifiers_apply_left_to_right() {
        insta::assert_snapshot!(dump("int<4>[2]", Parser::parse_type));
    }

    #[test]
    fn precedence_of_arithmetic() {
        insta::assert_snapshot!(dump("1 + 2 * 3", Parser::parse_expr));
    }

    #[test]
    fn power_is_right_associative() {
        insta::assert_snapshot!(dump("2 ** 3 ** 4", Parser::parse_expr));
    }

    #[test]
    fn subtraction_is_left_associative() {
        insta::assert_snapshot!(dump("1 - 2 - 3", Parser::parse_expr));
    }

    #[test]
    fn assignment_is_right_associative() {
        insta::assert_snapshot!(dump("a = b = c", Parser::parse_expr));
    }

    #[test]
    fn output_write_operator() {
        insta::assert_snapshot!(dump("out <- in * gain", Parser::parse_expr));
    }

    #[test]
    fn ternary_nests_to_the_right() {
        insta::assert_snapshot!(dump("a ? b : c ? d : e", Parser::parse_expr));
    }

    #[test]
    fn ternary_binds_looser_than_logical_or() {
        insta::assert_snapshot!(dump("a || b ? c : d", Parser::parse_expr));
    }

    #[test]
    fn unary_and_parens() {
        insta::assert_snapshot!(dump("-(1 + 2)", Parser::parse_expr));
    }

    #[test]
    fn prefix_and_postfix_increment() {
        insta::assert_snapshot!(dump("++x", Parser::parse_expr));
        insta::assert_snapshot!(dump("x++", Parser::parse_expr));
        insta::assert_snapshot!(dump("x--", Parser::parse_expr));
    }

    #[test]
    fn call_with_args() {
        insta::assert_snapshot!(dump("foo(1, 2 + 3)", Parser::parse_expr));
    }

    #[test]
    fn call_with_no_args() {
        insta::assert_snapshot!(dump("advance()", Parser::parse_expr));
    }

    #[test]
    fn index_and_field_postfix() {
        insta::assert_snapshot!(dump("x.left[3]", Parser::parse_expr));
    }

    #[test]
    fn let_statement() {
        insta::assert_snapshot!(dump("let x = 1;", Parser::parse_statement));
    }

    #[test]
    fn var_statement() {
        insta::assert_snapshot!(dump("var y;", Parser::parse_statement));
    }

    #[test]
    fn var_with_init_statement() {
        insta::assert_snapshot!(dump("var y = 3;", Parser::parse_statement));
    }

    #[test]
    fn typed_var_decl_statements() {
        insta::assert_snapshot!(dump(
            "wrap<5> w; clamp<5> c; int n = 1;",
            Parser::parse_statement
        ));
    }

    #[test]
    fn if_else_statement() {
        insta::assert_snapshot!(dump("if (a) { b; } else { c; }", Parser::parse_statement));
    }

    #[test]
    fn if_without_else() {
        insta::assert_snapshot!(dump("if (a) { b; }", Parser::parse_statement));
    }

    #[test]
    fn while_and_bounded_loop() {
        insta::assert_snapshot!(dump(
            "while (n > 0) { n = n - 1; } loop (4) { advance(); }",
            Parser::parse_statement
        ));
    }

    #[test]
    fn unbounded_loop_has_no_count() {
        insta::assert_snapshot!(dump("loop { advance(); }", Parser::parse_statement));
    }

    #[test]
    fn return_with_and_without_value() {
        insta::assert_snapshot!(dump("return; return x + 1;", Parser::parse_statement));
    }

    #[test]
    fn break_and_continue() {
        insta::assert_snapshot!(dump("loop { break; continue; }", Parser::parse_statement));
    }

    #[test]
    fn function() {
        insta::assert_snapshot!(dump(
            "int add(int a, int b) { return a + b; }",
            Parser::parse_statement
        ));
    }

    #[test]
    fn loop_with_unbraced_body() {
        insta::assert_snapshot!(dump(
            "void main() { loop advance(); }",
            Parser::parse_statement
        ));
    }

    #[test]
    fn processor_with_typed_specialisation_param() {
        insta::assert_snapshot!(dump(
            "processor SquareWave (int length) { output stream int out; }",
            Parser::parse_statement
        ));
    }

    #[test]
    fn processor_with_typed_specialisation_param_default_value() {
        insta::assert_snapshot!(dump(
            "processor Gain (int channelCount = 2) { output stream int out; }",
            Parser::parse_statement
        ));
    }

    #[test]
    fn processor_with_using_specialisation_param() {
        insta::assert_snapshot!(dump(
            "processor Source (using DataType) { output stream int out; }",
            Parser::parse_statement
        ));
    }

    #[test]
    fn processor_with_using_specialisation_param_default_type() {
        insta::assert_snapshot!(dump(
            "processor P (using T = float32) { output stream int out; }",
            Parser::parse_statement
        ));
    }

    #[test]
    fn graph_with_processor_specialisation_param() {
        insta::assert_snapshot!(dump(
            "graph Wrapper (processor Parameterised, int x) { output stream int out; }",
            Parser::parse_statement
        ));
    }

    #[test]
    fn namespace_with_specialisation_params() {
        insta::assert_snapshot!(dump(
            "namespace n (processor p, namespace ns) {}",
            Parser::parse_statement
        ));
    }

    #[test]
    fn multiple_specialisation_params_of_different_kinds() {
        insta::assert_snapshot!(dump(
            "processor P (using T, int length = 4) { output stream int out; }",
            Parser::parse_statement
        ));
    }

    #[test]
    fn processor_latency_assignment_is_not_a_container_decl() {
        insta::assert_snapshot!(dump(
            "processor.latency = length;",
            Parser::parse_statement
        ));
    }
}
