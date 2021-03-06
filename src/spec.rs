// Copyright 2017 Nerijus Arlauskas
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use ast;
use error::{At, FilePosition, ParseError, TemplateMatchError, TemplateWriteError};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::result;
use std::slice;
use std::str;
use tokens;

/// Specification parser options.
#[derive(Copy, Clone, Debug)]
pub struct Options<'a> {
    /// String that marks multiple lines to be skipped.
    pub skip_lines: &'a str,
    /// Prefix that marks the line as containing a parameter.
    pub marker: &'a str,
    /// Var start prefix.
    pub var_start: &'a str,
    /// Var end suffix.
    pub var_end: &'a str,
}

/// Parsed specification.
#[derive(Debug, Clone)]
pub struct Spec {
    ast: ast::Spec,
}

impl<'a> IntoIterator for &'a Spec {
    type Item = Item<'a>;
    type IntoIter = ItemIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ItemIter {
            inner: self.ast.items.iter(),
        }
    }
}

impl Spec {
    /// Parse specification from in-memory contents.
    pub fn parse<'a>(
        options: Options<'a>,
        contents: &'a [u8],
    ) -> result::Result<Spec, At<ParseError>> {
        Ok(Spec {
            ast: ast::Parser::new(tokens::tokenize(options.into(), contents).peekable())
                .parse_spec()?,
        })
    }

    /// Returns an iterator over the specification items.
    pub fn iter<'r>(&'r self) -> ItemIter<'r> {
        self.into_iter()
    }

    /// Filter items by a param key and return pairs of (&item, &value).
    pub fn iter_item_values<'r, 'p>(&'r self, key: &'p str) -> ItemValuesByKeyIter<'r, 'p> {
        ItemValuesByKeyIter {
            inner: self.iter(),
            key: key,
        }
    }
}

/// Specification item, that describes how a file should be matched against.
#[derive(Debug)]
pub struct Item<'s> {
    /// Specification item params, used to differentiate between items.
    pub params: &'s [ast::Param],
    /// Parsed specification AST.
    pub template: &'s [ast::Match],
}

impl<'s> Item<'s> {
    /// Finds a first param in params list that has specified key and contains a value.
    pub fn get_param(&self, key: &str) -> Option<&'s str> {
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
    pub fn write_contents<O: Write>(
        &'s self,
        output: &mut O,
        params: &HashMap<&str, &str>,
    ) -> result::Result<(), TemplateWriteError> {
        // validation

        for s in self.template {
            match *s {
                ast::Match::MultipleLines => {
                    return Err(TemplateWriteError::CanNotWriteMatchAnySymbols)
                }
                ast::Match::Var(ref key) if !params.contains_key(&key[..]) => {
                    return Err(TemplateWriteError::MissingParam(key.to_owned()))
                }
                _ => continue,
            }
        }

        for s in self.template {
            match *s {
                ast::Match::NewLine => {
                    output.write(b"\n")?;
                }
                ast::Match::Text(ref v) => write!(output, "{}", v)?,
                ast::Match::Var(ref v) => write!(output, "{}", params.get(&v[..]).unwrap())?, // validated above
                _ => unreachable!(),
            }
        }

        Ok(())
    }

    pub fn to_string(&self) -> result::Result<String, TemplateWriteError> {
        let mut source = Vec::new();
        self.write_contents(&mut source, &HashMap::new())?;
        Ok(String::from_utf8(source).map_err(|e| TemplateWriteError::TemplateIsNotValidUtf8(e))?)
    }

