//! Pure finding-to-target mapping rules for targets contract v1.0.

use super::DependencyHint;
use crate::languages::Language;
use crate::slice::SliceFinding;

#[derive(Debug, Clone)]
pub struct Mapped {
    pub kind: &'static str,
    pub property: &'static str,
    pub detail: Option<String>,
    pub hint: Option<DependencyHint>,
}

/// `(PairedPattern.description, unambiguous counterpart base, dependency kind)`.
/// The descriptions are copied verbatim from every row returned by
/// `absence_slice::default_pairs()`.
pub const ABSENCE_PAIRS: &[(&str, Option<&str>, Option<&str>)] = &[
    ("file open without close", Some("close"), Some("filesystem")),
    ("lock without unlock", Some("unlock"), None),
    ("connection opened without close", None, None),
    ("event subscription without unsubscribe", None, None),
    ("transaction begin without commit/rollback", None, None),
    ("allocation without free", None, None),
    ("timer set without clear", None, None),
    ("item added without removal path", None, None),
    ("span/timer started without end", None, None),
    (
        "resource acquisition without defer cleanup (Go)",
        None,
        None,
    ),
    ("kernel allocation without free", None, None),
    (
        "DMA allocation without free",
        Some("dma_free_coherent"),
        None,
    ),
    ("IRQ registered without free", Some("free_irq"), None),
    ("spinlock without unlock", None, None),
    (
        "clock enabled without disable",
        Some("clk_disable_unprepare"),
        None,
    ),
    (
        "platform driver registered without unregister",
        Some("platform_driver_unregister"),
        None,
    ),
    (
        "device tree node get without put",
        Some("of_node_put"),
        None,
    ),
    (
        "kernel mutex lock without unlock",
        Some("mutex_unlock"),
        None,
    ),
    ("rtnl lock without unlock", Some("rtnl_unlock"), None),
    ("kstrdup allocation without kfree", Some("kfree"), None),
    (
        "slab cache allocation without free",
        Some("kmem_cache_free"),
        None,
    ),
    (
        "RCU read lock without unlock",
        Some("rcu_read_unlock"),
        None,
    ),
    (
        "pthread mutex lock without unlock",
        Some("pthread_mutex_unlock"),
        None,
    ),
    ("semaphore wait without post", Some("sem_post"), None),
    ("mmap without munmap", Some("munmap"), Some("filesystem")),
    (
        "POSIX file descriptor opened without close",
        Some("close"),
        Some("filesystem"),
    ),
    (
        "directory opened without closedir",
        Some("closedir"),
        Some("filesystem"),
    ),
    (
        "pthread rwlock without unlock",
        Some("pthread_rwlock_unlock"),
        None,
    ),
    (
        "Python threading lock without release",
        Some("release"),
        None,
    ),
    ("Python multiprocessing pool without close/join", None, None),
    ("socket created without close", Some("close"), None),
    ("temporary file without cleanup", None, Some("filesystem")),
    ("Node.js stream without destroy/close/end", None, None),
    ("server created without close", None, None),
    ("database pool connection without release", None, None),
    ("fs.open without fs.close", None, Some("filesystem")),
    ("lock/acquire without release/unlock", None, None),
    ("Go sql.Open without db.Close", None, None),
    (
        "Go file created without Close",
        Some("Close"),
        Some("filesystem"),
    ),
    (
        "Go context without cancel (may leak goroutine)",
        Some("cancel"),
        None,
    ),
    ("WaitGroup Add without Wait", Some("Wait"), None),
    ("Go HTTP response body not closed", Some("Body.Close"), None),
    (
        "Rust file opened without explicit flush/drop",
        None,
        Some("filesystem"),
    ),
    (
        "advisory: Rust mutex lock held to end of scope (explicit drop() releases sooner)",
        Some("drop"),
        None,
    ),
    (
        "unsafe block without safety assertion or comment",
        None,
        None,
    ),
    ("Rust TCP connection without shutdown/drop", None, None),
    (
        "Rust Command created but never executed",
        Some("spawn"),
        None,
    ),
    ("Lua file opened without close", None, Some("filesystem")),
    ("Lua socket opened without close", None, None),
    (
        "Lua coroutine created but never resumed",
        Some("coroutine.resume"),
        None,
    ),
    ("S3 bucket missing encryption configuration", None, None),
    ("S3 bucket missing public access block", None, None),
    ("S3 bucket missing versioning configuration", None, None),
    ("Lambda function missing CloudWatch log group", None, None),
    ("Security group missing explicit rule resource", None, None),
    (
        "RDS instance missing storage_encrypted configuration",
        None,
        None,
    ),
    (
        "Temp file created with mktemp but never cleaned up",
        None,
        Some("filesystem"),
    ),
    (
        "Filesystem mounted but never unmounted",
        None,
        Some("filesystem"),
    ),
    ("pushd without matching popd", None, None),
    ("Signal trap set but never restored/cleared", None, None),
    (
        "File descriptor opened but never closed",
        None,
        Some("filesystem"),
    ),
    (
        "Lock file acquired but never released",
        None,
        Some("filesystem"),
    ),
    (
        "Firmware flash write (mtd) without hash verification",
        None,
        None,
    ),
    ("UCI config set without commit", None, None),
    (
        "Kernel module loaded without unload in cleanup path",
        None,
        None,
    ),
];

