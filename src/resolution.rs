//! S3 call-resolution: confidence types, owner-key normalization, and the
//! R1-R7 resolution ladder (impl on CallGraph lives here to keep
//! call_graph.rs under the size cap).

/// Normalize an owner type's source text to its bare index key:
/// strip refs/pointers, smart-pointer wrappers are NOT peeled here (that is
/// receiver peeling, `peel_type`), strip generic args, strip `dyn `/`impl `.
pub fn owner_key(text: &str) -> String {
    let t = text.trim();
    let t = t.trim_start_matches("&mut ").trim_start_matches('&');
    let t = t.trim_start_matches('*');
    let t = t.trim_start_matches("dyn ").trim_start_matches("impl ");
    let t = t.split('<').next().unwrap_or(t);
    // C++ out-of-line `ns::Foo` declarator prefix -> last segment.
    let t = t.rsplit("::").next().unwrap_or(t);
    t.trim().to_string()
}
