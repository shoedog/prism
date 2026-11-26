use super::{
    flask_facts, index_python, CallbackStatus, FlaskEvent, Language, ParsedFile, Reason, Receiver,
    SetupContext,
};
use anyhow::Context;
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    env, fs,
    io::{BufWriter, Write},
    path::Path,
};

fn parse_directory(
    root: &Path,
    directory: &Path,
    files: &mut BTreeMap<String, ParsedFile>,
) -> anyhow::Result<()> {
    for entry in fs::read_dir(directory).with_context(|| directory.display().to_string())? {
        let entry = entry?;
        let path = entry.path();
        let kind = entry.file_type()?;
        if kind.is_dir() {
            parse_directory(root, &path, files)?;
        } else if path.extension().is_some_and(|extension| extension == "py")
            && (kind.is_file() || kind.is_symlink() && path.is_file())
        {
            let relative = path
                .strip_prefix(root)?
                .to_str()
                .with_context(|| format!("non-UTF-8 Python path: {}", path.display()))?;
            let source = fs::read_to_string(&path).with_context(|| path.display().to_string())?;
            files.insert(
                relative.to_owned(),
                ParsedFile::parse(relative, &source, Language::Python)
                    .with_context(|| path.display().to_string())?,
            );
        }
    }
    Ok(())
}

fn receiver_reason(receiver: &Receiver) -> Option<Reason> {
    match receiver {
        Receiver::Router { .. } => None,
        Receiver::Unproven { reason } => Some(*reason),
    }
}

#[test]
#[ignore = "requires PRISM_ROUTES_MEASURE_DIR and PRISM_ROUTES_MEASURE_OUT"]
fn measure_flask_source_facts() -> anyhow::Result<()> {
    let root = env::var_os("PRISM_ROUTES_MEASURE_DIR").context("PRISM_ROUTES_MEASURE_DIR")?;
    let output = env::var_os("PRISM_ROUTES_MEASURE_OUT").context("PRISM_ROUTES_MEASURE_OUT")?;
    let root = Path::new(&root);
    let mut files = BTreeMap::new();
    parse_directory(root, root, &mut files)?;
    anyhow::ensure!(!files.is_empty(), "no Python files in {}", root.display());

    let bindings = index_python(&files);
    let context = SetupContext::default();
    let events: Vec<_> = files
        .keys()
        .flat_map(|path| flask_facts(&bindings, path, &context))
        .collect();
    let refusals = bindings.import_refusals();
    let mut refusals_by_file = BTreeMap::<_, Vec<_>>::new();
    for refusal in &refusals {
        refusals_by_file
            .entry(refusal.file.as_str())
            .or_default()
            .push(refusal);
    }

    let mut out = BufWriter::new(fs::File::create(&output).with_context(|| {
        format!(
            "cannot create measurement output {}",
            Path::new(&output).display()
        )
    })?);
    let mut rules = 0;
    let mut mounts = 0;
    for event in &events {
        let (site, kind, receiver, child, callbacks) = match event {
            FlaskEvent::Rule(rule) => {
                rules += 1;
                (
                    &rule.route.site,
                    "R",
                    &rule.route.receiver,
                    None,
                    &rule.route.callbacks[..],
                )
            }
            FlaskEvent::Mount(mount) => {
                mounts += 1;
                (&mount.site, "M", &mount.parent, Some(&mount.child), &[][..])
            }
            _ => continue,
        };
        let callback_proved = child.is_none().then(|| {
            !callbacks.is_empty()
                && callbacks.iter().all(|callback| {
                    matches!(
                        callback.status,
                        CallbackStatus::Resolved { .. } | CallbackStatus::Anonymous { .. }
                    )
                })
        });
        let import_refusals: Vec<Value> = refusals_by_file
            .get(site.file.as_str())
            .into_iter()
            .flatten()
            .filter(|refusal| {
                refusal.start_byte < site.end_byte && site.start_byte < refusal.end_byte
            })
            .map(|refusal| {
                json!({
                    "reason": format!("{:?}", refusal.reason),
                    "start_byte": refusal.start_byte,
                    "end_byte": refusal.end_byte,
                })
            })
            .collect();
        serde_json::to_writer(
            &mut out,
            &json!({
                "file": site.file,
                "line": site.line,
                "start_byte": site.start_byte,
                "end_byte": site.end_byte,
                "kind": kind,
                "receiver_proved": receiver_reason(receiver).is_none(),
                "receiver_unproven_reason": receiver_reason(receiver),
                "child_proved": child.map(|child| receiver_reason(child).is_none()),
                "child_unproven_reason": child.and_then(receiver_reason),
                "callback_proved": callback_proved,
                "callbacks": callbacks,
                "import_refusals": import_refusals,
            }),
        )?;
        writeln!(out)?;
    }
    out.flush()?;
    println!(
        "{} Python files, {rules} rules, {mounts} mounts",
        files.len()
    );
    Ok(())
}
