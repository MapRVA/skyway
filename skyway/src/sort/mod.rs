//! Utilities for sorting OSM elements.

#[cfg(feature = "cli")]
use clap::ValueEnum;

use crate::{
    chunks::{ChunkBuilder, ElementChunk},
    elements::{Element, ElementType},
    plan::{Order, OutputOrderRequest},
};

/// Enum that represents the different sorting strategies skyway supports.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
pub enum SortStrategy {
    // First nodes, then ways, then relations
    #[cfg_attr(feature = "cli", value(name = "type"))]
    Type,
    // All elements sorted by ID
    #[cfg_attr(feature = "cli", value(name = "id"))]
    Id,
    // Grouped by type, elements are sorted by ID within each type group
    #[cfg_attr(feature = "cli", value(name = "type-id"))]
    TypeAndId,
    // Explicit none, do not sort the elements regardless of default behavior
    #[cfg_attr(feature = "cli", value(name = "none"))]
    None,
}

impl From<SortStrategy> for OutputOrderRequest {
    fn from(strategy: SortStrategy) -> Self {
        match strategy {
            SortStrategy::Type => OutputOrderRequest::Sorted(Order::Type),
            SortStrategy::Id => OutputOrderRequest::Sorted(Order::Id),
            SortStrategy::TypeAndId => OutputOrderRequest::Sorted(Order::TypeAndId),
            SortStrategy::None => OutputOrderRequest::PreserveInput,
        }
    }
}

fn type_rank(element: &Element) -> u8 {
    match element.element_type {
        ElementType::Node { .. } => 0,
        ElementType::Way { .. } => 1,
        ElementType::Relation { .. } => 2,
    }
}

/// Sort `elements` into `order`. The sort is stable, so elements that compare
/// equal keep their existing relative sequence.
pub fn sort_elements(elements: &mut [Element], order: Order) {
    match order {
        Order::Type => elements.sort_by_key(type_rank),
        Order::Id => elements.sort_by_key(|e| e.id),
        Order::TypeAndId => elements.sort_by_key(|e| (type_rank(e), e.id)),
    }
}

/// Split `elements` into chunks of at most `chunk_size`, numbered from zero.
pub fn chunk_elements(
    elements: Vec<Element>,
    chunk_size: usize,
) -> impl Iterator<Item = ElementChunk> {
    ChunkBuilder::new(chunk_size).chunk_iterator(elements.into_iter())
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::elements::ElementKey;

    fn element(key: ElementKey) -> Element {
        Element {
            changeset: None,
            user: None,
            version: None,
            uid: None,
            id: key.id(),
            timestamp: None,
            visible: None,
            tags: HashMap::new(),
            element_type: match key {
                ElementKey::Node(_) => ElementType::Node { lat: 0, lon: 0 },
                ElementKey::Way(_) => ElementType::Way { nodes: Vec::new() },
                ElementKey::Relation(_) => ElementType::Relation {
                    members: Vec::new(),
                },
            },
        }
    }

    fn keys(elements: &[Element]) -> Vec<ElementKey> {
        elements.iter().map(Element::key).collect()
    }

    fn sample() -> Vec<Element> {
        use ElementKey::*;
        [
            Way(20),
            Node(3),
            Relation(10),
            Node(1),
            Way(5),
            Relation(2),
            Node(2),
        ]
        .into_iter()
        .map(element)
        .collect()
    }

    #[test]
    fn sort_by_type_is_stable() {
        use ElementKey::*;
        let mut elements = sample();
        sort_elements(&mut elements, Order::Type);
        assert_eq!(
            keys(&elements),
            vec![
                Node(3),
                Node(1),
                Node(2),
                Way(20),
                Way(5),
                Relation(10),
                Relation(2)
            ]
        );
    }

    #[test]
    fn sort_by_id_ignores_type() {
        use ElementKey::*;
        let mut elements = sample();
        sort_elements(&mut elements, Order::Id);
        assert_eq!(
            keys(&elements),
            vec![
                Node(1),
                Relation(2),
                Node(2),
                Node(3),
                Way(5),
                Relation(10),
                Way(20)
            ]
        );
    }

    #[test]
    fn sort_by_type_and_id() {
        use ElementKey::*;
        let mut elements = sample();
        sort_elements(&mut elements, Order::TypeAndId);
        assert_eq!(
            keys(&elements),
            vec![
                Node(1),
                Node(2),
                Node(3),
                Way(5),
                Way(20),
                Relation(2),
                Relation(10)
            ]
        );
    }

    #[test]
    fn chunking_numbers_from_zero() {
        let chunks: Vec<ElementChunk> = chunk_elements(sample(), 3).collect();
        assert_eq!(chunks.len(), 3);
        assert_eq!(
            chunks.iter().map(|c| c.index).collect::<Vec<_>>(),
            vec![0, 1, 2]
        );
        assert_eq!(
            chunks.iter().map(|c| c.content.len()).collect::<Vec<_>>(),
            vec![3, 3, 1]
        );
    }
}
