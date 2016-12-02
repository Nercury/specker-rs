use std::path::{Path, PathBuf};
use std::collections::HashMap;
use std::result;
use std::path;
use std::fs::{File, DirBuilder};
use std::io::{Read, Write};
use std::borrow::Cow;
use std::slice;
use error::{TemplateWriteError, FileMatchError, Result};
use ast;
use tokens;

#[derive(Copy, Clone, Debug)]
pub struct Options<'a> {
    pub skip_lines: &'a str,
    pub marker: &'a str,
    pub var_start: &'a str,
    pub var_end: &'a str,
}

#[derive(Debug, Clone)]
pub struct Spec {
    pub rel_path: PathBuf,
    ast: ast::Spec,
}

impl<'a> IntoIterator for &'a Spec {
    type Item = Item<'a>;
    type IntoIter = ItemIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ItemIter {
            inner: self.ast.items.iter()
        }
    }
}

impl Spec {
    pub fn parse<'a>(options: Options<'a>, rel_path: &'a Path, contents: &'a [u8]) -> Result<Spec> {
        Ok(Spec {
            rel_path: rel_path.into(),
            ast: ast::Parser::new(tokens::tokenize(options.into(), contents).peekable()).parse_spec()?
        })
    }

    /// Returns an iterator over the items.
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

pub struct Item<'s> {
    pub params: &'s [ast::Param],
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
    pub fn write_file(&'s self, path: &path::Path, params: &HashMap<&str, &str>)
                      -> result::Result<(), TemplateWriteError> {
        for s in self.template {
            match *s {
                ast::Match::MultipleLines =>
                    return Err(TemplateWriteError::CanNotWriteMatchAnySymbols),
                ast::Match::Var(ref key) if !params.contains_key(&key[..]) =>
                    return Err(TemplateWriteError::MissingParam(key.to_owned())),
                _ => continue,
            }
        }

        match path.parent() {
            Some(parent) => DirBuilder::new().recursive(true).create(parent)?,
            None => return Err(TemplateWriteError::PathMustBeFile(format!("{:?}", path))),
        }

        let mut f = File::create(path)?;
        f.write_all(b"Hello, world!")?;

        Ok(())
    }

    pub fn match_file(&'s self, path: &path::Path, params: &HashMap<&str, &str>)
                      -> result::Result<(), FileMatchError>
    {
        let mut file_contents = String::new();
        let mut file = File::open(path)?;
        file.read_to_string(&mut file_contents)?;

        println!("FILE {:?}", file_contents);

        Ok(())
    }

    /// Writes template contents to a file path constructed by joining the specified base path
    /// and relative path in universal format. "Universal" here means that on windows "some/path"
    /// is converted to "some\path".
    pub fn write_file_relative(&'s self, base_path: &path::Path, universal_relative_path: &str, params: &HashMap<&str, &str>)
                               -> result::Result<(), TemplateWriteError> {
        self.write_file(
            &base_path.join(
                universal_path_to_platform_path(universal_relative_path)
                    .as_ref()
            ),
            params
        )
    }

    pub fn match_file_relative(&'s self, base_path: &path::Path, universal_relative_path: &str, params: &HashMap<&str, &str>)
                               -> result::Result<(), FileMatchError> {
        self.match_file(
            &base_path.join(
                universal_path_to_platform_path(universal_relative_path)
                    .as_ref()
            ),
            params
        )
    }
}

pub struct ItemIter<'a> {
    inner: slice::Iter<'a, ast::Item>,
}

impl<'a> Iterator for ItemIter<'a> {
    type Item = Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|i| Item { params: &i.params, template: &i.template })
    }
}

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