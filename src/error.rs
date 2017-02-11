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
            ParseError::ExpectedDifferentToken { ref expected, ref found } => {
                write!(
                    f,
                    "Expected {}, buf found {}",
                    expected.iter()
                        .map(|t| format!("{}", t))
                        .collect::<Vec<_>>()
                        .join(" or "),
                    found
                )
            },
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
    ExpectedEof,
    ExpectedText {
        expected: String,
        found: String
    },
    ExpectedLineFoundEof,
    ExpectedTextFoundEof(String),
    MissingParam(String),
    Io(::std::io::Error),
}

impl TemplateMatchError {
    pub fn at(self, lo: FilePosition, hi: FilePosition) -> At<TemplateMatchError> {
        At {
            lo: lo,
            hi: hi,
            desc: self,
        }
    }
}

impl PartialEq for TemplateMatchError {
    fn eq(&self, other: &TemplateMatchError) -> bool {
        match (self, other) {
            (&TemplateMatchError::ExpectedEof, &TemplateMatchError::ExpectedEof) => true,
            (
                &TemplateMatchError::ExpectedText {
                    expected: ref expected_a,
                    found: ref found_a
                },
                &TemplateMatchError::ExpectedText {
                    expected: ref expected_b,
                    found: ref found_b
                }
            ) => expected_a.eq(expected_b) && found_a.eq(found_b),
            (&TemplateMatchError::ExpectedTextFoundEof(ref a), &TemplateMatchError::ExpectedTextFoundEof(ref b)) => a.eq(b),
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
            TemplateMatchError::ExpectedEof => "expected end of file",
            TemplateMatchError::ExpectedText { .. } => "expected text not found",
            TemplateMatchError::ExpectedTextFoundEof(_) => "expected text, found end of file",
            TemplateMatchError::ExpectedLineFoundEof => "expected line, found end of file",
            TemplateMatchError::MissingParam(_) => "missing template param",
            TemplateMatchError::Io(ref e) => e.description(),
        }
    }
}

impl fmt::Display for TemplateMatchError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TemplateMatchError::ExpectedEof => "Expected end of file".fmt(f),
            TemplateMatchError::ExpectedText { ref expected, ref found } => write!(f, "Expected {:?}, found {:?}", expected, found),
            TemplateMatchError::ExpectedTextFoundEof(ref p) => write!(f, "Expected {:?}, found end of file", p),
            TemplateMatchError::ExpectedLineFoundEof => "Expected line, found end of file".fmt(f),
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
pub struct At<T> where T: fmt::Debug {
    /// The low position at which this error is pointing at.
    pub lo: FilePosition,
    /// One byte beyond the last character at which this error is pointing at.
    pub hi: FilePosition,
    /// An inner error.
    pub desc: T,
}

impl<T: fmt::Debug> At<T> {
    pub fn assert_matches(&self, other_err: &T, lo: (usize, usize), hi: (usize, usize)) -> result::Result<(), String> where T: PartialEq {
        if !self.desc.eq(other_err) {
            return Err(format!("{:?} does not match {:?}", self.desc, other_err));
        }

        if self.lo.line != lo.0 {
            return Err(format!("expected error start line at {}, found {}", lo.0, self.lo.line));
        }

        if self.hi.line != hi.0 {
            return Err(format!("expected error end line at {}, found {}", hi.0, self.hi.line));
        }

        if self.lo.col != lo.1 {
            return Err(format!("expected error start col at {}, found {}", lo.1, self.lo.col));
        }

        if self.hi.col != hi.1 {
            return Err(format!("expected error end col at {}, found {}", hi.1, self.hi.col));
        }

        Ok(())
    }
}

impl<T: fmt::Debug> ::std::error::Error for At<T> where T: ::std::error::Error {
    fn description(&self) -> &str {
        self.desc.description()
    }
}

impl<T: fmt::Debug> PartialEq for At<T> where T: Eq + PartialEq {
    fn eq(&self, other: &At<T>) -> bool {
        self.desc == other.desc
    }
}

impl<T: fmt::Debug> fmt::Display for At<T> where T: fmt::Display {
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
        if bytes > 0 {
            self.byte += bytes;
            self.col = 0;
            self.line += 1;
        }
    }
}

impl fmt::Display for FilePosition {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "line {}, col {}", self.line, self.col)
    }
}