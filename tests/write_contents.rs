extern crate specker;

#[macro_use] mod support;

#[cfg(test)]
mod write_template_item {
    use specker::{self, Match};
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

    #[test]
    fn template_item_that_is_missing_param_should_produce_error() {
        let err = write(match_item(&[Match::Var("hi".into())]), &[]).err().expect("expected error");
        assert_eq!(err, specker::error::TemplateWriteError::MissingParam("hi".into()));
    }

    #[test]
    fn new_line() {
        let file = write(match_item(&[Match::NewLine]), &[]).unwrap();
        assert_contents!(
            &file,
            "\n"
        );
    }

    #[test]
    fn new_line_x2() {
        let file = write(match_item(&[Match::NewLine, Match::NewLine]), &[]).unwrap();
        assert_contents!(
            &file,
            "\n\n"
        );
    }

    #[test]
    fn text() {
        let file = write(match_item(&[Match::Text("hello".into())]), &[]).unwrap();
        assert_contents!(
            &file,
            "hello"
        );
    }

    #[test]
    fn text_x2() {
        let file = write(match_item(&[Match::Text("hello".into()), Match::Text("world".into())]), &[]).unwrap();
        assert_contents!(
            &file,
            "helloworld"
        );
    }

    #[test]
    fn param() {
        let file = write(match_item(&[Match::Var("a".into())]), &[("a", "hello")]).unwrap();
        assert_contents!(
            &file,
            "hello"
        );
    }

    #[test]
    fn param_x2() {
        let file = write(match_item(&[Match::Var("a".into()), Match::Var("a".into())]), &[("a", "hello")]).unwrap();
        assert_contents!(
            &file,
            "hellohello"
        );
    }

    #[test]
    fn two_params() {
        let file = write(match_item(&[
            Match::Var("a".into()),
            Match::Var("b".into()),
        ]), &[
            ("a", "hello"),
            ("b", "world"),
        ]).unwrap();
        assert_contents!(
            &file,
            "helloworld"
        );
    }

    #[test]
    fn mixed() {
        let file = write(match_item(&[
            Match::Var("a".into()),
            Match::NewLine,
            Match::Var("b".into()),
            Match::NewLine,
            Match::Text("and bye ".into()),
            Match::Var("b".into()),
            Match::NewLine,
            Match::Text(".".into()),
        ]), &[
            ("a", "hello"),
            ("b", "world"),
        ]).unwrap();
        assert_contents!(
            &file,
            "hello\nworld\nand bye world\n."
        );
    }
}