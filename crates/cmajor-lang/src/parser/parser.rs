use {
    crate::{
        ast::{Ast, Node, NodeId},
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
    let tokens = TokenStream::tokenize(source);
    let mut parser = Parser {
        tokens: &tokens,
        ast: Ast::new(),
        pos: 0,
    };

    let mut stmts = Vec::new();
    while parser.peek().is_some() {
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

struct Parser<'a> {
    tokens: &'a TokenStream,
    ast: Ast,
    pos: u32,
}

impl Parser<'_> {
    fn peek(&self) -> Option<Token> {
        self.tokens.token(TokenId(self.pos))
    }

    fn peek_kind(&self) -> Option<TokenKind> {
        self.peek().map(|token| token.kind)
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

    fn parse_expr(&mut self, min_binding_power: BindingPower) -> NodeId {
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
                    let then_branch = self.parse_expr(PrecedenceLevel::lowest());
                    self.expect(TokenKind::Colon);
                    let else_branch = self.parse_expr(binding_power.min_for_rhs);
                    self.ast.push(Node::Ternary {
                        question,
                        cond: lhs,
                        then_branch,
                        else_branch,
                    })
                }
                Infix::Binary(bin_op) => {
                    let op = self.bump();
                    let rhs = self.parse_expr(binding_power.min_for_rhs);
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
            let operand = self.parse_expr(PrecedenceLevel::Unary.base());
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
                Some(TokenKind::PlusPlus | TokenKind::MinusMinus) => {
                    let op = self.bump();
                    self.ast.push(Node::PostfixUnary { op, operand: expr })
                }
                _ => break,
            };
        }

        expr
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
                args.push(self.parse_expr(PrecedenceLevel::lowest()));
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
        let index = self.parse_expr(PrecedenceLevel::lowest());
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
                let inner = self.parse_expr(PrecedenceLevel::lowest());
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
        let ty = self.parse_type();
        let name = self.expect(TokenKind::Identifier);
        if self.peek_kind() == Some(TokenKind::ParenthesisLeft) {
            self.parse_function_decl(ty, name)
        } else {
            let init = if self.peek_kind() == Some(TokenKind::Equal) {
                self.bump();
                Some(self.parse_expr(PrecedenceLevel::lowest()))
            } else {
                None
            };
            self.expect(TokenKind::Semicolon);
            self.ast.push(Node::VarDeclStmt {
                ty,
                name,
                init,
                is_const,
            })
        }
    }

    fn parse_params(&mut self) -> Vec<NodeId> {
        self.expect(TokenKind::ParenthesisLeft);
        let mut params = Vec::new();
        if self.peek_kind() != Some(TokenKind::ParenthesisRight) {
            loop {
                let ty = self.parse_type();
                let name = self.expect(TokenKind::Identifier);
                params.push(self.ast.push(Node::Param { ty, name }));
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

    fn parse_function_decl(&mut self, ty: NodeId, name: TokenId) -> NodeId {
        let params = self.parse_params();
        let body = self.parse_block();
        self.ast.push(Node::FunctionDecl {
            ty,
            name,
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
        self.expect(TokenKind::BraceLeft);
        let mut items = Vec::new();
        while !matches!(self.peek_kind(), Some(TokenKind::BraceRight) | None) {
            items.push(self.parse_statement());
        }
        self.expect(TokenKind::BraceRight);
        self.ast.push(Node::NamespaceDecl {
            keyword,
            segments,
            items,
        })
    }

    fn parse_container(&mut self) -> NodeId {
        let keyword = self.bump();
        let name = self.expect(TokenKind::Identifier);
        self.expect(TokenKind::BraceLeft);
        let mut items = Vec::new();
        while !matches!(self.peek_kind(), Some(TokenKind::BraceRight) | None) {
            items.push(self.parse_statement());
        }
        self.expect(TokenKind::BraceRight);
        self.ast.push(Node::ContainerDecl {
            keyword,
            name,
            items,
        })
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
                self.expect(TokenKind::Colon);
                let value = self.parse_expr(PrecedenceLevel::lowest());
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

    fn parse_endpoint_member(&mut self) -> NodeId {
        let ty = self.parse_type();
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
        let name = self.expect(TokenKind::Identifier);
        self.expect(TokenKind::Equal);
        let init = self.parse_expr(PrecedenceLevel::lowest());
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::LetStmt { name, init })
    }

    fn parse_var(&mut self) -> NodeId {
        self.bump();
        let name = self.expect(TokenKind::Identifier);
        let init = if self.peek_kind() == Some(TokenKind::Equal) {
            self.bump();
            Some(self.parse_expr(PrecedenceLevel::lowest()))
        } else {
            None
        };
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::VarStmt { name, init })
    }

    fn parse_if(&mut self) -> NodeId {
        let keyword = self.bump();
        self.expect(TokenKind::ParenthesisLeft);
        let cond = self.parse_expr(PrecedenceLevel::lowest());
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
        let cond = self.parse_expr(PrecedenceLevel::lowest());
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
            let count = self.parse_expr(PrecedenceLevel::lowest());
            self.expect(TokenKind::ParenthesisRight);
            Some(count)
        } else {
            None
        };
        let body = self.parse_block();
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
            Some(self.parse_expr(PrecedenceLevel::lowest()))
        };
        self.expect(TokenKind::Semicolon);
        self.ast.push(Node::ReturnStmt { keyword, value })
    }

    fn parse_expr_stmt(&mut self) -> NodeId {
        let expr = self.parse_expr(PrecedenceLevel::lowest());
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
            Some(self.parse_expr(PrecedenceLevel::lowest()))
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
        let term = self.parse_expr(PrecedenceLevel::Shift.base());
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
    use super::*;

    #[derive(Debug, PartialEq)]
    enum Tag {
        Assign,
        AttributeList,
        Binary,
        Block,
        BoolLiteral,
        BreakStmt,
        Call,
        ChevronSuffix,
        ComplexLiteral,
        ContainerDecl,
        ContinueStmt,
        EndpointDecl,
        EndpointGroup,
        Error,
        EventHandlerDecl,
        ExprStmt,
        Field,
        FloatLiteral,
        FunctionDecl,
        Ident,
        IfStmt,
        Index,
        IntLiteral,
        LetStmt,
        LoopStmt,
        NamespaceDecl,
        Param,
        Paren,
        PostfixUnary,
        ReturnStmt,
        ScopeAccess,
        StringLiteral,
        Ternary,
        TypeArray,
        TypeName,
        Unary,
        VarDeclStmt,
        VarStmt,
        WhileStmt,
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
        let text_of = |token: &TokenId| tokens.text(source, *token).expect("invalid token");
        let child = |id: &NodeId| walk(ast, tokens, source, *id);

        match ast.get(id) {
            Node::IntLiteral { token } => leaf(Tag::IntLiteral, text_of(token)),
            Node::FloatLiteral { token } => leaf(Tag::FloatLiteral, text_of(token)),
            Node::StringLiteral { token } => leaf(Tag::StringLiteral, text_of(token)),
            Node::BoolLiteral { token } => leaf(Tag::BoolLiteral, text_of(token)),
            Node::ImaginaryLiteral { token } => leaf(Tag::ComplexLiteral, text_of(token)),
            Node::Ident { token } => leaf(Tag::Ident, text_of(token)),
            Node::Error { token } => leaf(Tag::Error, text_of(token)),
            Node::Paren { paren, inner } => node(Tag::Paren, text_of(paren), vec![child(inner)]),
            Node::Unary { op, operand } => node(Tag::Unary, text_of(op), vec![child(operand)]),
            Node::PostfixUnary { op, operand } => {
                node(Tag::PostfixUnary, text_of(op), vec![child(operand)])
            }
            Node::Binary { op, lhs, rhs } => {
                node(Tag::Binary, text_of(op), vec![child(lhs), child(rhs)])
            }
            Node::Assign { op, target, value } => {
                node(Tag::Assign, text_of(op), vec![child(target), child(value)])
            }
            Node::Ternary {
                question,
                cond,
                then_branch,
                else_branch,
            } => node(
                Tag::Ternary,
                text_of(question),
                vec![child(cond), child(then_branch), child(else_branch)],
            ),
            Node::Call {
                paren,
                callee,
                args,
            } => {
                let mut children = vec![child(callee)];
                children.extend(args.iter().map(child));
                node(Tag::Call, text_of(paren), children)
            }
            Node::Index {
                bracket,
                base,
                index,
            } => node(
                Tag::Index,
                text_of(bracket),
                vec![child(base), child(index)],
            ),
            Node::Field { name, base } => node(Tag::Field, text_of(name), vec![child(base)]),
            Node::Block { brace, stmts } => node(
                Tag::Block,
                text_of(brace),
                stmts.iter().map(child).collect(),
            ),
            Node::ExprStmt { expr } => node(Tag::ExprStmt, "", vec![child(expr)]),
            Node::LetStmt { name, init } => node(Tag::LetStmt, text_of(name), vec![child(init)]),
            Node::VarStmt { name, init } => node(
                Tag::VarStmt,
                text_of(name),
                init.iter().map(child).collect(),
            ),
            Node::VarDeclStmt { ty, name, init, .. } => {
                let mut children = vec![child(ty)];
                children.extend(init.iter().map(child));
                node(Tag::VarDeclStmt, text_of(name), children)
            }
            Node::IfStmt {
                keyword,
                cond,
                then_branch,
                else_branch,
            } => {
                let mut children = vec![child(cond), child(then_branch)];
                children.extend(else_branch.iter().map(child));
                node(Tag::IfStmt, text_of(keyword), children)
            }
            Node::WhileStmt {
                keyword,
                cond,
                body,
            } => node(
                Tag::WhileStmt,
                text_of(keyword),
                vec![child(cond), child(body)],
            ),
            Node::LoopStmt {
                keyword,
                count,
                body,
            } => {
                let mut children: Vec<_> = count.iter().map(child).collect();
                children.push(child(body));
                node(Tag::LoopStmt, text_of(keyword), children)
            }
            Node::ReturnStmt { keyword, value } => node(
                Tag::ReturnStmt,
                text_of(keyword),
                value.iter().map(child).collect(),
            ),
            Node::BreakStmt { keyword } => leaf(Tag::BreakStmt, text_of(keyword)),
            Node::ContinueStmt { keyword } => leaf(Tag::ContinueStmt, text_of(keyword)),

            Node::TypeName { segments } => {
                let path = segments.iter().map(&text_of).collect::<Vec<_>>().join("::");
                leaf(Tag::TypeName, &path)
            }
            Node::Array {
                bracket,
                element,
                size,
            } => {
                let mut children = vec![child(element)];
                children.extend(size.iter().map(child));
                node(Tag::TypeArray, text_of(bracket), children)
            }
            Node::ChevronSuffix {
                angle,
                element,
                term,
            } => node(
                Tag::ChevronSuffix,
                text_of(angle),
                vec![child(element), child(term)],
            ),

            Node::NamespaceDecl {
                keyword,
                segments,
                items,
            } => {
                let path = segments.iter().map(&text_of).collect::<Vec<_>>().join("::");
                node(
                    Tag::NamespaceDecl,
                    &format!("{} {path}", text_of(keyword)),
                    items.iter().map(child).collect(),
                )
            }
            Node::ContainerDecl {
                keyword,
                name,
                items,
            } => node(
                Tag::ContainerDecl,
                &format!("{} {}", text_of(keyword), text_of(name)),
                items.iter().map(child).collect(),
            ),
            Node::EndpointGroup {
                direction,
                kind,
                endpoints,
            } => node(
                Tag::EndpointGroup,
                &format!("{} {}", text_of(direction), text_of(kind)),
                endpoints.iter().map(child).collect(),
            ),
            Node::EndpointDecl {
                ty,
                name,
                attributes,
            } => {
                let mut children = vec![child(ty)];
                children.extend(attributes.iter().map(child));
                node(Tag::EndpointDecl, text_of(name), children)
            }
            Node::AttributeList { attrs } => node(
                Tag::AttributeList,
                "",
                attrs
                    .iter()
                    .map(|(key, value)| node(Tag::Param, text_of(key), vec![child(value)]))
                    .collect(),
            ),
            Node::FunctionDecl {
                ty,
                name,
                params,
                body,
            } => {
                let mut children = vec![child(ty)];
                children.extend(params.iter().map(child));
                children.push(child(body));
                node(Tag::FunctionDecl, text_of(name), children)
            }
            Node::Param { ty, name } => node(Tag::Param, text_of(name), vec![child(ty)]),
            Node::EventHandlerDecl {
                keyword,
                name,
                params,
                body,
            } => {
                let mut children: Vec<_> = params.iter().map(child).collect();
                children.push(child(body));
                node(
                    Tag::EventHandlerDecl,
                    &format!("{} {}", text_of(keyword), text_of(name)),
                    children,
                )
            }
            Node::ScopeAccess { name, base } => {
                node(Tag::ScopeAccess, text_of(name), vec![child(base)])
            }
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
                ChevronSuffix,
                "<",
                vec![leaf(TypeName, "int"), leaf(IntLiteral, "4")]
            )
        );
    }

    #[test]
    fn wrap_and_clamp_are_not_keywords_and_parse_as_generic_type_names() {
        use Tag::*;
        assert_eq!(
            parse_type_source("wrap<4>"),
            node(
                ChevronSuffix,
                "<",
                vec![leaf(TypeName, "wrap"), leaf(IntLiteral, "4")]
            )
        );
        assert_eq!(
            parse_type_source("clamp<10>"),
            node(
                ChevronSuffix,
                "<",
                vec![leaf(TypeName, "clamp"), leaf(IntLiteral, "10")]
            )
        );
    }

    #[test]
    fn angle_bracket_close_is_not_a_comparison() {
        use Tag::*;
        assert_eq!(
            parse_type_source("wrap<1 + 2>"),
            node(
                ChevronSuffix,
                "<",
                vec![
                    leaf(TypeName, "wrap"),
                    node(
                        Binary,
                        "+",
                        vec![leaf(IntLiteral, "1"), leaf(IntLiteral, "2")]
                    )
                ]
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
                        ChevronSuffix,
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
    fn prefix_and_postfix_increment() {
        use Tag::*;
        assert_eq!(parse_expr("++x"), node(Unary, "++", vec![leaf(Ident, "x")]));
        assert_eq!(
            parse_expr("x++"),
            node(PostfixUnary, "++", vec![leaf(Ident, "x")])
        );
        assert_eq!(
            parse_expr("x--"),
            node(PostfixUnary, "--", vec![leaf(Ident, "x")])
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
    fn typed_var_decl_statements() {
        use Tag::*;
        assert_eq!(
            parse_source("wrap<5> w; clamp<5> c; int n = 1;"),
            node(
                Block,
                "wrap",
                vec![
                    node(
                        VarDeclStmt,
                        "w",
                        vec![node(
                            ChevronSuffix,
                            "<",
                            vec![leaf(TypeName, "wrap"), leaf(IntLiteral, "5")]
                        )]
                    ),
                    node(
                        VarDeclStmt,
                        "c",
                        vec![node(
                            ChevronSuffix,
                            "<",
                            vec![leaf(TypeName, "clamp"), leaf(IntLiteral, "5")]
                        )]
                    ),
                    node(
                        VarDeclStmt,
                        "n",
                        vec![leaf(TypeName, "int"), leaf(IntLiteral, "1")]
                    ),
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
