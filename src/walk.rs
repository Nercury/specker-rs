// Copyright 2017 Nerijus Arlauskas
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::path::{Path, PathBuf};
use walkdir::{self, WalkDir};
use spec::{Options, Spec};
use std::fs::File;
use std::io::Read;
use Result;

/// Parsed specification at a path.
#[derive(Debug, Clone)]
pub struct SpecPath {
    pub spec: Spec,
    pub path: PathBuf,
}

/// Iterator over parsed specification files.
pub struct SpecWalkIter<'a> {
    extension: &'a str,
    walk_dir: walkdir::Iter,
    options: Options<'a>,
}

impl<'a> Iterator for SpecWalkIter<'a> {
    type Item = Result<SpecPath>;

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

impl<'a> SpecWalkIter<'a> {
    fn process_entry(&mut self, entry: &walkdir::DirEntry) -> Result<SpecPath> {
        let path: PathBuf = entry.path().into();
        let mut contents = String::new();
        File::open(&path)?.read_to_string(&mut contents)?;
        Spec::parse(self.options, contents.as_bytes())
            .map(|spec| SpecPath {
                spec: spec,
                path: (&path).clone(),
            })
            .map_err(move |e| (path, e).into())
    }
}

/// Walks spec directory and returns the iterator over all parsed `SpecPath` objects.
pub fn walk_spec_dir<'a>(path: &Path, extension: &'a str, options: Options<'a>) -> SpecWalkIter<'a> {
    SpecWalkIter {
        extension: extension,
        walk_dir: WalkDir::new(path).into_iter(),
        options: options,
    }
}