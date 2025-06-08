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
    SkywayError,
    chunks::{Chunk, ChunkBuilder, ElementChunk},
    elements::Metadata,
    sort::{ElementSorter, SortStrategy},
};

#[cfg(feature = "filter")]
use crate::filter::{ElementFilter, build_filter, build_keep_list};

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
    // Get metadata the channel. All readers are expected to pass something here.
    // (Even if there is no metadata in the input file, e.g. OPL)
    let mut metadata = metadata_receiver.into_iter().next().unwrap();

    // The user can, using the --preserve-generator option, request that we keep
    // the input file's generator metadata, rather than inject skyway version info.
    if !preserve_generator {
        metadata.generator = Some(format!("skyway v{}", env!("CARGO_PKG_VERSION")))
    }

    // This is where further modifications may be done to the metadata in the future.

    // Send the transformed metadata on.
    metadata_sender.send(metadata).unwrap();
}

pub trait Reader: Sized + Clone + Send + 'static {
    /// Reads data into skyway.
    ///
    /// * `src`: Path of input file, None if the input is standard input.
    /// * `metadata_sender`: Sender for a channel of (1) `Metadata`.
    /// * `chunk_builder`: `ChunkBuilder` for building Chunks as elements are read.
    //
    // This trait contains a lot of high-level logic that dictates
    // how conversions should run in skyway.
    //
    // I wish to one day separate most of this logic from the Reader trait.
    // It muddies the purpose of the Reader, and makes the skyway
    // codebase more difficult to work on. However, my attempts to
    // do so have been thwarted by the Rust compiler. Since I cannot
    // know the specific types of the Reader and Writer trait
    // implementors we will use until runtime, it is difficult to create
    // both of them in an outside function and pass one's output into the
    // other. I circumvent this limitation by creating one inside the
    // other's implementation, below.

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
        sort_strategy: SortStrategy,
        preserve_generator: bool,
    ) -> Result<(Receiver<ElementChunk>, Receiver<Metadata>), SkywayError> {
        // Channel for passing file metadata from the reader thread to
        // the metadata transformation thread.
        let (metadata_sender, metadata_receiver) = channel();

        // A ChunkBuilder for the reader to use, with the given chunk size.
        let chunk_builder = ChunkBuilder::new(chunk_size);

        // Any intermediate metadata transformations should happen in the
        // transform_metadata function.
        let (trans_metadata_sender, trans_metadata_receiver) = channel();
        thread::spawn(move || {
            transform_metadata(metadata_receiver, trans_metadata_sender, preserve_generator)
        });

        // Channel for sending from filter, receiving for sort.
        let (filter_chunk_sender, filter_chunk_receiver) = channel();

        // Channel for sending from sort, receiving for writing.
        let (sort_chunk_sender, final_chunk_receiver) = channel();

        // Spawn the sorting thread.
        //
        // This thread will receive ElementChunks from the reader
        // (and filters if applicable), and then sort them. It in
        // turn sends ElementChunks out for writing.
        thread::spawn(move || {
            // create sorter based on strategy
            let sorter = ElementSorter::new(sort_strategy);

            sorter.sort(filter_chunk_receiver, sort_chunk_sender);
        });

        // If there are filters, we will need to parse and run them first.
        #[cfg(feature = "filter")]
        if filters.len() > 0 {
            // References are when ways and relations reference other elements,
            // which need to be kept by default if their referring elements are kept.
            // So even if a filter decides to keep one relation, we need to ensure
            // that all elements that relation references will be kept, too.
            //
            // Processing is potentially much faster if the user opts to omit these
            // references using the --omit-references flag, so I've written the
            // following code to speed things up in that case.
            //
            // However, there are few real-world reasons to omit references, so
            // I doubt many users will use this option.
            if omit_references {
                // Building the filter means "compiling" the list of filters into a single
                // function for faster processing of ElementChunks
                let combined_filter = build_filter(filters);

                // Use the trait implementor's read_file function to read the input file,
                // and run those resulting ElementChunks through the combined_filter.
                let chunk_iterator = self
                    .read_file(source, metadata_sender, chunk_builder)
                    .map(|chunk| combined_filter(chunk));

                // Send out each of those filtered ElementChunks through the correct channel.
                chunk_iterator.for_each(|chunk| {
                    filter_chunk_sender
                        .send(chunk)
                        .expect("Unable to send chunk.")
                });
            } else {
                // We need to worry about elements referencing each other.
                //
                // This is a significant problem, because we'd rather not store all of the
                // elements from the input file in memory if we can help it. We want skyway
                // to work even if the user's computer does not have enough RAM to fit the
                // entire OpenStreetMap database.
                //
                // My current strategy, and this is subject to change: read the input file
                // twice. I believe that my reading functions are fast enough that it won't
                // cost us too much time. The first time around we filter every element, and
                // if it passes through the filter we remember its ID, as well as every ID it
                // references, in memory. The second time we just keep every element in the
                // list of
                //
                // This is a little bit of an oversimplification because we need to recursively
                // resolve every relation, as relations can reference other relations. More on
                // that in the build_keep_list function documentation.

                // Channel that will send ElementChunks from the reading thread to the filter
                let (first_filter_chunk_sender, first_filter_chunk_receiver) = channel();

                // Not ideal but for now I'm just cloning a bunch of things to let them pass
                // between thread easier.
                let self_clone = self.clone();
                let source_clone = source.clone();
                let metadata_sender_clone = metadata_sender.clone();

                // Spin off the read thread, which will send ElementChunks to the filter
                thread::spawn(move || {
                    self_clone
                        .read_file(source_clone, metadata_sender_clone, chunk_builder)
                        .for_each(|c| {
                            first_filter_chunk_sender
                                .send(c)
                                .expect("Unable to send chunk to channel.")
                        })
                });

                // Generate a HashSet of element IDs to keep. Please see the
                // build_keep_list function for further documentation.
                //
                // Please note that this is where all filtering happens!
                let keep_ids = build_keep_list(&filters, first_filter_chunk_receiver);

                // Make keep_ids thread-safe
                // TODO: Consider performance implications of this.
                let keep_ids = Arc::new(Mutex::new(keep_ids));

                // "Fake" channel that we won't use, to make the reader thread happy.
                // We already read the metadata the first time around.
                let (fake_metadata_sender, fake_metadata_receiver) = channel();

                // Building the filter means "compiling" the list of filters into a single
                // function for faster processing of ElementChunks
                // let combined_filter = build_filter(filters);

                // At this point, we know the IDs of every element we want to keep, in the
                // keep_ids Arc<Mutex<HashSet<i64>>>
                //
                // Now, let's re-read the input file, only keeping what's in keep_ids.
                self.read_file(source, fake_metadata_sender, chunk_builder)
                    // run the filter on each chunk now
                    // TODO: delete the following line?
                    // .map(|chunk| combined_filter(chunk))
                    // keep elements depending on if we determined they are necessary above
                    .for_each(move |chunk| {
                        // Vec that will hold each element we keep from this ElementChunk.
                        let mut elements = Vec::new();

                        // We are iterating through a ParallelIterator, which is why I am
                        // locking keep_ids here. I reckon there are better ways of
                        // accomplishing this!
                        let mut keep_ids_lock = keep_ids.lock().unwrap();

                        // For each element, push it to the elements Vec (to keep) if it
                        // existed in the keep_ids Vec. At the same time, remove the ID
                        // from keep_ids.
                        //
                        // TODO: is it really necessary to delete it? Are we really going
                        // to check if keep_ids is fully exhausted in the end?
                        for element in chunk.content {
                            if keep_ids_lock.remove(&element.id) {
                                elements.push(element);
                            }
                        }
                        // Drop the lock so parallel processes can have their fun with keep_ids
                        drop(keep_ids_lock);

                        // Send elements Vec out, neatly packaged as a Chunk
                        filter_chunk_sender
                            .send(Chunk {
                                content: elements.into_boxed_slice(),
                                index: chunk.index,
                            })
                            .expect("Unable to send chunk.")
                    });

                // Dropping the fake metadata receiver now means it was available during the
                // second read, even if we never intended to do anything with it...
                drop(fake_metadata_receiver);
            }
        } else {
            // No filters!
            //
            // This is great, no need to track references because we already
            // plan on keeping all elements.
            //
            // Read elements into chunks using the trait implementor's
            // read_file function, and send of those chunks along.
            self.read_file(source, metadata_sender, chunk_builder)
                .for_each(|chunk| {
                    filter_chunk_sender
                        .send(chunk)
                        .expect("Unable to send chunk.")
                });

            // Receiver will block if we don't explicitly drop this channel.
            drop(filter_chunk_sender);
        };

        #[cfg(not(feature = "filter"))]
        self.read_file(source, metadata_sender, chunk_builder)
            .for_each(|chunk| {
                filter_chunk_sender
                    .send(chunk)
                    .expect("Unable to send chunk.")
            });

        // Receiver will block if we don't explicitly drop this channel.
        #[cfg(not(feature = "filter"))]
        drop(filter_chunk_sender);

        Ok((final_chunk_receiver, trans_metadata_receiver))
    }
}