pub fn map_finding(finding: &SliceFinding) -> Mapped {
    let category = finding.category.as_deref().unwrap_or("uncategorized");
    match (finding.algorithm.as_str(), category) {
        ("echo", "missing_error_handling") => mapped(
            "external_call",
            "error_handled",
            None,
            echo_callee(&finding.description).map(|callee| hint(None, Some(callee), None)),
        ),
        ("membrane", "unprotected_caller") => mapped(
            "boundary",
            "error_handled",
            None,
            membrane_callee(&finding.description).map(|callee| hint(None, Some(callee), None)),
        ),
        (
            "absence",
            "missing_counterpart" | "missing_close_on_error_path" | "close_only_on_error_path",
        ) => {
            let pair = ABSENCE_PAIRS
                .iter()
                .find(|(description, _, _)| finding.description.starts_with(description));
            let dependency = pair.and_then(|(_, counterpart, kind)| {
                if counterpart.is_none() && kind.is_none() {
                    None
                } else {
                    Some(hint(*kind, None, *counterpart))
                }
            });
            mapped("resource_acquire", "resource_released", None, dependency)
        }
        ("absence", "double_close") => mapped(
            "resource_release",
            "resource_not_double_released",
            None,
            double_close_token(&finding.description)
                .map(|counterpart| hint(None, None, Some(counterpart))),
        ),
        ("contract", "contract_violation") => {
            let property = if finding.description.starts_with("Guard clause modified") {
                "precondition_holds"
            } else if finding.description.starts_with("Return behavior modified") {
                "postcondition_holds"
            } else {
                "unknown"
            };
            mapped("contract", property, None, None)
        }
        ("contract", "contract") => mapped("contract", "precondition_holds", None, None),
        ("contract", category) if category.starts_with("contract_precondition_") => {
            mapped("contract", "precondition_holds", None, None)
        }
        ("contract", category) if category.starts_with("contract_postcondition") => {
            mapped("contract", "postcondition_holds", None, None)
        }
        ("provenance", "untrusted_origin") => {
            let origin = provenance_origin(&finding.description);
            let detail = origin.map(|value| format!("{value} origin at use site"));
            let dependency = match origin {
                Some("database") => Some(hint(Some("db"), None, None)),
                Some("external_call") => Some(hint(Some("network"), None, None)),
                _ => None,
            };
            mapped("other", "origin_trusted", detail, dependency)
        }
        ("taint", "taint_source") => mapped("data_origin", "origin_trusted", None, None),
        ("taint", "taint_sink" | "unquoted_expansion") => {
            mapped("other", "not_reached_by_taint", None, None)
        }
        ("symmetry", "broken_symmetry") => mapped(
            "contract",
            "counterpart_present",
            None,
            symmetry_counterpart(&finding.description)
                .map(|counterpart| hint(None, None, Some(counterpart))),
        ),
        ("peer_consistency", "peer_guard_divergence") => {
            mapped("contract", "peer_consistent", None, None)
        }
        _ => mapped("other", "unknown", None, None),
    }
}

pub fn language_tag(lang: Language) -> &'static str {
    match lang {
        Language::Python => "python",
        Language::JavaScript => "javascript",
        Language::TypeScript => "typescript",
        Language::Tsx => "tsx",
        Language::Go => "go",
        Language::Java => "java",
        Language::C => "c",
        Language::Cpp => "cpp",
        Language::Rust => "rust",
        Language::Lua => "lua",
        Language::Terraform => "hcl",
        Language::Bash => "bash",
    }
}

fn mapped(
    kind: &'static str,
    property: &'static str,
    detail: Option<String>,
    hint: Option<DependencyHint>,
) -> Mapped {
    Mapped {
        kind,
        property,
        detail,
        hint,
    }
}

fn hint(kind: Option<&str>, callee: Option<&str>, counterpart: Option<&str>) -> DependencyHint {
    DependencyHint {
        kind: kind.map(str::to_string),
        callee: callee.map(str::to_string),
        counterpart: counterpart.map(str::to_string),
    }
}

fn echo_callee(description: &str) -> Option<&str> {
    let remainder = description.strip_prefix('\'')?;
    let (_, remainder) = remainder.split_once("' calls '")?;
    let (callee, _) = remainder.split_once("' without handling: ")?;
    (!callee.is_empty()).then_some(callee)
}

fn membrane_callee(description: &str) -> Option<&str> {
    let remainder = description.strip_prefix("unprotected call to '")?;
    let (callee, _) = remainder.split_once("' from '")?;
    (!callee.is_empty()).then_some(callee)
}

fn symmetry_counterpart(description: &str) -> Option<&str> {
    let remainder = description.strip_prefix('\'')?;
    let (_, remainder) = remainder.split_once("' changed but symmetric counterpart '")?;
    let counterpart = remainder.strip_suffix("' was not")?;
    (!counterpart.is_empty()).then_some(counterpart)
}

fn double_close_token(description: &str) -> Option<&str> {
    description.strip_prefix("potential double-close in '")?;
    let marker = "() at line ";
    let prefix = description.get(..description.find(marker)?)?;
    let token = prefix.split_whitespace().last()?;
    (!token.is_empty()).then_some(token)
}

fn provenance_origin(description: &str) -> Option<&str> {
    description.strip_prefix("variable '")?;
    let (_, remainder) = description.split_once(" has ")?;
    let (origin, _) = remainder.split_once(" origin:")?;
    match origin {
        "user_input" | "config" | "database" | "constant" | "env_var" | "function_param"
        | "external_call" | "hardware" | "unknown" => Some(origin),
        _ => None,
    }
}
