#[macro_use] extern crate log;
extern crate walkdir;

mod ast;
mod tokens;
pub mod error;

use std::result;
use std::fmt;
use std::io::{self, Read};
use std::path::{self, Path, PathBuf};
use std::fs::File;
use walkdir::WalkDir;

pub use ast::{Item, Param, Match, TemplateWriteError};
pub use tokens::Options;

#[derive(Debug)]
pub enum Error {
    WalkDir(walkdir::Error),
    Io(io::Error),
    StripPrefixError(path::StripPrefixError),
    Parse(error::At<error::ParseError>),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::WalkDir(ref e) => e.fmt(f),
            Error::Io(ref e) => e.fmt(f),
            Error::StripPrefixError(ref e) => e.fmt(f),
            Error::Parse(ref e) => e.fmt(f),
        }
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::WalkDir(ref e) => e.description(),
            Error::Io(ref e) => e.description(),
            Error::StripPrefixError(ref e) => e.description(),
            Error::Parse(ref e) => e.description(),
        }
    }
}

impl From<walkdir::Error> for Error {
    fn from(other: walkdir::Error) -> Error {
        Error::WalkDir(other)
    }
}

impl From<io::Error> for Error {
    fn from(other: io::Error) -> Error {
        Error::Io(other)
    }
}

impl From<path::StripPrefixError> for Error {
    fn from(other: path::StripPrefixError) -> Error {
        Error::StripPrefixError(other)
    }
}

impl From<error::At<error::ParseError>> for Error {
    fn from(other: error::At<error::ParseError>) -> Error {
        Error::Parse(other)
    }
}

pub type Result<T> = result::Result<T, Error>;

#[derive(Debug, Clone)]
pub struct Spec {
    pub rel_path: PathBuf,
    ast: ast::Spec,
}

impl<'a> IntoIterator for &'a Spec {
    type Item = &'a Item;
    type IntoIter = ::std::slice::Iter<'a, Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.ast.items.iter()
    }
}

impl Spec {
    /// Returns an iterator over the items.
    pub fn iter<'r>(&'r self) -> ::std::slice::Iter<'r, Item> {
        self.into_iter()
    }
}

pub struct Iter {
    base: PathBuf,
    extension: &'static str,
    walk_dir: walkdir::Iter,
    options: Options,
}

impl Iterator for Iter {
    type Item = Result<Spec>;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.walk_dir.next() {
                None => return None,
                Some(Err(e)) => return Some(Err(e.into())),
                Some(Ok(entry)) => {
                    return Some(match (entry.file_type().is_file(), entry.path().extension()) {
                        (true, Some(v)) if v == self.extension => self.process_entry(&entry),
                        _ => continue,
                    })
                },
            }
        }
    }
}

impl Iter {
    fn process_entry(&mut self, entry: &walkdir::DirEntry) -> Result<Spec> {
        let path = entry.path();
        let mut contents = String::new();
        try!(try!(File::open(path)).read_to_string(&mut contents));
        Ok(Spec {
            rel_path: try!(path.strip_prefix(&self.base)).into(),
            ast: try!(ast::Parser::new(tokens::tokenize(self.options, contents.as_bytes()).peekable()).parse_spec())
        })
    }
}

/// Walks spec directory and returns the iterator over all parsed `Spec` objects.
pub fn walk_spec_dir(path: &Path, extension: &'static str, options: Options) -> Iter {
    Iter {
        base: path.into(),
        extension: extension,
        walk_dir: WalkDir::new(path).into_iter(),
        options: options,
    }
}