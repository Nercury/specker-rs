#[macro_use] extern crate log;
extern crate walkdir;

pub mod ast;
pub mod tokens;
pub mod error;

use std::result;
use std::fmt;
use std::path::{self, Path, PathBuf};
use walkdir::WalkDir;

#[derive(Debug)]
pub enum Error {
    WalkDir(walkdir::Error),
    StripPrefixError(path::StripPrefixError),
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::WalkDir(ref e) => e.fmt(f),
            Error::StripPrefixError(ref e) => e.fmt(f),
        }
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::WalkDir(ref e) => e.description(),
            Error::StripPrefixError(ref e) => e.description(),
        }
    }
}

impl From<walkdir::Error> for Error {
    fn from(other: walkdir::Error) -> Error {
        Error::WalkDir(other)
    }
}

impl From<path::StripPrefixError> for Error {
    fn from(other: path::StripPrefixError) -> Error {
        Error::StripPrefixError(other)
    }
}

pub type Result<T> = result::Result<T, Error>;

pub struct Spec {
    pub rel_path: PathBuf,
}

pub struct Iter {
    base: PathBuf,
    extension: &'static str,
    walk_dir: walkdir::Iter,
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
        Ok(Spec {
            rel_path: try!(entry.path().strip_prefix(&self.base)).into()
        })
    }
}

pub fn walk_spec_dir(path: &Path, extension: &'static str) -> Iter {
    Iter {
        base: path.into(),
        extension: extension,
        walk_dir: WalkDir::new(path).into_iter(),
    }
}