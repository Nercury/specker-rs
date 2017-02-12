// Copyright 2017 Nerijus Arlauskas
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

use std::fmt;
use std::fs;
use std::path::Path;
use std::io::BufRead;
use std::io::BufReader;
use At;

/// Display nice error that combines line and column info with file contents.
pub fn display_error<E: DisplayError>(path: &Path, e: &E) -> String {
    e.display_error(path)
}

pub trait DisplayError {
    fn display_error(&self, path: &Path) -> String;
}

impl<T> DisplayError for At<T> where T: fmt::Display + fmt::Debug {
    fn display_error(&self, path: &Path) -> String {
        let file = fs::File::open(path);

        let mut extra_message = None;

        if self.lo.line == self.hi.line { // does not handle errors that span multiple lines
            if let Ok(file) = file {
                let mut lines: Option<Vec<String>> = None;

                for (i, rd_line) in BufReader::new(file).lines().enumerate() {
                    if let Ok(rd_line) = rd_line {
                        if i + 3 > self.lo.line && i <= self.lo.line {
                            let line = if rd_line.len() > 80 { format!("{}..", &rd_line[..78]) } else { rd_line.to_string() };
                            if let Some(ref mut lines) = lines {
                                lines.push(line);
                            } else {
                                lines = Some(vec![line])
                            }
                        }
                    }
                }
                if let None = lines {
                    lines = Some(vec![String::from("")]);
                }

                if let Some(lines) = lines {
                    let mut sb = String::new();

                    // print lines

                    let lines_len = lines.len();
                    let mut num_len = 0;
                    for (i, line) in lines.into_iter().enumerate() {
                        let num = format!("{} ", self.lo.line + i + 2 - lines_len);
                        num_len = num.len();

                        sb.push_str(&num);
                        sb.push_str("| ");
                        sb.push_str(&line);
                        sb.push_str("\n");
                    }

                    // print arrow

                    for _ in 0..num_len {
                        sb.push_str(" ");
                    }
                    sb.push_str("| ");

                    for _ in 0..self.lo.col {
                        sb.push_str(" ");
                    }
                    sb.push_str("^");
                    for _ in self.lo.col+1..self.hi.col {
                        sb.push_str("^");
                    }

                    sb.push_str("\n");

                    // print message

                    for _ in 0..num_len {
                        sb.push_str(" ");
                    }
                    sb.push_str("| ");

                    for _ in 0..self.lo.col {
                        sb.push_str(" ");
                    }
                    sb.push_str(&format!("{}", self.desc));

                    extra_message = Some(sb);
                }
            }
        }

        if let Some(extra_message) = extra_message {
            format!("in {:?}\n{}", path, extra_message)
        } else {
            if self.lo == self.hi {
                format!("{} in {:?} at {}", &self.desc, path, self.lo)
            } else {
                format!("{} in {:?} at {} - {}", &self.desc, path, self.lo, self.hi)
            }
        }
    }
}