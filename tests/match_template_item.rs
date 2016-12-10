extern crate specker;

mod support;

#[cfg(test)]
mod match_template_item {
    use specker::Match;
    use specker::error::TemplateMatchError;
    use support::{new_item, match_item};

    #[test]
    fn empty_item_matches_empty_file() {
        match_item(
            new_item(&[]),
            &[],
            ""
        ).expect("expected match");
    }

    #[test]
    fn empty_item_does_not_match() {
        let err = match_item(
            new_item(&[]),
            &[],
            "some text"
        ).err().expect("expected error");
        err.assert_matches(&TemplateMatchError::ExpectedEof, (0, 0), (0, 0)).unwrap();
    }

    #[test]
    fn multiple_lines_item_should_match_everything() {
        match_item(
            new_item(&[Match::MultipleLines]),
            &[],
            "some text"
        ).expect("expected match");
    }

    #[test]
    fn repeated_multiple_lines_item_should_match_everything() {
        match_item(
            new_item(&[Match::MultipleLines, Match::MultipleLines]),
            &[],
            "some text"
        ).expect("expected match");
    }

    #[test]
    fn text_line_match() {
        match_item(
            new_item(&[Match::Text("hi".into())]),
            &[],
            "hi"
        ).expect("expected match");
    }

    #[test]
    fn text_line_not_match_end() {
        let err = match_item(
            new_item(&[Match::Text("hi".into())]),
            &[],
            "hip"
        ).err().expect("expected error");
        err.assert_matches(
            &TemplateMatchError::ExpectedText {
                expected: "hi".into(),
                found: "hip".into()
            }, (0, 0), (0, 3)
        ).unwrap();
    }

    #[test]
    fn text_line_skip_and_match() {
        match_item(
            new_item(&[
                Match::MultipleLines,
                Match::Text("hi".into())
            ]),
            &[],
            "hip\nhi"
        ).expect("expected match");
    }
}