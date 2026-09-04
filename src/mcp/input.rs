use super::error::ToolError;
use crate::reasoning::seeds::{normalize_file_arg, normalize_loc_seed};
use serde::Deserialize;
use serde_json::{Map, Value};

pub const DEPTH_DEFAULT: usize = 1;
pub const DEPTH_MAX: usize = 5;
pub const HOPS_DEFAULT: usize = 1;
pub const HOPS_MAX: usize = 5;
pub const MAX_RESULTS_DEFAULT: usize = 50;
pub const MAX_RESULTS_CAP: usize = 1000;
pub const MAX_VIEW_BYTES_DEFAULT: usize = 12_000;
pub const MAX_VIEW_BYTES_CAP: usize = 80_000;

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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Verbosity {
    Concise,
    Detailed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViewFormat {
    CanonicalJson,
    AgentMarkdown,
    AgentJson,
}

impl ViewFormat {
    pub fn is_agent(self) -> bool {
        matches!(self, ViewFormat::AgentMarkdown | ViewFormat::AgentJson)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceProfile {
    Orientation,
    Seed,
    Impact,
    Dependencies,
    EditContext,
    Graph,
    Audit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SnippetPolicy {
    None,
    Line,
    SymbolHeader,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GroupPolicy {
    None,
    File,
    Symbol,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ViewOptions {
    pub format: ViewFormat,
    pub profile: EvidenceProfile,
    pub snippets: SnippetPolicy,
    pub group_by: GroupPolicy,
    pub group_by_explicit: bool,
    pub max_view_bytes: usize,
    pub requested: bool,
}

impl ViewOptions {
    pub fn canonical(default_profile: EvidenceProfile) -> Self {
        Self {
            format: ViewFormat::CanonicalJson,
            profile: default_profile,
            snippets: SnippetPolicy::None,
            group_by: GroupPolicy::None,
            group_by_explicit: false,
            max_view_bytes: MAX_VIEW_BYTES_DEFAULT,
            requested: false,
        }
    }

    pub fn agent_requested(self) -> bool {
        self.requested && self.format.is_agent()
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodesAtInput {
    pub file: String,
    pub line: usize,
    pub verbosity: Verbosity,
    pub view: ViewOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SymbolSpansInput {
    pub seed: SeedInput,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CallersInput {
    pub seed: SeedInput,
    pub depth: usize,
    pub max_results: usize,
    pub verbosity: Verbosity,
    pub view: ViewOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CalleesInput {
    pub seed: SeedInput,
    pub depth: usize,
    pub max_results: usize,
    pub verbosity: Verbosity,
    pub view: ViewOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EgoInput {
    pub seed: SeedInput,
    pub hops: usize,
    pub edges: Vec<String>,
    pub max_results: usize,
    pub view: ViewOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModuleDepsInput {
    pub file: String,
    pub max_results: usize,
    pub verbosity: Verbosity,
    pub view: ViewOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoMapInput {
    pub max_results: usize,
    pub view: ViewOptions,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TaintReachesInput {
    pub sources: Vec<SeedInput>,
    pub sinks: Option<Vec<SeedInput>>,
    pub max_results: usize,
    pub verbosity: Verbosity,
}

pub fn parse_nodes_at(args: &Value) -> Result<NodesAtInput, ToolError> {
    let obj = object(
        args,
        "nav_nodes_at",
        &[
            "file",
            "line",
            "verbosity",
            "format",
            "profile",
            "snippets",
            "group_by",
            "max_view_bytes",
        ],
    )?;
    let file = normalize_file_arg(required_string(obj, "file")?);
    let line = required_usize(obj, "line", 1, usize::MAX)?;
    let verbosity = parse_verbosity(obj.get("verbosity"))?;
    let view = parse_view_options(
        obj,
        "nav_nodes_at",
        EvidenceProfile::Seed,
        &[
            EvidenceProfile::Seed,
            EvidenceProfile::EditContext,
            EvidenceProfile::Audit,
        ],
    )?;
    Ok(NodesAtInput {
        file,
        line,
        verbosity,
        view,
    })
}

pub fn parse_symbol_spans(args: &Value) -> Result<SymbolSpansInput, ToolError> {
    let obj = object(args, "nav_symbol_spans", &["seed"])?;
    Ok(SymbolSpansInput {
        seed: parse_seed(required_value(obj, "seed")?)?,
    })
}

pub fn parse_callers(args: &Value) -> Result<CallersInput, ToolError> {
    let obj = object(
        args,
        "nav_callers",
        &[
            "seed",
            "depth",
            "max_results",
            "verbosity",
            "format",
            "profile",
            "snippets",
            "group_by",
            "max_view_bytes",
        ],
    )?;
    Ok(CallersInput {
        seed: parse_seed(required_value(obj, "seed")?)?,
        depth: optional_usize(obj, "depth", DEPTH_DEFAULT, 0, DEPTH_MAX)?,
        max_results: optional_usize(obj, "max_results", MAX_RESULTS_DEFAULT, 1, MAX_RESULTS_CAP)?,
        verbosity: parse_verbosity(obj.get("verbosity"))?,
        view: parse_view_options(
            obj,
            "nav_callers",
            EvidenceProfile::Impact,
            &[
                EvidenceProfile::Impact,
                EvidenceProfile::EditContext,
                EvidenceProfile::Audit,
            ],
        )?,
    })
}

pub fn parse_callees(args: &Value) -> Result<CalleesInput, ToolError> {
    let obj = object(
        args,
        "nav_callees",
        &[
            "seed",
            "depth",
            "max_results",
            "verbosity",
            "format",
            "profile",
            "snippets",
            "group_by",
            "max_view_bytes",
        ],
    )?;
    Ok(CalleesInput {
        seed: parse_seed(required_value(obj, "seed")?)?,
        depth: optional_usize(obj, "depth", DEPTH_DEFAULT, 0, DEPTH_MAX)?,
        max_results: optional_usize(obj, "max_results", MAX_RESULTS_DEFAULT, 1, MAX_RESULTS_CAP)?,
        verbosity: parse_verbosity(obj.get("verbosity"))?,
        view: parse_view_options(
            obj,
            "nav_callees",
            EvidenceProfile::Dependencies,
            &[
                EvidenceProfile::Dependencies,
                EvidenceProfile::EditContext,
                EvidenceProfile::Audit,
            ],
        )?,
    })
}

pub fn parse_ego(args: &Value) -> Result<EgoInput, ToolError> {
    let obj = object(
        args,
        "nav_ego_graph",
        &[
            "seed",
            "hops",
            "edges",
            "max_results",
            "format",
            "profile",
            "snippets",
            "group_by",
            "max_view_bytes",
        ],
    )?;
    Ok(EgoInput {
        seed: parse_seed(required_value(obj, "seed")?)?,
        hops: optional_usize(obj, "hops", HOPS_DEFAULT, 0, HOPS_MAX)?,
        edges: parse_edges(obj.get("edges"))?,
        max_results: optional_usize(obj, "max_results", MAX_RESULTS_DEFAULT, 1, MAX_RESULTS_CAP)?,
        view: parse_view_options(
            obj,
            "nav_ego_graph",
            EvidenceProfile::Graph,
            &[EvidenceProfile::Graph, EvidenceProfile::Audit],
        )?,
    })
}

pub fn parse_module_deps(args: &Value) -> Result<ModuleDepsInput, ToolError> {
    let obj = object(
        args,
        "nav_module_deps",
        &[
            "file",
            "max_results",
            "verbosity",
            "format",
            "profile",
            "snippets",
            "group_by",
            "max_view_bytes",
        ],
    )?;
    Ok(ModuleDepsInput {
        file: normalize_file_arg(required_string(obj, "file")?),
        max_results: optional_usize(obj, "max_results", MAX_RESULTS_DEFAULT, 1, MAX_RESULTS_CAP)?,
        verbosity: parse_verbosity(obj.get("verbosity"))?,
        view: parse_view_options(
            obj,
            "nav_module_deps",
            EvidenceProfile::Dependencies,
            &[
                EvidenceProfile::Dependencies,
                EvidenceProfile::Orientation,
                EvidenceProfile::Audit,
            ],
        )?,
    })
}

pub fn parse_repo_map(args: &Value) -> Result<RepoMapInput, ToolError> {
    let obj = object(
        args,
        "nav_repo_map",
        &[
            "max_results",
            "format",
            "profile",
            "snippets",
            "group_by",
            "max_view_bytes",
        ],
    )?;
    Ok(RepoMapInput {
        max_results: optional_usize(obj, "max_results", MAX_RESULTS_DEFAULT, 1, MAX_RESULTS_CAP)?,
        view: parse_view_options(
            obj,
            "nav_repo_map",
            EvidenceProfile::Orientation,
            &[
                EvidenceProfile::Orientation,
                EvidenceProfile::Graph,
                EvidenceProfile::Audit,
            ],
        )?,
    })
}

pub fn parse_taint_reaches(args: &Value) -> Result<TaintReachesInput, ToolError> {
    let obj = object(
        args,
        "taint_reaches",
        &["sources", "sinks", "max_results", "verbosity"],
    )?;
    Ok(TaintReachesInput {
        sources: parse_seed_array(required_value(obj, "sources")?, "sources")?,
        sinks: match obj.get("sinks") {
            Some(value) => Some(parse_seed_array(value, "sinks")?),
            None => None,
        },
        max_results: optional_usize(obj, "max_results", MAX_RESULTS_DEFAULT, 1, MAX_RESULTS_CAP)?,
        verbosity: parse_verbosity(obj.get("verbosity"))?,
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
            let (norm_file, norm_line) =
                normalize_loc_seed(file.clone(), *line).map_err(ToolError::BadArguments)?;
            *file = norm_file;
            *line = norm_line;
        }
    }
    Ok(seed)
}

fn parse_seed_array(value: &Value, key: &str) -> Result<Vec<SeedInput>, ToolError> {
    let Some(values) = value.as_array() else {
        return bad_args(format!("argument `{key}` must be an array"));
    };
    if values.is_empty() {
        return bad_args(format!("argument `{key}` must not be empty"));
    }
    values.iter().map(parse_seed).collect()
}

fn parse_verbosity(value: Option<&Value>) -> Result<Verbosity, ToolError> {
    match value {
        None => Ok(Verbosity::Concise),
        Some(Value::String(s)) if s == "concise" => Ok(Verbosity::Concise),
        Some(Value::String(s)) if s == "detailed" => Ok(Verbosity::Detailed),
        Some(_) => bad_args("argument `verbosity` must be `concise` or `detailed`"),
    }
}

fn parse_view_options(
    obj: &Map<String, Value>,
    tool: &str,
    default_profile: EvidenceProfile,
    allowed_profiles: &[EvidenceProfile],
) -> Result<ViewOptions, ToolError> {
    let requested = obj.contains_key("format")
        || obj.contains_key("profile")
        || obj.contains_key("snippets")
        || obj.contains_key("group_by")
        || obj.contains_key("max_view_bytes");
    if !requested {
        return Ok(ViewOptions::canonical(default_profile));
    }

    let format = parse_view_format(obj.get("format"))?;
    if !format.is_agent() {
        let only_canonical_format = obj.contains_key("format")
            && !obj.contains_key("profile")
            && !obj.contains_key("snippets")
            && !obj.contains_key("group_by")
            && !obj.contains_key("max_view_bytes");
        if only_canonical_format {
            return Ok(ViewOptions::canonical(default_profile));
        }
        return bad_args(
            "view controls require argument `format` to be `agent_markdown` or `agent_json`",
        );
    }

    let profile = parse_profile(obj.get("profile"), default_profile)?;
    if !allowed_profiles.contains(&profile) {
        return bad_args(format!(
            "profile `{}` is not supported for {tool}",
            profile.as_str()
        ));
    }
    Ok(ViewOptions {
        format,
        profile,
        snippets: parse_snippets(obj.get("snippets"))?,
        group_by: parse_group_by(obj.get("group_by"))?,
        group_by_explicit: obj.contains_key("group_by"),
        max_view_bytes: optional_usize(
            obj,
            "max_view_bytes",
            MAX_VIEW_BYTES_DEFAULT,
            1,
            MAX_VIEW_BYTES_CAP,
        )?,
        requested: true,
    })
}

fn parse_view_format(value: Option<&Value>) -> Result<ViewFormat, ToolError> {
    match value {
        None => Ok(ViewFormat::CanonicalJson),
        Some(Value::String(s)) if s == "canonical_json" => Ok(ViewFormat::CanonicalJson),
        Some(Value::String(s)) if s == "agent_markdown" => Ok(ViewFormat::AgentMarkdown),
        Some(Value::String(s)) if s == "agent_json" => Ok(ViewFormat::AgentJson),
        Some(_) => bad_args(
            "argument `format` must be `canonical_json`, `agent_markdown`, or `agent_json`",
        ),
    }
}

fn parse_profile(
    value: Option<&Value>,
    default_profile: EvidenceProfile,
) -> Result<EvidenceProfile, ToolError> {
    match value {
        None => Ok(default_profile),
        Some(Value::String(s)) => EvidenceProfile::from_str(s)
            .ok_or_else(|| ToolError::BadArguments(format!("unknown profile `{s}`"))),
        Some(_) => bad_args("argument `profile` must be a string"),
    }
}

fn parse_snippets(value: Option<&Value>) -> Result<SnippetPolicy, ToolError> {
    match value {
        None => Ok(SnippetPolicy::None),
        Some(Value::String(s)) if s == "none" => Ok(SnippetPolicy::None),
        Some(Value::String(s)) if s == "line" => Ok(SnippetPolicy::Line),
        Some(Value::String(s)) if s == "symbol_header" => Ok(SnippetPolicy::SymbolHeader),
        Some(_) => bad_args("argument `snippets` must be `none`, `line`, or `symbol_header`"),
    }
}

fn parse_group_by(value: Option<&Value>) -> Result<GroupPolicy, ToolError> {
    match value {
        None => Ok(GroupPolicy::None),
        Some(Value::String(s)) if s == "none" => Ok(GroupPolicy::None),
        Some(Value::String(s)) if s == "file" => Ok(GroupPolicy::File),
        Some(Value::String(s)) if s == "symbol" => Ok(GroupPolicy::Symbol),
        Some(_) => bad_args("argument `group_by` must be `none`, `file`, or `symbol`"),
    }
}

impl EvidenceProfile {
    pub fn as_str(self) -> &'static str {
        match self {
            EvidenceProfile::Orientation => "orientation",
            EvidenceProfile::Seed => "seed",
            EvidenceProfile::Impact => "impact",
            EvidenceProfile::Dependencies => "dependencies",
            EvidenceProfile::EditContext => "edit_context",
            EvidenceProfile::Graph => "graph",
            EvidenceProfile::Audit => "audit",
        }
    }

    fn from_str(value: &str) -> Option<Self> {
        match value {
            "orientation" => Some(EvidenceProfile::Orientation),
            "seed" => Some(EvidenceProfile::Seed),
            "impact" => Some(EvidenceProfile::Impact),
            "dependencies" => Some(EvidenceProfile::Dependencies),
            "edit_context" => Some(EvidenceProfile::EditContext),
            "graph" => Some(EvidenceProfile::Graph),
            "audit" => Some(EvidenceProfile::Audit),
            _ => None,
        }
    }
}

impl ViewFormat {
    pub fn as_str(self) -> &'static str {
        match self {
            ViewFormat::CanonicalJson => "canonical_json",
            ViewFormat::AgentMarkdown => "agent_markdown",
            ViewFormat::AgentJson => "agent_json",
        }
    }
}

impl SnippetPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            SnippetPolicy::None => "none",
            SnippetPolicy::Line => "line",
            SnippetPolicy::SymbolHeader => "symbol_header",
        }
    }
}

impl GroupPolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            GroupPolicy::None => "none",
            GroupPolicy::File => "file",
            GroupPolicy::Symbol => "symbol",
        }
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

fn bad_args<T>(message: impl Into<String>) -> Result<T, ToolError> {
    Err(ToolError::BadArguments(message.into()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::reasoning::seeds::{normalize_path, NormalizedPath};
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
        assert!(parse_taint_reaches(&json!({"sources":[]})).is_err());
        assert!(parse_taint_reaches(&json!({"sources":[{"kind":"symbol","name":"f"}]})).is_ok());
    }

    #[test]
    fn symbol_spans_input_is_strict_and_normalizes_seed() {
        let parsed = parse_symbol_spans(&json!({
            "seed":{"kind":"symbol","name":"f","file":"./src//lib.rs"}
        }))
        .unwrap();
        assert_eq!(
            parsed.seed,
            SeedInput::Symbol {
                name: "f".into(),
                file: Some("src/lib.rs".into())
            }
        );
        assert!(parse_symbol_spans(&json!({})).is_err());
        assert!(parse_symbol_spans(&json!({
            "seed":{"kind":"symbol","name":"f"},
            "extra":true
        }))
        .is_err());
        let escaping = parse_symbol_spans(&json!({
            "seed":{"kind":"loc","file":"/etc/passwd","line":1}
        }))
        .unwrap();
        assert_eq!(
            escaping.seed,
            SeedInput::Loc {
                file: "/etc/passwd".into(),
                line: 1
            }
        );
        assert!(parse_symbol_spans(&json!({
            "seed":{"kind":"loc","file":"a.py","line":0}
        }))
        .is_err());
    }

    #[test]
    fn view_options_require_agent_format_and_allowed_profile() {
        assert!(parse_callers(&json!({
            "seed":{"kind":"symbol","name":"f"},
            "snippets":"line"
        }))
        .is_err());
        let parsed = parse_callers(&json!({
            "seed":{"kind":"symbol","name":"f"},
            "format":"agent_markdown"
        }))
        .unwrap();
        assert!(parsed.view.agent_requested());
        assert_eq!(parsed.view.profile, EvidenceProfile::Impact);
        assert!(!parsed.view.group_by_explicit);
        assert!(parse_callers(&json!({
            "seed":{"kind":"symbol","name":"f"},
            "format":"agent_json",
            "profile":"dependencies"
        }))
        .is_err());
    }

    #[test]
    fn view_options_track_explicit_group_by_none() {
        let parsed = parse_callers(&json!({
            "seed":{"kind":"symbol","name":"f"},
            "format":"agent_markdown",
            "group_by":"none"
        }))
        .unwrap();
        assert!(parsed.view.group_by_explicit);
        assert_eq!(parsed.view.group_by, GroupPolicy::None);
    }
}
