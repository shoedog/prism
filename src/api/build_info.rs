/// Build identity for the Prism library binary that implements this facade.
#[non_exhaustive]
pub struct BuildInfo {
    pub package_version: &'static str,
    pub git_sha: &'static str,
    pub build_identity: &'static str,
    pub binary_input_dirty: bool,
    pub grammar_fingerprint: &'static str,
}

pub fn build_info() -> BuildInfo {
    BuildInfo {
        package_version: env!("CARGO_PKG_VERSION"),
        git_sha: env!("GIT_SHA"),
        build_identity: crate::cpg_cache::current_cache_build_identity(),
        binary_input_dirty: crate::cpg_cache::binary_input_dirty(),
        grammar_fingerprint: env!("GRAMMAR_FINGERPRINT"),
    }
}
