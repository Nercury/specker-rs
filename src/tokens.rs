use std::fmt;
use std::result;
use error::{At, FilePosition};
use std::collections::VecDeque;
use std::str;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TokenRef<'a> {
    Marker,
    Key(&'a str),
    Colon,
    Value(&'a str),
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
pub enum LexState {
    LineStart,
    ParamKey,
    ParamValue,
    Contents,
    Eol,
}

#[derive(Clone, Debug)]
pub enum IterState {
    Lex(LexState),
    Error(At<LexError>),
    End,
}

#[derive(Clone, Debug)]
pub struct Iter<'a> {
    options: Options,
    state: IterState,
    tokens: VecDeque<TokenRef<'a>>,
    cursor: FilePosition,
    input: &'a [u8],
}

impl<'a> Iter<'a> {
    fn eat_bytes(&mut self, state: LexState) -> Result<LexState> {
        Ok(match state {
            LexState::LineStart => if try_exact_bytes(&mut self.cursor, self.input, self.options.marker) {
                self.tokens.push_back(TokenRef::Marker);
                LexState::ParamKey
            } else {
                LexState::Contents
            },
            LexState::ParamKey => {
                let name = try!(expect_name(&mut self.cursor, self.input, b" \t", b"\n\r:"));
                self.tokens.push_back(TokenRef::Key(str::from_utf8(name).unwrap()));
                if try_exact_bytes(&mut self.cursor, self.input, b":") {
                    self.tokens.push_back(TokenRef::Colon);
                    LexState::ParamValue
                } else {
                    LexState::Eol
                }
            },
            LexState::ParamValue => LexState::ParamValue,
            LexState::Contents => LexState::Contents,
            LexState::Eol => LexState::Eol,
        })
    }
}

fn try_exact_bytes(cursor: &mut FilePosition, input: &[u8], other: &'static [u8]) -> bool {
    if input.len() - cursor.byte < other.len() {
        return false;
    }
    let matches = &input[cursor.byte..cursor.byte + other.len()] == other;
    if matches {
        cursor.advance(other.len());
    }
    matches
}

fn expect_name<'a>(cursor: &mut FilePosition, input: &'a [u8], whitespace: &'static [u8], terminators: &'static [u8]) -> Result<&'a [u8]> {

    let mut start = cursor.byte;
    loop {
        if start >= input.len() || terminators.contains(&input[start]) {
            return Err(LexError::ExpectedName.at(*cursor, cursor.advanced(start - cursor.byte)));
        }
        if whitespace.contains(&input[start]) {
            start += 1;
            continue;
        }
        break;
    }

    let mut name_end = start + 1;
    loop {
        if start >= input.len() || terminators.contains(&input[name_end]) {
            return Ok(&input[start..name_end]);
        }
        if whitespace.contains(&input[name_end]) {
            break;
        }
        name_end += 1;
    }

    let mut end = name_end + 1;
    loop {
        if start >= input.len() || terminators.contains(&input[end]) {
            break;
        }
        if !whitespace.contains(&input[end]) {
            return Err(
                LexError::UnexpectedSymbol {
                    expected: terminators,
                    found: input[end]
                }.at(cursor.advanced(start - cursor.byte), cursor.advanced(name_end - cursor.byte))
            );
        }
        end += 1;
    }

    cursor.advance(end - start);

    Ok(&input[start..name_end])
}

impl<'a> Iterator for Iter<'a> {
    type Item = Result<TokenRef<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.state {
                IterState::End => return None,
                IterState::Error(ref e) => return Some(Err(e.clone())),
                IterState::Lex(s) => match self.tokens.pop_front() {
                    Some(token) => return Some(Ok(token)),
                    None => self.state = match self.eat_bytes(s) {
                        Ok(lex_state) => {
                            if self.tokens.is_empty() {
                                IterState::End
                            } else {
                                IterState::Lex(lex_state)
                            }
                        },
                        Err(e) => IterState::Error(e),
                    }
                }
            }
        }
    }
}

pub fn tokenize<'a>(options: Options, input: &'a [u8]) -> Iter<'a> {
    Iter {
        options: options,
        state: IterState::Lex(LexState::LineStart),
        tokens: VecDeque::new(),
        cursor: FilePosition::new(),
        input: input,
    }
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum LexError {
    ExpectedName,
    UnexpectedSymbol {
        expected: &'static [u8],
        found: u8
    },
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LexError::ExpectedName => "expected name".fmt(f),
            LexError::UnexpectedSymbol { .. } => "unexpected symbol".fmt(f),
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
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Key("lib"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Colon)));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Value("hello"))));
    }
}