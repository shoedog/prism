//! Walk-proof switches: an env-driven measurement knob that makes the taint trace walk refuse the
//! crossing hops a candidate I-walk rule would refuse (lane 8, DESIGN 5.2 (i)-(v) plus the two
//! return rules), so the sink-level recall cost of each rule, alone and combined, can be read off
//! the real trace-review sink lines.
//!
//! OFF by default; with every switch off nothing is read, parsed or recorded and the walk's output
//! (stdout, stderr, cache) is exactly the walk's without this module.
//!
//! - `PRISM_TAINT_WALK_PROOFS_FILE=<path>`: JSON
//!   `{"version":1,"rules":{"<rule>":[{"relation":"CallDescent|ReturnFlow|ReturnInput",
//!   "from":"<key>","to":"<key>","root":"<key>"?}, ..]}}`. A key is the crossing ledger's
//!   `from=`/`to=`/`root=` value with its Debug quotes and escapes removed (the ledger prints each
//!   identity with `{:?}`) and its `#<digits>` node index dropped (`CodePropertyGraph::trace_key`);
//!   an absent `root` means the exclusion applies under every root.
//! - `PRISM_TAINT_WALK_PROOFS=<comma list>`: the rules switched ON, from `i,ii,iii,iv,v,
//!   return_flow,return_input` or `all`; unset or empty is every switch off.
//!
//! A switched-on rule refuses a filed hop exactly as a refused gate would (boundary record, ledger
//! line) with the reason of that rule: i NoMatchingSite, ii AmbiguousSite, iii NotExact,
//! iv UnprovenSlot, v AmbiguousSite, return_flow ReturnNotEnteredSite. A filed ReturnInput hop
//! under return_input is a cut (UnprovenReturnInput). A rule only matches hops of its own
//! relation (i..v: CallDescent, return_flow: ReturnFlow, return_input: ReturnInput), the first
//! matching rule in that order supplies the reason, and the descent gate cache never sees any of
//! it. The configuration is read, and validated, once per process when a walk starts (not only
//! when a hop reaches the hook); a value that is not UTF-8 is an error. When a switch is on, one
//! stderr line `taint-walk-proofs rule=<r> kept=<n> dropped_relation_mismatch=<m>` per
//! switched-on rule reports the filed entries kept and those dropped for a relation other than
//! the rule's. A misconfiguration (switches without a file, an unreadable or malformed file, an unknown
//! rule, a switched-on rule the file lacks) panics naming the variable: a measurement that is
//! misconfigured must not look like a clean baseline.
//!
//! Scope: the primary and bypass walks (`taint_trace_from_root`, used by trace-review and
//! navigation). The legacy flat taint walk of the default review is intentionally NOT affected.

use std::collections::{BTreeMap, BTreeSet};
use std::io::Write;
use std::sync::OnceLock;

use petgraph::graph::NodeIndex;
use serde::Deserialize;

use super::{CodePropertyGraph, LossReason, Relation};

const SWITCHES: &str = "PRISM_TAINT_WALK_PROOFS";
const FILE: &str = "PRISM_TAINT_WALK_PROOFS_FILE";
const SUMMARY: &str = "taint-walk-proofs ";

/// A variable's value; unset is `None`, a value that is not UTF-8 is a loud error.
fn env_text(name: &str) -> Option<String> {
    match std::env::var(name) {
        Ok(value) => Some(value),
        Err(std::env::VarError::NotPresent) => None,
        Err(std::env::VarError::NotUnicode(_)) => panic!("{name}: the value is not valid UTF-8"),
    }
}

