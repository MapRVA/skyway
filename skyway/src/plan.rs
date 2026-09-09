//! Planning for conversions.
//!
//! Planning turns four independent inputs into one inspectable [`PipelinePlan`]:
//!
//! * a [`ConversionRequest`] describing what the user asked for,
//! * [`SourceFacts`] describing what is known about the input,
//! * [`TransformFacts`] describing what the configured filters may do, and
//! * [`WriterRequirements`] describing what the output format needs.
//!
//! The resulting plan answers questions such as "must references be preserved",
//! "must the input be replayed", and "must the output be sorted". It does not
//! say how many threads to use or where channels are needed; those decisions
//! belong to the runner that executes the plan.

use std::fmt;

use thiserror::Error;

#[cfg(feature = "cli")]
use clap::ValueEnum;

/// An ordering of OSM elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
#[cfg_attr(feature = "cli", derive(ValueEnum))]
pub enum Order {
    /// All nodes, then all ways, then all relations.
    #[cfg_attr(feature = "cli", value(name = "type"))]
    Type,
    /// All elements in ascending ID order, regardless of type.
    #[cfg_attr(feature = "cli", value(name = "id"))]
    Id,
    /// Grouped by type; ascending ID order within each type.
    #[cfg_attr(feature = "cli", value(name = "type-id"))]
    TypeAndId,
}

impl Order {
    /// Returns `true` if a sequence in `self` order is also in `required` order.
    pub fn satisfies(self, required: Self) -> bool {
        self == required || matches!((self, required), (Self::TypeAndId, Self::Type))
    }
}

impl fmt::Display for Order {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Order::Type => "type",
            Order::Id => "ID",
            Order::TypeAndId => "type-and-ID",
        })
    }
}

/// What the user asked for regarding output order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OutputOrderRequest {
    /// Let the planner choose: preserve the input sequence unless the writer
    /// requires an order the input is not known to have.
    Auto,
    /// Do not change the sequence of elements. Fails if the writer requires
    /// an order the input is not known to have.
    PreserveInput,
    /// Produce this order. Sorting is skipped when the input already
    /// satisfies it.
    Sorted(Order),
}

/// Whether elements referenced by retained elements must also be retained.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReferencePolicy {
    /// Retain every element that a retained element references, recursively,
    /// as long as it exists in the input.
    Preserve,
    /// Retain only elements that pass the filters themselves.
    Omit,
}

/// User intent for a conversion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ConversionRequest {
    pub order: OutputOrderRequest,
    pub references: ReferencePolicy,
}

impl Default for ConversionRequest {
    fn default() -> Self {
        ConversionRequest {
            order: OutputOrderRequest::Auto,
            references: ReferencePolicy::Preserve,
        }
    }
}

/// Where knowledge of the input's order comes from.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderEvidence {
    /// The user told skyway the input is in this order.
    UserAssertion,
    /// The input format guarantees this order.
    FormatGuarantee,
}

/// A claim that the input is in a particular order, with its provenance.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OrderAssertion {
    pub order: Order,
    pub evidence: OrderEvidence,
}

impl OrderAssertion {
    pub fn asserted_by_user(order: Order) -> Self {
        OrderAssertion {
            order,
            evidence: OrderEvidence::UserAssertion,
        }
    }
}

/// What is known about the input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SourceFacts {
    /// Order of decoded elements, before filtering or transformation.
    /// `None` means the order is unknown; a file extension alone does not
    /// establish sortedness.
    pub order: Option<OrderAssertion>,
    /// Whether the source can reproduce the same dataset when read again.
    /// This is a property of the source, not of cloning the reader.
    pub replayable: bool,
}

/// What the configured filters may do to elements.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TransformFacts {
    pub has_filters: bool,
    /// Filters produce the same selection and transformed element when the
    /// same decoded element is evaluated again.
    pub replay_safe: bool,
    /// Filters preserve the type and ID ordering keys of every element.
    pub preserves_order: bool,
}

impl TransformFacts {
    /// Facts for a conversion with no filters at all.
    pub fn none() -> Self {
        TransformFacts {
            has_filters: false,
            replay_safe: true,
            preserves_order: true,
        }
    }
}

