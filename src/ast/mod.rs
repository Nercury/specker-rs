// Copyright 2017 Nerijus Arlauskas
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use tokens::{self, TokenValue, TokenRef, TokenValueRef};
use error::{FilePosition, ParseError, ParseResult};
use std::iter::Peekable;

/// Top item of specification AST.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Spec {
    /// Specification items.
    pub items: Vec<Item>,
}

/// Specification item that corresponds to a file match pattern.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Item {
    /// Item params used to differentiate items when running the specification match or write.
    pub params: Vec<Param>,
    /// Parsed item tokens.
    pub template: Vec<Match>,
}

/// Specification item parameter.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Param {
    /// Parameter key.
    pub key: String,
    /// Parameter value.
    pub value: Option<String>,
}

/// Specification token.
#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Match {
    /// Match one or more lines containing anything.
    MultipleLines,
    /// Match a newline.
    NewLine,
    /// Match specific text.
    Text(String),
    /// Match a variable from a map that will be provided when running match.
    Var(String),
}

/// Specification parser.
pub struct Parser<'s> {
    token_iter: Peekable<tokens::Iter<'s>>,
    pos: FilePosition,
}

impl<'s> Parser<'s> {
    pub fn new(token_iter: Peekable<tokens::Iter<'s>>) -> Parser<'s> {
        Parser {
            token_iter: token_iter,
            pos: FilePosition::new(),
        }
    }

    pub fn parse_spec(&mut self) -> ParseResult<Spec> {
        let mut items = Vec::new();

        while let Some(item) = self.parse_item()? {
            items.push(item);
        }

        Ok(Spec { items: items })
    }

    fn parse_item(&mut self) -> ParseResult<Option<Item>> {
        let item = Item {
            params: self.parse_params()?,
            template: self.parse_template()?,
        };

        if item.params.is_empty() && item.template.is_empty() {
            return Ok(None);
        }

        Ok(Some(item))
    }

    fn parse_template(&mut self) -> ParseResult<Vec<Match>> {
        let mut items = Vec::new();

        while self.check_next_token_is_template_item()? {
            items.push(match self.expect_template_token()? {
                TokenValueRef::MatchAnyNumberOfLines => Match::MultipleLines,
                TokenValueRef::MatchText(s) => Match::Text(s.into()),
                TokenValueRef::MatchNewline => Match::NewLine,
                TokenValueRef::Var(s) => Match::Var(s.into()),
                _ => break,
            });
        }

        Ok(items)
    }

    fn parse_params(&mut self) -> ParseResult<Vec<Param>> {
        let mut params = Vec::new();

        loop {
            if match self.check_next_token_is_key()? {
                None => return Ok(params),
                Some(v) => v,
            } {
                let key = self.expect_key()?;
                params.push(Param {
                    key: key.into(),
                    value: if self.check_next_token_is_value()? {
                        Some(self.expect_value()?.into())
                    } else {
                        None
                    },
                })
            } else {
                break;
            }
        }

        Ok(params)
    }

    fn check_next_token_is_template_item(&mut self) -> ParseResult<bool> {
        Ok(match self.token_iter.peek() {
            None => false,
            Some(&Err(ref e)) => return Err(e.clone().into()),
            Some(&Ok(TokenRef { value, .. })) => match value {
                TokenValueRef::MatchAnyNumberOfLines => true,
                TokenValueRef::MatchText(_) => true,
                TokenValueRef::MatchNewline => true,
                TokenValueRef::Var(_) => true,
                _ => false,
            }
        })
    }

    fn check_next_token_is_key(&mut self) -> ParseResult<Option<bool>> {
        Ok(match self.token_iter.peek() {
            None => None,
            Some(&Err(ref e)) => return Err(e.clone().into()),
            Some(&Ok(TokenRef { value, lo, hi })) => match value {
                TokenValueRef::Key(_) => Some(true),
                TokenValueRef::Value(_) => return Err(ParseError::ExpectedKeyFoundValue.at(lo, hi)),
                _ => Some(false),
            }
        })
    }

    fn check_next_token_is_value(&mut self) -> ParseResult<bool> {
        Ok(match self.token_iter.peek() {
            None => false,
            Some(&Err(ref e)) => return Err(e.clone().into()),
            Some(&Ok(TokenRef { value, .. })) => match value {
                TokenValueRef::Value(_) => true,
                _ => false,
            }
        })
    }

    fn expect_template_token(&mut self) -> ParseResult<TokenValueRef<'s>> {
        self.expect_token(|token: TokenValueRef<'s>| {
            match token {
                TokenValueRef::MatchAnyNumberOfLines
                | TokenValueRef::MatchText(_)
                | TokenValueRef::MatchNewline
                | TokenValueRef::Var(_) => Some(token),
                _ => None,
            }
        }, || vec![
        TokenValue::MatchAnyNumberOfLines,
        TokenValue::MatchText(String::from("_")),
        TokenValue::Var(String::from("_"))
        ])
    }

    fn expect_key(&mut self) -> ParseResult<&'s str> {
        self.expect_token(|token: TokenValueRef<'s>| {
            if let TokenValueRef::Key(s) = token {
                Some(s)
            } else {
                None
            }
        }, || vec![TokenValue::Key(String::from("_"))])
    }

    fn expect_value(&mut self) -> ParseResult<&'s str> {
        self.expect_token(|token: TokenValueRef<'s>| {
            if let TokenValueRef::Value(s) = token {
                Some(s)
            } else {
                None
            }
        }, || vec![TokenValue::Value(String::from("_"))])
    }

    fn expect_token<F, R, E>(&mut self, match_token: F, expected_token_value: E) -> ParseResult<R> where
        F: Fn(TokenValueRef<'s>) -> Option<R>,
        E: Fn() -> Vec<TokenValue>
    {
        match self.token_iter.next() {
            None => Err(ParseError::UnexpectedEndOfTokens.at(self.pos, self.pos)),
            Some(Err(e)) => Err(e.into()),
            Some(Ok(TokenRef { value, lo, hi })) => {
                self.pos = hi;
                if let Some(r) = match_token(value) {
                    Ok(r)
                } else {
                    Err(ParseError::ExpectedDifferentToken {
                        expected: expected_token_value(),
                        found: value.into()
                    }.at(lo, hi))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokens::{Options, tokenize};

    fn default_options() -> Options<'static> {
        Options {
            skip_lines: b"..",
            marker: b"##",
            var_start: b"${",
            var_end: b"}"
        }
    }

    #[test]
    fn test_parser() {
        let tokens = tokenize(default_options(), b"## a: x
..
Hello ${ X }
Bye
..
## a: y
## bbbb
${ X } woooo
${ Y }
");
        let mut parser = Parser::new(tokens.peekable());
        let spec = parser.parse_spec();

        assert_eq!(spec.unwrap(), Spec {
            items: vec![
            Item {
                params: vec![
                Param {
                    key: "a".into(),
                    value: Some("x".into()),
                }
                ],
                template: vec![
                Match::MultipleLines,
                Match::Text("Hello ".into()),
                Match::Var("X".into()),
                Match::NewLine,
                Match::Text("Bye".into()),
                Match::MultipleLines,
                ],
            },
            Item {
                params: vec![
                Param {
                    key: "a".into(),
                    value: Some("y".into()),
                },
                Param {
                    key: "bbbb".into(),
                    value: None,
                }
                ],
                template: vec![
                Match::Var("X".into()),
                Match::Text(" woooo".into()),
                Match::NewLine,
                Match::Var("Y".into()),
                ],
            }
            ],
        });
    }
}