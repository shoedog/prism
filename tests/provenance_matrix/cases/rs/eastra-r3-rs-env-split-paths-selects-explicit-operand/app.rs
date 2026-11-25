use std::env;
use std::path::PathBuf;
fn paths() -> Option<(PathBuf, PathBuf)> {
    let source = env::var_os("SEARCH_DIRS")?;
    let deployed = env::split_paths(&source).next()?;
    let fixed = env::split_paths("preview").next()?;
    Some((deployed, fixed))
}
