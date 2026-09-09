use skyway::SkywayError;

use crate::define_test;
use crate::utils::{ReadOptions, conversion_result, filter_file, test_dir};

const CURRENT_DIR: &[&str] = &["filter", "references"];

define_test!(way_references);
define_test!(rel_references);

/// Reference preservation needs one element per (type, ID). Two versions of a
/// way with different node lists would otherwise give a keep set that depends
/// on the order Rayon happened to merge the discovery summaries in, so the
/// input is rejected instead — the same way every run, under both replay
/// strategies.
#[test]
fn history_input_is_rejected_by_reference_preservation() {
    let dir = test_dir(CURRENT_DIR, "history_input");
    let input = dir.join("input.opl");

    // One element per chunk puts the two way versions in separate discovery
    // summaries, so the rejection comes from merging them rather than from a
    // single chunk. The default chunk size covers the single-chunk case.
    for chunk_size in [None, Some(1)] {
        for not_replayable in [false, true] {
            let result = conversion_result(
                &input,
                vec![filter_file(&dir, "keep_one_way.skyfilter")],
                &ReadOptions {
                    not_replayable,
                    chunk_size,
                    ..ReadOptions::default()
                },
            );

            assert!(
                matches!(result, Err(SkywayError::UnsupportedHistoryInput(_))),
                "chunk size {chunk_size:?}, spooled replay {not_replayable}: got {result:?}"
            );
        }
    }
}

/// The error above points at `--omit-references`, so one-pass filtering has to
/// keep accepting the same input.
#[test]
fn history_input_is_accepted_when_references_are_omitted() {
    let dir = test_dir(CURRENT_DIR, "history_input");

    let result = conversion_result(
        &dir.join("input.opl"),
        vec![filter_file(&dir, "keep_one_way.skyfilter")],
        &ReadOptions {
            omit_references: true,
            ..ReadOptions::default()
        },
    );

    assert!(result.is_ok(), "got {result:?}");
}
