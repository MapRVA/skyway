use std::path::Path;
use std::ffi::OsStr;

mod pbf;
use pbf::load_pbf;

mod json;
use json::load_json;


pub fn load_file(file_path: &Path) {

    // load filter somehow, pass it to loader?

    match file_path.extension().and_then(OsStr::to_str) {
        Some("pbf") => load_pbf(file_path),
        Some("json") => load_json(file_path),
        _ => println!("Filetype not supported!")
    }
}
