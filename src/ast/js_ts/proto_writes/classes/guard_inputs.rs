use super::*;

pub(in super::super) struct FactBatch {
    pub(in super::super) facts: Vec<ProtoWrite>,
    pub(in super::super) guard_refusals: BTreeMap<&'static str, usize>,
}

pub(in super::super) fn facts<'a>(file: &'a ParsedFile, analysis: &Analysis<'a>) -> FactBatch {
    let root = file.tree.root_node();
    let mut work = vec![(root, root)];
    let mut targets = BTreeMap::new();
    while let Some((node, previous_owner)) = work.pop() {
        let owner = if is_owner(node) { node } else { previous_owner };
        let target = match node.kind() {
            "assignment_expression" | "augmented_assignment_expression" => field(node, "left"),
            "update_expression" => field(node, "argument"),
            "for_in_statement" if field(node, "kind").is_none() => field(node, "left"),
            _ => None,
        };
        if let Some(target) = target {
            let (_, members) = targets
                .entry((owner.start_byte(), owner.end_byte()))
                .or_insert((owner, Vec::new()));
            collect_targets(target, node, members);
        }
        if is_owner(node) {
            work.extend(
                super::super::eager_owner_inputs(node).map(|child| (child, previous_owner)),
            );
        }
        work.extend(
            children(node)
                .filter(|child| {
                    !super::super::parameter_decorator(*child, node)
                        && !(is_owner(node)
                            && matches!(child.kind(), "decorator" | "computed_property_name"))
                        && child.kind() != "implements_clause"
                })
                .map(|child| (child, owner)),
        );
    }
    let mut classifier = Classifier {
        file,
        analysis,
        receivers: RefCell::default(),
        file_work: Cell::new(0),
        target_work: Cell::new(0),
        proof_work: Cell::new(0),
    };
    let mut result = Vec::new();
    for (owner, mut members) in targets.into_values() {
        if analysis.skipped(owner) {
            continue;
        }
        members.sort_by_key(|(target, _)| target.start_byte());
        result.extend(
            members
                .into_iter()
                .filter_map(|(target, store)| classifier.classify(owner, target, store)),
        );
    }
    classifier.classify_guards(result)
}

fn collect_targets<'a>(target: Node<'a>, store: Node<'a>, out: &mut Vec<(Node<'a>, Node<'a>)>) {
    let target = peel(target);
    if matches!(target.kind(), "member_expression" | "subscript_expression") {
        out.push((target, store));
    } else {
        for part in pattern_parts(target) {
            collect_targets(part, store, out);
        }
    }
}

impl<'a> Classifier<'a, '_> {
    pub(super) fn classify_guards(&self, targets: Vec<(ProtoWrite, GuardInput<'a>)>) -> FactBatch {
        let mut batch = FactBatch {
            facts: Vec::with_capacity(targets.len()),
            guard_refusals: BTreeMap::new(),
        };
        // Guard work starts only after the last structural fact and operand
        // has been collected. Charge it to the same file budget.
        let natives = guards::NativeWrites::collect(self.file, self.analysis, |cost| {
            let used = self.file_work.get().saturating_add(cost);
            self.file_work.set(used);
            used <= FILE_WORK_LIMIT
        });
        for (mut fact, input) in targets {
            self.target_work.set(0);
            let decision = guards::classify(self.file, self.analysis, &input, &natives, |cost| {
                self.charge(cost)
            });
            fact.guard = decision.state;
            fact.guard_forms = decision.forms;
            // A refusal explains a fact that stays below sufficient; one a
            // later form discharged is not an unrecognised guard.
            if decision.state != GuardState::Sufficient {
                for (_, reason) in decision.refusals {
                    *batch.guard_refusals.entry(reason).or_default() += 1;
                }
            }
            batch.facts.push(fact);
        }
        batch
            .facts
            .sort_by_key(|write| (write.line, write.start_byte));
        batch
    }
}
