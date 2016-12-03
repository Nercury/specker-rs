#[macro_use] extern crate log;
extern crate walkdir;

mod ast;
mod tokens;
mod spec;
mod walk;
pub mod error;

pub use ast::{Param, Match};
pub use spec::{Options, Spec, Item, ItemIter, ItemValuesByKeyIter};
pub use walk::{SpecWalkIter, walk_spec_dir};

use std::{io, fmt, path, result};

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