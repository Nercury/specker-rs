use error::{FilePosition, LexResult};

pub struct Contents<'a> {
    pub slice: &'a [u8],
    pub lo: FilePosition,
    pub hi: FilePosition,
}

impl<'a> Contents<'a> {
    pub fn new<'r>(input: &'r [u8], lo: FilePosition, hi: FilePosition) -> Contents<'r> {
        Contents {
            slice: &input[lo.byte..hi.byte],
            lo: lo,
            hi: hi,
        }
    }

    pub fn trimmed(self) -> Contents<'a> {
        let mut start = 0;
        let mut end = self.slice.len();
        while start < end {
            if b" \t".contains(&self.slice[start]) {
                start += 1;
            } else {
                break;
            }
        }
        while end > start {
            if b" \t".contains(&self.slice[end - 1]) {
                end -= 1;
            } else {
                break;
            }
        }

        Contents {
            slice: &self.slice[start..end],
            lo: self.lo.advanced(start),
            hi: self.lo.advanced(end),
        }
    }
}

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
        cursor.next_line(2);
        return true;
    }
    if input[cursor.byte..].starts_with(b"\n") {
        cursor.next_line(1);
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

pub fn expect_text<'a>(cursor: &mut FilePosition, input: &'a [u8]) -> LexResult<Contents<'a>> {
    let start_cursor = cursor.clone();
    let mut end = start_cursor.byte;
    loop {
        if end >= input.len() || input[end..].starts_with(b"\n") || input[end..].starts_with(b"\r\n") {
            break;
        }

        end += 1;
    }

    cursor.advance(end - start_cursor.byte);
    return Ok(Contents::new(input, start_cursor, *cursor));
}

pub fn expect_terminated_text<'a>(cursor: &mut FilePosition, input: &'a [u8], term_sequence: &'static [u8]) -> LexResult<(&'a [u8], TermType)> {
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