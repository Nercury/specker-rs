#![allow(dead_code)]

extern crate specker;

use specker::error::{TemplateMatchError, At};

macro_rules! assert_contents {
    ($a:expr, $b:expr) => { assert_eq!(unsafe {::std::str::from_utf8_unchecked($a)}, $b) };
}

pub fn new_item<'a>(match_list: &'a [specker::Match]) -> specker::Item<'a> {
    specker::Item {
        params: &[],
        template: match_list,
    }
}

pub fn match_item<'a>(item: specker::Item<'a>, params: &[(&str, &str)], contents: &str) -> Result<(), At<TemplateMatchError>> {
    let mut cursor = ::std::io::Cursor::new(contents.as_bytes());
    Ok(item.match_contents(&mut cursor, &params.iter().cloned().collect())?)
}

pub fn write<'a>(item: specker::Item<'a>, params: &[(&str, &str)]) -> Result<Vec<u8>, specker::error::TemplateWriteError> {
    let mut file = Vec::new();

    item.write_contents(&mut file, &params.iter().cloned().collect())?;

    Ok(file)
}