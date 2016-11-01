use tokens::{self, TokenValue, TokenRef, TokenValueRef};
use error::{FilePosition, ParseError, ParseResult};
use std::collections::HashMap;
use std::result;
use std::iter::Peekable;
use std::path;
use std::fmt;
use std::fs::{File, DirBuilder};
use std::io::Write;
use std::borrow::Cow;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Spec {
    pub items: Vec<Item>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Item {
    pub params: Vec<Param>,
    pub template: Vec<Match>,
}

#[derive(Debug)]
pub enum TemplateWriteError {
    CanNotWriteMatchAnySymbols,
    MissingParam(String),
    PathMustBeFile(String),
    Io(::std::io::Error),
}

impl ::std::error::Error for TemplateWriteError {
    fn description(&self) -> &str {
        match *self {
            TemplateWriteError::CanNotWriteMatchAnySymbols => "can not write template symbol to match any lines",
            TemplateWriteError::MissingParam(_) => "missing template param",
            TemplateWriteError::PathMustBeFile(_) => "path must be a file",
            TemplateWriteError::Io(ref e) => e.description(),
        }
    }
}

impl fmt::Display for TemplateWriteError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            TemplateWriteError::CanNotWriteMatchAnySymbols => "Can not write template symbol to match any lines".fmt(f),
            TemplateWriteError::MissingParam(ref p) => write!(f, "Missing template param {:?}", p),
            TemplateWriteError::PathMustBeFile(ref p) => write!(f, "Path to template output file {:?} must be a file", p),
            TemplateWriteError::Io(ref e) => e.fmt(f),
        }
    }
}

impl From<::std::io::Error> for TemplateWriteError {
    fn from(other: ::std::io::Error) -> Self {
        TemplateWriteError::Io(other)
    }
}

#[cfg(not(target_os = "windows"))]
fn universal_path_to_platform_path(universal: &str) -> Cow<str> {
    Cow::Borrowed(universal)
}

#[cfg(target_os = "windows")]
fn universal_path_to_platform_path(universal: &str) -> Cow<str> {
    Cow::Owned(s.chars()
        .map(|c| if c == '/' {
            '\\'
        } else {
            c
        })
        .collect::<String>())
}

impl Item {
    /// Finds a first param in params list that has specified key and contains a value.
    pub fn get_param<'a, 'r>(&'r self, key: &'a str) -> Option<&'r str> {
        for p in self.params.iter() {
            if p.key == key {
                match p.value {
                    Some(ref v) => return Some(&v[..]),
                    None => continue,
                }
            }
        }
        None
    }

    /// Writes template contents to specified path.
    pub fn write_file(&self, path: &path::Path, params: &HashMap<&str, &str>)
                      -> result::Result<(), TemplateWriteError> {
        for s in &self.template {
            match *s {
                Match::MultipleLines =>
                    return Err(TemplateWriteError::CanNotWriteMatchAnySymbols),
                Match::Var(ref key) if !params.contains_key(&key[..]) =>
                    return Err(TemplateWriteError::MissingParam(key.to_owned())),
                _ => continue,
            }
        }

        match path.parent() {
            Some(parent) => try!(DirBuilder::new().recursive(true).create(parent)),
            None => return Err(TemplateWriteError::PathMustBeFile(format!("{:?}", path))),
        }

        let mut f = try!(File::create(path));
        try!(f.write_all(b"Hello, world!"));

        Ok(())
    }

    /// Writes template contents to a file path constructed by joining the specified base path
    /// and relative path in universal format. "Universal" here means that on windows "some/path"
    /// is converted to "some\path".
    pub fn write_file_relative(&self, base_path: &path::Path, universal_relative_path: &str, params: &HashMap<&str, &str>)
                               -> result::Result<(), TemplateWriteError> {
        self.write_file(
            &base_path.join(
                universal_path_to_platform_path(universal_relative_path)
                    .as_ref()
            ),
            params
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Param {
    pub key: String,
    pub value: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Match {
    MultipleLines,
    NewLine,
    Text(String),
    Var(String),
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

    pub fn parse_spec(&mut self) -> ParseResult<Spec> {
        let mut items = Vec::new();

        while let Some(item) = try!(self.parse_item()) {
            items.push(item);
        }

        Ok(Spec { items: items })
    }

    fn parse_item(&mut self) -> ParseResult<Option<Item>> {
        let item = Item {
            params: try!(self.parse_params()),
            template: try!(self.parse_template()),
        };

        if item.params.is_empty() && item.template.is_empty() {
            return Ok(None);
        }

        Ok(Some(item))
    }

    fn parse_template(&mut self) -> ParseResult<Vec<Match>> {
        let mut items = Vec::new();

        while try!(self.check_next_token_is_template_item()) {
            items.push(match try!(self.expect_template_token()) {
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
            if match try!(self.check_next_token_is_key()) {
                None => return Ok(params),
                Some(v) => v,
            } {
                let key = try!(self.expect_key());
                params.push(Param {
                    key: key.into(),
                    value: if try!(self.check_next_token_is_value()) {
                        Some(try!(self.expect_value()).into())
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