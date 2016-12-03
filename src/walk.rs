use std::path::Path;
use walkdir::{self, WalkDir};
use spec::{Options, Spec};
use std::fs::File;
use std::io::Read;
use Result;

pub struct SpecWalkIter<'a> {
    extension: &'a str,
    walk_dir: walkdir::Iter,
    options: Options<'a>,
}

impl<'a> Iterator for SpecWalkIter<'a> {
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

impl<'a> SpecWalkIter<'a> {
    fn process_entry(&mut self, entry: &walkdir::DirEntry) -> Result<Spec> {
        let path = entry.path();
        let mut contents = String::new();
        File::open(path)?.read_to_string(&mut contents)?;
        Spec::parse(self.options, contents.as_bytes())
    }
}

/// Walks spec directory and returns the iterator over all parsed `Spec` objects.
pub fn walk_spec_dir<'a>(path: &Path, extension: &'a str, options: Options<'a>) -> SpecWalkIter<'a> {
    SpecWalkIter {
        extension: extension,
        walk_dir: WalkDir::new(path).into_iter(),
        options: options,
    }
}