    /// Separates tokens into groups where each groups is a line.
    fn get_multiline_match_groups(&'s self) -> Vec<MultilineMatchState<'s>> {
        // this could be written to return an iterator, but I leave this work to someone from future
        // good luck!

        let mut results = Vec::new();
        let mut prev_group: Option<Vec<&ast::Match>> = None;

        for state in self.template {
            match *state {
                ast::Match::MultipleLines => {
                    if let Some(group) = prev_group {
                        results.push(MultilineMatchState::Line(LineGroup::new(group)));
                    }
                    prev_group = None;
                    results.push(MultilineMatchState::MultipleLines);
                }
                ast::Match::NewLine => {
                    if let Some(group) = prev_group {
                        results.push(MultilineMatchState::Line(LineGroup::new(group)));
                    } else {
                        results.push(MultilineMatchState::Line(LineGroup::new(vec![])));
                    }
                    prev_group = Some(Vec::new());
                }
                ref other => {
                    if let Some(ref mut matches) = prev_group {
                        matches.push(other);
                    } else {
                        prev_group = Some(vec![other]);
                    }
                }
            }
        }

        if let Some(group) = prev_group {
            results.push(MultilineMatchState::Line(LineGroup::new(group)));
        }

        results
    }

    /// Try to match specification to input and return any errors if they don't match.
    ///
    /// The values from `params` map will be substituted in as template vars.
    pub fn match_contents<I: Read>(
        &'s self,
        input: &mut I,
        params: &HashMap<&str, &str>,
    ) -> result::Result<(), At<TemplateMatchError>> {
        let mut pos = FilePosition::new();
        let mut eol_pos = FilePosition::new();
        let mut contents = Vec::new();
        input
            .read_to_end(&mut contents)
            .map_err(|e| TemplateMatchError::from(e).at(pos, pos))?;

        let mut skip_lines_state = false;
        let mut had_new_line = true;
        update_eol(&pos, &mut eol_pos, &contents);

        // sort tokens into groups that ends with new line, multiple lines, or eof
        let line_groups = self.get_multiline_match_groups();

        for state in line_groups {
            match state {
                MultilineMatchState::MultipleLines => {
                    skip_lines_state = true;
                }
                MultilineMatchState::Line(line) => 'text: loop {
                    let pos_byte = pos.byte;
                    match line.matches(pos, &contents, params) {
                        Ok((bytes, end_bytes)) => {
                            if bytes == 0 && !had_new_line {
                                return Err(TemplateMatchError::ExpectedEol.at(pos, pos));
                            }

                            pos.advance(bytes);
                            pos.next_line(end_bytes);
                            had_new_line = end_bytes > 0;
                            skip_lines_state = false;
                            update_eol(&pos, &mut eol_pos, &contents);

                            break 'text;
                        }
                        Err(err_match) => if skip_lines_state {
                            if pos_byte >= contents.len() {
                                match err_match {
                                    LineGroupMatchErr::Text { pos: err_pos, text } => {
                                        return Err(TemplateMatchError::ExpectedTextFoundEof(
                                            text.to_string(),
                                        ).at(err_pos, eol_pos))
                                    }
                                    _ => (),
                                };
                            }

                            pos.advance(eol_pos.byte - pos_byte);
                            pos.next_line(
                                matches_newline(&eol_pos, &contents).expect("expected newline"),
                            );
                            update_eol(&pos, &mut eol_pos, &contents);

                            continue 'text;
                        } else {
                            match err_match {
                                LineGroupMatchErr::Text { pos, text } => {
                                    return Err(TemplateMatchError::ExpectedText {
                                        expected: text.to_string(),
                                        found: String::from_utf8_lossy(
                                            &contents[pos.byte..eol_pos.byte],
                                        ).into_owned(),
                                    }.at(pos, eol_pos))
                                }
                                LineGroupMatchErr::ParamNotFound { pos, key } => {
                                    return Err(TemplateMatchError::MissingParam(key.into()).at(pos, pos))
                                }
                                LineGroupMatchErr::NewLineOrEof { pos } => {
                                    return Err(TemplateMatchError::ExpectedEol.at(pos, pos))
                                }
                            }
                        },
                    }
                },
            }
        }

        if !skip_lines_state {
            if pos.byte < contents.len() || (had_new_line && contents.len() > 0) {
                return Err(TemplateMatchError::ExpectedEof.at(pos, pos));
            }
        }

        Ok(())
    }
}

/// Groups by line.
///
/// This separation was useful because the MultipleLines requires eager matching, which
/// is different than line match.
#[derive(Debug)]
enum MultilineMatchState<'a> {
    MultipleLines,
    Line(LineGroup<'a>),
}

