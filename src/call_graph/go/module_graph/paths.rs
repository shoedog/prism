use std::path::{Component, Path, PathBuf};

pub(super) fn normalize_repo_dir(repo_root: &Path, base_dir: &str, raw: &str) -> Option<String> {
    if Path::new(raw).is_absolute() {
        let root = lexical_absolute(repo_root)?;
        let target = lexical_absolute(Path::new(raw))?;
        return path_to_repo_string(target.strip_prefix(root).ok()?);
    }

    let mut parts = if base_dir.is_empty() {
        Vec::new()
    } else {
        base_dir.split('/').map(str::to_string).collect()
    };
    for component in Path::new(raw).components() {
        match component {
            Component::CurDir => {}
            Component::Normal(part) => parts.push(part.to_str()?.to_string()),
            Component::ParentDir => {
                parts.pop()?;
            }
            Component::RootDir | Component::Prefix(_) => return None,
        }
    }
    Some(parts.join("/"))
}

fn lexical_absolute(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let mut out = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => out.push(prefix.as_os_str()),
            Component::RootDir => out.push(Path::new("/")),
            Component::CurDir => {}
            Component::ParentDir => {
                if !out.pop() {
                    return None;
                }
            }
            Component::Normal(part) => out.push(part),
        }
    }
    Some(out)
}

pub(super) fn path_to_repo_string(path: &Path) -> Option<String> {
    Some(
        path.components()
            .map(|component| match component {
                Component::Normal(part) => part.to_str(),
                _ => None,
            })
            .collect::<Option<Vec<_>>>()?
            .join("/"),
    )
}
