use std::fmt;
use std::result;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LexError {
    ExpectedName,
    UnexpectedSymbol {
        expected: &'static [u8],
        found: Vec<u8>,
    },
    ExpectedSequenceFoundNewline {
        expected: &'static [u8],
    },
    ExpectedNewline,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LexError::ExpectedName => "expected name".fmt(f),
            LexError::UnexpectedSymbol { .. } => "unexpected symbol".fmt(f),
            LexError::ExpectedSequenceFoundNewline { .. } => "expected sequence, found newline".fmt(f),
            LexError::ExpectedNewline => "expected newline".fmt(f),
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

pub type LexResult<T> = result::Result<T, At<LexError>>;

#[derive(Debug, Clone)]
pub struct At<T> where T: fmt::Debug + Clone {
    /// The low position at which this error is pointing at.
    pub lo: FilePosition,
    /// One byte beyond the last character at which this error is pointing at.
    pub hi: FilePosition,
    /// An inner error.
    pub desc: T,
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

#[derive(Debug, Copy, Clone)]
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