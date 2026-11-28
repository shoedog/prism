//! Enumerate the pre-accept-list routes, including those the product will refuse.

use super::*;

pub(super) type Reason = &'static str;
pub(super) type Target = (FunctionId, (usize, usize));

#[derive(Clone)]
pub(super) struct Hop<'a> {
    pub parameter: Parameter<'a>,
    pub slot: ParamSlot,
    pub call: &'a CallSite,
    pub node: Option<Node<'a>>,
    pub ground_failure: Option<Reason>,
    pub value: Option<value::Value<'a>>,
    pub relay: Option<Parameter<'a>>,
}

pub(super) struct Route<'a> {
    pub hops: Vec<Hop<'a>>,
    pub terminal: Result<Target, Reason>,
}

#[derive(Default)]
pub(super) struct Baseline<'a> {
    pub routes: Vec<Route<'a>>,
    pub targets: BTreeSet<FunctionId>,
}

impl<'a> Context<'a> {
    pub(super) fn baseline(&self, candidate: &Candidate<'a>) -> Baseline<'a> {
        let mut result = Baseline::default();
        self.baseline_slot(
            &candidate.parameter,
            &candidate.slot,
            &mut Vec::new(),
            &mut result,
        );
        result
    }

    fn baseline_slot(
        &self,
        parameter: &Parameter<'a>,
        slot: &ParamSlot,
        hops: &mut Vec<Hop<'a>>,
        result: &mut Baseline<'a>,
    ) {
        let callers = (|| {
            if hops.iter().any(|hop| &hop.slot == slot) {
                return Err("cycle");
            }
            if hops.len() == MAX_DEPTH {
                return Err("depth");
            }
            if !self.unique_owner(&slot.holder, parameter.owner) {
                return Err("holder_identity");
            }
            self.callers.get(&slot.holder).ok_or("no_caller")
        })();
        let callers = match callers {
            Ok(callers) => callers,
            Err(reason) => {
                result.record(hops, Err(reason));
                return;
            }
        };
        let (holder_file, _) = self
            .files
            .get_key_value(&slot.holder.file)
            .expect("indexed holder");
        for &call in callers {
            let parsed = &self.files[&call.caller.file];
            let node = call_node(parsed, call);
            hops.push(Hop {
                parameter: parameter.clone(),
                slot: slot.clone(),
                call,
                node,
                ground_failure: None,
                value: None,
                relay: None,
            });
            match self.ground(slot, parameter, call) {
                Err(reason) => {
                    hops.last_mut().unwrap().ground_failure = Some(reason);
                    result.record(hops, Err(reason));
                }
                Ok((caller, node)) => {
                    for arm in self.values_of(holder_file, parameter, caller, call, node) {
                        let hop = hops.last_mut().unwrap();
                        hop.value = arm.ok();
                        hop.relay = None;
                        match arm.and_then(|value| self.terminal(value)) {
                            Ok(identity::Terminal::Callable(target, span)) => {
                                result.record(hops, Ok((target, span)));
                            }
                            Ok(identity::Terminal::Parameter(file, parameter)) => {
                                hops.last_mut().unwrap().relay = Some(parameter.clone());
                                if let Some(next) = super::slot(&self.files[file], file, &parameter)
                                {
                                    self.baseline_slot(&parameter, &next, hops, result);
                                } else {
                                    result.record(hops, Err("holder_identity"));
                                }
                            }
                            Err(reason) => result.record(hops, Err(reason)),
                        }
                    }
                }
            }
            hops.pop();
        }
    }
}

impl<'a> Baseline<'a> {
    fn record(&mut self, hops: &[Hop<'a>], terminal: Result<Target, Reason>) {
        if let Ok((id, _)) = &terminal {
            self.targets.insert(id.clone());
        }
        if self.targets.len() > MAX_TARGETS {
            self.routes.clear();
            return;
        }
        self.routes.push(Route {
            hops: hops.to_vec(),
            terminal,
        });
    }
}
