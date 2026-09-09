//! Reference discovery and closure for reference-preserving filtering.
//!
//! Reference-preserving filtering runs in two phases:
//!
//! ```text
//! discovery:
//!     decode → evaluate candidates → collect seeds and edges → closure
//!
//! emission:
//!     replay → keep-set lookup → transform retained elements
//!            → preserve sequence or sort → write
//! ```
//!
//! Discovery is independent of chunk order, so its per-chunk summaries can be
//! merged in any order. Emission does need sequence restoration when the
//! input sequence is being preserved.
//!
//! Only snapshot data is supported: one element per `(type, ID)`. That is
//! enforced rather than assumed, because merging summaries that disagree about
//! what an element references would otherwise resolve nondeterministically.
//! Enforcement covers every element that carries references; a duplicate node,
//! or a duplicate deleted way or relation, is not detected, because neither can
//! change the merged result.

use std::collections::{HashMap, HashSet, hash_map::Entry};

use crate::{SkywayError, chunks::ElementChunk, elements::ElementKey};

use super::FilterProgram;

/// Reject an input that has more than one element with the same identity.
fn duplicate_identity(key: ElementKey) -> SkywayError {
    SkywayError::UnsupportedHistoryInput(format!(
        "the input has more than one {key} carrying references. Reference \
         preservation needs one element per (type, ID), so it supports snapshot \
         data only, not full-history files. Use --omit-references to filter in \
         one pass."
    ))
}

/// What a discovery pass learned about a set of elements.
#[derive(Debug, Default)]
pub struct Discovery {
    /// Elements selected directly by the filters.
    pub selected: HashSet<ElementKey>,
    /// Elements referenced by each element's candidate output.
    pub edges: HashMap<ElementKey, Vec<ElementKey>>,
}

impl Discovery {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn is_empty(&self) -> bool {
        self.selected.is_empty() && self.edges.is_empty()
    }

    /// Combine two summaries. The smaller one is folded into the larger one.
    ///
    /// Two summaries that both describe the same element's references cannot be
    /// combined: silently keeping one of the two reference lists would make the
    /// keep set depend on the order summaries happen to be reduced in. Selection
    /// is a plain union, so repeating an identity there is harmless.
    ///
    /// Whether an input is rejected is deterministic; which duplicate is named
    /// is not, since an input with several of them fails on whichever is reduced
    /// first.
    pub fn try_merge(mut self, mut other: Self) -> Result<Self, SkywayError> {
        let size = |d: &Discovery| d.selected.len() + d.edges.len();
        if size(&other) > size(&self) {
            std::mem::swap(&mut self, &mut other);
        }

        self.selected.extend(other.selected);
        for (key, references) in other.edges {
            match self.edges.entry(key) {
                Entry::Occupied(_) => return Err(duplicate_identity(key)),
                Entry::Vacant(slot) => {
                    slot.insert(references);
                }
            }
        }

        Ok(self)
    }
}

/// Evaluate every element in `chunk`, recording which are selected and what
/// their candidate outputs reference.
///
/// References are taken after transformation, including for elements that
/// might later be retained only as dependencies, so that emission and
/// discovery agree on what a retained element references.
pub fn discover_chunk(
    chunk: ElementChunk,
    program: &dyn FilterProgram,
) -> Result<Discovery, SkywayError> {
    let mut result = Discovery::new();

    for mut element in chunk {
        let key = element.key();

        if program.evaluate(&mut element) {
            result.selected.insert(key);
        }

        let references = element.reference_keys()?;
        if !references.is_empty() && result.edges.insert(key, references).is_some() {
            return Err(duplicate_identity(key));
        }
    }

    Ok(result)
}

/// Compute the set of elements to keep: everything selected plus everything
/// reachable from a selected element through references.
///
/// Missing dependencies are simply absent from the input; they are added to
/// the keep set but never emitted. Cycles between relations terminate because
/// each key is visited at most once.
pub fn reference_closure(discovery: Discovery) -> HashSet<ElementKey> {
    let Discovery { selected, edges } = discovery;
    let mut pending: Vec<ElementKey> = selected.iter().copied().collect();
    let mut keep = selected;

    while let Some(key) = pending.pop() {
        if let Some(references) = edges.get(&key) {
            for &reference in references {
                if keep.insert(reference) {
                    pending.push(reference);
                }
            }
        }
    }

    keep
}

