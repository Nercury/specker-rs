extern crate specker;

mod support;

#[cfg(test)]
mod match_template_item {
    use specker::Match;
    use specker::TemplateMatchError;
    use support::{match_item, new_item};

    #[test]
    fn empty_item_matches_empty_file() {
        match_item(new_item(&[]), &[], "").expect("expected match");
    }

    #[test]
    fn empty_item_does_not_match() {
        let err = match_item(new_item(&[]), &[], "some text")
            .err()
            .expect("expected error");
        err.assert_matches(&TemplateMatchError::ExpectedEof, (0, 0), (0, 0))
            .unwrap();
    }

    #[test]
    fn multiple_lines_item_should_match_empty_lines() {
        match_item(new_item(&[Match::MultipleLines]), &[], "").expect("expected match");
    }

    #[test]
    fn multiple_lines_item_should_match_everything() {
        match_item(new_item(&[Match::MultipleLines]), &[], "some text").expect("expected match");
    }

    #[test]
    fn repeated_multiple_lines_item_should_match_everything() {
        match_item(
            new_item(&[Match::MultipleLines, Match::MultipleLines]),
            &[],
            "some text",
        ).expect("expected match");
    }

    #[test]
    fn text_line_match() {
        match_item(new_item(&[Match::Text("hi".into())]), &[], "hi").expect("expected match");
    }

    #[test]
    fn text_line_not_match_end() {
        let err = match_item(new_item(&[Match::Text("hi".into())]), &[], "hip")
            .err()
            .expect("expected error");
        err.assert_matches(&TemplateMatchError::ExpectedEol, (0, 2), (0, 2))
            .unwrap();
    }

    #[test]
    fn text_line_skip_and_match() {
        match_item(
            new_item(&[Match::MultipleLines, Match::Text("hi".into())]),
            &[],
            "hip\nhi",
        ).expect("expected match");
    }

    #[test]
    fn text_lines_skip_and_match() {
        match_item(
            new_item(&[Match::MultipleLines, Match::Text("hi".into())]),
            &[],
            "hip\nhop\nhi",
        ).expect("expected match");
    }

    #[test]
    fn text_and_multiple_lines_match() {
        match_item(
            new_item(&[Match::Text("hi".into()), Match::MultipleLines]),
            &[],
            "hi",
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
            "hi\nworld",
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
            "hi\n\nworld",
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
            "hi\na\nb\nworld",
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
            "hi\na\nb\nworld",
        ).err()
            .expect("expected error");
        err.assert_matches(
            &TemplateMatchError::ExpectedText {
                expected: "ho".into(),
                found: "hi".into(),
            },
            (0, 0),
            (0, 2),
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
            "hi\na\nb\nworld",
        ).err()
            .expect("expected error");
        err.assert_matches(
            &TemplateMatchError::ExpectedTextFoundEof("boo".into()),
            (3, 5),
            (3, 5),
        ).unwrap();
    }

    #[test]
    fn multiple_text_items_match() {
        match_item(
            new_item(&[Match::Text("hello".into()), Match::Text("world".into())]),
            &[],
            "helloworld",
        ).expect("expected match");
    }

    #[test]
    fn multiple_text_items_not_match() {
        let err = match_item(
            new_item(&[Match::Text("hello".into()), Match::Text("world".into())]),
            &[],
            "hell",
        ).err()
            .expect("expected error");
        err.assert_matches(
            &TemplateMatchError::ExpectedText {
                expected: "hello".into(),
                found: "hell".into(),
            },
            (0, 0),
            (0, 4),
        ).unwrap();
    }

    #[test]
    fn leading_newline_match() {
        match_item(
            new_item(&[Match::NewLine, Match::Text("hello".into())]),
            &[],
            "\nhello",
        ).expect("expected match");
    }

    #[test]
    fn leading_newline_not_match_1() {
        let err = match_item(
            new_item(&[Match::NewLine, Match::Text("hello".into())]),
            &[],
            "\n\nhello",
        ).err()
            .expect("expected error");
        err.assert_matches(
            &TemplateMatchError::ExpectedText {
                expected: "hello".into(),
                found: "".into(),
            },
            (1, 0),
            (1, 0),
        ).unwrap();
    }