#[derive(Debug)]
enum LineGroupMatchErr<'a> {
    Text { pos: FilePosition, text: &'a str },
    ParamNotFound { pos: FilePosition, key: &'a str },
    NewLineOrEof { pos: FilePosition },
}

/// All tokens for a line.
#[derive(Debug)]
struct LineGroup<'a> {
    tokens: Vec<&'a ast::Match>,
}

impl<'a> LineGroup<'a> {
    pub fn new<'r>(tokens: Vec<&'r ast::Match>) -> LineGroup<'r> {
        LineGroup { tokens: tokens }
    }

    /// Check if a line match template tokens `MultipleLines` and `NewLine` are handled by the
    /// called that separated tokens into lines.
    pub fn matches<'o, 'r>(
        &'a self,
        mut pos: FilePosition,
        content: &'o [u8],
        params: &HashMap<&str, &'r str>,
    ) -> result::Result<(usize, usize), LineGroupMatchErr<'r>>
    where
        'a: 'r,
    {
        let start_pos = pos;

        for token in &self.tokens {
            match **token {
                ast::Match::Text(ref text) => {
                    if let Some(bytes) = matches_content(&pos, content, text.as_bytes()) {
                        pos.advance(bytes);
                    } else {
                        return Err(LineGroupMatchErr::Text {
                            pos: pos,
                            text: text,
                        });
                    }
                }
                ast::Match::Var(ref key) => match params.get(&key[..]) {
                    Some(ref text) => {
                        if let Some(bytes) = matches_content(&pos, content, text.as_bytes()) {
                            pos.advance(bytes);
                        } else {
                            return Err(LineGroupMatchErr::Text {
                                pos: pos,
                                text: text,
                            });
                        }
                    }
                    None => {
                        return Err(LineGroupMatchErr::ParamNotFound {
                            pos: pos,
                            key: &key[..],
                        })
                    }
                },
                ast::Match::MultipleLines => unreachable!(),
                ast::Match::NewLine => unreachable!(),
            }
        }

        match matches_newline(&pos, content) {
            Some(newline_bytes) => Ok((pos.byte - start_pos.byte, newline_bytes)),
            None => Err(LineGroupMatchErr::NewLineOrEof { pos: pos }),
        }
    }
}

fn matches_content(pos: &FilePosition, content: &[u8], to_match: &[u8]) -> Option<usize> {
    if content[pos.byte..].starts_with(to_match) {
        return Some(to_match.len());
    }

    None
}

fn matches_newline(pos: &FilePosition, content: &[u8]) -> Option<usize> {
    let end = &content[pos.byte..];
    if end.is_empty() {
        return Some(0);
    } else if end.starts_with(b"\n") {
        return Some(1);
    } else if end.starts_with(b"\r\n") {
        return Some(2);
    }

    None
}

fn update_eol(pos: &FilePosition, eol_pos: &mut FilePosition, contents: &[u8]) {
    let mut eol = pos.byte;
    loop {
        if eol >= contents.len() {
            break;
        }

        let slice = &contents[eol..];

        if slice.starts_with(b"\n") || slice.starts_with(b"\r\n") {
            break;
        }

        eol += 1;
    }

    *eol_pos = pos.advanced(eol - pos.byte);
}

/// Specification item iterator.
pub struct ItemIter<'a> {
    inner: slice::Iter<'a, ast::Item>,
}

impl<'a> Iterator for ItemIter<'a> {
    type Item = Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|i| Item {
            params: &i.params,
            template: &i.template,
        })
    }
}

/// Iterator over the specification items that contain a specific key.
pub struct ItemValuesByKeyIter<'a, 'p> {
    inner: ItemIter<'a>,
    key: &'p str,
}

impl<'a, 'p> Iterator for ItemValuesByKeyIter<'a, 'p> {
    type Item = (Item<'a>, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next() {
                Some(item) => match item.get_param(self.key) {
                    Some(value) => return Some((item, value)),
                    None => continue,
                },
                None => return None,
            }
        }
    }
}
