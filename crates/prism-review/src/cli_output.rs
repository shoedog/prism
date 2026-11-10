pub(super) fn targets_exit_code(strict: bool, errors: &[prism::slice::AlgorithmError]) -> i32 {
    if strict && !errors.is_empty() {
        3
    } else {
        0
    }
}

pub(super) fn emit_warnings_to_stderr(warnings: &[prism::slice::DiagramWarning]) {
    use std::io::Write;
    let mut err = std::io::stderr().lock();
    for w in warnings {
        let title = w.graph_title.as_deref().unwrap_or("(no title)");
        let _ = writeln!(
            err,
            "prism: diagram warning: {}/{} - {:?}: {}",
            w.algorithm, title, w.kind, w.detail
        );
    }
}

pub(super) fn determine_exit_code(strict: bool, warnings: &[prism::slice::DiagramWarning]) -> i32 {
    if !strict {
        return 0;
    }
    if warnings.iter().any(|w| w.kind.is_bug()) {
        return 2;
    }
    0
}

#[cfg(test)]
mod exit_tests {
    use super::*;
    use prism::slice::{DiagramWarning, DiagramWarningKind};

    fn warn(kind: DiagramWarningKind) -> DiagramWarning {
        DiagramWarning {
            algorithm: "Taint".to_string(),
            graph_title: None,
            kind,
            detail: "x".to_string(),
        }
    }

    #[test]
    fn determine_exit_code_strict_with_bug_warning() {
        let warns = vec![warn(DiagramWarningKind::DanglingEdge)];
        assert_eq!(determine_exit_code(true, &warns), 2);
    }

    #[test]
    fn determine_exit_code_strict_with_only_informational() {
        let warns = vec![warn(DiagramWarningKind::NodeCapExceeded)];
        assert_eq!(determine_exit_code(true, &warns), 0);
    }

    #[test]
    fn determine_exit_code_strict_off() {
        let warns = vec![warn(DiagramWarningKind::DanglingEdge)];
        assert_eq!(determine_exit_code(false, &warns), 0);
    }

    #[test]
    fn determine_exit_code_strict_no_warnings() {
        let warns: Vec<DiagramWarning> = vec![];
        assert_eq!(determine_exit_code(true, &warns), 0);
    }

    #[test]
    fn targets_strict_exit_code_tracks_algorithm_errors() {
        let errors = vec![prism::slice::AlgorithmError {
            algorithm: "DeltaSlice".to_string(),
            error: "fixture error".to_string(),
        }];
        assert_eq!(targets_exit_code(true, &errors), 3);
        assert_eq!(targets_exit_code(false, &errors), 0);
        assert_eq!(targets_exit_code(true, &[]), 0);
    }
}
