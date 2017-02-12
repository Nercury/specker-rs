extern crate specker;

use std::fs;
use std::path::PathBuf;
use std::collections::HashMap;

fn main() {
    let src_dir: PathBuf = PathBuf::from(file!()).parent().map(|p| p.into()).unwrap();
    let spec_dir = src_dir.join("spec");

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