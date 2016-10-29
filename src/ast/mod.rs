use tokens::{self, TokenValue, TokenRef, TokenValueRef};
use error::{FilePosition, ParseError, ParseResult};
use std::iter::Peekable;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Spec<'a> {
    pub items: Vec<Item<'a>>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Item<'a> {
    pub params: Vec<Param<'a>>,
    pub template: Vec<Match<'a>>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub struct Param<'a> {
    pub key: &'a str,
    pub value: Option<&'a str>,
}

#[derive(Debug, Copy, Clone, Eq, PartialEq)]
pub enum Match<'a> {
    MultipleLines,
    NewLine,
    Text(&'a str),
    Var(&'a str),
}

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

    pub fn parse_spec(&mut self) -> ParseResult<Spec<'s>> {
        let mut items = Vec::new();

        while let Some(item) = try!(self.parse_item()) {
            items.push(item);
        }

        Ok(Spec { items: items })
    }

    fn parse_item(&mut self) -> ParseResult<Option<Item<'s>>> {
        let item = Item {
            params: try!(self.parse_params()),
            template: try!(self.parse_template()),
        };

        if item.params.is_empty() && item.template.is_empty() {
            return Ok(None);
        }

        Ok(Some(item))
    }

    fn parse_template(&mut self) -> ParseResult<Vec<Match<'s>>> {
        let mut items = Vec::new();

        while try!(self.check_next_token_is_template_item()) {
            items.push(match try!(self.expect_template_token()) {
                TokenValueRef::MatchAnyNumberOfLines => Match::MultipleLines,
                TokenValueRef::MatchText(s) => Match::Text(s),
                TokenValueRef::MatchNewline => Match::NewLine,
                TokenValueRef::Var(s) => Match::Var(s),
                _ => break,
            });
        }

        Ok(items)
    }

    fn parse_params(&mut self) -> ParseResult<Vec<Param<'s>>> {
        let mut params = Vec::new();

        loop {
            if match try!(self.check_next_token_is_key()) {
                None => return Ok(params),
                Some(v) => v,
            } {
                let key = try!(self.expect_key());
                params.push(Param {
                    key: key,
                    value: if try!(self.check_next_token_is_value()) {
                        Some(try!(self.expect_value()))
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

    fn default_options() -> Options {
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
                            key: "a",
                            value: Some("x"),
                        }
                    ],
                    template: vec![
                        Match::MultipleLines,
                        Match::Text("Hello "),
                        Match::Var("X"),
                        Match::NewLine,
                        Match::Text("Bye"),
                        Match::MultipleLines,
                    ],
                },
                Item {
                    params: vec![
                        Param {
                            key: "a",
                            value: Some("y"),
                        },
                        Param {
                            key: "bbbb",
                            value: None,
                        }
                    ],
                    template: vec![
                        Match::Var("X"),
                        Match::Text(" woooo"),
                        Match::NewLine,
                        Match::Var("Y"),
                    ],
                }
            ],
        });
    }
}