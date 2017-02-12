## Specker

[![Build Status](https://travis-ci.org/Nercury/specker-rs.svg?branch=master)](https://travis-ci.org/Nercury/specker-rs)

Checks if any number of files match some specification.
Designed for testing file generation.

Let's say we have this specification:

```
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

```
1 | <bddy>
  | ^^^^^^
  | Expected "<body>", found "<bddy>"
```

It also has iterators to run many such specification tests
in bulk.

Example code that iterates the "spec" dir, collects all "txt" specifications
and checks them:

```rust
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
        let spec_file = maybe_spec.unwrap();

        // go over spec items and check if file contents match
        for (item, input_file_name) in spec_file.spec.iter()
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
                    println!("{}", specker::display_error(&path, &e));
                    // print one-liner error
                    panic!("{}", e);
                }
            }
    }
}
```

## License

Licensed under either of

 * Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
 * MIT license ([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in the work by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.