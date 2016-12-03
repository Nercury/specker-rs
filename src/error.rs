use std::fmt;
use std::result;
use std::str;
use std::error::Error;
use tokens::TokenValue;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LexError {
    ExpectedSequenceFoundNewline {
        expected: Vec<u8>,
    },
    ExpectedNewline,
    Utf8(str::Utf8Error),
}

impl ::std::error::Error for LexError {
    fn description(&self) -> &str {
        match *self {
            LexError::ExpectedSequenceFoundNewline { .. } => "expected sequence, found newline",
            LexError::ExpectedNewline => "expected newline",
            LexError::Utf8(ref e) => e.description(),
        }
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LexError::ExpectedSequenceFoundNewline { ref expected } =>
                write!(f, "Expected \"{}\", found new line", String::from_utf8_lossy(expected)),
            LexError::ExpectedNewline => "Expected new line".fmt(f),
            LexError::Utf8(e) => e.fmt(f),
        }
    }
}

impl LexError {
    pub fn at(self, lo: FilePosition, hi: FilePosition) -> At<LexError> {
        At {
            lo: lo,
            hi: hi,
            desc: self,
        }
    }
}

impl From<str::Utf8Error> for LexError {
    fn from(other: str::Utf8Error) -> Self {
        LexError::Utf8(other)
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum ParseError {
    Lex(LexError),
    ExpectedKeyFoundValue,
    UnexpectedEndOfTokens,
    ExpectedDifferentToken {
        expected: Vec<TokenValue>,
        found: TokenValue
    },
}

impl ::std::error::Error for ParseError {
    fn description(&self) -> &str {
        match *self {
            ParseError::Lex(ref e) => e.description(),
            ParseError::ExpectedKeyFoundValue => "expected key, found value",
            ParseError::UnexpectedEndOfTokens => "unexpected end of tokens",
            ParseError::ExpectedDifferentToken { .. } => "expected different token",
        }
    }
}

impl From<At<LexError>> for At<ParseError> {
    fn from(At { lo, hi, desc }: At<LexError>) -> Self {
        ParseError::Lex(desc).at(lo, hi)
    }
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            ParseError::Lex(ref e) => e.fmt(f),
            ParseError::ExpectedKeyFoundValue => "Expected key, found value".fmt(f),
            ParseError::UnexpectedEndOfTokens => "Unexpected end of file".fmt(f),
            ParseError::ExpectedDifferentToken { ref expected, ref found } => write!(f, "Expected {}, buf found {}", expected.iter().map(|t| format!("{}", t)).collect::<Vec<_>>().join(" or "), found),
        }
    }
}

impl ParseError {
    pub fn at(self, lo: FilePosition, hi: FilePosition) -> At<ParseError> {
        At {
            lo: lo,
            hi: hi,
            desc: self,
        }
    }
}

#[derive(Debug)]
pub enum TemplateWriteError {
    CanNotWriteMatchAnySymbols,
    MissingParam(String),
    Io(::std::io::Error),
}

impl PartialEq for TemplateWriteError {
    fn eq(&self, other: &TemplateWriteError) -> bool {
        match (self, other) {
            (&TemplateWriteError::CanNotWriteMatchAnySymbols, &TemplateWriteError::CanNotWriteMatchAnySymbols) => true,
            (&TemplateWriteError::MissingParam(ref a), &TemplateWriteError::MissingParam(ref b)) => a.eq(b),
            (&TemplateWriteError::Io(ref a), &TemplateWriteError::Io(ref b)) => a.description() == b.description(),
            (_, _) => false,
        }
    }
}

impl Eq for TemplateWriteError {}

impl ::std::error::Error for TemplateWriteError {
    fn description(&self) -> &str {
        match *self {
            TemplateWriteError::CanNotWriteMatchAnySymbols => "can not write template symbol to match any lines",
            TemplateWriteError::MissingParam(_) => "missing template param",
            TemplateWriteError::Io(ref e) => e.description(),
        }
    }
}

impl fmt::Display for TemplateWriteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TemplateWriteError::CanNotWriteMatchAnySymbols => "Can not write template symbol to match any lines".fmt(f),
            TemplateWriteError::MissingParam(ref p) => write!(f, "Missing template param {:?}", p),
            TemplateWriteError::Io(ref e) => e.fmt(f),
        }
    }
}

impl From<::std::io::Error> for TemplateWriteError {
    fn from(other: ::std::io::Error) -> Self {
        TemplateWriteError::Io(other)
    }
}

#[derive(Debug)]
pub enum TemplateMatchError {
    MissingParam(String),
    Io(::std::io::Error),
}

impl PartialEq for TemplateMatchError {
    fn eq(&self, other: &TemplateMatchError) -> bool {
        match (self, other) {
            (&TemplateMatchError::MissingParam(ref a), &TemplateMatchError::MissingParam(ref b)) => a.eq(b),
            (&TemplateMatchError::Io(ref a), &TemplateMatchError::Io(ref b)) => a.description() == b.description(),
            (_, _) => false,
        }
    }
}

impl Eq for TemplateMatchError {}

impl ::std::error::Error for TemplateMatchError {
    fn description(&self) -> &str {
        match *self {
            TemplateMatchError::MissingParam(_) => "missing template param",
            TemplateMatchError::Io(ref e) => e.description(),
        }
    }
}

impl fmt::Display for TemplateMatchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TemplateMatchError::MissingParam(ref p) => write!(f, "Missing template param {:?}", p),
            TemplateMatchError::Io(ref e) => e.fmt(f),
        }
    }
}

impl From<::std::io::Error> for TemplateMatchError {
    fn from(other: ::std::io::Error) -> Self {
        TemplateMatchError::Io(other)
    }
}

pub type LexResult<T> = result::Result<T, At<LexError>>;
pub type ParseResult<T> = result::Result<T, At<ParseError>>;

#[derive(Debug, Clone)]
pub struct At<T> where T: fmt::Debug + Clone {
    /// The low position at which this error is pointing at.
    pub lo: FilePosition,
    /// One byte beyond the last character at which this error is pointing at.
    pub hi: FilePosition,
    /// An inner error.
    pub desc: T,
}

impl<T: fmt::Debug + Clone> ::std::error::Error for At<T> where T: ::std::error::Error {
    fn description(&self) -> &str {
        self.desc.description()
    }
}

impl<T: fmt::Debug + Clone> PartialEq for At<T> where T: Eq + PartialEq {
    fn eq(&self, other: &At<T>) -> bool {
        self.desc == other.desc
    }
}

impl<T: fmt::Debug + Clone> fmt::Display for At<T> where T: fmt::Display {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} at {} - {}", self.desc, self.lo, self.hi)
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct FilePosition {
    /// 0-based line of this position.
    pub line: usize,
    /// 0-based col of this position.
    pub col: usize,
    /// The byte position at which this position is pointing at.
    pub byte: usize,
}

impl FilePosition {
    pub fn new() -> FilePosition {
        FilePosition {
            line: 0,
            col: 0,
            byte: 0,
        }
    }

    pub fn advance(&mut self, bytes: usize) {
        self.byte += bytes;
        self.col += bytes;
    }

    pub fn advanced(&self, bytes: usize) -> FilePosition {
        let mut other = self.clone();
        other.advance(bytes);
        other
    }

    pub fn next_line(&mut self, bytes: usize) {
        self.byte += bytes;
        self.col = 0;
        self.line += 1;
    }
}

impl fmt::Display for FilePosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "line {}, col {}", self.line, self.col)
    }
}