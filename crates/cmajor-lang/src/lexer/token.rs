#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]

pub enum Keyword {
    Bool,
    Break,
    Case,
    Catch,
    Clamp,
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
    Wrap,
}

impl TryFrom<&str> for Keyword {
    type Error = ();

    fn try_from(s: &str) -> Result<Self, Self::Error> {
        Ok(match s {
            "bool" => Keyword::Bool,
            "break" => Keyword::Break,
            "case" => Keyword::Case,
            "catch" => Keyword::Catch,
            "clamp" => Keyword::Clamp,
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
            "wrap" => Keyword::Wrap,
            _ => return Err(()),
        })
    }
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]

pub enum Trivia {
    Whitespace,
    LineComment,
    BlockComment,
}

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SyntaxKind {
    Trivia(Trivia),
    Keyword(Keyword),
    Ident,
    IntLiteral,
    FloatLiteral,
    StringLiteral,
    Plus,
    Minus,
    Star,
    StarStar,
    Slash,
    Percent,
    Bang,
    Tilde,
    PlusPlus,
    MinusMinus,
    Ampersand,
    AmpersandAmpersand,
    Pipe,
    PipePipe,
    Caret,
    ShiftLeft,
    ShiftRight,
    ShiftRightShiftRight,
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
    EqualEqual,
    BangEqual,
    Equal,
    ArrowRight,
    ArrowLeft,
    ColonColon,
    Colon,
    Comma,
    Semicolon,
    Dot,
    Question,
    ParenthesisLeft,
    ParenthesisRight,
    BracketLeft,
    BracketRight,
    BraceLeft,
    BraceRight,
    EndOfFile,
    Error,
}

impl SyntaxKind {
    pub fn is_trivia(self) -> bool {
        matches!(self, SyntaxKind::Trivia(_))
    }
}

impl From<Keyword> for SyntaxKind {
    fn from(keyword: Keyword) -> Self {
        SyntaxKind::Keyword(keyword)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    pub kind: SyntaxKind,
    pub len: u32,
}
