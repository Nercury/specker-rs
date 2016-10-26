use tokens;
use error::{ParseResult};
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
    token_iter: Peekable<tokens::Iter<'s>>
}

impl<'s> Parser<'s> {
    pub fn new(token_iter: Peekable<tokens::Iter<'s>>) -> Parser<'s> {
        Parser {
            token_iter: token_iter
        }
    }

    pub fn parse_spec(&mut self) -> ParseResult<Spec<'s>> {
        Ok(Spec { items: vec![] })
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