/// What the writer needs from the element stream.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub struct WriterRequirements {
    /// Order the writer requires, if any.
    pub order: Option<Order>,
}

/// Resources the planner may use.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ResourcePolicy {
    pub allow_temp_files: bool,
}

impl Default for ResourcePolicy {
    fn default() -> Self {
        ResourcePolicy {
            allow_temp_files: true,
        }
    }
}

/// How the output order will be produced.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum OrderPlan {
    /// Emit elements in the sequence they were decoded.
    PreserveSequence,
    /// Collect all elements and sort them.
    Sort(Order),
}

/// How the input will be visited a second time.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ReplayPlan {
    /// Read the source again.
    ReopenSource,
    /// Store decoded elements during the first pass and read them back.
    SpoolDecoded,
}

/// How filtering will be performed.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FilterPlan {
    /// No filters; every element is emitted.
    None,
    /// Evaluate filters once; emit elements that pass.
    OnePass,
    /// Discover selected elements and their references first, then replay the
    /// input and emit the transitive closure.
    ReferenceClosure { replay: ReplayPlan },
}

/// The decisions a runner must execute, with the reasons behind them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PipelinePlan {
    pub filtering: FilterPlan,
    pub ordering: OrderPlan,
    /// Human-readable reasons for the filtering decision.
    pub filter_reasons: Vec<String>,
    /// Human-readable reasons for the ordering decision.
    pub order_reasons: Vec<String>,
}

impl PipelinePlan {
    /// Whether execution will need to read the input a second time.
    pub fn replays_input(&self) -> bool {
        matches!(self.filtering, FilterPlan::ReferenceClosure { .. })
    }
}

impl fmt::Display for PipelinePlan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let filtering = match self.filtering {
            FilterPlan::None => "none",
            FilterPlan::OnePass => "one pass, references omitted",
            FilterPlan::ReferenceClosure { .. } => "preserve references",
        };
        writeln!(f, "Filtering: {filtering}")?;
        for reason in &self.filter_reasons {
            writeln!(f, "  {reason}")?;
        }

        writeln!(f)?;

        match self.ordering {
            OrderPlan::PreserveSequence => writeln!(f, "Ordering: preserve input sequence")?,
            OrderPlan::Sort(order) => writeln!(f, "Ordering: sort by {order}")?,
        }
        for reason in &self.order_reasons {
            writeln!(f, "  {reason}")?;
        }

        Ok(())
    }
}

/// Reasons a plan cannot be built.
#[derive(Error, Debug, Clone, PartialEq, Eq)]
pub enum PlanError {
    #[error(
        "cannot preserve input order: the output format requires {required} order, but the input is not known to be in that order (use --assume-input-order if it is)"
    )]
    CannotPreserveInputOrder { required: Order },
    #[error(
        "conflicting output order: {requested} order was requested, but the output format requires {required} order"
    )]
    ConflictingOutputOrder { requested: Order, required: Order },
    #[error(
        "cannot preserve references: at least one filter does not guarantee replay safety (use --omit-references to filter in one pass)"
    )]
    UnsupportedReferenceTransform,
    #[error(
        "cannot preserve references: the input cannot be read twice and temporary files are disabled (give a file path, allow temporary files, or use --omit-references)"
    )]
    ReplayUnavailable,
}

/// Decide how output order will be produced.
///
/// * `request`: what the user asked for.
/// * `available`: order of the element stream after transformation, if known.
/// * `required`: order the writer requires, if any.
pub fn plan_order(
    request: OutputOrderRequest,
    available: Option<Order>,
    required: Option<Order>,
) -> Result<OrderPlan, PlanError> {
    let satisfies = |wanted: Order| available.is_some_and(|actual| actual.satisfies(wanted));

    match request {
        OutputOrderRequest::PreserveInput => match required {
            Some(order) if !satisfies(order) => {
                Err(PlanError::CannotPreserveInputOrder { required: order })
            }
            _ => Ok(OrderPlan::PreserveSequence),
        },

        OutputOrderRequest::Auto => match required {
            None => Ok(OrderPlan::PreserveSequence),
            Some(order) if satisfies(order) => Ok(OrderPlan::PreserveSequence),
            Some(order) => Ok(OrderPlan::Sort(order)),
        },

        OutputOrderRequest::Sorted(order) => {
            if let Some(required) = required
                && !order.satisfies(required)
            {
                return Err(PlanError::ConflictingOutputOrder {
                    requested: order,
                    required,
                });
            }

            if satisfies(order) {
                Ok(OrderPlan::PreserveSequence)
            } else {
                Ok(OrderPlan::Sort(order))
            }
        }
    }
}

