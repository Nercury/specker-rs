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
    fn multiple_lines_item_should_match_empty_lines() {
        match_item(
            new_item(&[Match::MultipleLines]),
            &[],
            ""
        ).expect("expected match");
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
            &TemplateMatchError::ExpectedEolOrEof,
            (0, 2), (0, 2)
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

    #[test]
    fn text_lines_skip_and_match() {
        match_item(
            new_item(&[
                Match::MultipleLines,
                Match::Text("hi".into())
            ]),
            &[],
            "hip\nhop\nhi"
        ).expect("expected match");
    }

    #[test]
    fn text_and_multiple_lines_match() {
        match_item(
            new_item(&[
                Match::Text("hi".into()),
                Match::MultipleLines,
            ]),
            &[],
            "hi"
        ).expect("expected match");
    }

    #[test]
    fn text_and_multiple_lines_and_text_match() {
        match_item(
            new_item(&[
                Match::Text("hi".into()),
                Match::MultipleLines,
                Match::Text("world".into()),
            ]),
            &[],
            "hi\nworld"
        ).expect("expected match");
    }

    #[test]
    fn text_and_multiple_lines_and_text_match_2() {
        match_item(
            new_item(&[
                Match::Text("hi".into()),
                Match::MultipleLines,
                Match::Text("world".into()),
            ]),
            &[],
            "hi\n\nworld"
        ).expect("expected match");
    }

    #[test]
    fn text_and_multiple_lines_and_text_match_3() {
        match_item(
            new_item(&[
                Match::Text("hi".into()),
                Match::MultipleLines,
                Match::Text("world".into()),
            ]),
            &[],
            "hi\na\nb\nworld"
        ).expect("expected match");
    }

    #[test]
    fn text_and_multiple_lines_and_text_not_match_first() {
        let err = match_item(
            new_item(&[
                Match::Text("ho".into()),
                Match::MultipleLines,
                Match::Text("world".into()),
            ]),
            &[],
            "hi\na\nb\nworld"
        ).err().expect("expected error");
        err.assert_matches(
            &TemplateMatchError::ExpectedText {
                expected: "ho".into(),
                found: "hi".into()
            }, (0, 0), (0, 2)
        ).unwrap();
    }

    #[test]
    fn text_and_multiple_lines_and_text_not_match_last() {
        let err = match_item(
            new_item(&[
                Match::Text("hi".into()),
                Match::MultipleLines,
                Match::Text("boo".into()),
            ]),
            &[],
            "hi\na\nb\nworld"
        ).err().expect("expected error");
        err.assert_matches(
            &TemplateMatchError::ExpectedTextFoundEof("boo".into()),
            (3, 5), (3, 5)
        ).unwrap();
    }

    #[test]
    fn multiple_text_items_match() {
        match_item(
            new_item(&[
                Match::Text("hello".into()),
                Match::Text("world".into()),
            ]),
            &[],
            "helloworld"
        ).expect("expected match");
    }

    #[test]
    fn multiple_text_items_not_match() {
        let err = match_item(
            new_item(&[
                Match::Text("hello".into()),
                Match::Text("world".into()),
            ]),
            &[],
            "hell"
        ).err().expect("expected error");
        err.assert_matches(
            &TemplateMatchError::ExpectedText {
                expected: "hello".into(),
                found: "hell".into(),
            },
            (0, 0), (0, 4)
        ).unwrap();
    }
}