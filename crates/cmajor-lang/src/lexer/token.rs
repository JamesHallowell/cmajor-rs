#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]

pub enum Keyword {
    Bool,
    Break,
    Case,
    Catch,
    Class,
    Complex,
    Complex32,
    Complex64,
    Connection,
    Const,
    Continue,
    Default,
    Do,
    Double,
    Else,
    Enum,
    Event,
    External,
    False,
    Fixed,
    Float,
    Float32,
    Float64,
    For,
    Graph,
    If,
    Import,
    Input,
    Int,
    Int32,
    Int64,
    Let,
    Loop,
    Namespace,
    Node,
    Operator,
    Output,
    Private,
    Processor,
    Public,
    Return,
    String,
    Struct,
    Switch,
    Throw,
    True,
    Try,
    Using,
    Var,
    Void,
    While,
}

impl TryFrom<&str> for Keyword {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Ok(match s {
            "bool" => Keyword::Bool,
            "break" => Keyword::Break,
            "case" => Keyword::Case,
            "catch" => Keyword::Catch,
            "class" => Keyword::Class,
            "complex" => Keyword::Complex,
            "complex32" => Keyword::Complex32,
            "complex64" => Keyword::Complex64,
            "connection" => Keyword::Connection,
            "const" => Keyword::Const,
            "continue" => Keyword::Continue,
            "default" => Keyword::Default,
            "do" => Keyword::Do,
            "double" => Keyword::Double,
            "else" => Keyword::Else,
            "enum" => Keyword::Enum,
            "event" => Keyword::Event,
            "external" => Keyword::External,
            "false" => Keyword::False,
            "fixed" => Keyword::Fixed,
            "float" => Keyword::Float,
            "float32" => Keyword::Float32,
            "float64" => Keyword::Float64,
            "for" => Keyword::For,
            "graph" => Keyword::Graph,
            "if" => Keyword::If,
            "import" => Keyword::Import,
            "input" => Keyword::Input,
            "int" => Keyword::Int,
            "int32" => Keyword::Int32,
            "int64" => Keyword::Int64,
            "let" => Keyword::Let,
            "loop" => Keyword::Loop,
            "namespace" => Keyword::Namespace,
            "node" => Keyword::Node,
            "operator" => Keyword::Operator,
            "output" => Keyword::Output,
            "private" => Keyword::Private,
            "processor" => Keyword::Processor,
            "public" => Keyword::Public,
            "return" => Keyword::Return,
            "string" => Keyword::String,
            "struct" => Keyword::Struct,
            "switch" => Keyword::Switch,
            "throw" => Keyword::Throw,
            "true" => Keyword::True,
            "try" => Keyword::Try,
            "using" => Keyword::Using,
            "var" => Keyword::Var,
            "void" => Keyword::Void,
            "while" => Keyword::While,
            _ => return Err(()),
        })
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]

pub enum Trivia {
    BlockComment,
    LineComment,
    Whitespace,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]

pub enum Literal {
    Int32,
    Int64,
    Float32,
    Float64,
    Imaginary32,
    Imaginary64,
    String,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Ampersand,
    AmpersandAmpersand,
    AmpersandAmpersandEqual,
    AmpersandEqual,
    ArrowLeft,
    ArrowRight,
    Bang,
    BangEqual,
    BraceLeft,
    BraceRight,
    BracketLeft,
    BracketRight,
    Caret,
    CaretEqual,
    Colon,
    ColonColon,
    Comma,
    Dot,
    DoubleBracketLeft,
    DoubleBracketRight,
    Equal,
    EqualEqual,
    Error,
    GreaterThan,
    GreaterThanOrEqual,
    Identifier,
    Keyword(Keyword),
    LessThan,
    LessThanOrEqual,
    Literal(Literal),
    Minus,
    MinusEqual,
    MinusMinus,
    ParenthesisLeft,
    ParenthesisRight,
    Percent,
    PercentEqual,
    Pipe,
    PipeEqual,
    PipePipe,
    PipePipeEqual,
    Plus,
    PlusEqual,
    PlusPlus,
    Question,
    Semicolon,
    ShiftLeft,
    ShiftLeftEqual,
    ShiftRight,
    ShiftRightEqual,
    ShiftRightShiftRight,
    ShiftRightShiftRightEqual,
    Slash,
    SlashEqual,
    Star,
    StarEqual,
    StarStar,
    Tilde,
    Trivia(Trivia),
}

impl TokenKind {
    pub fn is_trivia(self) -> bool {
        matches!(self, TokenKind::Trivia(_))
    }

    pub fn is_type(self) -> bool {
        let Self::Keyword(keyword) = self else {
            return false;
        };

        use Keyword::*;
        matches!(
            keyword,
            Bool | Complex
                | Complex32
                | Complex64
                | Double
                | Float
                | Float32
                | Float64
                | Int
                | Int32
                | Int64
                | String
                | Void
        )
    }
}

impl From<Keyword> for TokenKind {
    fn from(keyword: Keyword) -> Self {
        TokenKind::Keyword(keyword)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    pub kind: TokenKind,
    pub len: u32,
}