    #[test]
    fn leading_newline_not_match_2() {
        let err = match_item(
            new_item(&[Match::NewLine, Match::Text("hello".into())]),
            &[],
            "hello",
        ).err()
            .expect("expected error");
        err.assert_matches(&TemplateMatchError::ExpectedEol, (0, 0), (0, 0))
            .unwrap();
    }

    #[test]
    fn leading_newlines_match() {
        match_item(
            new_item(&[Match::NewLine, Match::NewLine, Match::Text("hello".into())]),
            &[],
            "\n\nhello",
        ).expect("expected match");
    }

    #[test]
    fn leading_newlines_not_match_1() {
        let err = match_item(
            new_item(&[Match::NewLine, Match::NewLine, Match::Text("hello".into())]),
            &[],
            "\nhello",
        ).err()
            .expect("expected error");
        err.assert_matches(&TemplateMatchError::ExpectedEol, (1, 0), (1, 0))
            .unwrap();
    }

    #[test]
    fn leading_newlines_not_match_2() {
        let err = match_item(
            new_item(&[Match::NewLine, Match::NewLine, Match::Text("hello".into())]),
            &[],
            "\n\n\nhello",
        ).err()
            .expect("expected error");
        err.assert_matches(
            &TemplateMatchError::ExpectedText {
                expected: "hello".into(),
                found: "".into(),
            },
            (2, 0),
            (2, 0),
        ).unwrap();
    }

    #[test]
    fn trailing_newline_match() {
        match_item(
            new_item(&[Match::Text("hello".into()), Match::NewLine]),
            &[],
            "hello\n",
        ).expect("expected match");
    }

    #[test]
    fn trailing_newline_not_match_1() {
        let err = match_item(
            new_item(&[Match::Text("hello".into()), Match::NewLine]),
            &[],
            "hello\n\n",
        ).err()
            .expect("expected error");
        err.assert_matches(&TemplateMatchError::ExpectedEof, (2, 0), (2, 0))
            .unwrap();
    }

    #[test]
    fn trailing_newline_not_match_2() {
        let err = match_item(
            new_item(&[Match::Text("hello".into()), Match::NewLine]),
            &[],
            "hello",
        ).err()
            .expect("expected error");
        err.assert_matches(&TemplateMatchError::ExpectedEol, (0, 5), (0, 5))
            .unwrap();
    }

    #[test]
    fn trailing_newlines_match() {
        match_item(
            new_item(&[Match::Text("hello".into()), Match::NewLine, Match::NewLine]),
            &[],
            "hello\n\n",
        ).expect("expected match");
    }

    #[test]
    fn trailing_newlines_not_match_1() {
        let err = match_item(
            new_item(&[Match::Text("hello".into()), Match::NewLine, Match::NewLine]),
            &[],
            "hello\n",
        ).err()
            .expect("expected error");
        err.assert_matches(&TemplateMatchError::ExpectedEol, (1, 0), (1, 0))
            .unwrap();
    }

    #[test]
    fn trailing_newlines_not_match_2() {
        let err = match_item(
            new_item(&[Match::Text("hello".into()), Match::NewLine, Match::NewLine]),
            &[],
            "hello\n\n\n",
        ).err()
            .expect("expected error");
        err.assert_matches(&TemplateMatchError::ExpectedEof, (3, 0), (3, 0))
            .unwrap();
    }

    #[test]
    fn multiple_text_items_separated_by_newline_match() {
        match_item(
            new_item(&[
                Match::Text("hello".into()),
                Match::NewLine,
                Match::Text("world".into()),
            ]),
            &[],
            "hello\nworld",
        ).expect("expected match");
    }

    #[test]
    fn multiple_text_items_separated_by_newline_not_match_1() {
        let err = match_item(
            new_item(&[
                Match::Text("hello".into()),
                Match::NewLine,
                Match::Text("world".into()),
            ]),
            &[],
            "hello\n\nworld",
        ).err()
            .expect("expected match");
        err.assert_matches(
            &TemplateMatchError::ExpectedText {
                expected: "world".into(),
                found: "".into(),
            },
            (1, 0),
            (1, 0),
        ).unwrap();
    }

