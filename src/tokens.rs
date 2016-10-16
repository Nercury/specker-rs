use std::fmt;
use std::result;
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
pub enum State {
    LineStart,
    ParamName,
    Contents,
}

#[derive(Copy, Clone, Debug)]
pub struct Iter<'a> {
    options: Options,
    state: State,
    remaining: &'a [u8],
}

impl<'a> Iter<'a> {
    fn next_bytes_eq(&self, other: &'static [u8]) -> bool {
        if self.remaining.len() < other.len() {
            return false;
        }
        &self.remaining[0..other.len()] == other
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Result<TokenRef<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        match self.state {
            State::LineStart => if self.next_bytes_eq(self.options.marker) {
                self.state = State::ParamName;
                return Some(Ok(TokenRef::Marker));
            } else {
                None
            },
            State::ParamName => None,
            State::Contents => None,
        }
    }
}

pub fn tokenize<'a>(options: Options, input: &'a [u8]) -> Iter<'a> {
    Iter {
        options: options,
        state: State::LineStart,
        remaining: input,
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

pub type Result<T> = result::Result<T, At<LexError>>;

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