/// Emit the elements of `chunk` that are in `keep`, transformed by `program`.
///
/// The keep set has already decided inclusion; the program is re-run only to
/// reproduce each candidate's transformation.
pub fn emit_chunk(
    chunk: ElementChunk,
    keep: &HashSet<ElementKey>,
    program: &dyn FilterProgram,
) -> ElementChunk {
    let index = chunk.index;

    let content = chunk
        .into_iter()
        .filter_map(|mut element| {
            if !keep.contains(&element.key()) {
                return None;
            }

            let _selected_directly = program.evaluate(&mut element);
            Some(element)
        })
        .collect::<Vec<_>>()
        .into_boxed_slice();

    ElementChunk { index, content }
}

/// Emit the elements of `chunk` that the program selects, in one pass.
pub fn filter_chunk(chunk: ElementChunk, program: &dyn FilterProgram) -> ElementChunk {
    let index = chunk.index;

    let content = chunk
        .into_iter()
        .filter_map(|mut element| program.evaluate(&mut element).then_some(element))
        .collect::<Vec<_>>()
        .into_boxed_slice();

    ElementChunk { index, content }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::*;
    use crate::elements::{Element, ElementType, Member, SimpleElementType};

    fn element(element_type: ElementType, id: i64, tags: &[(&str, &str)]) -> Element {
        Element {
            changeset: None,
            user: None,
            version: None,
            uid: None,
            id,
            timestamp: None,
            visible: None,
            tags: tags
                .iter()
                .map(|(k, v)| (k.to_string(), v.to_string()))
                .collect::<HashMap<_, _>>(),
            element_type,
        }
    }

    fn node(id: i64, tags: &[(&str, &str)]) -> Element {
        element(ElementType::Node { lat: 0, lon: 0 }, id, tags)
    }

    fn way(id: i64, nodes: &[i64], tags: &[(&str, &str)]) -> Element {
        element(
            ElementType::Way {
                nodes: nodes.to_vec(),
            },
            id,
            tags,
        )
    }

    fn relation(id: i64, members: &[(SimpleElementType, i64)], tags: &[(&str, &str)]) -> Element {
        element(
            ElementType::Relation {
                members: members
                    .iter()
                    .map(|(t, id)| Member {
                        t: Some(t.clone()),
                        id: *id,
                        role: None,
                    })
                    .collect(),
            },
            id,
            tags,
        )
    }

    fn chunk(elements: Vec<Element>) -> ElementChunk {
        ElementChunk {
            index: 0,
            content: elements.into_boxed_slice(),
        }
    }

    /// Keeps elements with a `keep` tag and strips a `drop-me` tag.
    struct KeepTagged;

    impl FilterProgram for KeepTagged {
        fn evaluate(&self, element: &mut Element) -> bool {
            element.tags.remove("drop-me");
            element.tags.contains_key("keep")
        }
    }

    #[test]
    fn discovery_records_selection_and_typed_edges() {
        let discovery = discover_chunk(
            chunk(vec![
                node(1, &[]),
                way(5, &[1, 2], &[("keep", "yes")]),
                relation(5, &[(SimpleElementType::Way, 5)], &[]),
            ]),
            &KeepTagged,
        )
        .unwrap();

        assert_eq!(
            discovery.selected,
            HashSet::from([ElementKey::Way(5)]),
            "only the tagged way is selected"
        );
        assert_eq!(
            discovery.edges.get(&ElementKey::Way(5)),
            Some(&vec![ElementKey::Node(1), ElementKey::Node(2)])
        );
        assert_eq!(
            discovery.edges.get(&ElementKey::Relation(5)),
            Some(&vec![ElementKey::Way(5)]),
            "a relation and a way with the same ID are distinct keys"
        );
        assert!(!discovery.edges.contains_key(&ElementKey::Node(1)));
    }

    #[test]
    fn untyped_members_are_an_error() {
        let untyped = Element {
            element_type: ElementType::Relation {
                members: vec![Member {
                    t: None,
                    id: 7,
                    role: None,
                }],
            },
            ..node(9, &[])
        };

        assert!(discover_chunk(chunk(vec![untyped]), &KeepTagged).is_err());
    }

    #[test]
    fn closure_follows_references_and_terminates_on_cycles() {
        let a = ElementKey::Relation(1);
        let b = ElementKey::Relation(2);
        let w = ElementKey::Way(3);
        let n = ElementKey::Node(4);
        let unrelated = ElementKey::Node(99);

        let discovery = Discovery {
            selected: HashSet::from([a]),
            edges: HashMap::from([
                (a, vec![b, w]),
                (b, vec![a]),
                (w, vec![n]),
                (unrelated, vec![n]),
            ]),
        };

        assert_eq!(reference_closure(discovery), HashSet::from([a, b, w, n]));
    }

    #[test]
    fn merge_unions_both_summaries() {
        let mut left = Discovery::new();
        left.selected.insert(ElementKey::Node(1));
        left.edges
            .insert(ElementKey::Way(1), vec![ElementKey::Node(1)]);

        let mut right = Discovery::new();
        right.selected.insert(ElementKey::Node(2));
        right
            .edges
            .insert(ElementKey::Way(2), vec![ElementKey::Node(2)]);
        right
            .edges
            .insert(ElementKey::Way(3), vec![ElementKey::Node(3)]);

        let merged = left.try_merge(right).unwrap();
        assert_eq!(merged.selected.len(), 2);
        assert_eq!(merged.edges.len(), 3);
    }

    #[test]
    fn duplicate_way_in_one_chunk_is_an_error() {
        let result = discover_chunk(
            chunk(vec![way(5, &[1, 2], &[]), way(5, &[3, 4], &[])]),
            &KeepTagged,
        );

        assert!(matches!(
            result,
            Err(SkywayError::UnsupportedHistoryInput(_))
        ));
    }

    #[test]
    fn duplicate_identity_across_summaries_is_an_error_in_either_order() {
        // Two versions of the same way, discovered in different chunks. Merging
        // them in either direction has to fail; picking a winner would make the
        // keep set depend on Rayon's reduction order.
        let summary = |nodes: &[i64]| {
            let mut d = Discovery::new();
            d.edges.insert(
                ElementKey::Way(5),
                nodes.iter().map(|id| ElementKey::Node(*id)).collect(),
            );
            d
        };

        assert!(matches!(
            summary(&[1, 2]).try_merge(summary(&[3, 4])),
            Err(SkywayError::UnsupportedHistoryInput(_))
        ));
        assert!(matches!(
            summary(&[3, 4]).try_merge(summary(&[1, 2])),
            Err(SkywayError::UnsupportedHistoryInput(_))
        ));
    }

    #[test]
    fn repeated_selection_is_not_a_duplicate_identity() {
        // Selection is a set union, so the same key on both sides is fine.
        let summary = || {
            let mut d = Discovery::new();
            d.selected.insert(ElementKey::Node(1));
            d
        };

        let merged = summary().try_merge(summary()).unwrap();
        assert_eq!(merged.selected, HashSet::from([ElementKey::Node(1)]));
    }

    #[test]
    fn duplicate_elements_without_references_are_not_detected() {
        // Nodes and deleted ways carry no references, so they never reach the
        // edge map and cannot make the merge nondeterministic. Documenting the
        // limit here so a future change to `reference_keys` shows up as a
        // failure rather than a silent widening.
        let discovery = discover_chunk(
            chunk(vec![node(1, &[]), node(1, &[]), way(5, &[], &[]), way(5, &[], &[])]),
            &KeepTagged,
        )
        .unwrap();

        assert!(discovery.is_empty());
    }

    #[test]
    fn emission_uses_keep_set_and_reproduces_transformation() {
        let keep = HashSet::from([ElementKey::Node(1), ElementKey::Way(5)]);

        let emitted = emit_chunk(
            chunk(vec![
                node(1, &[("drop-me", "x"), ("name", "kept as dependency")]),
                node(2, &[("keep", "yes")]),
                way(5, &[1], &[("keep", "yes"), ("drop-me", "x")]),
            ]),
            &keep,
            &KeepTagged,
        );

        let ids: Vec<ElementKey> = emitted.content.iter().map(|e| e.key()).collect();
        assert_eq!(ids, vec![ElementKey::Node(1), ElementKey::Way(5)]);
        assert!(
            emitted
                .content
                .iter()
                .all(|e| !e.tags.contains_key("drop-me")),
            "the transformation is applied to dependencies too"
        );
    }

    #[test]
    fn one_pass_filtering_keeps_only_selected_elements() {
        let emitted = filter_chunk(
            chunk(vec![
                node(1, &[]),
                node(2, &[("keep", "yes"), ("drop-me", "x")]),
            ]),
            &KeepTagged,
        );

        assert_eq!(emitted.content.len(), 1);
        assert_eq!(emitted.content[0].id, 2);
        assert!(!emitted.content[0].tags.contains_key("drop-me"));
    }
}
