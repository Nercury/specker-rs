use std::io;
use std::fmt;
use std::result;
use std::error;

#[derive(Copy, Clone, Debug)]
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
    type Item = TokenRef<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}

#[derive(Debug)]
pub enum Error {
    Io(io::Error),
}

impl error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::Io(ref e) => e.description(),
        }
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::Io(ref e) => write!(f, "I/O error: {}", e),
        }
    }
}

impl From<io::Error> for Error {
    fn from(other: io::Error) -> Self {
        Error::Io(other)
    }
}

pub type Result<T> = result::Result<T, Error>;

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
        let tokens: Vec<_> = tokenize(
            Options {
                marker: b"##",
                var_start: b"${",
                var_end: b"}"
            },
            b"## lib: hello"
        ).collect();
    }
}