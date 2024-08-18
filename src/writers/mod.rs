// use std::sync::mpsc::Receiver;
// 
// use crate::elements;
// 
// mod json;
// use json::write_json;
// 
// 
// pub fn write_file(sender: Sender<Box<dyn elements::Element + Send + Sync>>, file_path: PathBuf) {
// 
//     // load filter somehow, pass it to loader?
// 
//     match file_path.extension().and_then(OsStr::to_str) {
//         Some("pbf") => load_pbf(sender, file_path),
//         // Some("json") => load_json(file_path),
//         _ => panic!("Filetype not supported!")
//     }
// }
