// Copyright 2017 Nerijus Arlauskas
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

mod combinator;

use error::{At, FilePosition, LexError, LexResult};
use spec;
use std::collections::VecDeque;
use std::fmt;
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
    MatchNewline,
    MatchText(&'a str),
    Var(&'a str),
}

/// Lexer token value.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TokenValue {
    Key(String),
    Value(String),
    MatchAnyNumberOfLines,
    MatchNewline,
    MatchText(String),
    Var(String),
}

impl<'a> From<TokenValueRef<'a>> for TokenValue {
    fn from(other: TokenValueRef<'a>) -> Self {
        match other {
            TokenValueRef::Key(s) => TokenValue::Key(s.into()),
            TokenValueRef::Value(s) => TokenValue::Value(s.into()),
            TokenValueRef::MatchAnyNumberOfLines => TokenValue::MatchAnyNumberOfLines,
            TokenValueRef::MatchNewline => TokenValue::MatchNewline,
            TokenValueRef::MatchText(s) => TokenValue::MatchText(s.into()),
            TokenValueRef::Var(s) => TokenValue::Var(s.into()),
        }
    }
}

impl fmt::Display for TokenValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TokenValue::Key(_) => "key".fmt(f),
            TokenValue::Value(_) => "value".fmt(f),
            TokenValue::MatchAnyNumberOfLines => "match lines".fmt(f),
            TokenValue::MatchNewline => "match new line".fmt(f),
            TokenValue::MatchText(_) => "match text".fmt(f),
            TokenValue::Var(_) => "variable".fmt(f),
        }
    }
}

#[derive(Copy, Clone, Debug)]
pub struct Options<'a> {
    pub skip_lines: &'a [u8],
    pub marker: &'a [u8],
    pub var_start: &'a [u8],
    pub var_end: &'a [u8],
}

impl<'a> From<spec::Options<'a>> for Options<'a> {
    fn from(other: spec::Options<'a>) -> Options<'a> {
        Options {
            skip_lines: other.skip_lines.as_bytes(),
            marker: other.marker.as_bytes(),
            var_start: other.var_start.as_bytes(),
            var_end: other.var_end.as_bytes(),
        }
    }
}

#[derive(Copy, Clone, Debug)]
enum LexState {
    LineStart {
        content_line_end: Option<(FilePosition, FilePosition)>,
    },
    ParamKey,
    ParamValue,
    ContentStart {
        content_line_end: Option<(FilePosition, FilePosition)>,
    },
    Var,
    ContentContinued,
    ContentEol,
    Eol,
}

#[derive(Clone, Debug)]
enum IterState {
    Lex(LexState),
    Error(At<LexError>),
    End,
}

#[derive(Clone, Debug)]
pub struct Iter<'a> {
    options: Options<'a>,
    state: IterState,
    tokens: VecDeque<TokenRef<'a>>,
    cursor: FilePosition,
    input: &'a [u8],
}

impl<'a> Iter<'a> {
    fn token(&mut self, token: TokenValueRef<'a>, lo: FilePosition, hi: FilePosition) {
        self.tokens.push_back(TokenRef {
            value: token,
            lo: lo,
            hi: hi,
        });
    }

