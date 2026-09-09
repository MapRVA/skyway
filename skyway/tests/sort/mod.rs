use crate::define_test;

const CURRENT_DIR: &[&str] = &["sort"];

define_test!(id);
define_test!(by_type, "type");
define_test!(type_id, "type-id");
define_test!(assumed_order, "assumed-order");
define_test!(none);
