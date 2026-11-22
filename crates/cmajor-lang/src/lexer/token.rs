macro_rules! keywords {
    ($($variant:ident => $text:literal,)*) => {
        #[repr(u8)]
        #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
        pub enum Keyword {
            $($variant,)*
        }

        impl Keyword {
            pub fn as_str(self) -> &'static str {
                match self {
                    $(Keyword::$variant => $text,)*
                }
            }
        }

        impl TryFrom<&str> for Keyword {
            type Error = ();

            fn try_from(s: &str) -> Result<Self, Self::Error> {
                match s {
                    $($text => Ok(Keyword::$variant),)*
                    _ => Err(()),
                }
            }
        }
    };
}

keywords! {
    Bool => "bool",
    Break => "break",
    Case => "case",
    Catch => "catch",
    Class => "class",
    Complex => "complex",
    Complex32 => "complex32",
    Complex64 => "complex64",
    Connection => "connection",
    Const => "const",
    Continue => "continue",
    Default => "default",
    Do => "do",
    Double => "double",
    Else => "else",
    Enum => "enum",
    Event => "event",
    External => "external",
    False => "false",
    Fixed => "fixed",
    Float => "float",
    Float32 => "float32",
    Float64 => "float64",
    For => "for",
    ForwardBranch => "forward_branch",
    Graph => "graph",
    If => "if",
    Import => "import",
    Input => "input",
    Int => "int",
    Int32 => "int32",
    Int64 => "int64",
    Let => "let",
    Loop => "loop",
    Namespace => "namespace",
    Node => "node",
    Operator => "operator",
    Output => "output",
    Private => "private",
    Processor => "processor",
    Public => "public",
    Return => "return",
    String => "string",
    Struct => "struct",
    Switch => "switch",
    Throw => "throw",
    True => "true",
    Try => "try",
    Using => "using",
    Var => "var",
    Void => "void",
    While => "while",
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
pub enum LexError {
    IllegalCharacter(char),
    UnterminatedComment,
    IdentifierTooLong,
    LeadingUnderscore,
    OctalLiteral,
    IntegerLiteralTooLarge,
    IntegerLiteralNeedsSuffix,
    MalformedNumber,
    UnrecognisedLiteralSuffix,
    ImaginaryIntegerLiteral,
    UnterminatedString,
    MalformedEscapeSequence,
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
    Error(LexError),
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

    pub fn can_start_declaration(self) -> bool {
        self.is_type_like()
            || matches!(
                self,
                crate::token!(
                    const
                        | '('
                        | ::
                        | node
                        | connection
                        | graph
                        | processor
                        | struct
                        | enum
                        | var
                        | let
                        | event
                        | using
                        | namespace
                        | input
                        | output
                        | import
                        | external
                )
            )
    }
}

impl LexError {
    fn description(self) -> &'static str {
        match self {
            LexError::IllegalCharacter(_) => "unrecognised character",
            LexError::UnterminatedComment => "unterminated comment",
            LexError::IdentifierTooLong | LexError::LeadingUnderscore => "invalid identifier",
            LexError::OctalLiteral
            | LexError::IntegerLiteralTooLarge
            | LexError::IntegerLiteralNeedsSuffix
            | LexError::MalformedNumber
            | LexError::ImaginaryIntegerLiteral => "invalid numeric literal",
            LexError::UnrecognisedLiteralSuffix => "invalid literal suffix",
            LexError::UnterminatedString => "unterminated string",
            LexError::MalformedEscapeSequence => "invalid escape sequence",
        }
    }
}

impl Literal {
    fn description(self) -> &'static str {
        match self {
            Literal::Int32 => "int32 literal",
            Literal::Int64 => "int64 literal",
            Literal::Float32 => "float32 literal",
            Literal::Float64 => "float64 literal",
            Literal::Imaginary32 => "imag32 imaginary literal",
            Literal::Imaginary64 => "imag64 imaginary literal",
            Literal::String => "string literal",
        }
    }
}

impl std::fmt::Display for TokenKind {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let spelling = match *self {
            TokenKind::Identifier => return f.write_str("identifier"),
            TokenKind::EndOfFile => return f.write_str("end-of-file"),
            TokenKind::Error(error) => return f.write_str(error.description()),
            TokenKind::Literal(literal) => return f.write_str(literal.description()),
            TokenKind::Trivia(_) => return f.write_str("whitespace or comment"),
            TokenKind::Keyword(keyword) => keyword.as_str(),
            TokenKind::Ampersand => "&",
            TokenKind::AmpersandAmpersand => "&&",
            TokenKind::AmpersandAmpersandEqual => "&&=",
            TokenKind::AmpersandEqual => "&=",
            TokenKind::AngleBracketLeft => "<",
            TokenKind::AngleBracketLeftEqual => "<=",
            TokenKind::AngleBracketRight => ">",
            TokenKind::AngleBracketRightEqual => ">=",
            TokenKind::ArrowLeft => "<-",
            TokenKind::ArrowRight => "->",
            TokenKind::Bang => "!",
            TokenKind::BangEqual => "!=",
            TokenKind::BraceLeft => "{",
            TokenKind::BraceRight => "}",
            TokenKind::BracketLeft => "[",
            TokenKind::BracketRight => "]",
            TokenKind::Caret => "^",
            TokenKind::CaretEqual => "^=",
            TokenKind::Colon => ":",
            TokenKind::ColonColon => "::",
            TokenKind::Comma => ",",
            TokenKind::Dot => ".",
            TokenKind::Equal => "=",
            TokenKind::EqualEqual => "==",
            TokenKind::Minus => "-",
            TokenKind::MinusEqual => "-=",
            TokenKind::MinusMinus => "--",
            TokenKind::ParenthesisLeft => "(",
            TokenKind::ParenthesisRight => ")",
            TokenKind::Percent => "%",
            TokenKind::PercentEqual => "%=",
            TokenKind::Pipe => "|",
            TokenKind::PipeEqual => "|=",
            TokenKind::PipePipe => "||",
            TokenKind::PipePipeEqual => "||=",
            TokenKind::Plus => "+",
            TokenKind::PlusEqual => "+=",
            TokenKind::PlusPlus => "++",
            TokenKind::Question => "?",
            TokenKind::Semicolon => ";",
            TokenKind::ShiftLeft => "<<",
            TokenKind::ShiftLeftEqual => "<<=",
            TokenKind::ShiftRight => ">>",
            TokenKind::ShiftRightEqual => ">>=",
            TokenKind::ShiftRightShiftRight => ">>>",
            TokenKind::ShiftRightShiftRightEqual => ">>>=",
            TokenKind::Slash => "/",
            TokenKind::SlashEqual => "/=",
            TokenKind::Star => "*",
            TokenKind::StarEqual => "*=",
            TokenKind::StarStar => "**",
            TokenKind::Tilde => "~",
        };
        write!(f, "\"{spelling}\"")
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
