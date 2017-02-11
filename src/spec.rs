use std::collections::HashMap;
use std::result;
use std::io::{Read, Write};
use std::slice;
use std::str;
use error::{TemplateWriteError, TemplateMatchError, At, FilePosition};
use Result;
use ast;
use tokens;

#[derive(Copy, Clone, Debug)]
pub struct Options<'a> {
    pub skip_lines: &'a str,
    pub marker: &'a str,
    pub var_start: &'a str,
    pub var_end: &'a str,
}

#[derive(Debug, Clone)]
pub struct Spec {
    ast: ast::Spec,
}

impl<'a> IntoIterator for &'a Spec {
    type Item = Item<'a>;
    type IntoIter = ItemIter<'a>;

    fn into_iter(self) -> Self::IntoIter {
        ItemIter {
            inner: self.ast.items.iter()
        }
    }
}

impl Spec {
    pub fn parse<'a>(options: Options<'a>, contents: &'a [u8]) -> Result<Spec> {
        Ok(Spec {
            ast: ast::Parser::new(
                tokens::tokenize(options.into(), contents).peekable()
            ).parse_spec()?
        })
    }

    /// Returns an iterator over the items.
    pub fn iter<'r>(&'r self) -> ItemIter<'r> {
        self.into_iter()
    }

    /// Filter items by a param key and return pairs of (&item, &value).
    pub fn iter_item_values<'r, 'p>(&'r self, key: &'p str) -> ItemValuesByKeyIter<'r, 'p> {
        ItemValuesByKeyIter {
            inner: self.iter(),
            key: key,
        }
    }
}

pub struct Item<'s> {
    pub params: &'s [ast::Param],
    pub template: &'s [ast::Match],
}

