use tokens::{self, TokenValue, TokenRef, TokenValueRef};
use error::{FilePosition, ParseError, ParseResult};
use std::iter::Peekable;

#[derive(Debug, Clone)]
pub struct Spec<'a> {
    items: Vec<Item<'a>>,
}

#[derive(Debug, Clone)]
pub struct Item<'a> {
    params: Vec<Param<'a>>,
    template: Vec<Match<'a>>,
}

#[derive(Debug, Copy, Clone)]
pub struct Param<'a> {
    key: &'a str,
    value: &'a str,
}

#[derive(Debug, Copy, Clone)]
pub enum Match<'a> {
    MultipleLines,
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
            template: Vec::new()
        };

        if item.params.is_empty() && item.template.is_empty() {
            return Ok(None);
        }

        Ok(Some(item))
    }

    fn parse_params(&mut self) -> ParseResult<Vec<Param<'s>>> {
        let mut params = Vec::new();

        loop {
            if match try!(self.check_next_token_is_key()) {
                None => return Ok(params),
                Some(v) => v,
            } {
                params.push(Param {
                    key: try!(self.expect_key()),
                    value: try!(self.expect_value()),
                })
            } else {
                break;
            }
        }



        Ok(params)
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

    fn expect_token<F, R, E>(&mut self, match_token: F, expected_token_value: E) -> ParseResult<R> where
        F: Fn(TokenValueRef<'s>) -> Option<R>,
        E: Fn() -> TokenValue
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

    fn expect_key(&mut self) -> ParseResult<&'s str> {
        self.expect_token(|token: TokenValueRef<'s>| {
            if let TokenValueRef::Key(s) = token {
                Some(s)
            } else {
                None
            }
        }, || TokenValue::Key(String::from("_")))
    }

    fn expect_value(&mut self) -> ParseResult<&'s str> {
        self.expect_token(|token: TokenValueRef<'s>| {
            if let TokenValueRef::Value(s) = token {
                Some(s)
            } else {
                None
            }
        }, || TokenValue::Value(String::from("_")))
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
        let mut tokens = tokenize(default_options(), b"## a: x
..
Hello ${ X }
Bye
..
## a: y
${ X } woooo
${ Y }
");
        let mut parser = Parser::new(tokens.peekable());
        let spec = parser.parse_spec();

        println!("{:?}", spec);
    }
}