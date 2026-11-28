//! A narrow positive proof that a disjoint member access has no accessor or
//! exotic receiver that could still affect the selected path.
use super::*;

pub(super) type PlainObjects<'a> = BTreeMap<usize, Option<BTreeMap<String, Node<'a>>>>;

pub(in super::super) fn plain_access<'a>(
    parsed: &'a ParsedFile,
    bindings: &FileBindings<'a>,
    root: Node<'a>,
    usage: &PathUse<'a>,
    skip: usize,
) -> bool {
    let mut value = Some(root);
    let mut objects = bindings
        .references(parsed)
        .literals
        .lock()
        .expect("literal shapes");
    for key in &usage.keys[skip..] {
        let Some(key) = key.as_ref().filter(|key| key.as_str() != "__proto__") else {
            return false;
        };
        let Some(object) = value
            .and_then(param_slot::peel)
            .filter(|node| node.kind() == "object")
        else {
            return false;
        };
        let Some(fields) = objects
            .entry(object.id())
            .or_insert_with(|| plain_object(parsed, object))
        else {
            return false;
        };
        value = fields.get(key).copied();
    }
    true
}

fn plain_object<'a>(
    parsed: &'a ParsedFile,
    object: Node<'a>,
) -> Option<BTreeMap<String, Node<'a>>> {
    param_slot::children(object)
        .map(|entry| {
            let (key, value) = match entry.kind() {
                "pair" => (
                    entry.child_by_field_name("key")?,
                    entry.child_by_field_name("value")?,
                ),
                "shorthand_property_identifier" => (entry, entry),
                _ => return None,
            };
            let key = param_slot::property_key(parsed, key)?;
            // Even a disjoint write on a custom prototype can execute a setter.
            (key != "__proto__").then_some((key, value))
        })
        .collect()
}
