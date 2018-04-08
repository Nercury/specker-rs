// Copyright 2017 Nerijus Arlauskas
//
// Licensed under the Apache License, Version 2.0, <LICENSE-APACHE or
// http://apache.org/licenses/LICENSE-2.0> or the MIT license <LICENSE-MIT or
// http://opensource.org/licenses/MIT>, at your option. This file may not be
// copied, modified, or distributed except according to those terms.

/*!

Checks if any number of files match some specification.
Designed for testing file generation.

Let's say we have this specification:

```ignore
## file: output/index.html
..
<body>
..
## file: output/style.css
..
body {
..
}
..
```

Specker can check if there is a file named `output/index.html` containing
`<body>` in some line, as well as file `output/style.css`
containing `body {` and `}` lines. Symbol `..` matches any number of
lines.

If there is a match error, specker can print a nice message like:

```ignore
1 | <bddy>
  | ^^^^^^
  | Expected "<body>", found "<bddy>"
```

It also has iterators to run many such specification tests
in bulk.

Example code that iterates the "spec" dir, collects all "txt" specifications
and checks them:

```ignore
extern crate specker;

use std::fs;
use std::env;
use std::path::PathBuf;
use std::collections::HashMap;

#[test]
fn check_specifications() {
    let src_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    for maybe_spec in specker::walk_spec_dir(&spec_dir, "txt", specker::Options {
        skip_lines: "..",
        marker: "##",
        var_start: "${",
        var_end: "}",
    }) {
        let spec_path = maybe_spec.unwrap_or_else(|e| {
            // print nicely formatted error
            panic!("\n{}", specker::display_error(&e));
        });

        // go over spec items and check if file contents match
        for (item, input_file_name) in spec_path.spec.iter()
            .filter_map(
                |item| item.get_param("file")
                    .map(|param_value| (item, param_value))
            )
            {
                let path = spec_dir.join(input_file_name);
                let mut file = fs::File::open(&path)
                    .expect(&format!("failed to open file {:?}", &path));

                if let Err(e) = item.match_contents(&mut file, &HashMap::new()) {
                    // print nicely formatted error
                    println!("{}", specker::display_error_for_file(&path, &e));
                    // print one-liner error
                    panic!("{}", e);
                }
            }
    }
}
```

*/

extern crate walkdir;

mod ast;
mod display;
mod error;
mod spec;
mod tokens;
mod walk;

pub use ast::{Match, Param};
pub use display::{display_error, display_error_for_file, display_error_for_read};
pub use error::At;
pub use error::{LexError, ParseError, TemplateMatchError, TemplateWriteError};
pub use spec::{Item, ItemIter, ItemValuesByKeyIter, Options, Spec};
use std::{fmt, io, path, result};
pub use walk::{walk_spec_dir, SpecPath, SpecWalkIter};

/// Specification iteration or parsing error.
#[derive(Debug)]
pub enum Error {
    WalkDir(walkdir::Error),
    Io(io::Error),
    StripPrefixError(path::StripPrefixError),
    Parse {
        path: path::PathBuf,
        err: error::At<error::ParseError>,
    },
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            Error::WalkDir(ref e) => e.fmt(f),
            Error::Io(ref e) => e.fmt(f),
            Error::StripPrefixError(ref e) => e.fmt(f),
            Error::Parse {
                ref path,
                err: ref e,
            } => write!(f, "{} in {:?}", e, path),
        }
    }
}

impl ::std::error::Error for Error {
    fn description(&self) -> &str {
        match *self {
            Error::WalkDir(ref e) => e.description(),
            Error::Io(ref e) => e.description(),
            Error::StripPrefixError(ref e) => e.description(),
            Error::Parse { ref err, .. } => err.description(),
        }
    }
}

impl From<walkdir::Error> for Error {
    fn from(other: walkdir::Error) -> Error {
        Error::WalkDir(other)
    }
}

impl From<io::Error> for Error {
    fn from(other: io::Error) -> Error {
        Error::Io(other)
    }
}

impl From<path::StripPrefixError> for Error {
    fn from(other: path::StripPrefixError) -> Error {
        Error::StripPrefixError(other)
    }
}

impl From<(path::PathBuf, error::At<error::ParseError>)> for Error {
    fn from((path, other): (path::PathBuf, error::At<error::ParseError>)) -> Error {
        Error::Parse {
            path: path,
            err: other,
        }
    }
}

/// Specification iteration or parsing result.
pub type Result<T> = result::Result<T, Error>;
