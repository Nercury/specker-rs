use error::{At, FilePosition};
use std::fmt;
use std::result;
use std::str;

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum LexError {
    ExpectedName,
    UnexpectedSymbol {
        expected: &'static [u8],
        found: Vec<u8>,
    },
    ExpectedSequenceFoundNewline {
        expected: &'static [u8],
    }
}

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            LexError::ExpectedName => "expected name".fmt(f),
            LexError::UnexpectedSymbol { .. } => "unexpected symbol".fmt(f),
            LexError::ExpectedSequenceFoundNewline { .. } => "expected sequence, found newline".fmt(f),
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

pub fn try_new_line(cursor: &mut FilePosition, input: &[u8]) -> bool {
    if cursor.byte == input.len() {
        return false;
    }
    if cursor.byte + 2 >= input.len() && &input[cursor.byte..cursor.byte + 2] == b"\r\n" {
        cursor.advance(2);
        return true;
    }
    if cursor.byte + 1 >= input.len() && input[cursor.byte] == b'\n' {
        cursor.advance(1);
        return true;
    }
    return false;
}

pub fn try_exact_bytes(cursor: &mut FilePosition, input: &[u8], other: &'static [u8]) -> bool {
    if input.len() - cursor.byte < other.len() {
        return false;
    }
    let mut remaining = input.len() - cursor.byte;
    if remaining > 20 {
        remaining = 20;
    }
    debug!(
        "expect {:?} at {:?}",
        str::from_utf8(other).unwrap(),
        str::from_utf8(&input[cursor.byte..cursor.byte + remaining]).unwrap()
    );
    let matches = &input[cursor.byte..cursor.byte + other.len()] == other;
    if matches {
        cursor.advance(other.len());
    }
    matches
}

fn take_symbol_count<'a>(slice: &'a [u8], num: usize) -> &'a [u8] {
    if slice.len() < num {
        return slice;
    }
    &slice[0..num]
}

pub fn expect_exact_bytes(cursor: &mut FilePosition, input: &[u8], other: &'static [u8]) -> Result<()> {
    if input.len() - cursor.byte < other.len() {
        return Err(LexError::UnexpectedSymbol {
            expected: other,
            found: take_symbol_count(&input[cursor.byte..], 20).into()
        }.at(cursor.clone(), cursor.advanced(input.len() - cursor.byte)));
    }
    let existing = &input[cursor.byte..cursor.byte + other.len()];
    if existing != other {
        return Err(LexError::UnexpectedSymbol {
            expected: other,
            found: take_symbol_count(&input[cursor.byte..], 20).into()
        }.at(cursor.clone(), cursor.advanced(cursor.byte + other.len())));
    }
    cursor.advance(other.len());
    Ok(())
}

pub enum TermType {
    Sequence,
    EolOrEof,
}

pub fn expect_text_terminated_by_sequence_or_newline<'a>(cursor: &mut FilePosition, input: &'a [u8], term_sequence: &'static [u8]) -> Result<(&'a [u8], TermType)> {
    let start = cursor.byte;
    let mut end = start;
    loop {
        if end >= input.len() {
            break;
        }
        if input[end] == b'\n' || (&input[end..]).starts_with(b"\r\n") {
            break;
        }
        if (&input[end..]).starts_with(term_sequence) {
            cursor.advance(end - start);
            return Ok((&input[start..end], TermType::Sequence));
        }

        end += 1;
    }

    cursor.advance(end - start);
    return Ok((&input[start..end], TermType::EolOrEof));
}

pub fn expect_name<'a>(cursor: &mut FilePosition, input: &'a [u8], whitespace: &'static [u8], terminators: &'static [u8]) -> Result<&'a [u8]> {
    let mut start = cursor.byte;
    loop {
        if start >= input.len() || terminators.contains(&input[start]) {
            return Err(LexError::ExpectedName.at(*cursor, cursor.advanced(start - cursor.byte)));
        }
        if whitespace.contains(&input[start]) {
            start += 1;
            continue;
        }
        break;
    }

    let mut name_end = start + 1;
    loop {
        if name_end >= input.len() || terminators.contains(&input[name_end]) {
            cursor.advance(name_end - start + 1);
            return Ok(&input[start..name_end]);
        }
        if whitespace.contains(&input[name_end]) {
            break;
        }
        name_end += 1;
    }

    let mut end = name_end + 1;
    loop {
        if start >= input.len() || terminators.contains(&input[end]) {
            break;
        }
        if !whitespace.contains(&input[end]) {
            return Err(
                LexError::UnexpectedSymbol {
                    expected: terminators,
                    found: (&input[end..end+1]).into()
                }.at(cursor.advanced(start - cursor.byte), cursor.advanced(name_end - cursor.byte))
            );
        }
        end += 1;
    }

    cursor.advance(end - start);

    Ok(&input[start..name_end])
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