    #[test]
    fn multiple_text_items_separated_by_newline_not_match_2() {
        let err = match_item(
            new_item(&[
                Match::Text("hello".into()),
                Match::NewLine,
                Match::Text("world".into()),
            ]),
            &[],
            "helloworld",
        ).err()
            .expect("expected match");
        err.assert_matches(&TemplateMatchError::ExpectedEol, (0, 5), (0, 5))
            .unwrap();
    }

    #[test]
    fn multiple_text_items_separated_by_newlines_match() {
        match_item(
            new_item(&[
                Match::Text("hello".into()),
                Match::NewLine,
                Match::NewLine,
                Match::Text("world".into()),
            ]),
            &[],
            "hello\n\nworld",
        ).expect("expected match");
    }

    #[test]
    fn multiple_text_items_separated_by_newlines_not_match_1() {
        let err = match_item(
            new_item(&[
                Match::Text("hello".into()),
                Match::NewLine,
                Match::NewLine,
                Match::Text("world".into()),
            ]),
            &[],
            "hello\nworld",
        ).err()
            .expect("expected error");
        err.assert_matches(&TemplateMatchError::ExpectedEol, (1, 0), (1, 0))
            .unwrap();
    }

    #[test]
    fn multiple_text_items_separated_by_newlines_not_match_2() {
        let err = match_item(
            new_item(&[
                Match::Text("hello".into()),
                Match::NewLine,
                Match::NewLine,
                Match::Text("world".into()),
            ]),
            &[],
            "hello\n\n\nworld",
        ).err()
            .expect("expected error");
        err.assert_matches(
            &TemplateMatchError::ExpectedText {
                expected: "world".into(),
                found: "".into(),
            },
            (2, 0),
            (2, 0),
        ).unwrap();
    }

    #[test]
    fn multiple_text_items_separated_by_newlines_and_any_lines_match_1() {
        match_item(
            new_item(&[
                Match::Text("hello".into()),
                Match::NewLine,
                Match::NewLine,
                Match::MultipleLines,
                Match::Text("world".into()),
            ]),
            &[],
            "hello\n\n\nworld",
        ).expect("expected match");
    }

    #[test]
    fn multiple_text_items_separated_by_newlines_and_any_lines_match_2() {
        match_item(
            new_item(&[
                Match::Text("hello".into()),
                Match::NewLine,
                Match::MultipleLines,
                Match::NewLine,
                Match::Text("world".into()),
            ]),
            &[],
            "hello\n\n\nworld",
        ).expect("expected match");
    }

    #[test]
    fn multiple_text_items_separated_by_newlines_and_any_lines_match_3() {
        match_item(
            new_item(&[
                Match::Text("hello".into()),
                Match::MultipleLines,
                Match::NewLine,
                Match::NewLine,
                Match::Text("world".into()),
            ]),
            &[],
            "hello\n\n\nworld",
        ).expect("expected match");
    }

    #[test]
    fn var_match() {
        match_item(
            new_item(&[Match::Var("hello".into())]),
            &[("hello", "world")],
            "world",
        ).expect("expected match");
    }

    #[test]
    fn var_not_match() {
        let err = match_item(
            new_item(&[Match::Var("hello".into())]),
            &[("hello", "word")],
            "world",
        ).err()
            .expect("expected error");
        err.assert_matches(
            &TemplateMatchError::ExpectedText {
                expected: "word".into(),
                found: "world".into(),
            },
            (0, 0),
            (0, 5),
        ).unwrap();
    }

    #[test]
    fn multiple_var_match() {
        match_item(
            new_item(&[Match::Var("hello".into()), Match::Var("hello2".into())]),
            &[("hello2", "b"), ("hello", "a")],
            "ab",
        ).expect("expected match");
    }

    #[test]
    fn multiple_var_not_match() {
        let err = match_item(
            new_item(&[Match::Var("hello".into()), Match::Var("hello2".into())]),
            &[("hello2", "b"), ("hello", "a")],
            "a b",
        ).err()
            .expect("expected error");
        err.assert_matches(
            &TemplateMatchError::ExpectedText {
                expected: "b".into(),
                found: " b".into(),
            },
            (0, 1),
            (0, 3),
        ).unwrap();
    }
}
