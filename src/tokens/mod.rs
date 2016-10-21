mod combinator;

use error::{At, FilePosition};
use std::collections::VecDeque;
use std::str;

pub use self::combinator::{LexError, Result};

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TokenRef<'a> {
    Key(&'a str),
    Value(&'a str),
    MatchAnyNumberOfLines,
    MatchText(&'a str),
    Var(&'a str),
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
    ContentStart,
    Var,
    ContentContinued,
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
    fn token(&mut self, token: TokenRef<'a>) {
        debug!("token: {:?}", token);
        self.tokens.push_back(token);
    }

    fn eat_bytes(&mut self, mut state: LexState) -> Result<LexState> {
        while self.tokens.is_empty() {
            state = match state {
                LexState::LineStart => {
                    debug!("STATE LineStart");
                    if combinator::try_exact_bytes(&mut self.cursor, self.input, self.options.marker) {
                        LexState::ParamKey
                    } else {
                        LexState::ContentStart
                    }
                },
                LexState::ParamKey => {
                    debug!("STATE ParamKey");
                    let name = try!(combinator::expect_name(&mut self.cursor, self.input, b" \t", b"\n\r:"));
                    self.token(TokenRef::Key(str::from_utf8(name).unwrap()));
                    if combinator::try_exact_bytes(&mut self.cursor, self.input, b":") {
                        LexState::ParamValue
                    } else {
                        LexState::Eol
                    }
                },
                LexState::ParamValue => {
                    debug!("STATE ParamValue");
                    let name = try!(combinator::expect_name(&mut self.cursor, self.input, b" \t", b"\n\r"));
                    self.token(TokenRef::Value(str::from_utf8(name).unwrap()));
                    LexState::Eol
                },
                LexState::ContentStart => {
                    debug!("STATE ContentStart");
                    let (contents, termination) = try!(combinator::expect_text_terminated_by_sequence_or_newline(&mut self.cursor, self.input, self.options.var_start));
                    if contents.len() > 0 {
                        self.token(TokenRef::MatchText(str::from_utf8(contents).unwrap()));
                    }
                    match termination {
                        combinator::TermType::EolOrEof => {
                            self.token(TokenRef::MatchAnyNumberOfLines);
                            LexState::Eol
                        },
                        combinator::TermType::Sequence => {
                            try!(combinator::expect_exact_bytes(&mut self.cursor, self.input, self.options.var_start));
                            LexState::Var
                        }
                    }
                },
                LexState::Var => {
                    debug!("STATE Var");
                    let (contents, termination) = try!(combinator::expect_text_terminated_by_sequence_or_newline(
                        &mut self.cursor, self.input, self.options.var_end));
                    match termination {
                        combinator::TermType::EolOrEof => return Err(LexError::ExpectedSequenceFoundNewline {
                            expected: self.options.var_end
                        }.at(self.cursor.clone(), self.cursor.clone())),
                        combinator::TermType::Sequence => {
                            self.token(TokenRef::Var(str::from_utf8(combinator::trim(contents)).unwrap()));
                            try!(combinator::expect_exact_bytes(&mut self.cursor, self.input, self.options.var_end));
                            LexState::ContentContinued
                        }
                    }
                },
                LexState::ContentContinued => {
                    debug!("STATE ContentContinued");
                    LexState::Eol
                },
                LexState::Eol => {
                    debug!("STATE Eol");
                    if combinator::try_new_line(&mut self.cursor, self.input) {
                        LexState::LineStart
                    } else {
                        break;
                    }
                },
            };
        }

        Ok(state)
    }
}

impl<'a> Iterator for Iter<'a> {
    type Item = Result<TokenRef<'a>>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let return_error = match self.state {
                IterState::End => {
                    return None;
                },
                IterState::Error(ref e) => {
                    Some(Err(e.clone()))
                },
                IterState::Lex(s) => {
                    match self.tokens.pop_front() {
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
                    };
                    continue;
                }
            };

            if let Some(e) = return_error {
                self.state = IterState::End;
                return Some(e);
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

#[cfg(test)]
mod tests {
    extern crate env_logger;

    use super::*;

    #[test]
    fn test_single_param_line() {
        let _ = env_logger::init();

        let mut tokens = tokenize(
            Options {
                marker: b"##",
                var_start: b"${",
                var_end: b"}"
            },
            b"## lib: hello"
        );

        assert_eq!(tokens.next(), Some(Ok(TokenRef::Key("lib"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Value("hello"))));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_content_line() {
        let _ = env_logger::init();

        let mut tokens = tokenize(
            Options {
                marker: b"##",
                var_start: b"${",
                var_end: b"}"
            },
            b"Blah blah blah"
        );

        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchText("Blah blah blah"))));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_line_with_var() {
        let _ = env_logger::init();
        let options = Options {
            marker: b"##",
            var_start: b"${",
            var_end: b"}"
        };

        let mut tokens;

        tokens = tokenize(options, b"${ haha, yay }");
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Var("haha, yay"))));
        assert_eq!(tokens.next(), None);
    }
}