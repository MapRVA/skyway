use osmx::Database;
use std::path::PathBuf;
use std::sync::mpsc::Sender;

use crate::elements::{Element, Metadata};
use crate::readers::Reader;

pub struct OsmxReader {
    pub path: PathBuf,
}

fn extract_elements(database: Database) -> Vec<Vec<Element>> {
    unimplemented!()
}

impl Reader for OsmxReader {
    fn read(&mut self, sender: Sender<Vec<Element>>, metadata_sender: Sender<Metadata>) {
        // create an empty Metadata object
        let metadata = Metadata::default();

        // send metadata to main thread
        metadata_sender
            .send(metadata)
            .expect("Couldn't send metdata to main thread!");

        let osmx_database = match Database::open(&self.path) {
            Ok(v) => v,
            Err(e) => panic!("Unable to open OSM Express database: {e:?}"),
        };

        extract_elements(osmx_database).into_iter().for_each(|c| {
            sender
                .send(c)
                .expect("Unable to send chunk of elements to channel.")
        })
    }
}
