mod combinator;

use error::{At, FilePosition, LexError, LexResult};
use std::collections::VecDeque;
use std::str;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub struct TokenRef<'a> {
    pub value: TokenValueRef<'a>,
    /// The low position at which this token exists.
    pub lo: FilePosition,
    /// One byte beyond the last character at which token ends.
    pub hi: FilePosition,
}

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
pub enum TokenValueRef<'a> {
    Key(&'a str),
    Value(&'a str),
    MatchAnyNumberOfLines,
    MatchText(&'a str),
    Var(&'a str),
}

#[derive(Copy, Clone, Debug)]
pub struct Options {
    pub skip_lines: &'static [u8],
    pub marker: &'static [u8],
    pub var_start: &'static [u8],
    pub var_end: &'static [u8],
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
    fn token(&mut self, token: TokenValueRef<'a>, lo: FilePosition, hi: FilePosition) {
        debug!("token: {:?}", token);
        self.tokens.push_back(TokenRef { value: token, lo: lo, hi: hi });
    }

    fn eat_bytes(&mut self, mut state: LexState) -> LexResult<LexState> {
        while self.tokens.is_empty() {
            state = match state {
                LexState::LineStart => {
                    if combinator::check_exact_bytes(&mut self.cursor, self.input, self.options.marker) {
                        LexState::ParamKey
                    } else {
                        LexState::ContentStart
                    }
                },
                LexState::ParamKey => {
                    let (contents, termination) = try!(combinator::expect_terminated_text(&mut self.cursor, self.input, b":"));
                    let trimmed = contents.trimmed();
                    self.token(
                        TokenValueRef::Key(str::from_utf8(trimmed.slice).unwrap()),
                        trimmed.lo,
                        trimmed.hi
                    );
                    match termination {
                        combinator::TermType::EolOrEof => LexState::Eol,
                        combinator::TermType::Sequence => LexState::ParamValue,
                    }
                },
                LexState::ParamValue => {
                    let name = try!(combinator::expect_text(&mut self.cursor, self.input)).trimmed();
                    self.token(TokenValueRef::Value(str::from_utf8(name.slice).unwrap()), name.lo, name.hi);
                    LexState::Eol
                },
                LexState::ContentStart => {
                    if combinator::check_exact_bytes(&mut self.cursor, self.input, self.options.skip_lines) {
                        let pos = self.cursor.clone();
                        if combinator::check_new_line(&mut self.cursor, self.input) {
                            self.token(TokenValueRef::MatchAnyNumberOfLines, pos, pos);
                            LexState::LineStart
                        } else {
                            if self.cursor.byte == self.input.len() {
                                self.token(TokenValueRef::MatchAnyNumberOfLines, pos, pos);
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
                    let (contents, termination) = try!(combinator::expect_terminated_text(
                        &mut self.cursor, self.input, self.options.var_end));
                    match termination {
                        combinator::TermType::EolOrEof => return Err(LexError::ExpectedSequenceFoundNewline {
                            expected: self.options.var_end
                        }.at(self.cursor.clone(), self.cursor.clone())),
                        combinator::TermType::Sequence => {
                            let trimmed = contents.trimmed();
                            self.token(TokenValueRef::Var(str::from_utf8(trimmed.slice).unwrap()), trimmed.lo, trimmed.hi);
                            LexState::ContentContinued
                        }
                    }
                },
                LexState::ContentContinued => {
                    let (contents, termination) = try!(combinator::expect_terminated_text(&mut self.cursor, self.input, self.options.var_start));
                    if contents.slice.len() > 0 {
                        self.token(
                            TokenValueRef::MatchText(str::from_utf8(contents.slice).unwrap()),
                            contents.lo,
                            contents.hi
                        );
                    }
                    match termination {
                        combinator::TermType::EolOrEof => LexState::Eol,
                        combinator::TermType::Sequence => LexState::Var,
                    }
                },
                LexState::Eol => {
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
    type Item = LexResult<TokenRef<'a>>;

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

    pub fn expect_next<'a, 'r>(iter: &'r mut Iter<'a>) -> TokenValueRef<'a> {
        match iter.next() {
            Some(Ok(TokenRef { value, .. })) => value,
            o => panic!("expected token value but got {:?}", o),
        }
    }

    #[test]
    fn test_single_param_line() {
        let _ = env_logger::init();

        let mut tokens = tokenize(
            default_options(),
            b"## lib: hello"
        );

        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("lib"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("hello"));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_content_line() {
        let _ = env_logger::init();

        let mut tokens = tokenize(
            default_options(),
            b"Blah blah blah"
        );

        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText("Blah blah blah"));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_line_with_var() {
        let _ = env_logger::init();

        let mut tokens;

        tokens = tokenize(default_options(), b"${ haha, yay }");
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("haha, yay"));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_line_with_content_and_var() {
        let _ = env_logger::init();

        let mut tokens;

        tokens = tokenize(default_options(), b"Foo ${ haha, yay }");
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText("Foo "));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("haha, yay"));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_line_with_var_and_content() {
        let _ = env_logger::init();

        let mut tokens;

        tokens = tokenize(default_options(), b"${ haha, yay } Bar");
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("haha, yay"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText(" Bar"));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_line_mixed() {
        let _ = env_logger::init();

        let mut tokens;

        tokens = tokenize(default_options(), b"Foo ${ haha, yay } Bar");
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText("Foo "));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("haha, yay"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText(" Bar"));
        assert_eq!(tokens.next(), None);

        tokens = tokenize(default_options(), b"Foo ${zai} Bar${x}");
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText("Foo "));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("zai"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText(" Bar"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("x"));
        assert_eq!(tokens.next(), None);

        tokens = tokenize(default_options(), b"Foo ${}");
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText("Foo "));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var(""));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_multi_line_params_and_content() {
        let _ = env_logger::init();

        let mut tokens;

        tokens = tokenize(default_options(), b"## lib: hello
${ X }");
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("lib"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("hello"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("X"));
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
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("lib"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("hello"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchAnyNumberOfLines);
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("X"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchAnyNumberOfLines);
        assert_eq!(tokens.next(), None);

        tokens = tokenize(default_options(), b"## lib: hello
${ X }
..
");
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("lib"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("hello"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("X"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchAnyNumberOfLines);
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
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchAnyNumberOfLines);
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("X"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchAnyNumberOfLines);
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("a"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("b"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("c"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("d"));
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
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("a"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("b"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText("f "));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("X"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText(" b"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchAnyNumberOfLines);
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("c"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("d"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchAnyNumberOfLines);
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText("k "));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("Y"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText(" z"));
        assert_eq!(tokens.next(), None);
    }
}