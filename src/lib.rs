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