/// Reads the configuration now (once per process) so a misconfiguration panics at walk entry
/// even if no hop ever reaches the hook; one `Option` test when off.
pub(super) fn check_walk_proofs() {
    let _ = WalkProofs::global();
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Rule {
    I,
    Ii,
    Iii,
    Iv,
    V,
    ReturnFlow,
    ReturnInput,
}

impl Rule {
    const ALL: [Rule; 7] = [
        Rule::I,
        Rule::Ii,
        Rule::Iii,
        Rule::Iv,
        Rule::V,
        Rule::ReturnFlow,
        Rule::ReturnInput,
    ];

    fn name(self) -> &'static str {
        match self {
            Rule::I => "i",
            Rule::Ii => "ii",
            Rule::Iii => "iii",
            Rule::Iv => "iv",
            Rule::V => "v",
            Rule::ReturnFlow => "return_flow",
            Rule::ReturnInput => "return_input",
        }
    }

    fn reason(self) -> LossReason {
        match self {
            Rule::I => LossReason::NoMatchingSite,
            Rule::Ii | Rule::V => LossReason::AmbiguousSite,
            Rule::Iii => LossReason::NotExact,
            Rule::Iv => LossReason::UnprovenSlot,
            Rule::ReturnFlow => LossReason::ReturnNotEnteredSite,
            Rule::ReturnInput => LossReason::UnprovenReturnInput,
        }
    }

    /// The only hop relation this rule can match.
    fn relation(self) -> Relation {
        match self {
            Rule::ReturnFlow => Relation::ReturnFlow,
            Rule::ReturnInput => Relation::ReturnInput,
            _ => Relation::CallDescent,
        }
    }

    fn parse(name: &str) -> Option<Rule> {
        Rule::ALL.into_iter().find(|rule| rule.name() == name)
    }
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct FileModel {
    version: u32,
    rules: BTreeMap<String, Vec<EntryModel>>,
}

#[derive(Deserialize)]
#[serde(deny_unknown_fields)]
struct EntryModel {
    relation: RelationModel,
    from: String,
    to: String,
    root: Option<String>,
}

#[derive(Deserialize, Clone, Copy, PartialEq)]
enum RelationModel {
    CallDescent,
    ReturnFlow,
    ReturnInput,
}

impl RelationModel {
    fn relation(self) -> Relation {
        match self {
            Self::CallDescent => Relation::CallDescent,
            Self::ReturnFlow => Relation::ReturnFlow,
            Self::ReturnInput => Relation::ReturnInput,
        }
    }
}

/// `(from, to, root)`; the relation is the rule's own.
type Exclusion = (String, String, Option<String>);

/// A switched-on rule, its filed exclusions, and how many filed entries of another relation
/// than the rule's were dropped at load.
#[derive(Debug)]
struct RuleSet {
    rule: Rule,
    entries: BTreeSet<Exclusion>,
    dropped: usize,
}

/// The switched-on rules and their filed exclusions.
#[derive(Debug)]
pub(super) struct WalkProofs {
    rules: Vec<RuleSet>,
}

/// The rules a switch list turns on; empty when it is unset, blank or only separators.
fn parse_switches(switches: Option<&str>) -> Result<BTreeSet<Rule>, String> {
    let mut on = BTreeSet::new();
    for item in switches
        .unwrap_or_default()
        .split(',')
        .map(str::trim)
        .filter(|item| !item.is_empty())
    {
        if item == "all" {
            on.extend(Rule::ALL);
        } else {
            on.insert(Rule::parse(item).ok_or_else(|| {
                format!("{SWITCHES}: unknown rule `{item}` (expected i, ii, iii, iv, v, return_flow, return_input or all)")
            })?);
        }
    }
    Ok(on)
}

impl WalkProofs {
    /// `Ok(None)` when no switch is on (the file is then not even read). Errors name the variable.
    pub(super) fn from_parts(
        file_text: Option<&str>,
        switches: Option<&str>,
    ) -> Result<Option<Self>, String> {
        let on = parse_switches(switches)?;
        if on.is_empty() {
            return Ok(None);
        }
        let text = file_text.ok_or_else(|| {
            format!("{SWITCHES} is set but {FILE} is not: the walk proofs need their file")
        })?;
        let model: FileModel =
            serde_json::from_str(text).map_err(|error| format!("{FILE}: malformed: {error}"))?;
        if model.version != 1 {
            return Err(format!(
                "{FILE}: unsupported version {} (expected 1)",
                model.version
            ));
        }
        if let Some(name) = model.rules.keys().find(|name| Rule::parse(name).is_none()) {
            return Err(format!("{FILE}: unknown rule `{name}`"));
        }
        let mut rules = Vec::new();
        for rule in on {
            let entries = model.rules.get(rule.name()).ok_or_else(|| {
                format!(
                    "{SWITCHES} switches rule `{}` on but {FILE} has no such rule",
                    rule.name()
                )
            })?;
            // An entry of another relation than the rule's can never match a hop of it.
            let kept: BTreeSet<Exclusion> = entries
                .iter()
                .filter(|entry| entry.relation.relation() == rule.relation())
                .map(|e| (e.from.clone(), e.to.clone(), e.root.clone()))
                .collect();
            let dropped = entries
                .iter()
                .filter(|entry| entry.relation.relation() != rule.relation())
                .count();
            rules.push(RuleSet {
                rule,
                entries: kept,
                dropped,
            });
        }
        Ok(Some(Self { rules }))
    }

    /// The process-wide proofs, read once; panics on a misconfiguration. When a switch is on it
    /// prints one stderr line per switched-on rule with the entries kept and the entries dropped
    /// for a relation mismatch.
    pub(super) fn global() -> Option<&'static WalkProofs> {
        static PROOFS: OnceLock<Option<WalkProofs>> = OnceLock::new();
        PROOFS
            .get_or_init(|| {
                let switches = env_text(SWITCHES);
                let file = env_text(FILE);
                let on = parse_switches(switches.as_deref())
                    .unwrap_or_else(|message| panic!("{message}"));
                // The file is read only when a rule is on, exactly as `from_parts` decides.
                let text = file.filter(|_| !on.is_empty()).map(|path| {
                    std::fs::read_to_string(&path)
                        .unwrap_or_else(|error| panic!("{FILE}: cannot read {path}: {error}"))
                });
                let proofs = Self::from_parts(text.as_deref(), switches.as_deref())
                    .unwrap_or_else(|message| panic!("{message}"));
                for set in proofs.iter().flat_map(|proofs| &proofs.rules) {
                    // A diagnostic only: it has no bearing on the walk.
                    let _ = writeln!(
                        std::io::stderr().lock(),
                        "{SUMMARY}rule={} kept={} dropped_relation_mismatch={}",
                        set.rule.name(),
                        set.entries.len(),
                        set.dropped
                    );
                }
                proofs
            })
            .as_ref()
    }

    /// The reason the first switched-on rule filing this hop refuses it with.
    pub(super) fn refusal_for_keys(
        &self,
        relation: Relation,
        from: &str,
        to: &str,
        root: &str,
    ) -> Option<LossReason> {
        self.rules
            .iter()
            .filter(|set| set.rule.relation() == relation)
            .find(|set| {
                let hop = |root| (from.to_owned(), to.to_owned(), root);
                set.entries.contains(&hop(None))
                    || set.entries.contains(&hop(Some(root.to_owned())))
            })
            .map(|set| set.rule.reason())
    }
}

impl CodePropertyGraph {
    /// Whether a switched-on walk-proof rule refuses the hop `from -> to` under `root`; `None`
    /// (one `Option` test) when no switch is on.
    pub(super) fn walk_proof_refusal(
        &self,
        relation: Relation,
        from: NodeIndex,
        to: NodeIndex,
        root: NodeIndex,
    ) -> Option<LossReason> {
        let proofs = WalkProofs::global()?;
        proofs.refusal_for_keys(
            relation,
            &self.trace_key(from),
            &self.trace_key(to),
            &self.trace_key(root),
        )
    }
}