impl<'s> Item<'s> {
    /// Finds a first param in params list that has specified key and contains a value.
    pub fn get_param(&self, key: &str) -> Option<&'s str> {
        for p in self.params.iter() {
            if p.key == key {
                match p.value {
                    Some(ref v) => return Some(&v[..]),
                    None => continue,
                }
            }
        }
        None
    }

    /// Writes template contents to specified path.
    pub fn write_contents<O: Write>(&'s self, output: &mut O, params: &HashMap<&str, &str>)
                                    -> result::Result<(), TemplateWriteError> {
        // validation

        for s in self.template {
            match *s {
                ast::Match::MultipleLines =>
                    return Err(TemplateWriteError::CanNotWriteMatchAnySymbols),
                ast::Match::Var(ref key) if !params.contains_key(&key[..]) =>
                    return Err(TemplateWriteError::MissingParam(key.to_owned())),
                _ => continue,
            }
        }

        for s in self.template {
            match *s {
                ast::Match::NewLine => { output.write(b"\n")?; },
                ast::Match::Text(ref v) => write!(output, "{}", v)?,
                ast::Match::Var(ref v) => write!(output, "{}", params.get(&v[..]).unwrap())?, // validated above
                _ => unreachable!(),
            }
        }

        Ok(())
    }

    pub fn match_contents<I: Read>(&'s self, input: &mut I, params: &HashMap<&str, &str>)
                                   -> result::Result<(), At<TemplateMatchError>> {
        let mut pos = FilePosition::new();
        let mut eol_pos = FilePosition::new();
        let mut contents = Vec::new();
        input.read_to_end(&mut contents).map_err(|e| TemplateMatchError::from(e).at(pos, pos))?;

        let mut skip_lines_state = false;
        let mut skipped_lines = 0;
        update_eol(&pos, &mut eol_pos, &contents);

        for state in self.template {
            match *state {
                ast::Match::MultipleLines => {
                    skip_lines_state = true;
                    skipped_lines = 0;
                },
                ast::Match::Text(ref text) => {
                    let pos_byte = pos.byte;
                    if pos_byte >= contents.len() {
                        return Err(TemplateMatchError::ExpectedTextFoundEof(text.clone()).at(pos, pos));
                    }

                    if let Some((bytes, end_bytes)) = matches_content_with_newline(&pos, &contents, text.as_bytes()) {
                        pos.advance(bytes);
                        pos.next_line(end_bytes);
                        skip_lines_state = false;
                        update_eol(&pos, &mut eol_pos, &contents);
                    } else {
                        if skip_lines_state {
                            pos.advance(eol_pos.byte - pos_byte);
                            pos.next_line(matches_newline(&eol_pos, &contents).expect("expected newline"));
                            update_eol(&pos, &mut eol_pos, &contents);
                            skipped_lines += 1;
                        } else {
                            return Err(TemplateMatchError::ExpectedText {
                                expected: text.clone(),
                                found: String::from_utf8_lossy(&contents[pos.byte..eol_pos.byte]).into_owned(),
                            }.at(pos, eol_pos));
                        }
                    }
                },
                _ => unimplemented!(),
            }
        }

        if skip_lines_state {

        } else {
            if pos.byte < contents.len() {
                return Err(TemplateMatchError::ExpectedEof.at(pos, pos));
            }
        }

        Ok(())
    }

    //    pub fn match_file(&'s self, path: &path::Path, params: &HashMap<&str, &str>)
    //                      -> result::Result<(), FileMatchError>
    //    {
    //        let mut file_contents = String::new();
    //        let mut file = File::open(path)?;
    //        file.read_to_string(&mut file_contents)?;
    //
    //        println!("FILE {:?}", file_contents);
    //
    //        Ok(())
    //    }

    //    /// Writes template contents to a file path constructed by joining the specified base path
    //    /// and relative path in universal format. "Universal" here means that on windows "some/path"
    //    /// is converted to "some\path".
    //    pub fn write_file_relative(&'s self, base_path: &path::Path, universal_relative_path: &str, params: &HashMap<&str, &str>)
    //                               -> result::Result<(), TemplateWriteError> {
    //        self.write_file(
    //            &base_path.join(
    //                universal_path_to_platform_path(universal_relative_path)
    //                    .as_ref()
    //            ),
    //            params
    //        )
    //    }

    //    pub fn match_file_relative(&'s self, base_path: &path::Path, universal_relative_path: &str, params: &HashMap<&str, &str>)
    //                               -> result::Result<(), FileMatchError> {
    //        self.match_file(
    //            &base_path.join(
    //                universal_path_to_platform_path(universal_relative_path)
    //                    .as_ref()
    //            ),
    //            params
    //        )
    //    }
}

fn matches_content_with_newline(pos: &FilePosition, content: &[u8], to_match: &[u8]) -> Option<(usize, usize)> {
    if content[pos.byte..].starts_with(to_match) {
        let end = &content[pos.byte + to_match.len()..];
        if end.is_empty() {
            return Some((to_match.len(), 0));
        } else if end.starts_with(b"\n") {
            return Some((to_match.len(), 1));
        } else if end.starts_with(b"\r\n") {
            return Some((to_match.len(), 2));
        }
    }

    None
}

fn matches_newline(pos: &FilePosition, content: &[u8]) -> Option<usize> {
    let end = &content[pos.byte..];
    if end.is_empty() {
        return Some(0);
    } else if end.starts_with(b"\n") {
        return Some(1);
    } else if end.starts_with(b"\r\n") {
        return Some(2);
    }

    None
}

fn update_eol(pos: &FilePosition, eol_pos: &mut FilePosition, contents: &[u8]) {
    let mut eol = pos.byte;
    loop {
        if eol >= contents.len() {
            break;
        }

        let slice = &contents[eol..];

        if slice.starts_with(b"\n") || slice.starts_with(b"\r\n") {
            break;
        }

        eol += 1;
    }

        * eol_pos = pos.advanced(eol - pos.byte);
}

pub struct ItemIter<'a> {
    inner: slice::Iter<'a, ast::Item>,
}

impl<'a> Iterator for ItemIter<'a> {
    type Item = Item<'a>;

    fn next(&mut self) -> Option<Self::Item> {
        self.inner.next().map(|i| Item { params: &i.params, template: &i.template })
    }
}

pub struct ItemValuesByKeyIter<'a, 'p> {
    inner: ItemIter<'a>,
    key: &'p str,
}

impl<'a, 'p> Iterator for ItemValuesByKeyIter<'a, 'p> {
    type Item = (Item<'a>, &'a str);

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            match self.inner.next() {
                Some(item) => match item.get_param(self.key) {
                    Some(value) => return Some((item, value)),
                    None => continue,
                },
                None => return None,
            }
        }
    }
}

//#[cfg(not(target_os = "windows"))]
//fn universal_path_to_platform_path(universal: &str) -> Cow<str> {
//    Cow::Borrowed(universal)
//}
//
//#[cfg(target_os = "windows")]
//fn universal_path_to_platform_path(universal: &str) -> Cow<str> {
//    Cow::Owned(s.chars()
//        .map(|c| if c == '/' {
//            '\\'
//        } else {
//            c
//        })
//        .collect::<String>())
//}