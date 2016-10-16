use std::io;
use std::fmt;
use std::result;
use std::error;
use error::{At, FilePosition};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TokenRef<'a> {
    Marker,
    Name(&'a str),
    Colon,
    MatchAnyNumberOfLines,
    MatchText(&'a str),
    VarStart,
    VarEnd,
}

#[derive(Copy, Clone, Debug)]
pub struct Options {
    marker: &'static [u8],
    var_start: &'static [u8],
    var_end: &'static [u8],
}

#[derive(Copy, Clone, Debug)]
pub enum TokenizeState {
    ChunkStart,
}

#[derive(Copy, Clone, Debug)]
pub struct Iter<'a> {
    options: Options,
    state: TokenizeState,
    remaining: &'a [u8],
}

impl<'a> Iterator for Iter<'a> {
    type Item = Result<TokenRef<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

pub fn tokenize<'a>(options: Options, input: &'a [u8]) -> Iter<'a> {
    Iter {
        options: options,
        state: TokenizeState::ChunkStart,
        remaining: input,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tokenizer() {
        let mut tokens = tokenize(
            Options {
                marker: b"##",
                var_start: b"${",
                var_end: b"}"
            },
            b"## lib: hello"
        );

        assert_eq!(tokens.next(), Some(Ok(TokenRef::Marker)));
    }
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
    Lex(At<LexError>),
}

impl Eq for Error {}

impl PartialEq for Error {
    fn eq(&self, other: &Error) -> bool {
        match (self, other) {
            (&Error::Io(_), &Error::Io(_)) => true,
            (&Error::Lex(ref a), &Error::Lex(ref b)) => a == b,
            _ => false,
        }
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum LexError {}

impl fmt::Display for LexError {
    fn fmt(&self, _f: &mut fmt::Formatter) -> fmt::Result {
        match *self {}
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

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref e) => e.description(),
            Error::Lex(_) => "lexer error",
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref e) => write!(f, "I/O error: {}", e),
            Error::Lex(ref e) => e.fmt(f),
        }
    }
}

impl From<io::Error> for Error {
    fn from(other: io::Error) -> Self {
        Error::Io(other)
    }
}

pub type Result<T> = result::Result<T, Error>;