    fn eat_bytes(&mut self, mut state: LexState) -> LexResult<LexState> {
        while self.tokens.is_empty() {
            state = match state {
                LexState::LineStart { content_line_end } => {
                    if combinator::check_exact_bytes(
                        &mut self.cursor,
                        self.input,
                        self.options.marker,
                    ) {
                        LexState::ParamKey
                    } else {
                        LexState::ContentStart {
                            content_line_end: content_line_end,
                        }
                    }
                }
                LexState::ParamKey => {
                    let (contents, termination) =
                        combinator::expect_terminated_text(&mut self.cursor, self.input, b":")?;
                    let trimmed = contents.trimmed();
                    self.token(
                        TokenValueRef::Key(str::from_utf8(trimmed.slice)
                            .map_err(|e| LexError::from(e).at(trimmed.lo, trimmed.hi))?),
                        trimmed.lo,
                        trimmed.hi,
                    );
                    match termination {
                        combinator::TermType::EolOrEof => LexState::Eol,
                        combinator::TermType::Sequence => LexState::ParamValue,
                    }
                }
                LexState::ParamValue => {
                    let name = combinator::expect_text(&mut self.cursor, self.input)?.trimmed();
                    self.token(
                        TokenValueRef::Value(str::from_utf8(name.slice)
                            .map_err(|e| LexError::from(e).at(name.lo, name.hi))?),
                        name.lo,
                        name.hi,
                    );
                    LexState::Eol
                }
                LexState::ContentStart { content_line_end } => {
                    if combinator::check_exact_bytes(
                        &mut self.cursor,
                        self.input,
                        self.options.skip_lines,
                    ) {
                        let pos = self.cursor.clone();
                        if combinator::check_new_line(&mut self.cursor, self.input) {
                            self.token(TokenValueRef::MatchAnyNumberOfLines, pos, pos);
                            LexState::LineStart {
                                content_line_end: None,
                            }
                        } else {
                            if self.cursor.byte == self.input.len() {
                                self.token(TokenValueRef::MatchAnyNumberOfLines, pos, pos);
                                LexState::Eol
                            } else {
                                return Err(LexError::ExpectedNewline
                                    .at(self.cursor.clone(), self.cursor.clone()));
                            }
                        }
                    } else {
                        if let Some((new_line_start, new_line_end)) = content_line_end {
                            if !combinator::check_eof(&mut self.cursor, self.input) {
                                self.token(
                                    TokenValueRef::MatchNewline,
                                    new_line_start,
                                    new_line_end,
                                );
                            }
                        }
                        LexState::ContentContinued
                    }
                }
                LexState::Var => {
                    let (contents, termination) = combinator::expect_terminated_text(
                        &mut self.cursor,
                        self.input,
                        self.options.var_end,
                    )?;
                    match termination {
                        combinator::TermType::EolOrEof => {
                            return Err(LexError::ExpectedSequenceFoundNewline {
                                expected: self.options.var_end.into(),
                            }.at(self.cursor.clone(), self.cursor.clone()))
                        }
                        combinator::TermType::Sequence => {
                            let trimmed = contents.trimmed();
                            self.token(
                                TokenValueRef::Var(str::from_utf8(trimmed.slice)
                                    .map_err(|e| LexError::from(e).at(trimmed.lo, trimmed.hi))?),
                                trimmed.lo,
                                trimmed.hi,
                            );
                            LexState::ContentContinued
                        }
                    }
                }
                LexState::ContentContinued => {
                    let (contents, termination) = combinator::expect_terminated_text(
                        &mut self.cursor,
                        self.input,
                        self.options.var_start,
                    )?;
                    if contents.slice.len() > 0 {
                        self.token(
                            TokenValueRef::MatchText(str::from_utf8(contents.slice)
                                .map_err(|e| LexError::from(e).at(contents.lo, contents.hi))?),
                            contents.lo,
                            contents.hi,
                        );
                    }
                    match termination {
                        combinator::TermType::EolOrEof => LexState::ContentEol,
                        combinator::TermType::Sequence => LexState::Var,
                    }
                }
                LexState::ContentEol => {
                    let lo = self.cursor.clone();
                    if combinator::check_new_line(&mut self.cursor, self.input) {
                        LexState::LineStart {
                            content_line_end: Some((lo, self.cursor.clone())),
                        }
                    } else {
                        break;
                    }
                }
                LexState::Eol => {
                    if combinator::check_new_line(&mut self.cursor, self.input) {
                        LexState::LineStart {
                            content_line_end: None,
                        }
                    } else {
                        break;
                    }
                }
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
                }
                IterState::Error(ref e) => Some(Err(e.clone())),
                IterState::Lex(s) => {
                    match self.tokens.pop_front() {
                        Some(token) => return Some(Ok(token)),
                        None => {
                            self.state = match self.eat_bytes(s) {
                                Ok(lex_state) => {
                                    if self.tokens.is_empty() {
                                        IterState::End
                                    } else {
                                        IterState::Lex(lex_state)
                                    }
                                }
                                Err(e) => IterState::Error(e),
                            }
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

pub fn tokenize<'a>(options: Options<'a>, input: &'a [u8]) -> Iter<'a> {
    Iter {
        options: options,
        state: IterState::Lex(LexState::LineStart {
            content_line_end: None,
        }),
        tokens: VecDeque::new(),
        cursor: FilePosition::new(),
        input: input,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn default_options() -> Options<'static> {
        Options {
            skip_lines: b"..",
            marker: b"##",
            var_start: b"${",
            var_end: b"}",
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
        let mut tokens = tokenize(default_options(), b"## lib: hello");

        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("lib"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("hello"));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_content_line() {
        let mut tokens = tokenize(default_options(), b"Blah blah blah");

        assert_eq!(
            expect_next(&mut tokens),
            TokenValueRef::MatchText("Blah blah blah")
        );
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_line_with_var() {
        let mut tokens;

        tokens = tokenize(default_options(), b"${ haha, yay }");
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("haha, yay"));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_line_with_content_and_var() {
        let mut tokens;

        tokens = tokenize(default_options(), b"Foo ${ haha, yay }");
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText("Foo "));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("haha, yay"));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_line_with_var_and_content() {
        let mut tokens;

        tokens = tokenize(default_options(), b"${ haha, yay } Bar");
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("haha, yay"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText(" Bar"));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_single_line_mixed() {
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
        let mut tokens;

        tokens = tokenize(
            default_options(),
            b"## lib: hello
${ X }",
        );
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("lib"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("hello"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("X"));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_multi_line_params_and_content_with_skipped_lines() {
        let mut tokens;

        tokens = tokenize(
            default_options(),
            b"## lib: hello
..
${ X }
..",
        );
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("lib"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("hello"));
        assert_eq!(
            expect_next(&mut tokens),
            TokenValueRef::MatchAnyNumberOfLines
        );
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("X"));
        assert_eq!(
            expect_next(&mut tokens),
            TokenValueRef::MatchAnyNumberOfLines
        );
        assert_eq!(tokens.next(), None);

        tokens = tokenize(
            default_options(),
            b"## lib: hello
${ X }
..
",
        );
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("lib"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("hello"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("X"));
        assert_eq!(
            expect_next(&mut tokens),
            TokenValueRef::MatchAnyNumberOfLines
        );
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_multi_line_content_with_params() {
        let mut tokens;

        tokens = tokenize(
            default_options(),
            b"..
${ X }
..
## a: b
## c: d
",
        );
        assert_eq!(
            expect_next(&mut tokens),
            TokenValueRef::MatchAnyNumberOfLines
        );
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("X"));
        assert_eq!(
            expect_next(&mut tokens),
            TokenValueRef::MatchAnyNumberOfLines
        );
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("a"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("b"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("c"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("d"));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_multi_line_varying_content_and_params() {
        let mut tokens;

        tokens = tokenize(
            default_options(),
            b"## a: b
f ${ X } b
..
## c: d
..
k ${ Y } z
",
        );
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("a"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("b"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText("f "));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("X"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText(" b"));
        assert_eq!(
            expect_next(&mut tokens),
            TokenValueRef::MatchAnyNumberOfLines
        );
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Key("c"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Value("d"));
        assert_eq!(
            expect_next(&mut tokens),
            TokenValueRef::MatchAnyNumberOfLines
        );
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText("k "));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::Var("Y"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText(" z"));
        assert_eq!(tokens.next(), None);
    }

    #[test]
    fn test_newline_match_tokens() {
        let mut tokens;

        tokens = tokenize(
            default_options(),
            b"..
a
b
..
",
        );
        assert_eq!(
            expect_next(&mut tokens),
            TokenValueRef::MatchAnyNumberOfLines
        );
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText("a"));
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchNewline);
        assert_eq!(expect_next(&mut tokens), TokenValueRef::MatchText("b"));
        assert_eq!(
            expect_next(&mut tokens),
            TokenValueRef::MatchAnyNumberOfLines
        );
        assert_eq!(tokens.next(), None);
    }
}
