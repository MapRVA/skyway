//! Reads OSM data into skyway.

use rayon::prelude::*;

use std::{
    fs,
    io::{BufRead, BufReader, Read, stdin},
    path::PathBuf,
    sync::{
        Arc, Mutex,
        mpsc::{Receiver, Sender, channel},
    },
    thread,
};

use crate::{
    OsmFormat, SkywayError,
    chunks::{Chunk, ChunkBuilder, ElementChunk},
    elements::Metadata,
    sort::{ElementSorter, SortStrategy},
    writers::*,
};

#[cfg(feature = "filter")]
use crate::filter::{ElementFilter, build_filter, build_keep_list};

#[cfg(not(feature = "filter"))]
use std::convert::identity;

#[cfg(feature = "json")]
mod json;
#[cfg(feature = "json")]
pub use json::JsonReader;

#[cfg(feature = "opl")]
mod opl;
#[cfg(feature = "opl")]
pub use opl::OplReader;

#[cfg(feature = "osmx")]
mod osmx;

#[cfg(feature = "pbf")]
mod pbf;
#[cfg(feature = "pbf")]
pub use pbf::PbfReader;

#[cfg(feature = "xml")]
mod xml;
#[cfg(feature = "xml")]
pub use xml::XmlReader;

pub fn open(path: PathBuf) -> Box<dyn Read + Send> {
    match fs::File::open(path) {
        Ok(f) => Box::new(f) as Box<dyn Read + Send>,
        Err(e) => panic!("Unable to open input file: {e:?}"),
    }
}

pub fn get_reader(src: Option<PathBuf>) -> Box<dyn BufRead + Send> {
    Box::new(BufReader::new(match src {
        Some(path) => open(path),
        None => Box::new(stdin()),
    }))
}

fn transform_metadata(
    metadata_receiver: Receiver<Metadata>,
    metadata_sender: Sender<Metadata>,
    preserve_generator: bool,
) {
    let mut metadata = metadata_receiver.into_iter().next().unwrap();
    if !preserve_generator {
        metadata.generator = Some(format!("skyway v{}", env!("CARGO_PKG_VERSION")))
    }
    metadata_sender.send(metadata).unwrap();
}

pub trait Reader: Sized + Clone + Send + 'static {
    /// Create a new instance of this Reader

    /// Reads data into skyway.
    ///
    /// * `sender`: Sender for a channel of `Element`s.
    /// * `metadata_sender`: Sender for a channel of (1) `Metadata`.
    fn read_file(
        self,
        src: Option<PathBuf>,
        metadata_sender: Sender<Metadata>,
        chunk_builder: ChunkBuilder,
    ) -> impl ParallelIterator<Item = ElementChunk>;

    fn run_conversion(
        self,
        source: Option<PathBuf>,
        chunk_size: usize,
        #[cfg(feature = "filter")] filters: Vec<Box<dyn ElementFilter>>,
        #[cfg(feature = "filter")] omit_references: bool,
        output_format: OsmFormat,
        sort_strategy: SortStrategy,
        dest: Option<PathBuf>,
        preserve_generator: bool,
    ) -> Result<(), SkywayError> {
        let (metadata_sender, metadata_receiver) = channel();
        let chunk_builder = ChunkBuilder::new(chunk_size);

        // any intermediate metadata transformations should happen here
        let (trans_metadata_sender, trans_metadata_receiver) = channel();
        thread::spawn(move || {
            transform_metadata(metadata_receiver, trans_metadata_sender, preserve_generator)
        });

        // Send to sort
        let (filter_chunk_sender, filter_chunk_receiver) = channel();

        // Receive from sort
        let (sort_chunk_sender, final_chunk_receiver) = channel();

        thread::spawn(move || {
            // create sorter based on strategy
            let sorter = ElementSorter::new(sort_strategy);

            sorter.sort(filter_chunk_receiver, sort_chunk_sender);
        });

        #[cfg(feature = "filter")]
        if filters.len() > 0 {
            if omit_references {
                let combined_filter = build_filter(filters);

                let chunk_iterator = self
                    .read_file(source, metadata_sender, chunk_builder)
                    .map(|chunk| combined_filter(chunk));

                chunk_iterator.for_each(|chunk| {
                    filter_chunk_sender
                        .send(chunk)
                        .expect("Unable to send chunk.")
                });
            } else {
                let (first_filter_chunk_sender, first_filter_chunk_receiver) = channel();
                let self_clone = self.clone();
                let source_clone = source.clone();
                let metadata_sender_clone = metadata_sender.clone();
                thread::spawn(move || {
                    self_clone
                        .read_file(source_clone, metadata_sender_clone, chunk_builder)
                        .for_each(|c| {
                            first_filter_chunk_sender
                                .send(c)
                                .expect("Unable to send chunk to channel.")
                        })
                });

                // generate a list of element IDs to keep
                let keep_ids = build_keep_list(filters, first_filter_chunk_receiver);
                let keep_ids = Arc::new(Mutex::new(keep_ids));

                // re-read the input file, only keeping the elements in keep_ids
                self.read_file(source, metadata_sender, chunk_builder)
                    .for_each(move |chunk| {
                        let mut elements = Vec::new();
                        let mut keep_ids_lock = keep_ids.lock().unwrap();
                        for element in chunk.content {
                            if keep_ids_lock.remove(&element.id) {
                                elements.push(element);
                            }
                        }
                        drop(keep_ids_lock);

                        filter_chunk_sender
                            .send(Chunk {
                                content: elements.into_boxed_slice(),
                                index: chunk.index,
                            })
                            .expect("Unable to send chunk.")
                    });
            }
        } else {
            self.read_file(source, metadata_sender, chunk_builder)
                .for_each(|chunk| {
                    filter_chunk_sender
                        .send(chunk)
                        .expect("Unable to send chunk.")
                });
        };

        #[cfg(not(feature = "filter"))]
        let chunk_iterator: Box<impl ParallelIterator<ElementChunk>> =
            self.read_file(source, metadata_sender, chunk_builder);

        #[allow(unreachable_patterns)]
        match output_format {
            #[cfg(feature = "json")]
            OsmFormat::Json => JsonWriter { overpass: false }.write(
                final_chunk_receiver,
                trans_metadata_receiver,
                dest,
            ),
            #[cfg(feature = "o5m")]
            OsmFormat::O5m => {
                O5mWriter {}.write(final_chunk_receiver, trans_metadata_receiver, dest)
            }
            #[cfg(feature = "opl")]
            OsmFormat::Opl => {
                OplWriter {}.write(final_chunk_receiver, trans_metadata_receiver, dest)
            }
            #[cfg(feature = "json")]
            OsmFormat::Overpass => JsonWriter { overpass: true }.write(
                final_chunk_receiver,
                trans_metadata_receiver,
                dest,
            ),
            #[cfg(feature = "xml")]
            OsmFormat::Xml => {
                XmlWriter {}.write(final_chunk_receiver, trans_metadata_receiver, dest)
            }
            _ => Err(SkywayError::UnexpectedError(
                "A file conversion was attempted with an unknown output format.".to_owned(),
            )),
        }
    }
}
