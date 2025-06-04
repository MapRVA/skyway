//! Helpers for working with chunks of OSM elements.

use std::{cmp::Reverse, collections::BinaryHeap};

use crate::elements::Element;

/// Indexed chunk of objects of a single type.
pub struct Chunk<T> {
    pub index: usize,
    pub content: T,
}

/// Chunk of OSM Elements.
pub type ElementChunk = Chunk<Box<[Element]>>;

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
}

impl<I, T: Sized + Send> OrderedChunkIterator<I, T>
where
    I: Iterator<Item = Chunk<T>>,
{
    pub fn new(source: I) -> Self {
        OrderedChunkIterator {
            source,
            buffer: BinaryHeap::new(),
            next_index: 0,
        }
    }
}

impl<I, T: Sized + Send> Iterator for OrderedChunkIterator<I, T>
where
    I: Iterator<Item = Chunk<T>>,
{
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        loop {
            let source_exhausted = match self.source.next() {
                // does the source iterator have more output?
                Some(output) => {
                    // push the next output from the iterator into the buffer
                    self.buffer.push(output);
                    false
                }
                None => true,
            };

            loop {
                match self.buffer.peek() {
                    // does the buffer have content available?
                    Some(output_peek) => {
                        // does the next content in the buffer have the index we want?
                        if output_peek.index == self.next_index {
                            let output = self.buffer.pop().unwrap();
                            // increment the index to represent what we're looking for next
                            self.next_index += 1;
                            return Some(output.content);
                        } else {
                            // the next content on the buffer is not what we're looking fors
                            break;
                        }
                    }
                    None => {
                        // there is no content available in the buffer,
                        // was there any fresh output from the source iterator?
                        if source_exhausted {
                            // there was not, we are done here
                            return None;
                        } else {
                            // there was, implying there could be more!
                            break;
                        }
                    }
                }
            }

            // this should never happen!
            if source_exhausted && !self.buffer.is_empty() {
                panic!(
                    "ERROR: The source iterator of chunks was exhausted, but the next chunk could not be found (index {})",
                    self.next_index
                );
            }
        }
    }
}
