//! Helpers for working with chunks of OSM elements.

use std::{cmp::Reverse, collections::BinaryHeap, vec::IntoIter};

use crate::elements::Element;

/// Indexed chunk of objects of a single type.
pub struct Chunk<T> {
    pub index: usize,
    pub content: T,
}

/// Chunk of OSM Elements.
pub type ElementChunk = Chunk<Box<[Element]>>;

impl IntoIterator for ElementChunk {
    type Item = Element;
    type IntoIter = IntoIter<Element>;

    fn into_iter(self) -> Self::IntoIter {
        Vec::from(self.content).into_iter()
    }
}

/// Builds a series of chunks from an iterator of objects.
#[derive(Copy, Clone, Debug)]
pub struct ChunkBuilder {
    pub max_size: usize,
    current_index: usize,
}

/// Converts an iterator of Elements into an iterator of ElementChunks.
pub struct ElementsIntoChunkIterator<I>
where
    I: Iterator<Item = Element>,
{
    builder: ChunkBuilder,
    source: I,
}

impl<I> Iterator for ElementsIntoChunkIterator<I>
where
    I: Iterator<Item = Element>,
{
    type Item = ElementChunk;

    fn next(&mut self) -> Option<Self::Item> {
        let mut elements = Vec::with_capacity(self.builder.max_size);
        for _ in 0..self.builder.max_size {
            match self.source.next() {
                Some(element) => elements.push(element),
                None if elements.is_empty() => return None,
                None => break,
            }
        }

        Some(self.builder.build_next_chunk(elements.into_boxed_slice()))
    }
}

impl ChunkBuilder {
    pub fn new(max_size: usize) -> Self {
        ChunkBuilder {
            max_size,
            current_index: 0,
        }
    }

    pub fn build_next_chunk(&mut self, elements: Box<[Element]>) -> ElementChunk {
        let chunk = Chunk {
            index: self.current_index,
            content: elements,
        };
        self.current_index += 1;
        chunk
    }

    pub fn chunk_iterator<I>(&self, iter: I) -> ElementsIntoChunkIterator<I>
    where
        I: Iterator<Item = Element>,
    {
        ElementsIntoChunkIterator {
            builder: *self,
            source: iter,
        }
    }
}

impl<T: Sized + Send> Eq for Chunk<T> {}

impl<T: Sized + Send> PartialEq for Chunk<T> {
    fn eq(&self, other: &Self) -> bool {
        self.index == other.index
    }
}

impl<T: Sized + Send> Ord for Chunk<T> {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        Reverse(self.index).cmp(&Reverse(other.index))
    }
}

impl<T: Sized + Send> PartialOrd for Chunk<T> {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

/// Sorts iterators of Chunks by index.
pub struct OrderedChunkIterator<I, T: Sized + Send>
where
    I: Iterator<Item = Chunk<T>>,
{
    source: I,
    buffer: BinaryHeap<Chunk<T>>,
    next_index: usize,
    strict: bool,
    incomplete: bool,
}

impl<I, T: Sized + Send> OrderedChunkIterator<I, T>
where
    I: Iterator<Item = Chunk<T>>,
{
    /// Restore chunk order, panicking if the source ends with a chunk missing.
    pub fn new(source: I) -> Self {
        OrderedChunkIterator {
            source,
            buffer: BinaryHeap::new(),
            next_index: 0,
            strict: true,
            incomplete: false,
        }
    }

    /// Restore chunk order, ending quietly if the source ends with a chunk
    /// missing. Check [`Self::is_incomplete`] afterwards.
    pub fn tolerant(source: I) -> Self {
        OrderedChunkIterator {
            strict: false,
            ..Self::new(source)
        }
    }

    /// True if the source ended before every expected chunk arrived.
    pub fn is_incomplete(&self) -> bool {
        self.incomplete
    }
}

impl<I, T: Sized + Send> Iterator for OrderedChunkIterator<I, T>
where
    I: Iterator<Item = Chunk<T>>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            // Hand out the next chunk in sequence as soon as it is available.
            if self
                .buffer
                .peek()
                .is_some_and(|chunk| chunk.index == self.next_index)
            {
                let chunk = self.buffer.pop().unwrap();
                self.next_index += 1;
                return Some(chunk.content);
            }

            match self.source.next() {
                Some(chunk) => self.buffer.push(chunk),
                None => {
                    if !self.buffer.is_empty() {
                        if self.strict {
                            panic!(
                                "ERROR: The source iterator of chunks was exhausted, but the next chunk could not be found (index {})",
                                self.next_index
                            );
                        }
                        self.incomplete = true;
                    }
                    return None;
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunks(indexes: &[usize]) -> Vec<Chunk<usize>> {
        indexes
            .iter()
            .map(|&index| Chunk {
                index,
                content: index * 10,
            })
            .collect()
    }

    #[test]
    fn restores_sequence() {
        let ordered: Vec<usize> =
            OrderedChunkIterator::new(chunks(&[2, 0, 3, 1]).into_iter()).collect();
        assert_eq!(ordered, vec![0, 10, 20, 30]);
    }

    #[test]
    fn tolerant_reports_missing_chunks() {
        let mut ordered = OrderedChunkIterator::tolerant(chunks(&[0, 2]).into_iter());
        assert_eq!(ordered.next(), Some(0));
        assert_eq!(ordered.next(), None);
        assert!(ordered.is_incomplete());
    }

    #[test]
    #[should_panic]
    fn strict_panics_on_missing_chunks() {
        let _: Vec<usize> = OrderedChunkIterator::new(chunks(&[1]).into_iter()).collect();
    }
}
