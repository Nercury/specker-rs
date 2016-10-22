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
    skip_lines: &'static [u8],
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
                    if combinator::check_exact_bytes(&mut self.cursor, self.input, self.options.marker) {
                        LexState::ParamKey
                    } else {
                        LexState::ContentStart
                    }
                },
                LexState::ParamKey => {
                    debug!("STATE ParamKey");
                    let (name, termination) = try!(combinator::expect_terminated_text(&mut self.cursor, self.input, b":"));
                    self.token(TokenRef::Key(str::from_utf8(combinator::trim(name)).unwrap()));
                    match termination {
                        combinator::TermType::EolOrEof => LexState::Eol,
                        combinator::TermType::Sequence => LexState::ParamValue,
                    }
                },
                LexState::ParamValue => {
                    debug!("STATE ParamValue");
                    let name = try!(combinator::expect_text(&mut self.cursor, self.input));
                    self.token(TokenRef::Value(str::from_utf8(combinator::trim(name)).unwrap()));
                    LexState::Eol
                },
                LexState::ContentStart => {
                    debug!("STATE ContentStart");
                    if combinator::check_exact_bytes(&mut self.cursor, self.input, self.options.skip_lines) {
                        if combinator::check_new_line(&mut self.cursor, self.input) {
                            self.token(TokenRef::MatchAnyNumberOfLines);
                            LexState::LineStart
                        } else {
                            if self.cursor.byte == self.input.len() {
                                self.token(TokenRef::MatchAnyNumberOfLines);
                                LexState::Eol
                            } else {
                                return Err(LexError::ExpectedNewline.at(self.cursor.clone(), self.cursor.clone()));
                            }
                        }
                    } else {
                        LexState::ContentContinued
                    }
                },
                LexState::Var => {
                    debug!("STATE Var");
                    let (contents, termination) = try!(combinator::expect_terminated_text(
                        &mut self.cursor, self.input, self.options.var_end));
                    match termination {
                        combinator::TermType::EolOrEof => return Err(LexError::ExpectedSequenceFoundNewline {
                            expected: self.options.var_end
                        }.at(self.cursor.clone(), self.cursor.clone())),
                        combinator::TermType::Sequence => {
                            self.token(TokenRef::Var(str::from_utf8(combinator::trim(contents)).unwrap()));
                            LexState::ContentContinued
                        }
                    }
                },
                LexState::ContentContinued => {
                    debug!("STATE ContentContinued");
                    let (contents, termination) = try!(combinator::expect_terminated_text(&mut self.cursor, self.input, self.options.var_start));
                    if contents.len() > 0 {
                        self.token(TokenRef::MatchText(str::from_utf8(contents).unwrap()));
                    }
                    match termination {
                        combinator::TermType::EolOrEof => LexState::Eol,
                        combinator::TermType::Sequence => LexState::Var,
                    }
                },
                LexState::Eol => {
                    debug!("STATE Eol");
                    if combinator::check_new_line(&mut self.cursor, self.input) {
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

    fn default_options() -> Options {
        Options {
            skip_lines: b"..",
            marker: b"##",
            var_start: b"${",
            var_end: b"}"
        }
    }

    #[test]
    fn test_single_param_line() {
        let _ = env_logger::init();

        let mut tokens = tokenize(
            default_options(),
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
            default_options(),
            b"Blah blah blah"
        );

        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchText("Blah blah blah"))));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_line_with_var() {
        let _ = env_logger::init();

        let mut tokens;

        tokens = tokenize(default_options(), b"${ haha, yay }");
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Var("haha, yay"))));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_line_with_content_and_var() {
        let _ = env_logger::init();

        let mut tokens;

        tokens = tokenize(default_options(), b"Foo ${ haha, yay }");
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchText("Foo "))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Var("haha, yay"))));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_line_with_var_and_content() {
        let _ = env_logger::init();

        let mut tokens;

        tokens = tokenize(default_options(), b"${ haha, yay } Bar");
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Var("haha, yay"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchText(" Bar"))));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_line_mixed() {
        let _ = env_logger::init();

        let mut tokens;

        tokens = tokenize(default_options(), b"Foo ${ haha, yay } Bar");
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchText("Foo "))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Var("haha, yay"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchText(" Bar"))));
        assert_eq!(tokens.next(), None);

        tokens = tokenize(default_options(), b"Foo ${zai} Bar${x}");
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchText("Foo "))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Var("zai"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchText(" Bar"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Var("x"))));
        assert_eq!(tokens.next(), None);

        tokens = tokenize(default_options(), b"Foo ${}");
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchText("Foo "))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Var(""))));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_multi_line_params_and_content() {
        let _ = env_logger::init();

        let mut tokens;

        tokens = tokenize(default_options(), b"## lib: hello
${ X }");
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Key("lib"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Value("hello"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Var("X"))));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_multi_line_params_and_content_with_skipped_lines() {
        let _ = env_logger::init();

        let mut tokens;

        tokens = tokenize(default_options(), b"## lib: hello
..
${ X }
..");
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Key("lib"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Value("hello"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchAnyNumberOfLines)));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Var("X"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchAnyNumberOfLines)));
        assert_eq!(tokens.next(), None);

        tokens = tokenize(default_options(), b"## lib: hello
${ X }
..
");
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Key("lib"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Value("hello"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Var("X"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchAnyNumberOfLines)));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_multi_line_content_with_params() {
        let _ = env_logger::init();

        let mut tokens;

        tokens = tokenize(default_options(), b"..
${ X }
..
## a: b
## c: d
");
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchAnyNumberOfLines)));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Var("X"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchAnyNumberOfLines)));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Key("a"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Value("b"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Key("c"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Value("d"))));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_multi_line_varying_content_and_params() {
        let _ = env_logger::init();

        let mut tokens;

        tokens = tokenize(default_options(), b"## a: b
f ${ X } b
..
## c: d
..
k ${ Y } z
");
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Key("a"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Value("b"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchText("f "))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Var("X"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchText(" b"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchAnyNumberOfLines)));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Key("c"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Value("d"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchAnyNumberOfLines)));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchText("k "))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::Var("Y"))));
        assert_eq!(tokens.next(), Some(Ok(TokenRef::MatchText(" z"))));
        assert_eq!(tokens.next(), None);
    }
}