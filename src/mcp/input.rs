use super::error::ToolError;
use serde::Deserialize;
use serde_json::{Map, Value};

pub const DEPTH_DEFAULT: usize = 1;
pub const DEPTH_MAX: usize = 5;
pub const HOPS_DEFAULT: usize = 1;
pub const HOPS_MAX: usize = 5;
pub const MAX_RESULTS_DEFAULT: usize = 50;
pub const MAX_RESULTS_CAP: usize = 1000;

const DEFAULT_EDGES: [&str; 4] = ["Call", "Return", "DataFlow", "Contains"];
const VALID_EDGES: [&str; 6] = [
    "Call",
    "Return",
    "DataFlow",
    "Contains",
    "ControlFlow",
    "FieldOf",
];

#[derive(Debug, Clone, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SeedInput {
    Symbol { name: String, file: Option<String> },
    Loc { file: String, line: usize },
}

impl SeedInput {
    pub fn to_triple(&self) -> (Option<&str>, Option<&str>, Option<String>) {
        match self {
            SeedInput::Symbol { name, file } => (Some(name.as_str()), file.as_deref(), None),
            SeedInput::Loc { file, line } => (None, None, Some(format!("{file}:{line}"))),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NormalizedPath {
    RepoRelative(String),
    EscapesRoot,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Concise,
    Detailed,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodesAtInput {
    pub file: String,
    pub line: usize,
    pub verbosity: Verbosity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallersInput {
    pub seed: SeedInput,
    pub depth: usize,
    pub max_results: usize,
    pub verbosity: Verbosity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalleesInput {
    pub seed: SeedInput,
    pub depth: usize,
    pub max_results: usize,
    pub verbosity: Verbosity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EgoInput {
    pub seed: SeedInput,
    pub hops: usize,
    pub edges: Vec<String>,
    pub max_results: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDepsInput {
    pub file: String,
    pub max_results: usize,
    pub verbosity: Verbosity,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoMapInput {
    pub max_results: usize,
}

pub fn normalize_path(path: &str) -> NormalizedPath {
    if path.starts_with('/') {
        return NormalizedPath::EscapesRoot;
    }

    let mut parts = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                if parts.pop().is_none() {
                    return NormalizedPath::EscapesRoot;
                }
            }
            segment => parts.push(segment),
        }
    }

    NormalizedPath::RepoRelative(parts.join("/"))
}

pub fn parse_nodes_at(args: &Value) -> Result<NodesAtInput, ToolError> {
    let obj = object(args, "nav_nodes_at", &["file", "line", "verbosity"])?;
    let file = normalize_file_arg(required_string(obj, "file")?);
    let line = required_usize(obj, "line", 1, usize::MAX)?;
    let verbosity = parse_verbosity(obj.get("verbosity"))?;
    Ok(NodesAtInput {
        file,
        line,
        verbosity,
    })
}

pub fn parse_callers(args: &Value) -> Result<CallersInput, ToolError> {
    let obj = object(
        args,
        "nav_callers",
        &["seed", "depth", "max_results", "verbosity"],
    )?;
    Ok(CallersInput {
        seed: parse_seed(required_value(obj, "seed")?)?,
        depth: optional_usize(obj, "depth", DEPTH_DEFAULT, 0, DEPTH_MAX)?,
        max_results: optional_usize(obj, "max_results", MAX_RESULTS_DEFAULT, 1, MAX_RESULTS_CAP)?,
        verbosity: parse_verbosity(obj.get("verbosity"))?,
    })
}

pub fn parse_callees(args: &Value) -> Result<CalleesInput, ToolError> {
    let obj = object(
        args,
        "nav_callees",
        &["seed", "depth", "max_results", "verbosity"],
    )?;
    Ok(CalleesInput {
        seed: parse_seed(required_value(obj, "seed")?)?,
        depth: optional_usize(obj, "depth", DEPTH_DEFAULT, 0, DEPTH_MAX)?,
        max_results: optional_usize(obj, "max_results", MAX_RESULTS_DEFAULT, 1, MAX_RESULTS_CAP)?,
        verbosity: parse_verbosity(obj.get("verbosity"))?,
    })
}

pub fn parse_ego(args: &Value) -> Result<EgoInput, ToolError> {
    let obj = object(
        args,
        "nav_ego_graph",
        &["seed", "hops", "edges", "max_results"],
    )?;
    Ok(EgoInput {
        seed: parse_seed(required_value(obj, "seed")?)?,
        hops: optional_usize(obj, "hops", HOPS_DEFAULT, 0, HOPS_MAX)?,
        edges: parse_edges(obj.get("edges"))?,
        max_results: optional_usize(obj, "max_results", MAX_RESULTS_DEFAULT, 1, MAX_RESULTS_CAP)?,
    })
}

pub fn parse_module_deps(args: &Value) -> Result<ModuleDepsInput, ToolError> {
    let obj = object(
        args,
        "nav_module_deps",
        &["file", "max_results", "verbosity"],
    )?;
    Ok(ModuleDepsInput {
        file: normalize_file_arg(required_string(obj, "file")?),
        max_results: optional_usize(obj, "max_results", MAX_RESULTS_DEFAULT, 1, MAX_RESULTS_CAP)?,
        verbosity: parse_verbosity(obj.get("verbosity"))?,
    })
}

pub fn parse_repo_map(args: &Value) -> Result<RepoMapInput, ToolError> {
    let obj = object(args, "nav_repo_map", &["max_results"])?;
    Ok(RepoMapInput {
        max_results: optional_usize(obj, "max_results", MAX_RESULTS_DEFAULT, 1, MAX_RESULTS_CAP)?,
    })
}

fn object<'a>(
    value: &'a Value,
    tool: &str,
    allowed: &[&str],
) -> Result<&'a Map<String, Value>, ToolError> {
    let Some(obj) = value.as_object() else {
        return bad_args(format!("{tool} arguments must be a JSON object"));
    };
    for key in obj.keys() {
        if !allowed.contains(&key.as_str()) {
            return bad_args(format!("unknown argument `{key}` for {tool}"));
        }
    }
    Ok(obj)
}

fn required_value<'a>(obj: &'a Map<String, Value>, key: &str) -> Result<&'a Value, ToolError> {
    obj.get(key)
        .ok_or_else(|| ToolError::BadArguments(format!("missing required argument `{key}`")))
}

fn required_string(obj: &Map<String, Value>, key: &str) -> Result<String, ToolError> {
    required_value(obj, key)?
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| ToolError::BadArguments(format!("argument `{key}` must be a string")))
}

fn required_usize(
    obj: &Map<String, Value>,
    key: &str,
    min: usize,
    max: usize,
) -> Result<usize, ToolError> {
    let value = required_value(obj, key)?;
    parse_usize_value(value, key, min, max)
}

fn optional_usize(
    obj: &Map<String, Value>,
    key: &str,
    default: usize,
    min: usize,
    max: usize,
) -> Result<usize, ToolError> {
    match obj.get(key) {
        Some(value) => parse_usize_value(value, key, min, max),
        None => Ok(default),
    }
}

fn parse_usize_value(value: &Value, key: &str, min: usize, max: usize) -> Result<usize, ToolError> {
    let Some(n) = value.as_u64() else {
        return bad_args(format!("argument `{key}` must be an integer"));
    };
    let n = usize::try_from(n)
        .map_err(|_| ToolError::BadArguments(format!("argument `{key}` is out of range")))?;
    if n < min || n > max {
        return bad_args(format!("argument `{key}` must be between {min} and {max}"));
    }
    Ok(n)
}

fn parse_seed(value: &Value) -> Result<SeedInput, ToolError> {
    let mut seed: SeedInput = serde_json::from_value(value.clone())
        .map_err(|err| ToolError::BadArguments(format!("invalid seed: {err}")))?;
    match &mut seed {
        SeedInput::Symbol {
            file: Some(file), ..
        } => {
            *file = normalize_file_arg(file.clone());
        }
        SeedInput::Symbol { file: None, .. } => {}
        SeedInput::Loc { file, line } => {
            if *line < 1 {
                return bad_args("seed loc line must be at least 1");
            }
            *file = normalize_file_arg(file.clone());
        }
    }
    Ok(seed)
}

fn parse_verbosity(value: Option<&Value>) -> Result<Verbosity, ToolError> {
    match value {
        None => Ok(Verbosity::Concise),
        Some(Value::String(s)) if s == "concise" => Ok(Verbosity::Concise),
        Some(Value::String(s)) if s == "detailed" => Ok(Verbosity::Detailed),
        Some(_) => bad_args("argument `verbosity` must be `concise` or `detailed`"),
    }
}

fn parse_edges(value: Option<&Value>) -> Result<Vec<String>, ToolError> {
    let Some(value) = value else {
        return Ok(DEFAULT_EDGES
            .iter()
            .map(|edge| (*edge).to_owned())
            .collect());
    };
    let Some(edges) = value.as_array() else {
        return bad_args("argument `edges` must be an array");
    };
    let mut out = Vec::with_capacity(edges.len());
    for edge in edges {
        let Some(edge) = edge.as_str() else {
            return bad_args("argument `edges` must contain only strings");
        };
        if !VALID_EDGES.contains(&edge) {
            return bad_args(format!("unknown edge `{edge}`"));
        }
        out.push(edge.to_owned());
    }
    Ok(out)
}

fn normalize_file_arg(file: String) -> String {
    match normalize_path(&file) {
        NormalizedPath::RepoRelative(path) => path,
        NormalizedPath::EscapesRoot => file,
    }
}

fn bad_args<T>(message: impl Into<String>) -> Result<T, ToolError> {
    Err(ToolError::BadArguments(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn normalize_path_cases() {
        use NormalizedPath::*;
        assert!(matches!(normalize_path("src//lib.rs"), RepoRelative(p) if p=="src/lib.rs"));
        assert!(matches!(normalize_path("./src/lib.rs"), RepoRelative(p) if p=="src/lib.rs"));
        assert!(matches!(normalize_path("src/../src/x.rs"), RepoRelative(p) if p=="src/x.rs"));
        assert!(matches!(normalize_path("/abs/x"), EscapesRoot));
        assert!(matches!(normalize_path("../escape"), EscapesRoot));
    }

    #[test]
    fn bounds() {
        assert!(parse_callers(&json!({"seed":{"kind":"symbol","name":"f"},"depth":99})).is_err());
        assert!(parse_callers(&json!({"seed":{"kind":"symbol","name":"f"}})).is_ok());
        assert!(parse_nodes_at(&json!({"file":"a.rs","line":0})).is_err());
        assert!(parse_nodes_at(&json!({"file":"a.rs","line":1})).is_ok());
    }
}