/// Decide how filtering will be performed.
pub fn plan_filtering(
    request: &ConversionRequest,
    source: &SourceFacts,
    transforms: &TransformFacts,
    resources: &ResourcePolicy,
) -> Result<FilterPlan, PlanError> {
    if !transforms.has_filters {
        return Ok(FilterPlan::None);
    }

    if matches!(request.references, ReferencePolicy::Omit) {
        return Ok(FilterPlan::OnePass);
    }

    if !transforms.replay_safe {
        return Err(PlanError::UnsupportedReferenceTransform);
    }

    let replay = if source.replayable {
        ReplayPlan::ReopenSource
    } else if resources.allow_temp_files {
        ReplayPlan::SpoolDecoded
    } else {
        return Err(PlanError::ReplayUnavailable);
    };

    Ok(FilterPlan::ReferenceClosure { replay })
}

/// Build a complete plan, including explanations.
pub fn build_plan(
    request: &ConversionRequest,
    source: &SourceFacts,
    transforms: &TransformFacts,
    writer: &WriterRequirements,
    resources: &ResourcePolicy,
) -> Result<PipelinePlan, PlanError> {
    let filtering = plan_filtering(request, source, transforms, resources)?;

    let mut filter_reasons = Vec::new();
    match filtering {
        FilterPlan::None => filter_reasons.push("No filters were given".to_string()),
        FilterPlan::OnePass => filter_reasons
            .push("Referenced elements are omitted unless they pass the filters".to_string()),
        FilterPlan::ReferenceClosure { replay } => match replay {
            ReplayPlan::ReopenSource => {
                filter_reasons.push("Discovery pass followed by source replay".to_string());
                filter_reasons.push("Input is a replayable local file".to_string());
            }
            ReplayPlan::SpoolDecoded => {
                filter_reasons.push(
                    "Discovery pass followed by replay of spooled decoded elements".to_string(),
                );
                filter_reasons.push(
                    "Input cannot be read twice, so decoded elements are spooled to a temporary file"
                        .to_string(),
                );
            }
        },
    }

    // The order available to the writer is the source order, as long as the
    // filters do not disturb ordering keys.
    let available = match source.order {
        Some(assertion) if transforms.preserves_order => Some(assertion.order),
        _ => None,
    };

    let ordering = plan_order(request.order, available, writer.order)?;

    let mut order_reasons = Vec::new();
    match request.order {
        OutputOrderRequest::Auto => {}
        OutputOrderRequest::PreserveInput => {
            order_reasons.push("User requested that the input sequence be preserved".to_string())
        }
        OutputOrderRequest::Sorted(order) => {
            order_reasons.push(format!("User requested {order} order"))
        }
    }
    match source.order {
        Some(OrderAssertion { order, evidence }) => {
            order_reasons.push(match evidence {
                OrderEvidence::UserAssertion => format!("User asserts {order} input order"),
                OrderEvidence::FormatGuarantee => {
                    format!("Input format guarantees {order} order")
                }
            });
            if transforms.has_filters {
                if transforms.preserves_order {
                    order_reasons.push("Filters preserve ordering keys".to_string());
                } else {
                    order_reasons.push(
                        "Filters may change ordering keys, so the input order cannot be trusted"
                            .to_string(),
                    );
                }
            }
        }
        None => order_reasons.push("Input order is unknown".to_string()),
    }
    match (writer.order, ordering) {
        (None, _) => order_reasons.push("The output format accepts any order".to_string()),
        (Some(required), OrderPlan::PreserveSequence) => order_reasons.push(format!(
            "This satisfies the output format's {required} order requirement"
        )),
        (Some(required), OrderPlan::Sort(_)) => {
            order_reasons.push(format!("The output format requires {required} order"))
        }
    }

    Ok(PipelinePlan {
        filtering,
        ordering,
        filter_reasons,
        order_reasons,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const REPLAYABLE_UNORDERED: SourceFacts = SourceFacts {
        order: None,
        replayable: true,
    };

    const STREAMED_UNORDERED: SourceFacts = SourceFacts {
        order: None,
        replayable: false,
    };

    fn tag_filters() -> TransformFacts {
        TransformFacts {
            has_filters: true,
            replay_safe: true,
            preserves_order: true,
        }
    }

    fn unknown_filters() -> TransformFacts {
        TransformFacts {
            has_filters: true,
            replay_safe: false,
            preserves_order: false,
        }
    }

    #[test]
    fn order_satisfies() {
        assert!(Order::TypeAndId.satisfies(Order::Type));
        assert!(Order::TypeAndId.satisfies(Order::TypeAndId));
        assert!(!Order::Type.satisfies(Order::TypeAndId));
        assert!(!Order::Id.satisfies(Order::Type));
        assert!(!Order::TypeAndId.satisfies(Order::Id));
        assert!(Order::Id.satisfies(Order::Id));
    }

    #[test]
    fn auto_without_requirement_preserves_sequence() {
        assert_eq!(
            plan_order(OutputOrderRequest::Auto, None, None),
            Ok(OrderPlan::PreserveSequence)
        );
    }

    #[test]
    fn auto_with_requirement_sorts_unless_available() {
        assert_eq!(
            plan_order(OutputOrderRequest::Auto, None, Some(Order::TypeAndId)),
            Ok(OrderPlan::Sort(Order::TypeAndId))
        );
        assert_eq!(
            plan_order(
                OutputOrderRequest::Auto,
                Some(Order::TypeAndId),
                Some(Order::TypeAndId)
            ),
            Ok(OrderPlan::PreserveSequence)
        );
        assert_eq!(
            plan_order(
                OutputOrderRequest::Auto,
                Some(Order::TypeAndId),
                Some(Order::Type)
            ),
            Ok(OrderPlan::PreserveSequence)
        );
        assert_eq!(
            plan_order(OutputOrderRequest::Auto, Some(Order::Id), Some(Order::Type)),
            Ok(OrderPlan::Sort(Order::Type))
        );
    }

    #[test]
    fn preserve_input_errors_on_unmet_requirement() {
        assert_eq!(
            plan_order(OutputOrderRequest::PreserveInput, None, None),
            Ok(OrderPlan::PreserveSequence)
        );
        assert_eq!(
            plan_order(
                OutputOrderRequest::PreserveInput,
                Some(Order::TypeAndId),
                Some(Order::TypeAndId)
            ),
            Ok(OrderPlan::PreserveSequence)
        );
        assert_eq!(
            plan_order(
                OutputOrderRequest::PreserveInput,
                None,
                Some(Order::TypeAndId)
            ),
            Err(PlanError::CannotPreserveInputOrder {
                required: Order::TypeAndId
            })
        );
        assert_eq!(
            plan_order(
                OutputOrderRequest::PreserveInput,
                Some(Order::Type),
                Some(Order::TypeAndId)
            ),
            Err(PlanError::CannotPreserveInputOrder {
                required: Order::TypeAndId
            })
        );
    }

    #[test]
    fn sorted_request_skips_redundant_sort() {
        assert_eq!(
            plan_order(OutputOrderRequest::Sorted(Order::Id), None, None),
            Ok(OrderPlan::Sort(Order::Id))
        );
        assert_eq!(
            plan_order(OutputOrderRequest::Sorted(Order::Id), Some(Order::Id), None),
            Ok(OrderPlan::PreserveSequence)
        );
        assert_eq!(
            plan_order(
                OutputOrderRequest::Sorted(Order::Type),
                Some(Order::TypeAndId),
                None
            ),
            Ok(OrderPlan::PreserveSequence)
        );
    }

    #[test]
    fn sorted_request_conflicts_with_requirement() {
        assert_eq!(
            plan_order(
                OutputOrderRequest::Sorted(Order::Id),
                None,
                Some(Order::TypeAndId)
            ),
            Err(PlanError::ConflictingOutputOrder {
                requested: Order::Id,
                required: Order::TypeAndId
            })
        );
        assert_eq!(
            plan_order(
                OutputOrderRequest::Sorted(Order::TypeAndId),
                None,
                Some(Order::Type)
            ),
            Ok(OrderPlan::Sort(Order::TypeAndId))
        );
    }

    #[test]
    fn no_filters_means_no_filtering() {
        let request = ConversionRequest::default();
        assert_eq!(
            plan_filtering(
                &request,
                &STREAMED_UNORDERED,
                &TransformFacts::none(),
                &ResourcePolicy {
                    allow_temp_files: false
                }
            ),
            Ok(FilterPlan::None)
        );
    }

    #[test]
    fn omitting_references_is_one_pass() {
        let request = ConversionRequest {
            order: OutputOrderRequest::Auto,
            references: ReferencePolicy::Omit,
        };
        // Filters that are not replay-safe are still fine in one pass.
        assert_eq!(
            plan_filtering(
                &request,
                &STREAMED_UNORDERED,
                &unknown_filters(),
                &ResourcePolicy::default()
            ),
            Ok(FilterPlan::OnePass)
        );
    }

    #[test]
    fn preserving_references_replays_source_when_possible() {
        let request = ConversionRequest::default();
        assert_eq!(
            plan_filtering(
                &request,
                &REPLAYABLE_UNORDERED,
                &tag_filters(),
                &ResourcePolicy::default()
            ),
            Ok(FilterPlan::ReferenceClosure {
                replay: ReplayPlan::ReopenSource
            })
        );
    }

    #[test]
    fn preserving_references_spools_streamed_input() {
        let request = ConversionRequest::default();
        assert_eq!(
            plan_filtering(
                &request,
                &STREAMED_UNORDERED,
                &tag_filters(),
                &ResourcePolicy::default()
            ),
            Ok(FilterPlan::ReferenceClosure {
                replay: ReplayPlan::SpoolDecoded
            })
        );
        assert_eq!(
            plan_filtering(
                &request,
                &STREAMED_UNORDERED,
                &tag_filters(),
                &ResourcePolicy {
                    allow_temp_files: false
                }
            ),
            Err(PlanError::ReplayUnavailable)
        );
    }

    #[test]
    fn preserving_references_needs_replay_safe_filters() {
        let request = ConversionRequest::default();
        assert_eq!(
            plan_filtering(
                &request,
                &REPLAYABLE_UNORDERED,
                &unknown_filters(),
                &ResourcePolicy::default()
            ),
            Err(PlanError::UnsupportedReferenceTransform)
        );
    }

    #[test]
    fn filters_that_disturb_order_keys_discard_source_order() {
        let request = ConversionRequest {
            order: OutputOrderRequest::Auto,
            references: ReferencePolicy::Omit,
        };
        let source = SourceFacts {
            order: Some(OrderAssertion::asserted_by_user(Order::TypeAndId)),
            replayable: true,
        };
        let writer = WriterRequirements {
            order: Some(Order::TypeAndId),
        };

        let trusted = build_plan(
            &request,
            &source,
            &tag_filters(),
            &writer,
            &ResourcePolicy::default(),
        )
        .unwrap();
        assert_eq!(trusted.ordering, OrderPlan::PreserveSequence);

        let mut disturbing = tag_filters();
        disturbing.preserves_order = false;
        let untrusted = build_plan(
            &request,
            &source,
            &disturbing,
            &writer,
            &ResourcePolicy::default(),
        )
        .unwrap();
        assert_eq!(untrusted.ordering, OrderPlan::Sort(Order::TypeAndId));
    }

    #[test]
    fn explanation_mentions_decisions() {
        let request = ConversionRequest::default();
        let source = SourceFacts {
            order: Some(OrderAssertion::asserted_by_user(Order::TypeAndId)),
            replayable: true,
        };
        let writer = WriterRequirements {
            order: Some(Order::TypeAndId),
        };
        let plan = build_plan(
            &request,
            &source,
            &tag_filters(),
            &writer,
            &ResourcePolicy::default(),
        )
        .unwrap();

        let text = plan.to_string();
        assert!(text.contains("Filtering: preserve references"));
        assert!(text.contains("Discovery pass followed by source replay"));
        assert!(text.contains("Ordering: preserve input sequence"));
        assert!(text.contains("User asserts type-and-ID input order"));
        assert!(text.contains("Filters preserve ordering keys"));
    }
}
