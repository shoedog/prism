//! Accepted REAL depth-one recipe: one located boundary message plus one typed run note.
use super::{notices, Documents, Sample};
use crate::algorithms::taint::boundary::records::ContextRunNoteKind;

pub(super) fn assert_depth_bound_once(sample: &Sample, document: &Documents) {
    let crate::api::context_run::ContextState::Expanded(context) = &sample.run.context else {
        panic!("depth-one review must retain actual expanded state");
    };
    assert_eq!(context.round, 1);
    assert!(matches!(
        context.stop,
        crate::algorithms::taint::rounds::Stop::DepthReached
    ));
    assert_eq!(
        context
            .context_files
            .iter()
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["a.js"]
    );
    let display = sample.run.diagnostics.as_ref().unwrap();
    assert_eq!(display.run_notes().len(), 1);
    let note = &display.run_notes()[0];
    assert_eq!(note.kind(), ContextRunNoteKind::DepthBound);
    assert_eq!(note.location(), None);
    assert_eq!(
        note.message(),
        "taint-context: depth bound 1 reached; 1 boundary calls not followed"
    );
    assert_eq!(display.located().len(), 1);
    let boundary = &display.located()[0];
    assert_eq!(boundary.file(), "a.js");
    assert_eq!(boundary.line(), 2);
    assert_eq!(boundary.callee(), "g");
    let notifications = notices(&document.sarif.value);
    let warnings = document.json.value["warnings"].as_array().unwrap();
    // These are two different messages. Once means once per actual producer message.
    assert_eq!(notifications.len(), 2);
    assert_eq!(warnings.len(), 2);
    for message in [boundary.message(), note.message()] {
        assert_eq!(
            notifications
                .iter()
                .filter(|n| n["message"]["text"] == message)
                .count(),
            1
        );
        assert_eq!(
            warnings
                .iter()
                .filter(|n| n.as_str() == Some(message))
                .count(),
            1
        );
    }
    assert_eq!(
        notifications
            .iter()
            .filter(|n| n["message"]["text"] == note.message())
            .next()
            .unwrap()["level"],
        "note"
    );
    assert_eq!(
        document.sarif.value["runs"][0]["invocations"][0]["executionSuccessful"],
        true
    );
}
