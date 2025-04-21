use std::{
    sync::mpsc::{Receiver, Sender, channel},
    thread,
};

#[cfg(feature = "cli")]
use clap::ValueEnum;

use crate::{
    chunks::ElementChunk,
    elements::{Element, ElementType},
};

/// Enum that represents the different sorting strategies skyway supports.
#[cfg(feature = "cli")]
#[derive(Clone, ValueEnum)]
pub enum SortStrategy {
    // First nodes, then ways, then relations
    #[value(name = "type")]
    Type,
    // All elements sorted by ID
    #[value(name = "id")]
    Id,
    // Grouped by type, elements are sorted by ID within each type group
    #[value(name = "type-id")]
    TypeAndId,
    // Explicit none, do not sort the elements regardless of default behavior
    #[value(name = "none")]
    None,
}

enum ElementStorage {
    ById {
        elements: Vec<Element>,
    },
    ByType {
        nodes: Vec<Element>,
        ways: Vec<Element>,
        relations: Vec<Element>,
    },
    ByTypeAndId {
        nodes: Vec<Element>,
        ways: Vec<Element>,
        relations: Vec<Element>,
    },
    None {
        elements: Vec<Element>,
    },
}

impl ElementStorage {
    pub fn append(&mut self, element: Element) {
        match self {
            ElementStorage::ById { elements } => elements.push(element),
            ElementStorage::ByType {
                nodes,
                ways,
                relations,
            } => match element.element_type {
                ElementType::Node { .. } => nodes.push(element),
                ElementType::Way { .. } => ways.push(element),
                ElementType::Relation { .. } => relations.push(element),
            },
            ElementStorage::ByTypeAndId {
                nodes,
                ways,
                relations,
            } => match element.element_type {
                ElementType::Node { .. } => nodes.push(element),
                ElementType::Way { .. } => ways.push(element),
                ElementType::Relation { .. } => relations.push(element),
            },
            ElementStorage::None { elements } => elements.push(element),
        }
    }

    pub fn new(sort_strategy: SortStrategy) -> Self {
        match sort_strategy {
            SortStrategy::Id => ElementStorage::ById {
                elements: Vec::new(),
            },
            SortStrategy::Type => ElementStorage::ByType {
                nodes: Vec::new(),
                ways: Vec::new(),
                relations: Vec::new(),
            },
            SortStrategy::TypeAndId => ElementStorage::ByTypeAndId {
                nodes: Vec::new(),
                ways: Vec::new(),
                relations: Vec::new(),
            },
            SortStrategy::None => ElementStorage::None {
                elements: Vec::new(),
            },
        }
    }

    pub fn sort(mut self) -> Self {
        match &mut self {
            ElementStorage::ById { elements } => {
                elements.sort_by(|a, b| a.id.cmp(&b.id));
            }
            ElementStorage::ByType { .. } => (),
            ElementStorage::ByTypeAndId {
                nodes,
                ways,
                relations,
            } => {
                nodes.sort_by(|a, b| a.id.cmp(&b.id));
                ways.sort_by(|a, b| a.id.cmp(&b.id));
                relations.sort_by(|a, b| a.id.cmp(&b.id));
            }
            ElementStorage::None { .. } => (),
        }
        self
    }
}

struct ElementStorageChunker {
    storage: ElementStorage,
    chunk_size: usize,
    current_index: usize,
}

impl ElementStorageChunker {
    fn new(storage: ElementStorage, chunk_size: usize) -> Self {
        ElementStorageChunker {
            storage,
            chunk_size,
            current_index: 0,
        }
    }

    fn extract_next_chunk(&mut self, size: usize) -> Option<Vec<Element>> {
        match &mut self.storage {
            ElementStorage::ById { elements } | ElementStorage::None { elements } => {
                if elements.is_empty() {
                    return None;
                }

                let take_count = elements.len().min(size);
                let remaining = elements.len() - take_count;

                // Take elements from the end to avoid shifting the entire vector
                let chunk: Vec<Element> = elements.drain(remaining..).collect();

                Some(chunk)
            }
            ElementStorage::ByType {
                nodes,
                ways,
                relations,
            }
            | ElementStorage::ByTypeAndId {
                nodes,
                ways,
                relations,
            } => {
                // FIXME: this won't send a partial chunk once one element type is completed
                let mut chunk = Vec::with_capacity(size);

                while !nodes.is_empty() && chunk.len() < size {
                    chunk.push(nodes.pop().unwrap());
                }

                while !ways.is_empty() && chunk.len() < size {
                    chunk.push(ways.pop().unwrap());
                }

                while !relations.is_empty() && chunk.len() < size {
                    chunk.push(relations.pop().unwrap());
                }

                if chunk.is_empty() { None } else { Some(chunk) }
            }
        }
    }
}

impl Iterator for ElementStorageChunker {
    type Item = ElementChunk;

    fn next(&mut self) -> Option<Self::Item> {
        let elements = self.extract_next_chunk(self.chunk_size)?;

        let chunk = ElementChunk {
            index: self.current_index,
            content: elements.into_boxed_slice(),
        };

        self.current_index += 1;
        Some(chunk)
    }
}

pub struct ElementSorter {
    sort_strategy: SortStrategy,
}

impl ElementSorter {
    pub fn new(sort_strategy: SortStrategy) -> Self {
        ElementSorter { sort_strategy }
    }

    pub fn sort(self, chunk_receiver: Receiver<ElementChunk>, chunk_sender: Sender<ElementChunk>) {
        let mut element_storage = ElementStorage::new(self.sort_strategy);

        let (new_chunk_sender, new_chunk_receiver) = channel::<ElementChunk>();

        thread::spawn(move || {
            element_storage = element_storage.sort();

            for chunk in new_chunk_receiver {
                for element in chunk.content {
                    element_storage.append(element);
                }
            }

            let element_storage_chunker = ElementStorageChunker::new(element_storage, 8000);
            for chunk in element_storage_chunker.into_iter() {
                chunk_sender.send(chunk).expect("Unable to send chunk.")
            }
        });

        for chunk in chunk_receiver {
            new_chunk_sender.send(chunk).expect("Unable to send chunk.")
        }
    }
}
