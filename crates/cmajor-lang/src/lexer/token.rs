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
    ForwardBranch,
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
            "forward_branch" => Keyword::ForwardBranch,
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
    AngleBracketLeft,
    AngleBracketLeftEqual,
    AngleBracketRight,
    AngleBracketRightEqual,
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
    EndOfFile,
    Equal,
    EqualEqual,
    Error,
    Identifier,
    Keyword(Keyword),
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

impl Keyword {
    pub fn is_type(self) -> bool {
        use Keyword::*;
        matches!(
            self,
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

impl TokenKind {
    pub fn is_trivia(self) -> bool {
        matches!(self, TokenKind::Trivia(_))
    }

    pub fn is_type_like(self) -> bool {
        match self {
            TokenKind::Identifier => true,
            TokenKind::Keyword(keyword) => keyword.is_type(),
            _ => false,
        }
    }

    pub fn is_end_of_file(self) -> bool {
        self == TokenKind::EndOfFile
    }
}

impl From<Keyword> for TokenKind {
    fn from(keyword: Keyword) -> Self {
        TokenKind::Keyword(keyword)
    }
}

impl From<Literal> for TokenKind {
    fn from(literal: Literal) -> Self {
        TokenKind::Literal(literal)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Token {
    pub kind: TokenKind,
    pub len: u32,
}

#[macro_export]
macro_rules! token {
    (&) => {
        $crate::lexer::TokenKind::Ampersand
    };
    (&&) => {
        $crate::lexer::TokenKind::AmpersandAmpersand
    };
    (&&=) => {
        $crate::lexer::TokenKind::AmpersandAmpersandEqual
    };
    (&=) => {
        $crate::lexer::TokenKind::AmpersandEqual
    };
    (<) => {
        $crate::lexer::TokenKind::AngleBracketLeft
    };
    (<=) => {
        $crate::lexer::TokenKind::AngleBracketLeftEqual
    };
    (>) => {
        $crate::lexer::TokenKind::AngleBracketRight
    };
    (>=) => {
        $crate::lexer::TokenKind::AngleBracketRightEqual
    };
    (<-) => {
        $crate::lexer::TokenKind::ArrowLeft
    };
    (->) => {
        $crate::lexer::TokenKind::ArrowRight
    };
    (!) => {
        $crate::lexer::TokenKind::Bang
    };
    (!=) => {
        $crate::lexer::TokenKind::BangEqual
    };
    (^) => {
        $crate::lexer::TokenKind::Caret
    };
    (^=) => {
        $crate::lexer::TokenKind::CaretEqual
    };
    (:) => {
        $crate::lexer::TokenKind::Colon
    };
    (::) => {
        $crate::lexer::TokenKind::ColonColon
    };
    (,) => {
        $crate::lexer::TokenKind::Comma
    };
    (.) => {
        $crate::lexer::TokenKind::Dot
    };
    (=) => {
        $crate::lexer::TokenKind::Equal
    };
    (==) => {
        $crate::lexer::TokenKind::EqualEqual
    };
    (-) => {
        $crate::lexer::TokenKind::Minus
    };
    (-=) => {
        $crate::lexer::TokenKind::MinusEqual
    };
    (--) => {
        $crate::lexer::TokenKind::MinusMinus
    };
    (%) => {
        $crate::lexer::TokenKind::Percent
    };
    (%=) => {
        $crate::lexer::TokenKind::PercentEqual
    };
    (|) => {
        $crate::lexer::TokenKind::Pipe
    };
    (|=) => {
        $crate::lexer::TokenKind::PipeEqual
    };
    (||) => {
        $crate::lexer::TokenKind::PipePipe
    };
    (||=) => {
        $crate::lexer::TokenKind::PipePipeEqual
    };
    (+) => {
        $crate::lexer::TokenKind::Plus
    };
    (+=) => {
        $crate::lexer::TokenKind::PlusEqual
    };
    (++) => {
        $crate::lexer::TokenKind::PlusPlus
    };
    (?) => {
        $crate::lexer::TokenKind::Question
    };
    (;) => {
        $crate::lexer::TokenKind::Semicolon
    };
    (<<) => {
        $crate::lexer::TokenKind::ShiftLeft
    };
    (<<=) => {
        $crate::lexer::TokenKind::ShiftLeftEqual
    };
    (>>) => {
        $crate::lexer::TokenKind::ShiftRight
    };
    (>>=) => {
        $crate::lexer::TokenKind::ShiftRightEqual
    };
    (>>>) => {
        $crate::lexer::TokenKind::ShiftRightShiftRight
    };
    (>>>=) => {
        $crate::lexer::TokenKind::ShiftRightShiftRightEqual
    };
    (/) => {
        $crate::lexer::TokenKind::Slash
    };
    (/=) => {
        $crate::lexer::TokenKind::SlashEqual
    };
    (*) => {
        $crate::lexer::TokenKind::Star
    };
    (*=) => {
        $crate::lexer::TokenKind::StarEqual
    };
    (**) => {
        $crate::lexer::TokenKind::StarStar
    };
    (~) => {
        $crate::lexer::TokenKind::Tilde
    };
    ('(') => {
        $crate::lexer::TokenKind::ParenthesisLeft
    };
    (')') => {
        $crate::lexer::TokenKind::ParenthesisRight
    };
    ('[') => {
        $crate::lexer::TokenKind::BracketLeft
    };
    (']') => {
        $crate::lexer::TokenKind::BracketRight
    };
    ("[[") => {
        $crate::lexer::TokenKind::DoubleBracketLeft
    };
    ("]]") => {
        $crate::lexer::TokenKind::DoubleBracketRight
    };
    ('{') => {
        $crate::lexer::TokenKind::BraceLeft
    };
    ('}') => {
        $crate::lexer::TokenKind::BraceRight
    };
    (bool) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Bool)
    };
    (break) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Break)
    };
    (case) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Case)
    };
    (catch) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Catch)
    };
    (class) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Class)
    };
    (complex) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Complex)
    };
    (complex32) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Complex32)
    };
    (complex64) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Complex64)
    };
    (connection) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Connection)
    };
    (const) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Const)
    };
    (continue) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Continue)
    };
    (default) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Default)
    };
    (double) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Double)
    };
    (else) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Else)
    };
    (enum) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Enum)
    };
    (event) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Event)
    };
    (external) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::External)
    };
    (false) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::False)
    };
    (fixed) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Fixed)
    };
    (float) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Float)
    };
    (float32) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Float32)
    };
    (float64) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Float64)
    };
    (for) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::For)
    };
    (forward_branch) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::ForwardBranch)
    };
    (graph) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Graph)
    };
    (if) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::If)
    };
    (import) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Import)
    };
    (input) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Input)
    };
    (int) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Int)
    };
    (int32) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Int32)
    };
    (int64) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Int64)
    };
    (let) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Let)
    };
    (loop) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Loop)
    };
    (namespace) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Namespace)
    };
    (node) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Node)
    };
    (operator) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Operator)
    };
    (output) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Output)
    };
    (private) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Private)
    };
    (processor) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Processor)
    };
    (public) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Public)
    };
    (return) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Return)
    };
    (string) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::String)
    };
    (struct) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Struct)
    };
    (switch) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Switch)
    };
    (throw) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Throw)
    };
    (true) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::True)
    };
    (try) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Try)
    };
    (using) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Using)
    };
    (var) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Var)
    };
    (void) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::Void)
    };
    (while) => {
        $crate::lexer::TokenKind::Keyword($crate::lexer::Keyword::While)
    };
    (eof) => {
        $crate::lexer::TokenKind::EndOfFile
    };
    (@or [$($acc:tt)+]) => {
        $crate::token!($($acc)+)
    };
    (@or [$($acc:tt)+] | $($rest:tt)+) => {
        $crate::token!($($acc)+) | $crate::token!(@or [] $($rest)+)
    };
    (@or [$($acc:tt)*] $next:tt $($rest:tt)*) => {
        $crate::token!(@or [$($acc)* $next] $($rest)*)
    };
    ($first:tt $($rest:tt)+) => {
        $crate::token!(@or [$first] $($rest)+)
    };
}
