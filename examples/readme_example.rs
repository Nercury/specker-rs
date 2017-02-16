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
                    panic!("\n{}", specker::display_error_for_file(&path, &e));
                }
            }
    }

    println!("example succeeded, try to modify /spec files to see some errors")
}