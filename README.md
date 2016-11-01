## Specker

Specker is designed to be used in tests. 
It checks if any number of files match some specification.

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

It also has iterators to run many such specification tests
in bulk.

Example code that iterates the "spec" dir, collects all "txt" specifications
and checks them:

```rust
extern crate specker;

#[test]
fn check_specifications() {
    let _ = env_logger::init();

    let out_dir = env::temp_dir();
    let src_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap());

    debug!("src dir: {:?}", src_dir);

    for maybe_spec in specker::walk_spec_dir(&src_dir.join("spec"), "txt", specker::Options {
        skip_lines: b"..",
        marker: b"##",
        var_start: b"${",
        var_end: b"}",
    }) {
        let spec = maybe_spec.unwrap();
        let test_dir_path = out_dir.join(&spec.rel_path);

        // this is your custom test function
        run_generation_and_output_to(&test_dir_path);

        // go over spec items and check if file contents match
        for (item, input_file_name) in spec.iter()
            .filter_map(
                |item| item.get_param("file")
                    .map(|param_value| (item, param_value))
            )
            {
                item.match_file_relative(&test_dir_path, input_file_name, &params)
                    .expect(&format!("failed to match file {:?}", &spec.rel_path));
            }
    }
}
```