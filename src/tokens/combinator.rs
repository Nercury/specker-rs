use error::{At, FilePosition};
use std::fmt;
use std::result;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LexError {
    ExpectedName,
    UnexpectedSymbol {
        expected: &'static [u8],
        found: Vec<u8>,
    },
    ExpectedSequenceFoundNewline {
        expected: &'static [u8],
    },
    ExpectedNewline,
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LexError::ExpectedName => "expected name".fmt(f),
            LexError::UnexpectedSymbol { .. } => "unexpected symbol".fmt(f),
            LexError::ExpectedSequenceFoundNewline { .. } => "expected sequence, found newline".fmt(f),
            LexError::ExpectedNewline => "expected newline".fmt(f),
        }
    }
}

impl LexError {
    pub fn at(self, lo: FilePosition, hi: FilePosition) -> At<LexError> {
        At {
            lo: lo,
            hi: hi,
            desc: self,
        }
    }
}

pub type Result<T> = result::Result<T, At<LexError>>;

pub fn trim(text: &[u8]) -> &[u8] {
    let mut start = 0;
    let mut end = text.len();
    while start < end {
        if b" \t".contains(&text[start]) {
            start += 1;
        } else {
            break;
        }
    }
    while end > start {
        if b" \t".contains(&text[end - 1]) {
            end -= 1;
        } else {
            break;
        }
    }
    &text[start..end]
}

pub fn check_new_line(cursor: &mut FilePosition, input: &[u8]) -> bool {
    if input[cursor.byte..].starts_with(b"\r\n") {
        cursor.advance(2);
        return true;
    }
    if input[cursor.byte..].starts_with(b"\n") {
        cursor.advance(1);
        return true;
    }
    return false;
}

pub fn check_exact_bytes(cursor: &mut FilePosition, input: &[u8], other: &'static [u8]) -> bool {
    if input[cursor.byte..].starts_with(other) {
        cursor.advance(other.len());
        return true;
    }
    false
}

pub enum TermType {
    Sequence,
    EolOrEof,
}

pub fn expect_text<'a>(cursor: &mut FilePosition, input: &'a [u8]) -> Result<&'a [u8]> {
    let start = cursor.byte;
    let mut end = start;
    loop {
        if end >= input.len() || input[end..].starts_with(b"\n") || input[end..].starts_with(b"\r\n") {
            break;
        }

        end += 1;
    }

    cursor.advance(end - start);
    return Ok(&input[start..end]);
}

pub fn expect_terminated_text<'a>(cursor: &mut FilePosition, input: &'a [u8], term_sequence: &'static [u8]) -> Result<(&'a [u8], TermType)> {
    let start = cursor.byte;
    let mut end = start;
    loop {
        if end >= input.len() || input[end..].starts_with(b"\n") || input[end..].starts_with(b"\r\n") {
            break;
        }
        if input[end..].starts_with(term_sequence) {
            cursor.advance(end - start + term_sequence.len());
            return Ok((&input[start..end], TermType::Sequence));
        }

        end += 1;
    }

    cursor.advance(end - start);
    return Ok((&input[start..end], TermType::EolOrEof));
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_trim() {
        assert_eq!(trim(b""), b"");
        assert_eq!(trim(b" "), b"");
        assert_eq!(trim(b"\t"), b"");
        assert_eq!(trim(b"a"), b"a");
        assert_eq!(trim(b" a"), b"a");
        assert_eq!(trim(b"a "), b"a");
        assert_eq!(trim(b" a "), b"a");
    }
}