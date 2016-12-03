extern crate specker;

#[macro_use] mod support;

use specker::Match;
use support::{match_item, write};

#[test]
fn empty_item_should_produce_empty_file() {
    let file = write(match_item(&[]), &[]).unwrap();
    assert_contents!(
        &file,
        ""
    );
}

#[test]
fn template_item_that_contains_multiple_lines_should_produce_error() {
    let err = write(match_item(&[Match::MultipleLines]), &[]).err().expect("expected error");
    assert_eq!(err, specker::error::TemplateWriteError::CanNotWriteMatchAnySymbols);
}