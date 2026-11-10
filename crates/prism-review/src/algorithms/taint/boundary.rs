//! Review-owned matching policy; not wired into the slicing/output path.
#[allow(dead_code)]
pub(super) mod collect;
#[allow(dead_code)]
pub(super) mod display;
#[allow(dead_code)] // S1b.3 will consume this independently tested primitive.
pub(super) mod occurrences;
#[allow(dead_code)]
mod records;
#[allow(dead_code)]
pub(super) mod recovery;
